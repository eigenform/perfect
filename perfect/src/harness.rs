
use std::collections::*;
use std::pin;
use std::os::fd::{ AsRawFd, FromRawFd };
use rand::rngs::ThreadRng;
use perf_event::{ Builder, Group, Counter };
use perf_event::events::*;
use perf_event::hooks::sys::bindings::perf_event_mmap_page;
use dynasmrt::{
    dynasm,
    DynasmApi,
    DynasmLabelApi,
    Assembler,
    AssemblyOffset,
    ExecutableBuffer,
    x64::X64Relocation
};
use crate::util;
use crate::asm::{ X64Assembler, X64AssemblerFixed, Emitter, Gpr, VectorGpr, };
use crate::asm::{ NOP6, NOP8 };

/// Type of a function eligible for measurement via [`PerfectHarness`].
pub type MeasuredFn = extern "C" fn(usize, usize) -> usize;

/// Type of the harness function associated with [`PerfectHarness`].
pub type HarnessFn = extern "C" fn(rdi: usize, rsi: usize, measured_fn: usize)
    -> usize;

/// Auto-implemented on function types that are suitable for generating
/// input to a measured function.
///
/// [PerfectHarness::measure_vary] expects a type like this for varying the
/// inputs on each iteration of a test. Returns a tuple `(usize, usize)` with
/// values passed to the measured function via RDI and RSI.
///
/// The arguments to this function are:
///
/// - A mutable reference to the harness' [`ThreadRng`]
/// - The current iteration/test index for the associated input
///
pub trait InputGenerator:
    Fn(&mut ThreadRng, usize) -> (usize, usize) {}
impl <F: Fn(&mut ThreadRng, usize) -> (usize, usize)>
    InputGenerator for F {}

/// Strategy used by [PerfectHarness] to compute the set of inputs to the
/// measured function across all test runs.
pub enum InputMethod<'a> {
    /// Fix the value of both arguments (RDI and RSI) across all test runs.
    Fixed(usize, usize),

    /// Provide a function/closure which computes the arguments (RDI and RSI)
    /// by using:
    /// - A mutable reference to the [`ThreadRng`] owned by the harness
    /// - The index of the current test run
    Random(&'static dyn Fn(&mut ThreadRng, usize) -> (usize, usize)),

    /// Provide a precomputed list of arguments (RDI and RSI).
    List(&'a Vec<(usize, usize)>),
}

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
    pub fn read_gpr(&self, gpr: Gpr) -> usize { self.0[gpr as usize] }
    pub fn rax(&self) -> usize { self.0[0] }
    pub fn rcx(&self) -> usize { self.0[1] }
    pub fn rdx(&self) -> usize { self.0[2] }
    pub fn rbx(&self) -> usize { self.0[3] }
    pub fn rsp(&self) -> usize { self.0[4] }
    pub fn rbp(&self) -> usize { self.0[5] }
    pub fn rsi(&self) -> usize { self.0[6] }
    pub fn rdi(&self) -> usize { self.0[7] }
    pub fn r8(&self)  -> usize { self.0[8] }
    pub fn r9(&self)  -> usize { self.0[9] }
    pub fn r10(&self) -> usize { self.0[10] }
    pub fn r11(&self) -> usize { self.0[11] }
    pub fn r12(&self) -> usize { self.0[12] }
    pub fn r13(&self) -> usize { self.0[13] }
    pub fn r14(&self) -> usize { self.0[14] }
    pub fn r15(&self) -> usize { self.0[15] }
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

/// Saved vector register state.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct VectorGprState(pub [[u64; 4]; 16]);
impl VectorGprState {
    pub fn new() -> Self { Self([[0; 4]; 16]) }
    pub fn clear(&mut self) { self.0 = [[0; 4]; 16] }
    pub fn read_vgpr(&self, vgpr: VectorGpr) -> [u64; 4] { self.0[vgpr as usize] }
    pub fn ymm0(&self)  -> [u64; 4] { self.0[0] }
    pub fn ymm1(&self)  -> [u64; 4] { self.0[1] }
    pub fn ymm2(&self)  -> [u64; 4] { self.0[2] }
    pub fn ymm3(&self)  -> [u64; 4] { self.0[3] }
    pub fn ymm4(&self)  -> [u64; 4] { self.0[4] }
    pub fn ymm5(&self)  -> [u64; 4] { self.0[5] }
    pub fn ymm6(&self)  -> [u64; 4] { self.0[6] }
    pub fn ymm7(&self)  -> [u64; 4] { self.0[7] }
    pub fn ymm8(&self)  -> [u64; 4] { self.0[8] }
    pub fn ymm9(&self)  -> [u64; 4] { self.0[9] }
    pub fn ymm10(&self) -> [u64; 4] { self.0[10] }
    pub fn ymm11(&self) -> [u64; 4] { self.0[11] }
    pub fn ymm12(&self) -> [u64; 4] { self.0[12] }
    pub fn ymm13(&self) -> [u64; 4] { self.0[13] }
    pub fn ymm14(&self) -> [u64; 4] { self.0[14] }
    pub fn ymm15(&self) -> [u64; 4] { self.0[15] }
}
impl std::fmt::Debug for VectorGprState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GprState")
         .field("ymm0", &self.0[0])
         .field("ymm1", &self.0[1])
         .field("ymm2", &self.0[2])
         .field("ymm3", &self.0[3])
         .field("ymm4", &self.0[4])
         .field("ymm5", &self.0[5])
         .field("ymm6", &self.0[6])
         .field("ymm7", &self.0[7])
         .field("ymm8",  &self.0[8])
         .field("ymm9",  &self.0[9])
         .field("ymm10", &self.0[10])
         .field("ymm11", &self.0[11])
         .field("ymm12", &self.0[12])
         .field("ymm13", &self.0[13])
         .field("ymm14", &self.0[14])
         .field("ymm15", &self.0[15])
         .finish()
    }
}

