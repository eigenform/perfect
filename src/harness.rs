
use std::collections::*;
use std::os::fd::{ AsRawFd, FromRawFd };
use rand::rngs::ThreadRng;
use perf_event::{ Builder, Group, Counter };
use perf_event::events::*;
use perf_event::hooks::sys::bindings::perf_event_mmap_page;
use dynasmrt::{
    dynasm, 
    DynasmApi, 
    Assembler, 
    AssemblyOffset, 
    ExecutableBuffer, 
    x64::X64Relocation
};
use crate::util;

/// Type of a function eligible for measurement via [PerfectHarness]. 
pub type MeasuredFn = fn(usize, usize) -> usize;

/// Type of the harness function emitted by [PerfectHarness].
pub type HarnessFn = fn(rdi: usize, rsi: usize, measured_fn: usize) -> usize;

/// Auto-implemented on function types that are suitable for generating 
/// input to a measured function. 
///
/// [PerfectHarness::measure_vary] expects a type like this for varying the
/// inputs on each iteration of a test. Returns a tuple `(usize, usize)` with
/// values passed to the measured function via RDI and RSI.
///
/// The arguments to this function are:
///
/// - A mutable reference to the harness [ThreadRng]
/// - The current iteration/test index
///
pub trait InputGenerator: 
    Fn(&mut ThreadRng, usize) -> (usize, usize) {}
impl <F: Fn(&mut ThreadRng, usize) -> (usize, usize)> 
    InputGenerator for F {}


/// Harness stack layout. 
#[repr(C, align(0x10000))]
pub struct HarnessStack { data: [u8; 0x8000], }
impl HarnessStack {
    pub fn new() -> Self { Self { data: [0; 0x8000] } }
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { self.data.as_ptr().offset(0x3000) }
    }
}

/// Saved general-purpose register state. 
#[repr(C)]
#[derive(Copy, Clone)]
pub struct GprState(pub [usize; 16]);
impl GprState {
    pub fn new() -> Self { Self([0; 16]) }
    pub fn clear(&mut self) { self.0 = [0; 16]; }
}
impl std::fmt::Debug for GprState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GprState")
         .field("rax", &self.0[0])
         .field("rcx", &self.0[1])
         .field("rdx", &self.0[2])
         .field("rbx", &self.0[3])
         .field("rsp", &self.0[4])
         .field("rbp", &self.0[5])
         .field("rsi", &self.0[6])
         .field("rdi", &self.0[7])
         .field("r8",  &self.0[8])
         .field("r9",  &self.0[9])
         .field("r10", &self.0[10])
         .field("r11", &self.0[11])
         .field("r12", &self.0[12])
         .field("r13", &self.0[13])
         .field("r14", &self.0[14])
         .field("r15", &self.0[15])
         .finish()
    }
}

enum InputStrategy {
    Fixed((usize, usize)),
    Variable(Box<dyn InputGenerator>),
}

struct MeasureContext { 
    results: Vec<usize>,
    event: u16,
    mask: u8,
    cfg: u64,
    gpr_dumps: Option<Vec<GprState>>,
    cpu: usize,

    input_strategy: InputStrategy,
    inputs: Vec<(usize, usize)>,
}

/// Results returned by [PerfectHarness::measure]. 
pub struct MeasureResults {
    /// Observations from the performance counters
    pub data: Vec<usize>,
    /// The PMC event used during measurement
    pub event: u16,
    /// The PMC user mask used during measurement
    pub mask: u8,

    /// Set of recorded GPR states across all test runs
    pub gpr_dumps: Option<Vec<GprState>>,

    /// Inputs (RDI and RSI) across all test runs.
    pub inputs: Option<Vec<(usize, usize)>>,
}
impl MeasureResults {
    /// Return the minimum observed value
    pub fn get_min(&self) -> usize { *self.data.iter().min().unwrap() }
    /// Return the maximum observed value
    pub fn get_max(&self) -> usize { *self.data.iter().max().unwrap() }

