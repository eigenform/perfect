
use crate::asm::*;
use crate::experiments::*;
use clap::ValueEnum;

/// A group of emitters.
pub struct TestGroup { 
    pub name: &'static str,

    /// A prologue common to all emitters in this group, executed before 
    /// the start of the measurement. 
    pub prologue: Option<fn(&mut X64Assembler)>,

    /// An epilogue common to all emitters in this group, executed after 
    /// the end of the measurement. 
    pub epilogue: Option<fn(&mut X64Assembler)>,

    /// A common block of code emitted *after* the start of the measurement,
    /// for all emitters in this group. 
    pub common: Option<fn(&mut X64Assembler)>,

    pub floor: Option<fn(&mut X64Assembler)>,

    pub emitters: &'static [fn(&mut X64Assembler)],
}

/// Identifier for a statically-defined [TestGroup]. 
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TestGroupId {
    /// NOP encodings
    NopEncodings,

    /// Register-register 64-bit integer instructions
    Rr64Integer,
    /// Immediate-register 64-bit integer instructions
    Ri64Integer,

    /// Register-to-register moves
    RrMov,
    /// Immediate-to-register moves
    IrMov,
    /// Floating-point/vector register to integer register moves
    FpIntMov,
    /// Integer register to floating-point/vector register moves
    IntFpMov,
    /// Floating-point to floating-point moves
    FpFpMov,

    /// Memory-to-register moves (loads)
    RmMov,
    /// Register-to-memory moves (stores)
    MrMov,

    /// LEA instructions
    Lea,

    /// Conditional branch instructions
    BranchDirect,
    /// Direct jump instructions
    JmpDirect,
    /// Indirect jump instructions
    JmpIndirect,
    /// Direct call instructions
    CallDirect,
    /// Indirect call instructions
    CallIndirect,
    /// Return instructions
    Ret,

    Hazard,


    //IntegerDependencies,

    /// Bit-manipulation instructions
    Bmi,
    /// Flag-manipulation instructions
    FlagManip,
    /// Cache control instructions
    CacheControl,
    /// Legacy instructions
    Legacy,
    /// Memory fence instructions
    Fence,
    /// RDRAND/RDSEED instructions
    Rand,
    /// Miscellaneous instructions
    Misc,

    /// Stack-use instructions
    Stack,

    /// Integer multiplication instructions
    Mul,
    /// 64-bit integer division instructions
    Div64,
    /// 32-bit integer division instructions
    Div32,

    /// Unsorted instructions
    Unsorted,
    
}
impl TestGroupId {

    pub const ALL_GROUPS: &'static [Self; 30] = &[
        Self::NopEncodings,
        Self::Rr64Integer,
        Self::Ri64Integer,
        Self::RrMov,
        Self::IrMov,
        Self::FpIntMov,
        Self::IntFpMov,
        Self::FpFpMov,
        Self::RmMov,
        Self::MrMov,
        Self::Lea,
        Self::BranchDirect,
        Self::JmpDirect,
        Self::JmpIndirect,
        Self::CallDirect,
        Self::CallIndirect,
        Self::Ret,
        Self::Hazard,
        Self::Bmi,
        Self::FlagManip,
        Self::CacheControl,
        Self::Legacy,
        Self::Fence,
        Self::Rand,
        Self::Misc,
        Self::Stack,
        Self::Mul,
        Self::Div64,
        Self::Div32,
        Self::Unsorted,
    ];

    pub fn group(&self) -> &'static TestGroup { 
        match self { 
            Self::NopEncodings        => &GRP_NOP_ENCODINGS,

            Self::Rr64Integer         => &GRP_RR64_INTEGER,
            Self::Ri64Integer         => &GRP_RI64_INTEGER,

            Self::Hazard => &GRP_HAZ_RAW_INTEGER,

            Self::IrMov        => &GRP_IR_MOV,
            Self::RrMov        => &GRP_RR_MOV,
            Self::MrMov        => &GRP_MR_MOV,
            Self::RmMov        => &GRP_RM_MOV,
            Self::IntFpMov     => &GRP_INT_FP_MOV,
            Self::FpIntMov     => &GRP_FP_INT_MOV,
            Self::FpFpMov      => &GRP_FP_FP_MOV,

            Self::Lea          => &GRP_LEA,

            //Self::IntegerDependencies => &GRP_INTEGER_DEPS,

            Self::BranchDirect => &GRP_BRANCH,
            Self::JmpDirect    => &GRP_JMP_DIRECT,
            Self::CallDirect   => &GRP_CALL_DIRECT,
            Self::JmpIndirect  => &GRP_JMP_INDIRECT,
            Self::CallIndirect => &GRP_CALL_INDIRECT,
            Self::Ret          => &GRP_RETURN,

            Self::Bmi          => &GRP_BMI,
            Self::CacheControl => &GRP_CACHE_CTL,

            Self::Legacy       => &GRP_LEGACY,
            Self::Fence        => &GRP_FENCE,
            Self::Rand         => &GRP_RAND,
            Self::FlagManip    => &GRP_FLAG_MANIP,
            Self::Misc         => &GRP_MISC,
            Self::Stack        => &GRP_STACK,
            Self::Mul          => &GRP_MUL,
            Self::Div64        => &GRP_DIV64,
            Self::Div32        => &GRP_DIV32,
            Self::Unsorted     => &GRP_UNSORTED,
            _ => unimplemented!("{:?}", self),
        }
    }
}

