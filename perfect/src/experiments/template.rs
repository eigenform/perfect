/// Templates for emitting certain experiments. 

use crate::experiments::*;
use crate::harness::TargetPlatform;

#[derive(Clone, Copy, Debug)]
pub enum RdpmcStrategy { 
    /// Save initial RDPMC results in a general-purpose register
    Gpr(Gpr),

    /// Save initial RDPMC results to some statically-known memory address.
    MemStatic(i32),
}

#[derive(Clone, Copy, Debug)]
pub enum MispredictionStrategy {
    /// Mispredict a return instruction. 
    Return,

    /// Mispredict an indirect jump instruction. 
    IndirectJmp
}

/// Options passed to some [MispredictedReturnTemplate].
#[derive(Clone, Copy, Debug)]
pub struct MispredictedReturnOptions<I> { 
    /// RDPMC counter index
    pub ctr_idx: i32,

    /// Target platform
    pub platform: TargetPlatform,

    /// Strategy for using RDPMC
    pub rdpmc_strat: RdpmcStrategy,

    /// Strategy for mispredicting a branch
    pub misprediction_strat: MispredictionStrategy,

    /// Pad window to a 64B boundary
    pub pad_body: bool,

    /// Try to release any hanging physical registers after the initial RDPMC
    pub free_pregs: bool,

    /// Mark the end of the body with a speculative 'PREFETCH' instruction
    pub prefetch_marker: Option<Gpr>,

    /// Mark the end of the body with a speculative 'FNOP' instruction
    pub fnop_marker: bool,

    /// Emit LFENCE immediately after the body
    pub explicit_lfence: bool,

    /// Optional prologue emitter [emitted before measurement]
    pub prologue_fn: Option<fn(&mut X64Assembler, I)>,

    /// Optional prologue emitter [emitted immediately *after* measurement]
    pub post_prologue_fn: Option<fn(&mut X64Assembler, I)>,

    /// Optional *speculative* epilogue emitter [emitted immediately after
    /// speculative user code].
    pub speculative_epilogue_fn: Option<fn(&mut X64Assembler, I)>,

}
impl <I> MispredictedReturnOptions<I> {
    pub fn zen2_defaults() -> Self { 
        Self { 
            ctr_idx: 0,
            platform: TargetPlatform::Zen2,
            pad_body: true,
            misprediction_strat: MispredictionStrategy::Return,
            explicit_lfence: false,
            prefetch_marker: None,
            fnop_marker: false,
            free_pregs: false,
            prologue_fn: None,
            post_prologue_fn: None,
            speculative_epilogue_fn: None,
            rdpmc_strat: RdpmcStrategy::Gpr(Gpr::R15),
        }
    }

    pub fn tremont_defaults() -> Self { 
        Self { 
            ctr_idx: 0,
            platform: TargetPlatform::Tremont,
            pad_body: true,
            misprediction_strat: MispredictionStrategy::Return,
            explicit_lfence: false,
            prefetch_marker: None,
            fnop_marker: false,
            free_pregs: false,
            prologue_fn: None,
            post_prologue_fn: None,
            speculative_epilogue_fn: None,
            rdpmc_strat: RdpmcStrategy::Gpr(Gpr::R15),
        }
    }

    pub fn ctr_idx(mut self, x: i32) -> Self { 
        self.ctr_idx = x;
        self
    }

    pub fn pad_body(mut self, x: bool) -> Self { 
        self.pad_body = x;
        self
    }

    pub fn misprediction_strat(mut self, x: MispredictionStrategy) -> Self { 
        self.misprediction_strat = x;
        self
    }

    pub fn rdpmc_strat(mut self, x: RdpmcStrategy) -> Self { 
        self.rdpmc_strat = x;
        self
    }

    pub fn explicit_lfence(mut self, x: bool) -> Self { 
        self.explicit_lfence = x;
        self
    }

    pub fn prefetch_marker(mut self, x: Option<Gpr>) -> Self { 
        self.prefetch_marker = x;
        self
    }
    pub fn fnop_marker(mut self, x: bool) -> Self { 
        self.fnop_marker = x;
        self
    }


    pub fn free_pregs(mut self, x: bool) -> Self { 
        self.free_pregs = x;
        self
    }

