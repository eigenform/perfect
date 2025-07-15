use crate::asm::*;
use crate::experiments::*;
use clap::ValueEnum;

pub mod int_instr;
pub mod fp_instr;

pub use int_instr::*;
pub use fp_instr::*;

use crate::experiments::pmcdisc::{TestEmitter, TestGroup};

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
    /// Conditional branch instructions (simple loop)
    BranchDirectLoop,

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

    /// Integer read-after-write (RAW) hazards
    Hazard,

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

    /// AVX instructions
    Avx,
    /// AVX2 instructions
    Avx2,
    /// SSE instructions
    Sse,

    /// 87 floating point instructions
    X87,

    /// Unsorted instructions
    Unsorted,
    
}
impl TestGroupId {

    pub const ALL_GROUPS: &'static [Self; 35] = &[
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
        Self::BranchDirectLoop,
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

        Self::Avx,
        Self::Avx2,
        Self::Sse,
        Self::X87,

        Self::Unsorted,
    ];

    pub fn group(&self) -> &'static TestGroup { 
        match self { 
            Self::NopEncodings => &GRP_NOP_ENCODINGS,

            Self::Rr64Integer  => &GRP_RR64_INTEGER,
            Self::Ri64Integer  => &GRP_RI64_INTEGER,

            Self::Hazard       => &GRP_HAZ_RAW_INTEGER,

            Self::IrMov        => &GRP_IR_MOV,
            Self::RrMov        => &GRP_RR_MOV,
            Self::MrMov        => &GRP_MR_MOV,
            Self::RmMov        => &GRP_RM_MOV,
            Self::IntFpMov     => &GRP_INT_FP_MOV,
            Self::FpIntMov     => &GRP_FP_INT_MOV,
            Self::FpFpMov      => &GRP_FP_FP_MOV,

            Self::Lea          => &GRP_LEA,

            Self::BranchDirect => &GRP_BRANCH,
            Self::BranchDirectLoop => &GRP_BRANCH_LOOP,
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
            Self::Avx          => &GRP_AVX_OPS,
            Self::Avx2         => &GRP_AVX2_OPS,
            Self::Sse          => &GRP_SSE_OPS,

            Self::X87          => &GRP_X87,

            Self::Unsorted     => &GRP_UNSORTED,
            _ => unimplemented!("{:?}", self),
        }
    }
}

pub static GRP_BRANCH_LOOP: TestGroup = TestGroup {
    name: "Direct [conditional] branch instructions (512 iterations)",
    floor: None,
    // NOTE: Ideally this is in the prologue, but RCX is clobbered by RDPMC
    common_measured: Some(|mut f| { 
        dynasm!(f
            ; mov rcx, 512
        )
    }),
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new("", |mut f| { dynasm!(f
            ; next:
            ; dec rcx
            ; jnz BYTE  <next
        )}),
        TestEmitter::new("", |mut f| { dynasm!(f
            ; next:
            ; loop <next
        )}),
    ],
};

pub static GRP_FENCE: TestGroup = TestGroup {
    name: "Memory fence instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; lfence ) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sfence ) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mfence ) }),
    ],
};

pub static GRP_CACHE_CTL: TestGroup = TestGroup {
    name: "Cache control instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; clflush [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; clzero) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; prefetch    [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; prefetchw   [0x0000_0180]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; prefetchnta [0x0000_0280]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; prefetcht0  [0x0000_0480]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; prefetcht1  [0x0000_0880]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; prefetcht2  [0x0000_1080]) }),
    ],
};

pub static GRP_LEGACY: TestGroup = TestGroup {
    name: "Legacy instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; lsl  rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lar  rax, rax ) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; verr  ax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; verw  ax) }),

        // NOTE: SMSW and STR are trapped by the kernel, don't test them
        // NOTE: LLDT gives #GP
    ],
};

pub static GRP_FLAG_MANIP: TestGroup = TestGroup {
    name: "Flag-manipulation instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; lahf) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sahf) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; clc) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cld) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; ucomisd xmm0, xmm0) }),
        // NOTE: STI and CLI are probably faulting
        // NOTE: CLAC and STAC give #UD ?
    ],
};

pub static GRP_MISC: TestGroup = TestGroup {
    name: "Miscellaneous instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; vzeroupper) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vzeroall) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cpuid) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rdtsc) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rdtscp) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rdpmc) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; crc32 rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; xgetbv) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; xsave  [0x0000_0100]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; xrstor [0x0000_0100]) }),
    ],
};

pub static GRP_RAND: TestGroup = TestGroup {
    name: "RDRAND/RDSEED instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; rdrand rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rdseed rax) }),
    ],
};

pub static GRP_STACK: TestGroup = TestGroup {
    name: "Stack-use instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; push rsp) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; pop  rsp) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; push rdi) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; pop  rdi) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovq rsp, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovd esp, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movq  rsp, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movd  esp, xmm0) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; movmskpd  rsp, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movmskps  rsp, xmm0) }),
    ],
};


pub static GRP_UNSORTED: TestGroup = TestGroup {
    name: "Unsorted",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; rep lodsq) }),

        //|mut f| { dynasm!(f; emms) },
        //|mut f| { dynasm!(f; frstor [0x0000_0080]) },

        // Unsupported?
        //|mut f| { dynasm!(f; vpcmov xmm0, xmm1, xmm2, xmm3) },
        //|mut f| { dynasm!(f; vpcmov ymm0, ymm1, ymm2, ymm3) },
        //|mut f| { dynasm!(f; vfrczpd xmm0, xmm1) },
        //|mut f| { dynasm!(f; vfrczps xmm0, xmm1) },

    ],
};

pub static GRP_HAZ_RAW_INTEGER: TestGroup = TestGroup {
    name: "RAW hazards",
    floor: None,
    common_measured: None,
    prologue: Some(|mut f| { 
        dynasm!(f
            ; mov r8, 0xdeadc0de
            ; mov [0x0000_1200], r8 
            ; movnti [0x0000_1300], r8 
        )
    }),
    epilogue: None,
    emitters: &[
        TestEmitter::new("inc chain (8)", |mut f| {
            for _ in 0..8 { dynasm!(f ; inc r9) }
        }),
        TestEmitter::new("dependent add (reg)", 
            |mut f| dynasm!(f ; add rax, r8)
        ),
        TestEmitter::new("dependent add (mem)", 
            |mut f| dynasm!(f ; add rax, [0x0000_1200])
        ),
        TestEmitter::new("dependent add (slow mem)", 
            |mut f| dynasm!(f ; add rax, [0x0000_1300])
        ),
        TestEmitter::new("immediate add", 
            |mut f| dynasm!(f ; add rax, 1)
        ),
    ],
};

pub static GRP_INTEGER_DEPS: TestGroup = TestGroup {
    name: "Dependencies",
    floor: None,
    common_measured: Some(|mut f| { 
        dynasm!(f
            ; mov rax, 0xdeadbeef
        );
    }),
    prologue: Some(|mut f| {
        dynasm!(f
            ; mov rax, 0xdeadbeef
            ; movnti [0x0000_0111], rax 
            ; mfence
        );
    }),
    epilogue: None,
    emitters: &[
        TestEmitter::new("", |mut f| { dynasm!(f
            ; mov rax, [0x0000_0111]
            ; add rbx, rax
            ; lfence
        )}),
        TestEmitter::new("", |mut f| { dynasm!(f
            ; add rbx, rax
            ; lfence
        )}),
        TestEmitter::new("", |mut f| { dynasm!(f
            ; add rbx, rax
            ; add rcx, rbx
            ; add rdx, rcx
            ; lfence
        )}),
    ],
};