static GRP_NOP_ENCODINGS: TestGroup = TestGroup {
    name: "NOP encodings",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f;  nop) },
        |mut f| { dynasm!(f; .bytes NOP2) },
        |mut f| { dynasm!(f; .bytes NOP3) },
        |mut f| { dynasm!(f; .bytes NOP4) },
        |mut f| { dynasm!(f; .bytes NOP5) },
        |mut f| { dynasm!(f; .bytes NOP6) },
        |mut f| { dynasm!(f; .bytes NOP7) },
        |mut f| { dynasm!(f; .bytes NOP8) },
        |mut f| { dynasm!(f; .bytes NOP9) },
        |mut f| { dynasm!(f; .bytes NOP10) },
        |mut f| { dynasm!(f; .bytes NOP11) },
        |mut f| { dynasm!(f; .bytes NOP12) },
        |mut f| { dynasm!(f; .bytes NOP13) },
        |mut f| { dynasm!(f; .bytes NOP14) },
        |mut f| { dynasm!(f; .bytes NOP15) },

    ],
};

static GRP_HAZ_RAW_INTEGER: TestGroup = TestGroup {
    name: "RAW hazards",
    floor: None,
    common: None,
    prologue: Some(|mut f| { 
        dynasm!(f
            ; mov r8, 0xdeadc0de
        )
    }),
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f ; add rax, r8) },
        |mut f| { dynasm!(f ; add rax, r9) },
    ],
};

static GRP_MUL: TestGroup = TestGroup { 
    name: "Integer multiplication instructions",
    floor: None,
    common: Some(|mut f| {
        dynasm!(f
            ; mov rax, QWORD 0xa5a5_a5a5_a5a5
        )
    }),
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f ; imul rax, rax) },
        |mut f| { dynasm!(f ; imul eax, eax) },
        |mut f| { dynasm!(f ; imul  ax,  ax) },

        |mut f| { dynasm!(f ; mulx rax, rax, rax) },
        |mut f| { dynasm!(f ; mulx eax, eax, eax) },

        |mut f| { dynasm!(f ; mul rax) },
        |mut f| { dynasm!(f ; mul eax) },
        |mut f| { dynasm!(f ; mul  ax) },
        |mut f| { dynasm!(f ; mul  ah) },
        |mut f| { dynasm!(f ; mul  al) },
    ],
};


static GRP_DIV64: TestGroup = TestGroup { 
    name: "64-bit integer division instructions",
    floor: None,
    common: Some(|mut f| {
        dynasm!(f
            // Dividend
            ; xor rax, rax
            ; xor rdx, rdx
            ; mov rax, QWORD 0x5a5a_5a5a_5a5a_5a5a
            // Divisor
            ; mov r8, 31
        )
    }),
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f ; div  r8)},
        |mut f| { dynasm!(f ; idiv r8)},

    ],
};