    /// Collate observations into a distribution.
    ///
    /// The resulting map is keyed by the observed value and records the 
    /// number of times each value was observed. 
    pub fn get_distribution(&self) -> BTreeMap<usize, usize> {
        let mut dist = BTreeMap::new();
        for r in self.data.iter() {
            if let Some(cnt) = dist.get_mut(r) {
                *cnt += 1;
            } else {
                dist.insert(*r, 1);
            }
        }
        dist
    }

    pub fn find(&self, val: usize) -> Vec<usize> {
        self.data.iter().enumerate().filter(|(idx, x)| **x == val)
            .map(|(idx, x)| idx).collect()
    }
    pub fn filter(&self, mut f: impl FnMut(usize) -> bool) -> Vec<usize> {
        self.data.iter().enumerate().filter(|(idx, x)| f(**x))
            .map(|(idx, x)| idx).collect()
    }


}




/// Configuration passed to [PerfectHarness::emit]. 
#[derive(Clone, Copy)]
pub struct HarnessConfig {
    dump_gpr: bool,
    cmp_rdi: Option<i32>,
}
impl Default for HarnessConfig {
    fn default() -> Self {
        Self { 
            dump_gpr: false,
            cmp_rdi: None,
        }
    }
}
impl HarnessConfig {
    /// Dump integer GPRs after each exit from measured code. 
    pub fn dump_gpr(mut self, x: bool) -> Self {
        self.dump_gpr = x;
        self
    }

    /// Compare RDI to some value before entering measured code. 
    pub fn cmp_rdi(mut self, x: i32) -> Self {
        self.cmp_rdi = Some(x);
        self
    }
}
impl HarnessConfig {
    /// Create and emit a [PerfectHarness] using this configuration.
    pub fn emit(self) -> PerfectHarness {
        let mut res = PerfectHarness::new(self);
        res
    }
}


/// Harness used to configure PMCs and call into measured code. 
///
/// See [PerfectHarness::emit] for more details about the binary interface. 
///
/// Handling PMCs
/// =============
///
/// The harness *starts* counting for a particular PMC, but it does not
/// read the counters by itself. Instead, measured code is expected to use 
/// the `RDPMC` instruction for reading the counters at a particular point 
/// in time. 
///
/// Measured code is expected to return the number of observed events 
/// (ie. after taking the difference between two uses of `RDPMC`).
/// See [Emitter::emit_rdpmc_start] and [Emitter::emit_rdpmc_end] for more.
///
pub struct PerfectHarness {

    // Backing allocation with emitted code implementing the harness.
    // Created after [PerfectHarness::emit] is called.
    harness_buf: Option<ExecutableBuffer>,
    harness_fn: Option<HarnessFn>,

    /// Saved stack pointer.
    pub harness_state: Box<[u64; 16]>,

    /// Scratchpad memory for JIT'ed code.
    pub harness_stack: Box<HarnessStack>,

    /// Scratchpad memory for saving GPR state when JIT'ed code exits.
    pub gpr_state: Box<GprState>,

    cfg: HarnessConfig,
    rng: ThreadRng,
}

impl PerfectHarness {
    pub fn new(cfg: HarnessConfig) -> Self { 
        //let mut harness = Assembler::<X64Relocation>::new().unwrap();
        let mut harness_state = Box::new([0; 16]);
        let mut harness_stack = Box::new(HarnessStack::new());
        let mut gpr_state = Box::new(GprState::new());

        let mut res = Self { 
            //harness: Some(harness),
            harness_buf: None,
            harness_fn: None,
            cfg,
            rng: rand::thread_rng(),
            harness_state: Box::new([0; 16]),
            harness_stack: Box::new(HarnessStack::new()),
            gpr_state: Box::new(GprState::new()),
        };
        res.emit();
        res
    }