    pub fn prologue_fn(mut self, x: Option<fn(&mut X64Assembler, I)>) -> Self { 
        self.prologue_fn = x;
        self
    }
    pub fn post_prologue_fn(mut self, x: Option<fn(&mut X64Assembler, I)>) 
        -> Self
    { 
        self.post_prologue_fn = x;
        self
    }


    pub fn speculative_epilogue_fn(mut self, 
        x: Option<fn(&mut X64Assembler, I)>) -> Self 
    { 
        self.speculative_epilogue_fn = x;
        self
    }

}


/// Template for emitting code in the shadow of a costly mispredicted branch.
///
/// This is a useful tool for writing microbenchmarks - especially when trying
/// to measure the capacity of different microarchitectural resources. 
/// Historically, most examples of this use very slow loads for generating 
/// latency.
///
/// In this case, we generate substantial latency by causing a misprediction 
/// and deferring resolution of the target address for as long as possible
/// by creating a dependency on a very slow load. 
///
/// Notes on Physical Register Use
/// ==============================
///
/// The use of RDPMC requires two allocations: 
/// - For the counter index (in RCX)
/// - For the result of RDPMC (in RAX)
///
/// You can recover these *after* the first RDPMC with moves from a zeroed
/// register. You can also save an allocation by writing the RDPMC result 
/// to memory instead of keeping it in a GPR. 
///
/// Our strategy with RET requires two allocations: 
/// - For the value that will be written over the saved return address 
/// - For RSP, which needs to be nonzero (since we're using CALL/RET)
///
///
pub trait MispredictedReturnTemplate<I: Copy> {
    const ARENA_SAVED_RDPMC: i32 = 0x0001_0280;
    const ARENA_SAVED_RSP: i32   = 0x0001_0380;
    const ARENA_INDIR_TGT: i32   = 0x0001_0180;

    fn emit_gadget_indirect(
        f: &mut X64Assembler,
        opts: MispredictedReturnOptions<I>,
        input: I,
        user_fn: fn(&mut X64Assembler, I), 
    )
    {
        let lab = f.new_dynamic_label();

        // Save the stack pointer
        dynasm!(f ; mov [Self::ARENA_SAVED_RSP], rsp);

        // Flush the BTB
        match opts.platform {
            TargetPlatform::Zen2 => {
                f.emit_flush_btb(0x4000);
            },
            TargetPlatform::Tremont => {
                unimplemented!();
            },
        }

        // Write the indirect branch target *through* to memory somewhere.
        match opts.platform {
            TargetPlatform::Zen2 => {
                dynasm!(f
                    ; lea r15, [=>lab]
                    ; movnti [Self::ARENA_INDIR_TGT], r15
                    ; sfence
                );
            },
            TargetPlatform::Tremont => unimplemented!(),
        }

        if opts.pad_body {
            dynasm!(f
                ; .align 64
                ; .bytes NOP8
                ; .bytes NOP8
                ; .bytes NOP8
                ; .bytes NOP8

                ; .bytes NOP8
                ; .bytes NOP8
                ; .bytes NOP8
                ; lfence
                ; jmp QWORD [Self::ARENA_INDIR_TGT] // ???
            );
        } 
        else { 
            dynasm!(f
                ; lfence
                ; jmp QWORD [Self::ARENA_INDIR_TGT]
            );
        }

        user_fn(f, input);

        f.emit_nop_sled(4096);
        f.emit_lfence();
        dynasm!(f
            ; .align 64
            ; =>lab
        );

        dynasm!(f
            ; lfence
            ; mov rcx, opts.ctr_idx
            ; lfence
            ; rdpmc
            ; lfence
            ; mov rbx, [Self::ARENA_SAVED_RDPMC]
            ; sub rax, rbx
        );

        // Restore the stack pointer
        dynasm!(f ; mov rsp, [Self::ARENA_SAVED_RSP]);

        f.emit_ret();

    }