static GRP_DIV32: TestGroup = TestGroup { 
    name: "32-bit integer division instructions",
    floor: None,
    common: Some(|mut f| {
        dynasm!(f
            // Dividend
            ; xor rax, rax
            ; xor rdx, rdx
            ; mov rax, QWORD 0x5a5a_5a5a
            // Divisor
            ; mov r8, 31
        )
    }),
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f ; div  r8d)},
        |mut f| { dynasm!(f ; idiv r8d)},

    ],
};




static GRP_RR64_INTEGER: TestGroup = TestGroup { 
    name: "Register-register 64-bit integer instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; add    rax, rax) },
        |mut f| { dynasm!(f; sub    rax, rax) },
        |mut f| { dynasm!(f; xor    rax, rax) },
        |mut f| { dynasm!(f; or     rax, rax) },
        |mut f| { dynasm!(f; and    rax, rax) },
        |mut f| { dynasm!(f; imul   rax, rax) },
        |mut f| { dynasm!(f; cmp    rax, rax) },
        |mut f| { dynasm!(f; test   rax, rax) },

        |mut f| { dynasm!(f; adc    rax, rax) },
        |mut f| { dynasm!(f; adcx   rax, rax) },
        |mut f| { dynasm!(f; adox   rax, rax) },

        |mut f| { dynasm!(f; neg    rax) },
        |mut f| { dynasm!(f; not    rax) },
        |mut f| { dynasm!(f; inc    rax) },
        |mut f| { dynasm!(f; dec    rax) },
        |mut f| { dynasm!(f; bswap  rax) },

        |mut f| { dynasm!(f; cbw) },
        |mut f| { dynasm!(f; cwde) },
        |mut f| { dynasm!(f; cdqe) },
        |mut f| { dynasm!(f; cwd) },
        |mut f| { dynasm!(f; cdq) },
        |mut f| { dynasm!(f; cqo) },

    ],
};

static GRP_RI64_INTEGER: TestGroup = TestGroup {
    name: "Register-immediate 64-bit integer instructions",
    prologue: None,
    epilogue: None,
    floor: None,
    common: None,
    emitters: &[
        |mut f| { dynasm!(f; add  rax, 0) },
        |mut f| { dynasm!(f; add  rsp, 0) },

        |mut f| { dynasm!(f; sub  rax, 0) },
        |mut f| { dynasm!(f; sub  rsp, 0) },

        |mut f| { dynasm!(f; xor  rax, 0) },
        |mut f| { dynasm!(f; xor  rsp, 0) },

        |mut f| { dynasm!(f; or   rax, 0) },
        |mut f| { dynasm!(f; and  rax, 0) },
        |mut f| { dynasm!(f; adc  rax, 0) },

        |mut f| { dynasm!(f; cmp  rax, 0) },
        |mut f| { dynasm!(f; test rax, 0) },
        |mut f| { dynasm!(f; shl  rax, 0) },
        |mut f| { dynasm!(f; shr  rax, 0) },
    ],
};



static GRP_INTEGER_DEPS: TestGroup = TestGroup {
    name: "Dependencies",
    floor: None,
    common: None,
    prologue: Some(|mut f| {
        dynasm!(f
            ; mov rax, 0xdeadbeef
            ; movnti [0x0000_0111], rax 
            ; sfence
            ; sfence
            ; sfence
            ; sfence
            ; sfence
            ; sfence
            ; sfence
            ; sfence
            ; sfence
            ; lfence
        );
    }),
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f
            ; mov rax, [0x0000_0111]
            ; add rbx, rax
        )},
    ],
};