    /// Emit the harness. 
    ///
    /// Binary Interface
    /// ================
    ///
    /// On entry to the measured function:
    /// - RDI and RSI are passed thru from the harness
    /// - R15 clobbered with the address of the measured function
    /// - RSP set to the address of `harness_state`
    /// - All other integer GPRs are zeroed
    fn emit(&mut self) {
        //let mut harness = self.harness.take().unwrap();
        let mut harness = Assembler::<X64Relocation>::new().unwrap();

        dynasm!(harness
            ; .arch     x64

            // Save nonvolatile registers 
            ; push      rbp
            ; push      rbx
            ; push      rdi
            ; push      rsi
            ; push      r12
            ; push      r13
            ; push      r14
            ; push      r15

            // Pointer to measured code
            ; mov r15, rdx

            // Save the stack pointer
            ; mov rax, QWORD self.harness_state.as_ptr() as _
            ; mov [rax], rsp

            // Set the stack pointer
            ; mov rsp, QWORD self.harness_stack.as_ptr() as _

            // Zero most of the GPRs before we enter measured code:
            //  - RSI and RDI are passed through as arguments
            //  - R15 is necessarily polluted (for the indirect call)

            ; xor rax, rax
            ; xor rcx, rcx
            ; xor rdx, rdx
            ; xor rbx, rbx
            //; xor rsi, rsi
            //; xor rdi, rdi
            ; xor rbp, rbp
            ; xor  r8, r8
            ; xor  r9, r9
            ; xor r10, r10
            ; xor r11, r11
            ; xor r12, r12
            ; xor r13, r13
            ; xor r14, r14
            //; xor r15, r15
        );

        // Optionally use RDI to prepare the initial state of the flags
        // before entering measured code. 
        if let Some(val) = self.cfg.cmp_rdi {
            dynasm!(harness
                ; cmp rdi, val
            );
        }

        // Indirectly call the tested function
        dynasm!(harness
            ; call r15
            ; lfence
        );

        // Optionally capture the GPRs after exiting measured code. 
        if self.cfg.dump_gpr {
            dynasm!(harness
                ; mov r15, QWORD self.gpr_state.0.as_ptr() as _
                ; mov [r15 + 0x00], rax
                ; mov [r15 + 0x08], rcx
                ; mov [r15 + 0x10], rdx
                ; mov [r15 + 0x18], rbx
                ; mov [r15 + 0x20], rsp
                ; mov [r15 + 0x28], rbp
                ; mov [r15 + 0x30], rsi
                ; mov [r15 + 0x38], rdi
                ; mov [r15 + 0x40], r8
                ; mov [r15 + 0x48], r9
                ; mov [r15 + 0x50], r10
                ; mov [r15 + 0x58], r11
                ; mov [r15 + 0x60], r12
                ; mov [r15 + 0x68], r13
                ; mov [r15 + 0x70], r14
                ; mov [r15 + 0x78], r15
                ; sfence
            );
        }

        dynasm!(harness
            // Restore the stack pointer
            ; mov rcx, QWORD self.harness_state.as_ptr() as _
            ; mov rsp, [rcx]

            ; pop r15
            ; pop r14
            ; pop r13
            ; pop r12
            ; pop rsi
            ; pop rdi
            ; pop rbx
            ; pop rbp
            ; ret
        );

        let harness_buf = harness.finalize().unwrap();
        let harness_fn: HarnessFn = unsafe { 
            std::mem::transmute(harness_buf.ptr(AssemblyOffset(0)))
        };
        self.harness_buf = Some(harness_buf);
        self.harness_fn  = Some(harness_fn);
    }
}


impl PerfectHarness {