    fn emit_gadget_ret(
        f: &mut X64Assembler,
        opts: MispredictedReturnOptions<I>,
        input: I,
        user_fn: fn(&mut X64Assembler, I), 
    )
    {
        let done_label = f.new_dynamic_label();

        // Optionally emit a prologue before the gadget
        if let Some(fun) = opts.post_prologue_fn {
            fun(f, input);
        }

        // Optionally add padding so that the instruction *after* the CALL 
        // begins on a 64-byte boundary.
        if opts.pad_body {
            dynasm!(f
                ; lea r14, [=>done_label]
                ; .align 64
                ; .bytes NOP8
                ; .bytes NOP8
                ; .bytes NOP8
                ; .bytes NOP8

                ; .bytes NOP8
                ; .bytes NOP8
                ; .bytes NOP8
                ; lfence      // 3 bytes
                ; call ->func // 5 bytes
            );
        } 
        else { 
            dynasm!(f
                ; lea r14, [=>done_label]
                ; lfence
                ; call ->func
            );
        }

        // Emit all of the instructions that are going to be part of the 
        // incorrectly speculated path. 
        user_fn(f, input);

        if let Some(fun) = opts.speculative_epilogue_fn {
            fun(f, input);
        }

        // Optionally emit a marker with the FNOP instruction. 
        if opts.fnop_marker {
            dynasm!(f; fnop);
        }
        // Optionally emit a marker with the PREFETCH instruction. 
        if let Some(gpr) = opts.prefetch_marker {
            dynasm!(f; prefetch [Rq(gpr as u8)]);
        }

        // Optionally emit a speculative LFENCE after user code.
        if opts.explicit_lfence {
            dynasm!(f; lfence);
        }

        // Emit padding NOPs to prevent speculative dispatch from reaching 
        // into the function we called. 
        f.emit_nop_sled(4096);
        f.emit_lfence();


        // Mispredict and defer resolution of the actual return address. 
        match opts.platform {
            // MOVNTI has more than enough latency on Zen2.
            TargetPlatform::Zen2 => {
               dynasm!(f
                    ; .align 64
                    ; ->func:
                    ; movnti [rsp], r14
                    ; ret
                    ; lfence
                    ; .align 64
                    ; =>done_label
                );
            },

            // On Tremont [and other newer Intel machines?], non-temporal hints
            // don't seem sufficient to create massive latency like on Zen2. 
            // The MOVDIRI instruction seems like a good alternative. 
            TargetPlatform::Tremont => {
               dynasm!(f
                    ; .align 64
                    ; ->func:
                    //; movdiri [rsp], r14
                    ; .bytes &[0x4c, 0x0f, 0x38, 0xf9, 0x34, 0x24]
                    ; ret
                    ; lfence
                    ; .align 64
                    ; =>done_label
                );
            },
        }
    }