static GRP_BRANCH: TestGroup = TestGroup {
    name: "Direct [conditional] branch instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; jnz BYTE  >next; next:) },
        |mut f| { dynasm!(f; jnz DWORD >next; next:) },
        |mut f| { dynasm!(f; jz  BYTE  >next; next:) },
        |mut f| { dynasm!(f; jz  DWORD >next; next:) },

        |mut f| { dynasm!(f; jne BYTE  >next; next:) },
        |mut f| { dynasm!(f; jne DWORD >next; next:) },
        |mut f| { dynasm!(f; je  BYTE  >next; next:) },
        |mut f| { dynasm!(f; je  DWORD >next; next:) },

        |mut f| { dynasm!(f; jle BYTE  >next; next:) },
        |mut f| { dynasm!(f; jle DWORD >next; next:) },
        |mut f| { dynasm!(f; jg  BYTE  >next; next:) },
        |mut f| { dynasm!(f; jg  DWORD >next; next:) },

        |mut f| { dynasm!(f; jc  BYTE  >next; next:) },
        |mut f| { dynasm!(f; jc  DWORD >next; next:) },
        |mut f| { dynasm!(f; jnc BYTE  >next; next:) },
        |mut f| { dynasm!(f; jnc DWORD >next; next:) },

        |mut f| { dynasm!(f; jo  BYTE  >next; next:) },
        |mut f| { dynasm!(f; jo  DWORD >next; next:) },
        |mut f| { dynasm!(f; jno BYTE  >next; next:) },
        |mut f| { dynasm!(f; jno DWORD >next; next:) },

        |mut f| { dynasm!(f; jecxz  BYTE >next; next:) },
        |mut f| { dynasm!(f; jrcxz  BYTE >next; next:) },

    ],
};

static GRP_JMP_DIRECT: TestGroup = TestGroup {
    name: "Direct [unconditional] jump instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; jmp BYTE  >next; next:) },
        |mut f| { dynasm!(f; jmp DWORD >next; next:) },
    ],
};

static GRP_JMP_INDIRECT: TestGroup = TestGroup {
    name: "Indirect [unconditional] jump instructions",
    floor: None,
    common: None,
    prologue: Some(|mut f| { 
        dynasm!(f
            ; lea r14, [->lab] 
        )
    }),
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; jmp r14; ->lab:) },
    ],
};


static GRP_CALL_DIRECT: TestGroup = TestGroup {
    name: "Direct call instructions",
    floor: None,
    common: None,
    prologue: Some(|mut f| { dynasm!(f
        ; mov r14, [rsp]
    )}),
    epilogue: Some(|mut f| { dynasm!(f
        ; mov [rsp], r14
    )}),
    emitters: &[
        |mut f| { dynasm!(f; call >next; next:) },
        |mut f| { dynasm!(f; call DWORD >next; next:) },
    ],
};

static GRP_CALL_INDIRECT: TestGroup = TestGroup {
    name: "Indirect call instructions",
    floor: None,
    common: None,
    prologue: Some(|mut f| { dynasm!(f
        ; mov r14, [rsp]
        ; lea r13, [->lab]
    )}),
    epilogue: Some(|mut f| { dynasm!(f
        ; mov [rsp], r14
    )}),
    emitters: &[
        |mut f| { dynasm!(f; call r13; ->lab:) },
    ],
};


 
static GRP_RETURN: TestGroup = TestGroup {
    name: "Return instructions",
    floor: None,
    common: None,
    prologue: Some(|mut f| { 
        dynasm!(f
            ; mov r14, [rsp]
            ; lea rax, [->lab]
            ; mov [rsp], rax
        )
    }),
    epilogue: Some(|mut f| { 
        dynasm!(f ; mov [rsp], r14)
    }),
    emitters: &[
        |mut f| { dynasm!(f ; ret; ->lab:) },
    ],
};
 




static GRP_RR_MOV: TestGroup = TestGroup {
    name: "Register-to-register moves",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; mov rax, rax); },
        |mut f| { dynasm!(f; mov eax, eax); },
        |mut f| { dynasm!(f; mov  ax,  ax); },
        |mut f| { dynasm!(f; mov  ah,  ah); },
        |mut f| { dynasm!(f; mov  al,  al); },

        |mut f| { dynasm!(f; mov rax, rbx); },
        |mut f| { dynasm!(f; mov eax, ebx); },
        |mut f| { dynasm!(f; mov  ax,  bx); },
        |mut f| { dynasm!(f; mov  ah,  bh); },
        |mut f| { dynasm!(f; mov  al,  bl); },

        |mut f| { dynasm!(f; mov  al,  bh); },
        |mut f| { dynasm!(f; mov  ah,  bl); },
        |mut f| { dynasm!(f; mov  al,  ah); },
        |mut f| { dynasm!(f; mov  ah,  al); },

        |mut f| { dynasm!(f; mov rax, rsp); },
        |mut f| { dynasm!(f; mov eax, esp); },
        |mut f| { dynasm!(f; mov  ax,  sp); },
    ],
};