/// Results returned by [PerfectHarness::measure].
#[derive(Clone)]
pub struct MeasureResults {
    /// Set of observations from the performance counters
    pub data: Vec<usize>,

    /// The PMC event associated with the result data.
    pub event: u16,

    /// The PMC user mask associated with the result data.
    pub mask: u8,

    /// Set of recorded [integer] GPR states across all test runs
    pub gpr_dumps: Option<Vec<GprState>>,

    /// Set of recorded [vector] GPR states across all test runs
    pub vgpr_dumps: Option<Vec<VectorGprState>>,

    /// Set of inputs (from RDI and RSI) across all test runs.
    pub inputs: Option<Vec<(usize, usize)>>,
}
impl MeasureResults {
    /// Return the minimum observed value
    pub fn get_min(&self) -> usize { *self.data.iter().min().unwrap() }
    /// Return the maximum observed value
    pub fn get_max(&self) -> usize { *self.data.iter().max().unwrap() }

    /// Collate observations into buckets.
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

    pub fn count(&self, val: usize) -> usize { 
        self.data.iter().filter(|x| **x == val).count()
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

/// The target platform for generated code. 
#[derive(Clone, Copy, Debug)]
pub enum TargetPlatform {
    Zen2,
    Tremont,
}

/// A strategy used by [`PerfectHarness`] for zeroing out the integer 
/// general-purpose registers before entering measured code. 
#[derive(Clone, Copy)]
pub enum ZeroStrategy {
    None,
    XorIdiom,
    MovFromZero,
}

/// A strategy used by [`PerfectHarness`] for zeroing out the vector 
/// general-purpose registers before entering measured code. 
#[derive(Clone, Copy)]
pub enum ZeroStrategyFp {
    None,
    Vzeroall,
    XorIdiom,
    MovFromZero,
}

/// Configuration passed to [PerfectHarness::emit].
#[derive(Clone, Copy)]
pub struct HarnessConfig {
    /// The target platform
    platform: TargetPlatform,

    harness_addr: usize,
    harness_size: usize,

    /// Optionally dump the integer general-purpose registers before exiting
    /// from the harness. 
    dump_gpr: bool,

    /// Optionally dump the vector general-purpose registers before exiting
    /// from the harness. 
    dump_vgpr: bool,

    /// Optionally compare RDI to a constant value before entering measured
    /// code. 
    cmp_rdi: Option<i32>,

    /// Optionally pin the caller to a specific hardware thread. 
    pinned_core: Option<usize>,

    /// Optionally allocate a fixed memory region for use by measured code.
    arena_alloc: Option<(usize, usize)>, 

    /// Optionally [try to] flush the BTB. 
    flush_btb: Option<usize>,

    /// The strategy for zeroing integer general-purpose registers before 
    /// entering measured code.
    zero_strat: ZeroStrategy,

    /// The strategy for zeroing vector general-purpose registers before 
    /// entering measured code.
    zero_strat_fp: ZeroStrategyFp,
}

impl HarnessConfig {

    /// Default base address for the harness. 
    const DEFAULT_ADDR: usize = 0x0000_1337_0000_0000;

    /// Default allocation size for the harness (64MiB)
    const DEFAULT_SIZE: usize = 0x0000_0000_0400_0000;

    pub fn default_zen2() -> Self { 
        Self {
            pinned_core: Some(15),
            harness_addr: Self::DEFAULT_ADDR,
            harness_size: Self::DEFAULT_SIZE,
            arena_alloc: Some((0x0000_0000, 0x1000_0000)),
            dump_gpr: false,
            dump_vgpr: false,
            cmp_rdi: None,
            flush_btb: None,
            platform: TargetPlatform::Zen2,
            zero_strat: ZeroStrategy::MovFromZero,
            zero_strat_fp: ZeroStrategyFp::None,
        }
    }

    pub fn default_tremont() -> Self { 
        Self {
            pinned_core: Some(15),
            harness_addr: Self::DEFAULT_ADDR,
            harness_size: Self::DEFAULT_SIZE,
            arena_alloc: Some((0x0000_0000, 0x1000_0000)),
            dump_gpr: false,
            dump_vgpr: false,
            cmp_rdi: None,
            flush_btb: None,
            platform: TargetPlatform::Tremont,
            zero_strat: ZeroStrategy::MovFromZero,
            zero_strat_fp: ZeroStrategyFp::None,
        }
    }
}

impl HarnessConfig {
    pub fn harness_addr(mut self, addr: usize) -> Self { 
        self.harness_addr = addr;
        self
    }
    pub fn harness_size(mut self, size: usize) -> Self { 
        self.harness_size = size;
        self
    }

    pub fn dump_gpr(mut self, x: bool) -> Self {
        self.dump_gpr = x;
        self
    }

    pub fn dump_vgpr(mut self, x: bool) -> Self {
        self.dump_vgpr = x;
        self
    }

    pub fn pinned_core(mut self, x: Option<usize>) -> Self {
        self.pinned_core = x;
        self
    }

    pub fn arena_alloc(mut self, base: usize, len: usize) -> Self {
        self.arena_alloc = Some((base, len));
        self
    }

    pub fn cmp_rdi(mut self, x: i32) -> Self {
        self.cmp_rdi = Some(x);
        self
    }

    pub fn platform(mut self, x: TargetPlatform) -> Self {
        self.platform = x;
        self
    }

    pub fn flush_btb(mut self, x: usize) -> Self {
        self.flush_btb = Some(x);
        self
    }

    pub fn zero_strategy(mut self, x: ZeroStrategy) -> Self { 
        self.zero_strat = x;
        self
    }

    pub fn zero_strategy_fp(mut self, x: ZeroStrategyFp) -> Self { 
        self.zero_strat_fp = x;
        self
    }

}

impl HarnessConfig {
    /// Create a [PerfectHarness] using this configuration.
    pub fn emit(self) -> PerfectHarness {
        if let Some(pinned_core) = self.pinned_core {
            util::PerfectEnv::pin_to_core(pinned_core);
            println!("[*] Pinned to core {}", pinned_core);
        }
        if let Some((base, len)) = self.arena_alloc {
            let mmap_min_addr = util::PerfectEnv::procfs_mmap_min_addr();
            if base < mmap_min_addr {
                println!("[!] The harness arena is set to {:016x}, \
                    but vm.mmap_min_addr is set to {:016x}", 
                    base, mmap_min_addr);
                panic!("[!] Cannot mmap for harness arena at {:016x}", base);
            }
            let _ = util::PerfectEnv::mmap_fixed(base, len);
        }

        let mut res = PerfectHarness::new(self);
        res
    }
}

/// Harness used to configure PMCs and call into measured code.
///
/// **NOTE:** Since there's some overhead associated with setting up and 
/// managing all of the state associated with this structure: this may be too 
/// heavy-handed for certain cases.
///
/// See [PerfectHarness::emit] for more details about the binary interface
/// (ie. how input is passed from the harness to measured code, etc). 
///
/// Virtual Addressing
/// ==================
///
/// The actual harness code is implemented with [X64AssemblerFixed]. 
/// This is emitted during runtime when calling [PerfectHarness::new]. 
///
/// By default, we allocate 64MiB at `0x0000_1337_0000_0000` for emitting the 
/// harness code. Users are expected to be aware of this reservation when
/// using [X64AssemblerFixed] for emitting tests.
///
/// Handling PMCs
/// =============
///
/// The harness *starts* counting for a particular PMC by interacting with 
/// the 'perf' subsystem, but it does not read the counters by itself. 
/// Instead, measured code (provided by the user) is expected to use the 
/// `RDPMC` instruction for reading the counters at a particular point in 
/// time during a test.
///
/// Measured code is expected to return the number of observed events
/// (ie. after taking the difference between two uses of `RDPMC`).
/// See [Emitter::emit_rdpmc_start] and [Emitter::emit_rdpmc_end] for more.
///
pub struct PerfectHarness {

    // Backing allocation with emitted code implementing the harness.
    // Created after [PerfectHarness::emit] is called.
    //harness_buf: Option<ExecutableBuffer>,
    //harness_fn: Option<HarnessFn>,

    /// Fixed backing allocation for emitted code implementing the harness. 
    assembler: X64AssemblerFixed,

    /// Saved stack pointer (for exiting the harness). 
    pub harness_state: Box<[u64; 16]>,

    /// Scratchpad memory for JIT'ed code.
    pub harness_stack: Box<HarnessStack>,

    /// Scratchpad memory for saving GPR state when JIT'ed code exits.
    pub gpr_state: Box<GprState>,
    pub vgpr_state: Box<VectorGprState>,

    /// Harness configuration.
    cfg: HarnessConfig,

    /// Enable/disable PMC configuration when running a measured function.
    /// This is enabled by default. 
    ///
    /// When disabled, the harness does not configure PMCs before running
    /// the measured function. 
    pmc_use: bool,

    pub rng: ThreadRng,
}

impl PerfectHarness {
    fn new(cfg: HarnessConfig) -> Self {
        let mut harness_state = Box::new([0; 16]);
        let mut harness_stack = Box::new(HarnessStack::new());
        let mut gpr_state = Box::new(GprState::new());

        let assembler = X64AssemblerFixed::new(
            cfg.harness_addr, cfg.harness_size
        );
        let mut res = Self {
            assembler,
            cfg,
            pmc_use: true,
            rng: rand::thread_rng(),
            harness_state: Box::new([0; 16]),
            harness_stack: Box::new(HarnessStack::new()),
            gpr_state: Box::new(GprState::new()),
            vgpr_state: Box::new(VectorGprState::new()),
        };
        res.emit();
        res
    }

    /// Disable/enable the use of PMCs when calling [`PerfectHarness::measure`].
    pub fn set_pmc_use(&mut self, x: bool) {
        self.pmc_use = x;
    }

    /// Print disassembly for the entire harness. 
    pub fn disas(&self) {
        self.assembler.disas(AssemblyOffset(0), None);
        //if let Some(buf) = &self.harness_buf {
        //    crate::util::disas(&buf, AssemblyOffset(0), None);
        //}
    }

    /// Emit the actual harness function during runtime.
    ///
    /// Binary Interface
    /// ================
    ///
    /// On entry to the measured function:
    /// - RDI and RSI are passed thru from the harness
    /// - R15 clobbered with the address of the measured function
    /// - RSP set to the address of `harness_state`
    /// - All other integer GPRs are zeroed
    ///
    /// - Measured functions are expected to end with a return instruction.
    /// - Measured functions are expected to return a result in RAX.
    ///
    /// FIXME: You probably want to implement this with [X64AssemblerFixed] 
    /// instead of the default one. This means that the use of addresses 
    /// leading up to measured code is more likely to be deterministic 
    /// (which might matter if you're trying to, for instance, prepare some 
    /// branch predictor state before reaching measured code). 
    ///
    fn emit(&mut self) {
        //let mut harness = X64Assembler::new().unwrap();

        let state_ptr = self.harness_state.as_ptr();
        let stack_ptr = self.harness_stack.as_ptr();

        dynasm!(self.assembler
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

            // Save the stack pointer.
            // NOTE: Allocates for RAX. 
            ; mov rax, QWORD state_ptr as _
            ; mov [rax], rsp

            // Set the stack pointer
            // NOTE: Allocates for RSP. 
            ; mov rsp, QWORD stack_ptr as _
        );

        // Optionally zero most of the GPRs before we enter measured code:
        //
        //  - RSI and RDI are passed through as arguments
        //  - R15 is necessarily polluted (for the indirect call)
        //  - RSP has the harness stack pointer
        //
        // NOTE: In some implementations, zero idioms may not be completely
        // free. Instead of zeroing all registers, you might want to try just 
        // zeroing one of them and then renaming the rest (in an attempt to 
        // avoid allocating any physical registers).
        //
        match self.cfg.zero_strat {
            // Emit a zero idiom for each register we need to clear.
            ZeroStrategy::XorIdiom => {
                dynasm!(self.assembler
                    ; xor rax, rax
                    ; xor rcx, rcx
                    ; xor rdx, rdx
                    ; xor rbx, rbx
                    ; xor rbp, rbp
                    ; xor r8,  r8
                    ; xor r9,  r9
                    ; xor r10, r10
                    ; xor r11, r11
                    ; xor r12, r12
                    ; xor r13, r13
                    ; xor r14, r14
                );
            },

            // Emit a single zero idiom for one register, and then rename 
            // all of the other registers to it.
            ZeroStrategy::MovFromZero => {
                dynasm!(self.assembler
                    ; xor rax, rax
                    ; mov rcx, rax
                    ; mov rdx, rax
                    ; mov rbx, rax
                    ; mov rbp, rax
                    ; mov r8,  rax
                    ; mov r9,  rax
                    ; mov r10, rax
                    ; mov r11, rax
                    ; mov r12, rax
                    ; mov r13, rax
                    ; mov r14, rax
                );
            },
            // Do nothing
            ZeroStrategy::None => {},
        }

        match self.cfg.zero_strat_fp {
            ZeroStrategyFp::Vzeroall => {
                dynasm!(self.assembler
                    ; vzeroall
                );
            },
            ZeroStrategyFp::XorIdiom => {
                dynasm!(self.assembler
                    ; vpxor ymm0, ymm0, ymm0
                    ; vpxor ymm1, ymm1, ymm1
                    ; vpxor ymm2, ymm2, ymm2
                    ; vpxor ymm3, ymm3, ymm3
                    ; vpxor ymm4, ymm4, ymm4
                    ; vpxor ymm5, ymm5, ymm5
                    ; vpxor ymm6, ymm6, ymm6
                    ; vpxor ymm7, ymm7, ymm7
                    ; vpxor ymm8, ymm8, ymm8
                    ; vpxor ymm9, ymm9, ymm9
                    ; vpxor ymm10, ymm10, ymm10
                    ; vpxor ymm11, ymm11, ymm11
                    ; vpxor ymm12, ymm12, ymm12
                    ; vpxor ymm13, ymm13, ymm13
                    ; vpxor ymm14, ymm14, ymm14
                    ; vpxor ymm15, ymm15, ymm15
                );
            },
            ZeroStrategyFp::MovFromZero => {
                dynasm!(self.assembler
                    ; vpxor ymm0, ymm0, ymm0
                    ; vmovdqu ymm1, ymm0
                    ; vmovdqu ymm2, ymm0
                    ; vmovdqu ymm3, ymm0
                    ; vmovdqu ymm4, ymm0
                    ; vmovdqu ymm5, ymm0
                    ; vmovdqu ymm6, ymm0
                    ; vmovdqu ymm7, ymm0
                    ; vmovdqu ymm8, ymm0
                    ; vmovdqu ymm9, ymm0
                    ; vmovdqu ymm10, ymm0
                    ; vmovdqu ymm11, ymm0
                    ; vmovdqu ymm12, ymm0
                    ; vmovdqu ymm13, ymm0
                    ; vmovdqu ymm14, ymm0
                    ; vmovdqu ymm15, ymm0
                );
            },
            ZeroStrategyFp::None => {},
        }

        // Optionally attempt to flush the BTB with some number of 
        // unconditional branches before entering measured code. 
        // Clobbers RAX and flags.
        //
        // FIXME: Uhhh what are you trying to do here anyway? Redo this. 
        if let Some(num) = self.cfg.flush_btb {
            dynasm!(self.assembler
                ; .align 64
            );
            for _ in 0..num {
                dynasm!(self.assembler
                    ; jmp BYTE >lab
                    ; .bytes NOP8
                    ; .bytes NOP8
                    ; .bytes NOP8
                    ; .bytes NOP8
                    ; .bytes NOP8
                    ; .bytes NOP8
                    ; .bytes NOP8
                    ; .bytes NOP6
                    ; lab:
                );
            }
        }

        // Optionally use RDI to prepare the initial state of the flags
        // before entering measured code.
        if let Some(val) = self.cfg.cmp_rdi {
            dynasm!(self.assembler
                ; cmp rdi, val
            );
        }

        // Indirectly call the tested function
        dynasm!(self.assembler
            ; call r15
            ; lfence
        );

        // Optionally capture the GPRs after exiting measured code.
        if self.cfg.dump_gpr {
            dynasm!(self.assembler
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

        // Optionally dump vector registers after exiting measured code
        if self.cfg.dump_vgpr {
            dynasm!(self.assembler
                ; mov r15, QWORD self.vgpr_state.0.as_ptr() as _
                ; vmovupd [r15 + 0x00], ymm0
                ; vmovupd [r15 + 0x20], ymm1
                ; vmovupd [r15 + 0x40], ymm2
                ; vmovupd [r15 + 0x60], ymm3
                ; vmovupd [r15 + 0x80], ymm4
                ; vmovupd [r15 + 0xa0], ymm5
                ; vmovupd [r15 + 0xc0], ymm6
                ; vmovupd [r15 + 0xe0], ymm7
                ; vmovupd [r15 + 0x100], ymm8
                ; vmovupd [r15 + 0x120], ymm9
                ; vmovupd [r15 + 0x140], ymm10
                ; vmovupd [r15 + 0x160], ymm11
                ; vmovupd [r15 + 0x180], ymm12
                ; vmovupd [r15 + 0x1a0], ymm13
                ; vmovupd [r15 + 0x1c0], ymm14
                ; vmovupd [r15 + 0x1e0], ymm15
                ; sfence
            );
        }

        dynasm!(self.assembler
            // Restore the stack pointer
            ; mov rcx, QWORD state_ptr as _
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

        self.assembler.commit().unwrap();

        //let harness_buf = harness.finalize().unwrap();
        //let harness_fn: HarnessFn = unsafe {
        //    std::mem::transmute(harness_buf.ptr(AssemblyOffset(0)))
        //};
        //self.harness_buf = Some(harness_buf);
        //self.harness_fn  = Some(harness_fn);
    }
}


impl PerfectHarness {

    // NOTE: The kernel configures PMC counter 0 for counting cycles and uses
    // it for its own purposes (IIRC for interrupt timing?). This is annoying 
    // because most of the code here is written to read from counter 0,
    // and if you forget to boot with isolcpus=, all measurements using it 
    // will just return elapsed cycles. 
    //
    // It would be nice to detect when this is the case and automatically 
    // handle this in cases where you aren't using isolcpus=. 

    /// Resolve the actual index of the hardware counter being used by 'perf'.
    //unsafe fn resolve_hw_pmc_idx(ctr: &Counter) -> usize {
    //    let file = unsafe {
    //        std::fs::File::from_raw_fd(ctr.as_raw_fd())
    //    };
    //    let mmap = unsafe {
    //        memmap2::MmapOptions::new()
    //            .len(std::mem::size_of::<perf_event_mmap_page>())
    //            .map(&file)
    //            .unwrap()
    //    };
    //    let mmap_page = unsafe {
    //        &*(mmap.as_ptr() as *const perf_event_mmap_page)
    //    };
    //    let index = mmap_page.index;
    //    index as usize
    //}

    /// Generate the config bits for the raw perf_event (Intel).
    pub fn make_cfg_intel(event: u8, mask: u8) -> u64 { 
        let event_num = event as u64;
        let mask_num = mask as u64;
        (mask_num << 8) | event_num
    }

    /// Generate the config bits for the raw perf_event (AMD).
    pub fn make_cfg_amd(event: u16, mask: u8) -> u64 {
        let event_num = event as u64 & 0b1111_1111_1111;
        let event_lo  = event_num & 0b0000_1111_1111;
        let event_hi  = (event_num & 0b1111_0000_0000) >> 8;
        let mask_num  = mask as u64;
        (event_hi << 32) | (mask_num << 8) | event_lo
    }
}

impl PerfectHarness {

    /// Generate a list of inputs to measured code.
    fn generate_inputs(&mut self, iters: usize, input: InputMethod) 
        -> Vec<(usize, usize)> 
    {
        let mut inputs: Vec<(usize, usize)> = vec![(0, 0); iters];
        match input {
            InputMethod::Fixed(rsi, rdi) => {
                for val in inputs.iter_mut() {
                    *val = (rsi, rdi);
                }
            },
            InputMethod::Random(input_fn) => {
                for (idx, val) in inputs.iter_mut().enumerate() {
                    *val = input_fn(&mut self.rng, idx);
                }
            }
            InputMethod::List(data) => {
                assert!(data.len() >= iters,
                    "InputMethod::List must provide at least {} elements",
                    iters
                );
                inputs.copy_from_slice(&data);
            },
        }
        inputs
    }

    /// Run the provided function with the harness.
    pub fn call(&mut self, rdi: usize, rsi: usize, measured_fn: MeasuredFn) 
        -> usize
    { 
        let harness_fn = self.assembler.as_harness_fn();
        let res = harness_fn(rdi, rsi, measured_fn as usize);
        return res
    }

    /// Run and *measure* the provided function with the harness. 
    pub fn measure(&mut self,
        measured_fn: MeasuredFn,
        event: u16,
        mask: u8,
        iters: usize,
        input: InputMethod,
   ) -> Result<MeasureResults, &str>
    {
        let inputs = self.generate_inputs(iters, input);
        //let harness_fn = self.harness_fn.unwrap();
        let harness_fn = self.assembler.as_harness_fn();

        // Allocate for output data produced while running the harness
        let mut results = vec![0; iters];
        let mut gpr_dumps = if self.cfg.dump_gpr {
            Some(Vec::new()) 
        } else { 
            None 
        };
        let mut vgpr_dumps = if self.cfg.dump_vgpr { 
            Some(Vec::new()) 
        } else { 
            None 
        };

        // NOTE: The event select MSRs are different between Intel and AMD,
        // so the bits passed through a raw 'perf' event will be different. 
        let mut ctr = match self.cfg.platform { 
            TargetPlatform::Zen2 => {
                let cfg = Self::make_cfg_amd(event, mask);
                Builder::new()
                .kind(Event::Raw(cfg))
                .build().unwrap()
            },
            TargetPlatform::Tremont => {
                let cfg = Self::make_cfg_intel(event as u8, mask);
                Builder::new()
                .kind(Event::Raw(cfg))
                .build().unwrap()
            },
        };

        if self.pmc_use {
            ctr.reset().unwrap();
            ctr.enable().unwrap();
        }

        // For each requested iteration:
        // - Call the harness with the requested function and inputs
        // - Save the result value
        // - Optionally save the general-purpose register state
        for i in 0..iters {
            let (rdi, rsi) = inputs[i];
            let res = harness_fn(rdi, rsi, measured_fn as usize);
            results[i] = res;
            if let Some(data) = &mut gpr_dumps {
                data.push(*self.gpr_state);
            }
            if let Some(data) = &mut vgpr_dumps {
                data.push(*self.vgpr_state);
            }
        }

        if self.pmc_use {
            ctr.disable().unwrap();
        }

        self.gpr_state.clear();
        self.vgpr_state.clear();

        Ok(MeasureResults {
            data: results,
            event, mask,
            gpr_dumps, vgpr_dumps,
            inputs: Some(inputs),
        })
    }

}

// NOTE: It would be really nice to have some way of catching faults from 
// measured code ..

//static mut FAULT_INFO: *mut nix::libc::siginfo_t = std::ptr::null_mut();
//static mut RECOVERY_POINT: *mut nix::libc::ucontext_t = std::ptr::null_mut();
//static mut LAST_ERROR_SIGNAL: i32 = 0;
//extern "C" fn recover_from_hardware_error(
//    signal: i32,
//    info: *mut nix::libc::siginfo_t,
//    voidctx: *mut std::ffi::c_void,
//) {
//    unsafe { 
//        LAST_ERROR_SIGNAL = signal;
//        FAULT_INFO = info;
//        nix::libc::setcontext(RECOVERY_POINT);
//        unreachable!("uhhhh");
//    }
//}
//
//impl PerfectHarness {
//    pub fn register_signal_handler(&mut self) {
//        use nix::sys::signal;
//        use nix::sys::signal::Signal;
//        use nix::libc::{setcontext, getcontext};
//
//        // unblock signal
//        let mut sigset = signal::SigSet::empty();
//        sigset.add(signal::SIGILL);
//        signal::sigprocmask(signal::SigmaskHow::SIG_UNBLOCK, Some(&sigset), None)
//            .unwrap();
//
//        // install handler
//        unsafe { 
//            signal::sigaction(
//                signal::SIGILL, 
//                &signal::SigAction::new(
//                    signal::SigHandler::SigAction(recover_from_hardware_error),
//                    (signal::SaFlags::SA_NODEFER | signal::SaFlags::SA_SIGINFO),
//                    signal::SigSet::empty(),
//                )
//            ).unwrap();
//            LAST_ERROR_SIGNAL = 0;
//        }
//
//        // Set our recovery point
//        unsafe { 
//            RECOVERY_POINT = Box::leak(
//                Box::<nix::libc::ucontext_t>::new(std::mem::zeroed())
//            );
//            FAULT_INFO = Box::leak(
//                Box::<nix::libc::siginfo_t>::new(std::mem::zeroed())
//            );
//
//            nix::libc::getcontext(RECOVERY_POINT);
//        }
//        unsafe { 
//            if LAST_ERROR_SIGNAL != 0 {
//                println!("{:x?}", (*FAULT_INFO));
//                println!("{:x?}", (*FAULT_INFO).si_addr());
//                return;
//            }
//        }
//
//        // Do something dangerous
//        unsafe { 
//            core::arch::asm!("mov rax, #0xdead");
//            core::arch::asm!("mov rbx, #0xdead");
//            core::arch::asm!("mov rcx, #0xdead");
//            core::arch::asm!("mov rdx, #0xdead");
//            core::arch::asm!("mov rdi, #0xdead");
//            core::arch::asm!("mov rsi, #0xdead");
//            core::arch::asm!("ud2");
//        }
//
//
//        // restore default handler
//        unsafe {
//            signal::sigaction(
//                signal::SIGILL, 
//                &signal::SigAction::new(
//                    signal::SigHandler::SigDfl,
//                    signal::SaFlags::empty(),
//                    signal::SigSet::empty(),
//                )
//            ).unwrap();
//        }
//    }
//}

