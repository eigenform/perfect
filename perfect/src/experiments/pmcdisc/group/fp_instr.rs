use crate::experiments::pmcdisc::group::*;

pub static GRP_FP_INT_MOV: TestGroup = TestGroup {
    name: "Floating-point-to-Integer move instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovq rax, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovd eax, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movq  rax, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movd  eax, xmm0) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; movmskpd  rax, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movmskps  rax, xmm0) }),
    ],
};
pub static GRP_INT_FP_MOV: TestGroup = TestGroup {
    name: "Integer-to-Floating-point move instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovq xmm0, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovd xmm0, eax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movq  xmm0, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movd  xmm0, eax) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovq xmm0, rsp) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovd xmm0, esp) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movq  xmm0, rsp) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movd  xmm0, esp) }),

    ],
};

pub static GRP_FP_FP_MOV: TestGroup = TestGroup {
    name: "Floating-point to floating-point move instructions",
    floor: None,
    common_measured: None,
    prologue: Some(|mut f| { dynasm!(f; vzeroall) }),
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; movapd xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movaps xmm0, xmm1) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; movsd xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movss xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovsd xmm0, xmm1, xmm2) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovss xmm0, xmm1, xmm2) }),


        TestEmitter::new_anon(|mut f| { dynasm!(f; movupd xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movups xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovupd xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovups xmm0, xmm1) }),


        TestEmitter::new_anon(|mut f| { dynasm!(f; movshdup xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movsldup xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovshdup xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovsldup xmm0, xmm1) }),


        TestEmitter::new_anon(|mut f| { dynasm!(f; movq  xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movddup xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movdqa xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movdqu xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movhlps xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movlhps xmm0, xmm1) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovq xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovddup xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovdqa xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovdqu xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovhlps xmm0, xmm1, xmm2) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovlhps xmm0, xmm1, xmm2) }),

    ],
};

pub static GRP_SSE_OPS: TestGroup = TestGroup {
    name: "SSE instructions",
    floor: None,
    common_measured: None,
    prologue: Some(|mut f| { dynasm!(f ; stmxcsr [0x1000]) }),
    epilogue: Some(|mut f| { dynasm!(f ; ldmxcsr [0x1000]) }),
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; comiss xmm0, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; divps xmm0, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; addsubps xmm0, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cmpss xmm0, xmm0, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cvtsi2ss xmm0, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cvtss2si rax, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; maxps xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; minps xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mulps xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; pinsrw xmm0, eax, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcpss xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rsqrtps xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sqrtps xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; shufps xmm0, xmm1, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; stmxcsr [0x1000]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; ldmxcsr [0x1000]) }),
    ],
};

pub static GRP_AVX2_OPS: TestGroup = TestGroup {
    name: "AVX2 instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; movntdqa xmm0, [0x1000]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovntdqa xmm0, [0x1000]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmovntdqa ymm0, [0x1000]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vmpsadbw ymm0, ymm2, [0x1000], 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vpabsb ymm0, ymm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vpabsd ymm0, ymm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vpabsw ymm0, ymm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vpbroadcastq ymm0, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vinserti128 ymm0, ymm1, [0x1000], 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vinserti128 ymm0, ymm1, xmm2, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vpblendd xmm0, xmm1, xmm2, 0) }),

        // TODO: Figure out how the hell VSIB addressing works
        //|mut f| { dynasm!(f; vgatherqpd ymm0, [rdi + 0x1000], ymm1) },


    ],
};

pub static GRP_AVX_OPS: TestGroup = TestGroup {
    name: "AVX instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; vpermq ymm0, ymm0, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vdivps xmm0, xmm0, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vaddsubps xmm0, xmm0, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vcvtsi2ss xmm0, xmm1, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vcvtss2si rax, xmm0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; insertq xmm0, xmm1, 0, 24) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; insertq xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; vhaddpd xmm0, xmm1, xmm2) }),


    ],
};

pub static GRP_X87: TestGroup = TestGroup {
    name: "x87 floating-point instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; f2xm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; fabs) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; fbld [0x1000]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; fcmovb st0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; ffree) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; fxch) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; fxtract) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; fyl2x) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; fyl2xp1) }),
    ],
};

pub static GRP_AES: TestGroup = TestGroup {
    name: "AES instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; aesdeclast xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; aesenc xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; aesenclast xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; aesimc xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; aeskeygenassist xmm0, xmm1, 0) }),
    ],
};

pub static GRP_SHA: TestGroup = TestGroup {
    name: "SHA instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; sha1rnds4 xmm0, xmm1, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sha1nexte xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sha1msg1 xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sha1msg2 xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sha256rnds2 xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sha256msg1 xmm0, xmm1) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sha256msg2 xmm0, xmm1) }),


    ],
};