static GRP_IR_MOV: TestGroup = TestGroup {
    name: "Immediate-to-register moves",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; mov rax, 0); },
        |mut f| { dynasm!(f; mov rax, 0xdead); },
        |mut f| { dynasm!(f; mov eax, 0); },
        |mut f| { dynasm!(f; mov eax, 0xdead); },
        |mut f| { dynasm!(f; mov  ax, 0); },
        |mut f| { dynasm!(f; mov  ax, 0xde); },
        |mut f| { dynasm!(f; mov  ah, 0); },
        |mut f| { dynasm!(f; mov  ah, 0xde as _); },
        |mut f| { dynasm!(f; mov  al, 0); },
        |mut f| { dynasm!(f; mov  al, 0xde as _); },

    ],
};

static GRP_MR_MOV: TestGroup = TestGroup {
    name: "Memory-to-register moves (loads)",
    floor: None,
    common: None,
    prologue: Some(|mut f| {
        dynasm!(f; prefetch [0x0000_0080])
    }),
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; mov    rax, [0x0000_0080]) },
        |mut f| { dynasm!(f; mov    eax, [0x0000_0080]) },
        |mut f| { dynasm!(f; mov     ax, [0x0000_0080]) },
        |mut f| { dynasm!(f; mov     ah, [0x0000_0080]) },
        |mut f| { dynasm!(f; mov     al, [0x0000_0080]) },

        |mut f| { dynasm!(f; movbe  rax, [0x0000_0080]) },
        |mut f| { dynasm!(f; movbe  eax, [0x0000_0080]) },
        |mut f| { dynasm!(f; movbe   ax, [0x0000_0080]) },

        |mut f| { dynasm!(f; movsx  rax, [0x0000_0080]) },
        |mut f| { dynasm!(f; movsx  eax, [0x0000_0080]) },
        |mut f| { dynasm!(f; movsx   ax, [0x0000_0080]) },

        |mut f| { dynasm!(f; movsxd rax, [0x0000_0080]) },

        |mut f| { dynasm!(f; movzx  rax, [0x0000_0080]) },
        |mut f| { dynasm!(f; movzx  eax, [0x0000_0080]) },
        |mut f| { dynasm!(f; movzx   ax, [0x0000_0080]) },
    ],
};

static GRP_RM_MOV: TestGroup = TestGroup {
    name: "Register-to-Memory moves (stores)",
    floor: None,
    common: None,
    prologue: Some(|mut f| {
        dynasm!(f; prefetch [0x0000_0080])
    }),
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; mov    [0x0000_0080], rax) },
        |mut f| { dynasm!(f; mov    [0x0000_0080], eax) },
        |mut f| { dynasm!(f; mov    [0x0000_0080],  ax) },
        |mut f| { dynasm!(f; mov    [0x0000_0080],  ah) },
        |mut f| { dynasm!(f; mov    [0x0000_0080],  al) },

        |mut f| { dynasm!(f; movbe  [0x0000_0080], rax) },
        |mut f| { dynasm!(f; movbe  [0x0000_0080], eax) },
        |mut f| { dynasm!(f; movbe  [0x0000_0080],  ax) },

        |mut f| { dynasm!(f; movnti [0x0000_0080], rax) },
        |mut f| { dynasm!(f; movnti [0x0000_0080], eax) },
    ],
};




static GRP_FENCE: TestGroup = TestGroup {
    name: "Memory fence instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; lfence ) },
        |mut f| { dynasm!(f; sfence ) },
        |mut f| { dynasm!(f; mfence ) },
    ],
};

static GRP_CACHE_CTL: TestGroup = TestGroup {
    name: "Cache control instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; clflush [0x0000_0080]) },
        |mut f| { dynasm!(f; clzero) },

        |mut f| { dynasm!(f; prefetch    [0x0000_0080]) },
        |mut f| { dynasm!(f; prefetchw   [0x0000_0180]) },
        |mut f| { dynasm!(f; prefetchnta [0x0000_0280]) },
        |mut f| { dynasm!(f; prefetcht0  [0x0000_0480]) },
        |mut f| { dynasm!(f; prefetcht1  [0x0000_0880]) },
        |mut f| { dynasm!(f; prefetcht2  [0x0000_1080]) },
    ],
};