    fn emit(
        opts: MispredictedReturnOptions<I>,
        input: I,
        user_fn: fn(&mut X64Assembler, I), 
    ) -> X64Assembler 
    {
        let mut f = X64Assembler::new().unwrap();

        // Optionally emit some prologue before the measurement starts. 
        if let Some(func) = opts.prologue_fn { func(&mut f, input); }

        // NOTE: These necessarily allocate three registers (RCX for the 
        // counter index, RAX/RDX for the result of rdpmc).
        match opts.rdpmc_strat {
            RdpmcStrategy::Gpr(reg) => {
                f.emit_rdpmc_start(opts.ctr_idx, reg as _);
            },
            RdpmcStrategy::MemStatic(addr) => {
                f.emit_rdpmc_to_addr(opts.ctr_idx, addr);
            },
        }

        // Optionally try to recover physical registers *after* we use RDPMC.
        // NOTE: RSP cannot be recovered since we depend on CALL/RET.
        // NOTE: You probably don't have to repeat this multiple times
        // FIXME: This doesn't account for RdpmcStrategy::Gpr
        if opts.free_pregs {
            for _ in 0..1 {

                dynasm!(f
                    ; mov rax, 0
                    ; mov rbx, 0
                    ; mov rcx, 0
                    ; mov rdx, 0
                    ; mov rdi, 0
                    ; mov rsi, 0
                    ; mov rbp, 0
                    ; mov r8,  0
                    ; mov r9,  0
                    ; mov r10, 0
                    ; mov r11, 0
                    ; mov r12, 0
                    ; mov r13, 0
                    ; mov r14, 0
                    ; mov r15, 0
                );

                dynasm!(f
                    ; mov eax,  0
                    ; mov ebx,  0
                    ; mov ecx,  0
                    ; mov edx,  0
                    ; mov edi,  0
                    ; mov esi,  0
                    ; mov ebp,  0
                    ; mov r8d,  0
                    ; mov r9d,  0
                    ; mov r10d, 0
                    ; mov r11d, 0
                    ; mov r12d, 0
                    ; mov r13d, 0
                    ; mov r14d, 0
                    ; mov r15d, 0
                );

                dynasm!(f
                    ; mov ax,  0
                    ; mov bx,  0
                    ; mov cx,  0
                    ; mov dx,  0
                    ; mov di,  0
                    ; mov si,  0
                    ; mov bp,  0

                    ; xor ax, ax
                    ; xor bx, bx
                    ; xor cx, cx
                    ; xor dx, dx
                    ; xor di, di
                    ; xor si, si
                );

                dynasm!(f
                    ; mov ah,  0
                    ; mov bh,  0
                    ; mov ch,  0
                    ; mov dh,  0
                    ; xor ah, ah
                    ; xor bh, bh
                    ; xor ch, ch
                    ; xor dh, dh
                );




                dynasm!(f
                    ; xor rax, rax
                    ; xor rbx, rbx
                    ; xor rcx, rcx
                    ; xor rdx, rdx
                    ; xor rdi, rdi
                    ; xor rsi, rsi
                    ; xor rbp, rbp
                    ; xor r8,  r8
                    ; xor r9,  r9
                    ; xor r10, r10
                    ; xor r11, r11
                    ; xor r12, r12
                    ; xor r13, r13
                    ; xor r14, r14
                    ; xor r15, r15
                );

                dynasm!(f
                    ; mov rax, rax
                    ; mov rbx, rbx
                    ; mov rcx, rcx
                    ; mov rdx, rdx
                    ; mov rdi, rdi
                    ; mov rsi, rsi
                    ; mov rbp, rbp
                    ; mov r8,  r8
                    ; mov r9,  r9
                    ; mov r10, r10
                    ; mov r11, r11
                    ; mov r12, r12
                    ; mov r13, r13
                    ; mov r14, r14
                    ; mov r15, r15
                );
 
                // Try to free any hanging references to physical registers 
                // in the register map by renaming all available architectural 
                // registers to a known-zero register. R8 should be free. 
                dynasm!(f
                    ; mov rax, r8
                    ; mov rbx, r8
                    ; mov rcx, r8
                    ; mov rdx, r8
                    ; mov rdi, r8
                    ; mov rsi, r8
                    ; mov rbp, r8
                    ; mov r8,  r8
                    ; mov r9,  r8
                    ; mov r10, r8
                    ; mov r11, r8
                    ; mov r12, r8
                    ; mov r13, r8
                    ; mov r14, r8
                    ; mov r15, r8
                );

                // Try to free any hanging references to physical registers 
                // *in the store queue* by filling the store queue with writes 
                // that depend on a known-zero register.
                dynasm!(f
                    ; mov [0x0000_0200], rax
                    ; mov [0x0000_0200], rbx
                    ; mov [0x0000_0200], rcx
                    ; mov [0x0000_0200], rdx
                    ; mov [0x0000_0200], rdi
                    ; mov [0x0000_0200], rsi
                    ; mov [0x0000_0200], rbp
                    ; mov [0x0000_0200], r8
                    ; mov [0x0000_0200], r9
                    ; mov [0x0000_0200], r10
                    ; mov [0x0000_0200], r11
                    ; mov [0x0000_0200], r12
                    ; mov [0x0000_0200], r13
                    ; mov [0x0000_0200], r14
                    ; mov [0x0000_0200], r15
                );
            }
        }

        match opts.misprediction_strat {
            MispredictionStrategy::Return => {
                Self::emit_gadget_ret(&mut f, opts, input, user_fn);
            },
            MispredictionStrategy::IndirectJmp => {
                Self::emit_gadget_indirect(&mut f, opts, input, user_fn);
            },
        }
        match opts.rdpmc_strat {
            RdpmcStrategy::Gpr(reg) => {
                f.emit_rdpmc_end(opts.ctr_idx, reg as _, Gpr::Rax as _);
            },
            RdpmcStrategy::MemStatic(addr) => {
                dynasm!(f
                    ; lfence
                    ; mov rcx, opts.ctr_idx
                    ; lfence
                    ; rdpmc
                    ; lfence
                    ; mov rbx, [addr]
                    ; sub rax, rbx
                );
            },
        }

        f.emit_ret();
        f.commit().unwrap();
        f
    }
}


