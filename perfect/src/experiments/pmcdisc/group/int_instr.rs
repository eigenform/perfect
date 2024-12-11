use crate::experiments::pmcdisc::group::*;

pub static GRP_NOP_ENCODINGS: TestGroup = TestGroup {
    name: "NOP encodings",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| dynasm!(f;  nop)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP2)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP3)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP4)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP5)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP6)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP7)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP8)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP9)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP10)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP11)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP12)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP13)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP14)),
        TestEmitter::new_anon(|mut f| dynasm!(f; .bytes NOP15)),

    ],
};


pub static GRP_MUL: TestGroup = TestGroup { 
    name: "Integer multiplication instructions",
    floor: None,
    common_measured: Some(|mut f| {
        dynasm!(f
            ; mov rax, QWORD 0xa5a5_a5a5_a5a5
        )
    }),
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f ; imul rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f ; imul eax, eax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f ; imul  ax,  ax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f ; mulx rax, rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f ; mulx eax, eax, eax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f ; mul rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f ; mul eax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f ; mul  ax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f ; mul  ah) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f ; mul  al) }),
    ],
};


pub static GRP_DIV64: TestGroup = TestGroup { 
    name: "64-bit integer division instructions",
    floor: None,
    common_measured: Some(|mut f| {
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
        TestEmitter::new_anon(|mut f| { dynasm!(f ; div  r8)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f ; idiv r8)}),

    ],
};

pub static GRP_DIV32: TestGroup = TestGroup { 
    name: "32-bit integer division instructions",
    floor: None,
    common_measured: Some(|mut f| {
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
        TestEmitter::new_anon(|mut f| { dynasm!(f ; div  r8d)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f ; idiv r8d)}),

    ],
};




pub static GRP_RR64_INTEGER: TestGroup = TestGroup { 
    name: "Register-register 64-bit integer instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; add    rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sub    rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; xor    rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; or     rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; and    rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; imul   rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cmp    rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; test   rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; adc    rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; adcx   rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; adox   rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; neg    rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; not    rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; inc    rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; dec    rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; bswap  rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cbw) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cwde) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cdqe) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cwd) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cdq) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; cqo) }),

    ],
};

pub static GRP_RR_INTEGER: TestGroup = TestGroup { 
    name: "Register-register integer instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcl rax, cl)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcl eax, cl)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcl  ax, cl)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcl  ah, cl)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcl  al, cl)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcl  al, 1)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcl  al, 5)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcr rax, cl)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcr eax, cl)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcr  ax, cl)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcr  ah, cl)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcr  al, cl)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcr  al, 1)}),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rcr  al, 5)}),
    ],
};



pub static GRP_RI64_INTEGER: TestGroup = TestGroup {
    name: "Register-immediate 64-bit integer instructions",
    prologue: None,
    epilogue: None,
    floor: None,
    common_measured: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; add  rax, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; add  rsp, 0) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; sub  rax, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sub  rsp, 0) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; xor  rax, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; xor  rsp, 0) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; or   rax, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; and  rax, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; adc  rax, 0) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; cmp  rax, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; test rax, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; shl  rax, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; shr  rax, 0) }),
    ],
};

pub static GRP_BRANCH: TestGroup = TestGroup {
    name: "Direct [conditional] branch instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| dynasm!(f; jnz BYTE  >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jnz DWORD >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jz  BYTE  >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jz  DWORD >next; next:)),

        TestEmitter::new_anon(|mut f| dynasm!(f; jne BYTE  >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jne DWORD >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; je  BYTE  >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; je  DWORD >next; next:)),

        TestEmitter::new_anon(|mut f| dynasm!(f; jle BYTE  >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jle DWORD >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jg  BYTE  >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jg  DWORD >next; next:)),

        TestEmitter::new_anon(|mut f| dynasm!(f; jc  BYTE  >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jc  DWORD >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jnc BYTE  >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jnc DWORD >next; next:)),

        TestEmitter::new_anon(|mut f| dynasm!(f; jo  BYTE  >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jo  DWORD >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jno BYTE  >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jno DWORD >next; next:)),

        TestEmitter::new_anon(|mut f| dynasm!(f; jecxz  BYTE >next; next:)),
        TestEmitter::new_anon(|mut f| dynasm!(f; jrcxz  BYTE >next; next:)),

    ],
};

pub static GRP_JMP_DIRECT: TestGroup = TestGroup {
    name: "Direct [unconditional] jump instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; jmp BYTE  >next; next:) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; jmp DWORD >next; next:) }),
    ],
};

pub static GRP_JMP_INDIRECT: TestGroup = TestGroup {
    name: "Indirect [unconditional] jump instructions",
    floor: None,
    common_measured: None,
    prologue: Some(|mut f| { 
        dynasm!(f
            ; lea r14, [->lab] 
        )
    }),
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; jmp r14; ->lab:) }),
    ],
};


pub static GRP_CALL_DIRECT: TestGroup = TestGroup {
    name: "Direct call instructions",
    floor: None,
    common_measured: None,
    prologue: Some(|mut f| { dynasm!(f
        ; mov r14, [rsp]
    )}),
    epilogue: Some(|mut f| { dynasm!(f
        ; mov [rsp], r14
    )}),
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; call >next; next:) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; call DWORD >next; next:) }),
    ],
};