static GRP_BMI: TestGroup = TestGroup {
    name: "Bit-manipulation instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[

        // Base instruction set
        |mut f| { dynasm!(f; bsf rax, rax) },
        |mut f| { dynasm!(f; bsr rax, rax) },
        |mut f| { dynasm!(f; bt  rax, rax) },
        |mut f| { dynasm!(f; btc rax, rax) },
        |mut f| { dynasm!(f; btr rax, rax) },
        |mut f| { dynasm!(f; bts rax, rax) },

        // BMI1
        |mut f| { dynasm!(f; andn   rax, rax, rax) },
        |mut f| { dynasm!(f; blsi   rax, rax) },
        |mut f| { dynasm!(f; blsmsk rax, rax) },
        |mut f| { dynasm!(f; blsr   rax, rax) },
        |mut f| { dynasm!(f; tzcnt  rax, rax) },

        // BMI2
        |mut f| { dynasm!(f; bzhi   rax, rax, rax) },
        |mut f| { dynasm!(f; mulx   rax, rax, rax) },
        |mut f| { dynasm!(f; pext   rbx, rbx, rbx) },
        |mut f| { dynasm!(f; pdep   rbx, rbx, rbx) },
        |mut f| { dynasm!(f; rorx   rax, rax, 0) },
        |mut f| { dynasm!(f; sarx   rax, rax, rax) },
        |mut f| { dynasm!(f; shlx   rax, rax, rax) },
        |mut f| { dynasm!(f; shrx   rax, rax, rax) },

        // The register form is from the BMI extension,
        // but the immediate form is from TBM (unsupported)
        |mut f| { dynasm!(f; bextr   rax, rax, rax) },

        // TBM (these are unsupported on Zen2) 
        //|mut f| { dynasm!(f; blcfill rax, rax) },
        //|mut f| { dynasm!(f; blci    rax, rax) },
        //|mut f| { dynasm!(f; blcic   rax, rax) },
        //|mut f| { dynasm!(f; blcmsk  rax, rax) },
        //|mut f| { dynasm!(f; blcs    rax, rax) },
        //|mut f| { dynasm!(f; blsfill rax, rax) },
        //|mut f| { dynasm!(f; blsic   rax, rax) },
        //|mut f| { dynasm!(f; t1mskc  rax, rax) },
        //|mut f| { dynasm!(f; tzmsk   rax, rax) },

        // ABM
        |mut f| { dynasm!(f; lzcnt  rax, rax) },
        |mut f| { dynasm!(f; popcnt rax, rax) },

    ],
};

static GRP_LEGACY: TestGroup = TestGroup {
    name: "Legacy instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; lsl  rax, rax) },
        |mut f| { dynasm!(f; lar  rax, rax ) },
        |mut f| { dynasm!(f; verr  ax) },
        |mut f| { dynasm!(f; verw  ax) },

        // This is apparently trapped + emulated by the kernel?
        //|mut f| { dynasm!(f; smsw  rax) },
        // This is apparently trapped + emulated by the kernel?
        //|mut f| { dynasm!(f; str  rax) },
        // #GP
        //|mut f| { dynasm!(f; lldt  ax) },
    ],
};

static GRP_FLAG_MANIP: TestGroup = TestGroup {
    name: "Flag-manipulation instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; lahf) },
        |mut f| { dynasm!(f; sahf) },

        |mut f| { dynasm!(f; clc) },
        |mut f| { dynasm!(f; cld) },

        // Probably also faulting (haven't tested)
        //|mut f| { dynasm!(f; sti) },
        //|mut f| { dynasm!(f; cli) },
        // #UD 
        //|mut f| { dynasm!(f; clac) },
        //|mut f| { dynasm!(f; stac) },
    ],
};

static GRP_MISC: TestGroup = TestGroup {
    name: "Miscellaneous instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; vzeroupper) },
        |mut f| { dynasm!(f; vzeroall) },
        |mut f| { dynasm!(f; cpuid) },
        |mut f| { dynasm!(f; rdtsc) },
        |mut f| { dynasm!(f; rdtscp) },
        |mut f| { dynasm!(f; rdpmc) },

        |mut f| { dynasm!(f; xgetbv) },
        //|mut f| { dynasm!(f; xsetbv) },

        |mut f| { dynasm!(f; xsave  [0x0000_0100]) },
        //|mut f| { dynasm!(f; xrstor [0x0000_0100]) },
    ],
};