    /// Resolve the actual index of the hardware counter being used by 'perf'.
    unsafe fn resolve_hw_pmc_idx(ctr: &Counter) -> usize {
        let file = unsafe { 
            std::fs::File::from_raw_fd(ctr.as_raw_fd())
        };
        let mmap = unsafe { 
            memmap2::MmapOptions::new()
                .len(std::mem::size_of::<perf_event_mmap_page>())
                .map(&file)
                .unwrap()
        };
        let mmap_page = unsafe { 
            &*(mmap.as_ptr() as *const perf_event_mmap_page)
        };
        let index = mmap_page.index;
        index as usize
    }


    /// Generate the config bits for the raw perf_event. 
    fn make_cfg(event: u16, mask: u8) -> u64 {
        let event_num = event as u64 & 0b1111_1111_1111;
        let event_lo  = event_num & 0b0000_1111_1111;
        let event_hi  = (event_num & 0b1111_0000_0000) >> 8;
        let mask_num  = mask as u64;
        (event_hi << 32) | (mask_num << 8) | event_lo
    }

    // NOTE: This *assumes* the user has called [PerfectHarness::emit].
    fn setup_measure_context(&mut self, event: u16, mask: u8, 
        input_strategy: InputStrategy) 
        -> MeasureContext
    {
        let this_cpu = nix::sched::sched_getcpu().unwrap();
        let mut results = Vec::new();
        let mut gpr_dumps = if self.cfg.dump_gpr { 
            Some(Vec::new()) 
        } else { 
            None 
        };
        let cfg = Self::make_cfg(event, mask);
        MeasureContext {
            results,
            event,
            mask,
            cfg,
            gpr_dumps,
            cpu: this_cpu,
            input_strategy,
            inputs: Vec::new(),
        }
    }
}

impl PerfectHarness {
    pub fn measure(&mut self, 
        measured_fn: MeasuredFn, 
        event: u16,
        mask: u8,
        iters: usize, 
        rdi: usize,
        rsi: usize
    ) -> Result<MeasureResults, &str>
    {
        let mut ctx = self.setup_measure_context(
            event, mask, InputStrategy::Fixed((rdi, rsi))
        );

        let harness_fn = self.harness_fn.unwrap();
        let mut ctr = Builder::new()
            .kind(Event::Raw(ctx.cfg))
            .build().unwrap();

        for i in 0..iters {
            self.gpr_state.clear();
            ctr.enable().unwrap();
            let res = harness_fn(rdi, rsi, measured_fn as usize);
            ctr.disable().unwrap();
            ctr.reset().unwrap();
            ctx.results.push(res);
            if let Some(ref mut dumps) = ctx.gpr_dumps {
                dumps.push(*self.gpr_state);
            }
        }

        Ok(MeasureResults {
            data: ctx.results,
            event, mask,
            gpr_dumps: ctx.gpr_dumps,
            inputs: None,
        })
    }

    pub fn measure_vary(&mut self, 
        measured_fn: MeasuredFn,
        event: u16, mask: u8, iters: usize, 
        input_fn: impl InputGenerator + Copy + 'static,
    ) -> Result<MeasureResults, &str>
    {
        let harness_fn = self.harness_fn.unwrap();
        let mut ctx = self.setup_measure_context(
            event, mask, InputStrategy::Variable(Box::new(input_fn)),
        );
        let mut ctr = Builder::new()
            .kind(Event::Raw(ctx.cfg))
            .build().unwrap();

        for i in 0..iters {
            let (rdi, rsi) = input_fn(&mut self.rng, i);
            ctx.inputs.push((rdi, rsi));

            self.gpr_state.clear();
            ctr.reset().unwrap();
            ctr.enable().unwrap();
            let res = harness_fn(rdi, rsi, measured_fn as usize);
            ctr.disable().unwrap();
            ctx.results.push(res);
            if let Some(ref mut dumps) = ctx.gpr_dumps {
                dumps.push(*self.gpr_state);
            }
        }

        Ok(MeasureResults {
            data: ctx.results,
            event,
            mask,
            gpr_dumps: ctx.gpr_dumps,
            inputs: Some(ctx.inputs),
        })
    }
}