pub static GRP_CALL_INDIRECT: TestGroup = TestGroup {
    name: "Indirect call instructions",
    floor: None,
    common_measured: None,
    prologue: Some(|mut f| { dynasm!(f
        ; mov r14, [rsp]
        ; lea r13, [->lab]
    )}),
    epilogue: Some(|mut f| { dynasm!(f
        ; mov [rsp], r14
    )}),
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; call r13; ->lab:) }),
    ],
};


 
pub static GRP_RETURN: TestGroup = TestGroup {
    name: "Return instructions",
    floor: None,
    common_measured: None,
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
        TestEmitter::new_anon(|mut f| { dynasm!(f ; ret; ->lab:) }),
    ],
};
 




pub static GRP_RR_MOV: TestGroup = TestGroup {
    name: "Register-to-register moves",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov rax, rax); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov eax, eax); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  ax,  ax); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  ah,  ah); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  al,  al); }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; mov rax, rbx); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov eax, ebx); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  ax,  bx); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  ah,  bh); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  al,  bl); }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  al,  bh); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  ah,  bl); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  al,  ah); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  ah,  al); }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; mov rax, rsp); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov eax, esp); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  ax,  sp); }),
    ],
};

pub static GRP_IR_MOV: TestGroup = TestGroup {
    name: "Immediate-to-register moves",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov rax, 0); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov rax, 0xdead); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov eax, 0); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov eax, 0xdead); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  ax, 0); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  ax, 0xde); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  ah, 0); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  ah, 0xde as _); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  al, 0); }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov  al, 0xde as _); }),

    ],
};

pub static GRP_MR_MOV: TestGroup = TestGroup {
    name: "Memory-to-register moves (loads)",
    floor: None,
    common_measured: None,
    prologue: Some(|mut f| {
        dynasm!(f; prefetch [0x0000_0080])
    }),
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov    rax, [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov    eax, [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov     ax, [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov     ah, [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov     al, [0x0000_0080]) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; movbe  rax, [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movbe  eax, [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movbe   ax, [0x0000_0080]) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; movsx  rax, [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movsx  eax, [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movsx   ax, [0x0000_0080]) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; movsxd rax, [0x0000_0080]) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; movzx  rax, [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movzx  eax, [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movzx   ax, [0x0000_0080]) }),
    ],
};

pub static GRP_RM_MOV: TestGroup = TestGroup {
    name: "Register-to-Memory moves (stores)",
    floor: None,
    common_measured: None,
    prologue: Some(|mut f| {
        //dynasm!(f; prefetch [0x0000_0080])
    }),
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov    [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov    [0x0000_0080], eax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov    [0x0000_0080],  ax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov    [0x0000_0080],  ah) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mov    [0x0000_0080],  al) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; movbe  [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movbe  [0x0000_0080], eax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movbe  [0x0000_0080],  ax) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; movnti [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; movnti [0x0000_0080], eax) }),
    ],
};

pub static GRP_LEA: TestGroup = TestGroup {
    name: "LEA instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; lea rax, [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lea rax, [0x0000_0080 * 8]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lea rax, [0x0000_0080 + rax]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lea rax, [0x0000_0080 + rax * 8]) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; lea rax, [rip]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lea rax, [rip + 0x0000_0020]) }),

        TestEmitter::new_anon(|mut f| { dynasm!(f; lea rax, [rsp]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lea rax, [rsp + 0x0000_0020]) }),
    ],
};

pub static GRP_BMI: TestGroup = TestGroup {
    name: "Bit-manipulation instructions",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[

        // Base instruction set
        TestEmitter::new_anon(|mut f| { dynasm!(f; bsf rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; bsr rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; bt  rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; btc rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; btr rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; bts rax, rax) }),

        // BMI1
        TestEmitter::new_anon(|mut f| { dynasm!(f; andn   rax, rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; blsi   rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; blsmsk rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; blsr   rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; tzcnt  rax, rax) }),

        // BMI2
        TestEmitter::new_anon(|mut f| { dynasm!(f; bzhi   rax, rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; mulx   rax, rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; pext   rbx, rbx, rbx) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; pdep   rbx, rbx, rbx) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; rorx   rax, rax, 0) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; sarx   rax, rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; shlx   rax, rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; shrx   rax, rax, rax) }),

        // The register form is from the BMI extension,
        // but the immediate form is from TBM (unsupported)
        TestEmitter::new_anon(|mut f| { dynasm!(f; bextr   rax, rax, rax) }),

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
        TestEmitter::new_anon(|mut f| { dynasm!(f; lzcnt  rax, rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; popcnt rax, rax) }),

    ],
};

pub static GRP_LOCK_STORES: TestGroup = TestGroup {
    name: "LOCK prefix stores",
    floor: None,
    common_measured: None,
    prologue: None,
    epilogue: None,
    emitters: &[
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock adc [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock add [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock and [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock btc [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock btr [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock bts [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock cmpxchg [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock cmpxchg8b [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock cmpxchg16b [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock or [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock sbb [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock sub [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock xadd [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock xor [0x0000_0080], rax) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock dec [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock inc [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock neg [0x0000_0080]) }),
        TestEmitter::new_anon(|mut f| { dynasm!(f; lock not [0x0000_0080]) }),
    ],
};