static GRP_RAND: TestGroup = TestGroup {
    name: "RDRAND/RDSEED instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; rdrand rax) },
        |mut f| { dynasm!(f; rdseed rax) },
    ],
};

static GRP_STACK: TestGroup = TestGroup {
    name: "Stack instructions",
    floor: None,
    common: None,
    prologue: Some(|mut f| { 
        //dynasm!(f
        //    ; mov rbp, rsp
        //    ; sub rsp, 0x100
        //);
    }),
    epilogue: Some(|mut f| { 
        //dynasm!(f
        //    ; mov rsp, rbp
        //);
    }),
    emitters: &[
        |mut f| { dynasm!(f; push rsp) },
        |mut f| { dynasm!(f; pop  rsp) },
        |mut f| { dynasm!(f; push rdi) },
        |mut f| { dynasm!(f; pop  rdi) },

        |mut f| { dynasm!(f; vmovq rsp, xmm0) },
        |mut f| { dynasm!(f; vmovd esp, xmm0) },
        |mut f| { dynasm!(f; movq  rsp, xmm0) },
        |mut f| { dynasm!(f; movd  esp, xmm0) },

        |mut f| { dynasm!(f; movmskpd  rsp, xmm0) },
        |mut f| { dynasm!(f; movmskps  rsp, xmm0) },


    ],
};


static GRP_LEA: TestGroup = TestGroup {
    name: "LEA instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; lea rax, [0x0000_0080]) },
        |mut f| { dynasm!(f; lea rax, [0x0000_0080 * 8]) },
        |mut f| { dynasm!(f; lea rax, [0x0000_0080 + rax]) },
        |mut f| { dynasm!(f; lea rax, [0x0000_0080 + rax * 8]) },

        |mut f| { dynasm!(f; lea rax, [rip]) },
        |mut f| { dynasm!(f; lea rax, [rip + 0x0000_0020]) },

        |mut f| { dynasm!(f; lea rax, [rsp]) },
        |mut f| { dynasm!(f; lea rax, [rsp + 0x0000_0020]) },
    ],
};

static GRP_FP_INT_MOV: TestGroup = TestGroup {
    name: "Floating-point-to-Integer move instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; vmovq rax, xmm0) },
        |mut f| { dynasm!(f; vmovd eax, xmm0) },
        |mut f| { dynasm!(f; movq  rax, xmm0) },
        |mut f| { dynasm!(f; movd  eax, xmm0) },

        |mut f| { dynasm!(f; movmskpd  rax, xmm0) },
        |mut f| { dynasm!(f; movmskps  rax, xmm0) },
    ],
};
static GRP_INT_FP_MOV: TestGroup = TestGroup {
    name: "Integer-to-Floating-point move instructions",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; vmovq xmm0, rax) },
        |mut f| { dynasm!(f; vmovd xmm0, eax) },
        |mut f| { dynasm!(f; movq  xmm0, rax) },
        |mut f| { dynasm!(f; movd  xmm0, eax) },

        |mut f| { dynasm!(f; vmovq xmm0, rsp) },
        |mut f| { dynasm!(f; vmovd xmm0, esp) },
        |mut f| { dynasm!(f; movq  xmm0, rsp) },
        |mut f| { dynasm!(f; movd  xmm0, esp) },

    ],
};

