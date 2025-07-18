
pub mod state;
pub mod config;
pub mod input;
pub use config::*;
pub use state::*;
pub use input::*;

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
use crate::stats::*;
use crate::asm::{ X64Assembler, X64AssemblerFixed, Emitter, Gpr, VectorGpr, };
use crate::asm::{ NOP6, NOP8 };
use crate::experiments::ExperimentArgs;
use crate::events::{ EventDesc, AsEventDesc, EventSet };

/// Type of a function eligible for measurement via [`PerfectHarness`].
/// The is the function signature used to call measured code from Rust.
pub type MeasuredFn = extern "C" fn(usize, usize) -> usize;

/// Type of the harness function associated with [`PerfectHarness`].
/// The is the function signature used to call the harness from Rust.
pub type HarnessFn = extern "C" fn(rdi: usize, rsi: usize, measured_fn: usize)
    -> usize;


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
            rng: rand::thread_rng(),
            harness_state: Box::new([0; 16]),
            harness_stack: Box::new(HarnessStack::new()),
            gpr_state: Box::new(GprState::new()),
            vgpr_state: Box::new(VectorGprState::new()),
        };
        res.emit();
        res
    }

    /// Print disassembly for the entire harness. 
    pub fn disas(&self) {
        self.assembler.disas(AssemblyOffset(0), None);
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
    fn emit(&mut self) {
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

        if let Some(ctr) = self.cfg.auto_rdpmc {
            unimplemented!();
            // Indirectly call the tested function
            dynasm!(self.assembler
                ; mov rcx, ctr as i32
                ; lfence
                ; rdpmc
                ; lfence

                // NOTE: This is included in the measurement
                ; mov rcx, QWORD state_ptr as _
                ; mov [rcx + 0x10], rax
                ; lfence
                ; sfence


                ; call r15
                ; lfence

                // NOTE: This is included in the measurement
                ; mov rcx, ctr as i32

                ; lfence
                ; rdpmc
                ; lfence

                ; mov rcx, QWORD state_ptr as _
                ; mov rcx, [rcx + 0x10]
                ; sub rax, rcx
            );

        } 
        else {
            // Indirectly call the tested function
            dynasm!(self.assembler
                ; call r15
                ; lfence
            );
        }

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
    }
}


impl PerfectHarness {
    /// Generate the config bits for the raw perf_event (Intel).
    pub fn make_perf_cfg_intel(event: u8, mask: u8) -> u64 { 
        let event_num = event as u64;
        let mask_num = mask as u64;
        (mask_num << 8) | event_num
    }

    /// Generate the config bits for the raw perf_event (AMD).
    pub fn make_perf_cfg_amd(event: u16, mask: u8) -> u64 {
        let event_num = event as u64 & 0b1111_1111_1111;
        let event_lo  = event_num & 0b0000_1111_1111;
        let event_hi  = (event_num & 0b1111_0000_0000) >> 8;
        let mask_num  = mask as u64;
        (event_hi << 32) | (mask_num << 8) | event_lo
    }

    /// Build a [`perf_event::Counter`] for controlling a PMC. 
    ///
    /// NOTE: The event select MSRs are different between Intel and AMD,
    /// so the bits passed through a raw 'perf' event will be different. 
    fn make_perf_cfg(platform: TargetPlatform, event: &EventDesc)
        -> Counter
    {
        match platform { 
            TargetPlatform::Zen2 | 
            TargetPlatform::Zen3 => {
                let cfg = Self::make_perf_cfg_amd(event.id(), event.mask());
                Builder::new().kind(Event::Raw(cfg)).build().unwrap()
            },
            TargetPlatform::Tremont => {
                let cfg = Self::make_perf_cfg_intel(event.id() as u8, event.mask());
                Builder::new().kind(Event::Raw(cfg)).build().unwrap()
            },
        }
    }
}

impl PerfectHarness {
    /// Generate a list of inputs to measured code.
    fn generate_inputs(rng: &mut ThreadRng, iters: usize, input: InputMethod) 
        -> Vec<(usize, usize)>
    {
        let mut inputs: Vec<(usize, usize)> = vec![(0, 0); iters];

        match input {
            InputMethod::Fixed(rdi, rsi) => {
                for val in inputs.iter_mut() {
                    *val = (rdi, rsi);
                }
            },
            InputMethod::Random(input_fn) => {
                for (idx, val) in inputs.iter_mut().enumerate() {
                    *val = input_fn(rng, idx);
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
}

impl PerfectHarness {
    /// Run the provided function a single time with the harness *without* 
    /// configuring performance counters. 
    pub fn call(&mut self, rdi: usize, rsi: usize, measured_fn: MeasuredFn) 
        -> usize
    { 
        let harness_fn = self.assembler.as_harness_fn();
        let res = harness_fn(rdi, rsi, measured_fn as usize);
        return res
    }

    /// Run and measure the provided function using a single PMC event. 
    pub fn measure_event(&mut self, 
        measured_fn: MeasuredFn,
        event: impl AsEventDesc,
        iters: usize,
        input: InputMethod,
    ) -> Result<MeasureResults, &str>
    {
        let edesc = event.as_desc();
        self.measure(measured_fn, &edesc, iters, input)
    }

    /// Run and measure the provided function using one or more PMC events. 
    ///
    /// FIXME: Ideally generate inputs *once* here, not in `.measure()`. 
    pub fn measure_events<E: AsEventDesc>(&mut self, 
        measured_fn: MeasuredFn,
        events: &EventSet<E>,
        iters: usize,
        input: InputMethod,
    ) -> Result<Vec<MeasureResults>, &str>
    {
        let mut results = Vec::new();

        for event in events.iter() { 
            let edesc = event.as_desc();
            let result = self.measure(
                measured_fn, &edesc, iters, input.clone(),
            ).unwrap();
            results.push(result);
        }
        Ok(results)
    }

    /// Run the provided function with the harness after configuring the 
    /// performance counters with the given event. 
    pub fn measure(&mut self,
        measured_fn: MeasuredFn,
        event: &EventDesc,
        iters: usize,
        input: InputMethod,
   ) -> Result<MeasureResults, &str>
    {
        let inputs = Self::generate_inputs(&mut self.rng, iters, input);
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

        // Configure the appropriate counter with the requested event
        let mut ctr = Self::make_perf_cfg(self.cfg.platform, &event);

        ctr.reset().unwrap();
        ctr.enable().unwrap();

        // NOTE: This is a critical loop. 
        // Ideally, measured code is totally decoupled from effects on the 
        // machine that might be imparted by the body of this loop. 
        // 
        // For each requested iteration, we need to:
        // - Load the inputs for this iteration
        // - Call the harness with the requested function and inputs
        // - Save the result from this iteration
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

        ctr.disable().unwrap();

        self.gpr_state.clear();
        self.vgpr_state.clear();

        Ok(MeasureResults {
            data: RawResults(results),
            event: event.clone(),
            gpr_dumps,
            vgpr_dumps,
            inputs: Some(inputs),
        })
    }
}