static GRP_FP_FP_MOV: TestGroup = TestGroup {
    name: "Floating-point to floating-point move instructions",
    floor: None,
    common: None,
    prologue: Some(|mut f| { dynasm!(f; vzeroall) }),
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; movapd xmm0, xmm1) },
        |mut f| { dynasm!(f; movaps xmm0, xmm1) },

        |mut f| { dynasm!(f; movsd xmm0, xmm1) },
        |mut f| { dynasm!(f; movss xmm0, xmm1) },
        |mut f| { dynasm!(f; vmovsd xmm0, xmm1, xmm2) },
        |mut f| { dynasm!(f; vmovss xmm0, xmm1, xmm2) },


        |mut f| { dynasm!(f; movupd xmm0, xmm1) },
        |mut f| { dynasm!(f; movups xmm0, xmm1) },
        |mut f| { dynasm!(f; vmovupd xmm0, xmm1) },
        |mut f| { dynasm!(f; vmovups xmm0, xmm1) },


        |mut f| { dynasm!(f; movshdup xmm0, xmm1) },
        |mut f| { dynasm!(f; movsldup xmm0, xmm1) },
        |mut f| { dynasm!(f; vmovshdup xmm0, xmm1) },
        |mut f| { dynasm!(f; vmovsldup xmm0, xmm1) },


        |mut f| { dynasm!(f; movq  xmm0, xmm1) },
        |mut f| { dynasm!(f; movddup xmm0, xmm1) },
        |mut f| { dynasm!(f; movdqa xmm0, xmm1) },
        |mut f| { dynasm!(f; movdqu xmm0, xmm1) },
        |mut f| { dynasm!(f; movhlps xmm0, xmm1) },
        |mut f| { dynasm!(f; movlhps xmm0, xmm1) },

        |mut f| { dynasm!(f; vmovq xmm0, xmm1) },
        |mut f| { dynasm!(f; vmovddup xmm0, xmm1) },
        |mut f| { dynasm!(f; vmovdqa xmm0, xmm1) },
        |mut f| { dynasm!(f; vmovdqu xmm0, xmm1) },
        |mut f| { dynasm!(f; vmovhlps xmm0, xmm1, xmm2) },
        |mut f| { dynasm!(f; vmovlhps xmm0, xmm1, xmm2) },

    ],
};






static GRP_UNSORTED: TestGroup = TestGroup {
    name: "Unsorted",
    floor: None,
    common: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        |mut f| { dynasm!(f; crc32 rax, rax) },

        |mut f| { dynasm!(f; lock adc [0x0000_0080], rax) },
        |mut f| { dynasm!(f; lock add [0x0000_0080], rax) },
        |mut f| { dynasm!(f; lock and [0x0000_0080], rax) },
        |mut f| { dynasm!(f; lock btc [0x0000_0080], rax) },
        |mut f| { dynasm!(f; lock btr [0x0000_0080], rax) },
        |mut f| { dynasm!(f; lock bts [0x0000_0080], rax) },
        |mut f| { dynasm!(f; lock cmpxchg [0x0000_0080], rax) },
        |mut f| { dynasm!(f; lock cmpxchg8b [0x0000_0080]) },
        |mut f| { dynasm!(f; lock cmpxchg16b [0x0000_0080]) },
        |mut f| { dynasm!(f; lock or [0x0000_0080], rax) },
        |mut f| { dynasm!(f; lock sbb [0x0000_0080], rax) },
        |mut f| { dynasm!(f; lock sub [0x0000_0080], rax) },
        |mut f| { dynasm!(f; lock xadd [0x0000_0080], rax) },
        |mut f| { dynasm!(f; lock xor [0x0000_0080], rax) },

        |mut f| { dynasm!(f; lock dec [0x0000_0080]) },
        |mut f| { dynasm!(f; lock inc [0x0000_0080]) },
        |mut f| { dynasm!(f; lock neg [0x0000_0080]) },
        |mut f| { dynasm!(f; lock not [0x0000_0080]) },

        |mut f| { dynasm!(f; rcl rax, cl)},
        |mut f| { dynasm!(f; rcl eax, cl)},
        |mut f| { dynasm!(f; rcl ax, cl)},
        |mut f| { dynasm!(f; rcl ah, cl)},
        |mut f| { dynasm!(f; rcl al, cl)},
        |mut f| { dynasm!(f; rcl al, 1)},
        |mut f| { dynasm!(f; rcl al, 5)},
        |mut f| { dynasm!(f; rcr rax, cl)},
        |mut f| { dynasm!(f; rcr eax, cl)},
        |mut f| { dynasm!(f; rcr ax, cl)},
        |mut f| { dynasm!(f; rcr ah, cl)},
        |mut f| { dynasm!(f; rcr al, cl)},
        |mut f| { dynasm!(f; rcr al, 1)},
        |mut f| { dynasm!(f; rcr al, 5)},


        |mut f| { dynasm!(f; rep lodsq) },

    ],
};


