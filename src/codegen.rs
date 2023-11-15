
pub use rand::prelude::*;
pub use rand::Rng;
pub use rand::distributions::{Distribution, Standard};

pub use dynasmrt::{
    dynasm, 
    DynasmApi, 
    DynasmLabelApi, 
    DynamicLabel,
    Assembler, 
    AssemblyOffset, 
    ExecutableBuffer, 
    Executor,
    x64::X64Relocation
};

use crate::ir::*;

pub const NOP2:  [u8; 2] = [ 0x66, 0x90 ];
pub const NOP3:  [u8; 3] = [ 0x0f, 0x1f, 0x00 ];
pub const NOP4:  [u8; 4] = [ 0x0f, 0x1f, 0x40, 0x00 ];
pub const NOP5:  [u8; 5] = [ 0x0f, 0x1f, 0x44, 0x00, 0x00 ];
pub const NOP6:  [u8; 6] = [ 0x66, 0x0f, 0x1f, 0x44, 0x00, 0x00 ];
pub const NOP7:  [u8; 7] = [ 0x0f, 0x1f, 0x80, 0x00, 0x00, 0x00, 0x00 ];
pub const NOP8:  [u8; 8] = [ 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00 ];
pub const NOP9:  [u8; 9] = [ 
    0x66, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00 
];
pub const NOP15: [u8; 15] = [ 
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 
    0x0F, 0x1F, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00
];

// Setup the target of a future indirect jump
pub fn lea_new_indir_target(
    asm: &mut Assembler<X64Relocation>, 
    reg: u8,
) -> DynamicLabel 
{
    let label = asm.new_dynamic_label();

    dynasm!(asm
        ; lea Rq(reg), [=>label]
    );
    //asm.commit().unwrap();
    label
}

// Actually define the label
pub fn set_indir_target(
    asm: &mut Assembler<X64Relocation>,
    label: DynamicLabel, align: Option<usize>)
{
    if let Some(x) = align {
        dynasm!(asm
            ; .align x
            ; ud2
            ; =>label
        );
    } 
    else {
        dynasm!(asm
            ; ud2
            ; =>label
        );
    }
}




pub fn emit_ret(asm: &mut Assembler<X64Relocation>) {
    dynasm!(asm ; ret);
    //asm.commit().unwrap();
}


pub fn emit_btb_flush(asm: &mut Assembler<X64Relocation>) {
    for _ in 0..8192 {
        dynasm!(asm
            ; jmp >label
            ; label:
        );
    }
    dynasm!(asm ; lfence);
    //asm.commit().unwrap();
}

pub fn emit_align(asm: &mut Assembler<X64Relocation>, align: usize) {
    dynasm!(asm 
        ; .align align
    );
}

pub fn emit_rdpmc_start_i5(asm: &mut Assembler<X64Relocation>, reg: u8) {
    dynasm!(asm 
        ; nop
        ; nop
        ; mov rcx, 1
        ; xor Rq(reg), Rq(reg)
        ; lfence

        ; nop
        ; nop
        ; nop
        ; rdpmc
        ; lfence

        ; sub Rq(reg), rax
        ; nop
        ; nop
        ; nop
        ; nop
    );
}

pub fn emit_rdpmc_end_i5(asm: &mut Assembler<X64Relocation>, reg: u8) {
    dynasm!(asm 
        ; mov rcx, 1
        ; nop
        ; nop
        ; nop
        ; lfence

        ; nop
        ; nop
        ; nop
        ; rdpmc
        ; lfence

        ; add rax, Rq(reg)
        ; nop
        ; nop
        ; nop
        ; nop
    );
    //asm.commit().unwrap();
}



pub fn emit_rdpmc_start(asm: &mut Assembler<X64Relocation>, reg: u8) {
    dynasm!(asm 
        //; .align 64
        ; lfence
        ; mov rcx, 1
        ; xor Rq(reg), Rq(reg)
        ; lfence
        ; rdpmc
        ; lfence
        ; sub Rq(reg), rax
        ; lfence
    );
    //asm.commit().unwrap();
}

pub fn emit_rdpmc_end(asm: &mut Assembler<X64Relocation>, reg: u8) {
    dynasm!(asm 
        ; mov rcx, 1
        ; lfence
        ; rdpmc
        ; lfence
        ; add rax, Rq(reg)
        ; lfence
    );
    //asm.commit().unwrap();
}

pub fn emit_mov_r64_r64(asm: &mut Assembler<X64Relocation>, dst: u8, src: u8) {
    dynasm!(asm ; mov Rq(dst), Rq(src));
    //asm.commit().unwrap();
}

pub fn emit_mov_qword_imm(asm: &mut Assembler<X64Relocation>, reg: u8, qword: i64) {
    dynasm!(asm ; mov Rq(reg), QWORD qword);
    //asm.commit().unwrap();
}

pub fn emit_lfence_trailing(asm: &mut Assembler<X64Relocation>) {
    dynasm!(asm 
        ; .align 64
        ; .bytes NOP8
        ; .bytes NOP8
        ; .bytes NOP8
        ; .bytes NOP8

        ; .bytes NOP8
        ; .bytes NOP8
        ; .bytes NOP8
        ; .bytes NOP5
        ; lfence
    );
}

pub fn emit_lfence(asm: &mut Assembler<X64Relocation>) {
    dynasm!(asm ; lfence);
    //asm.commit().unwrap();
}

pub fn emit_indirect_jmp(asm: &mut Assembler<X64Relocation>, reg: u8) {
    dynasm!(asm ; jmp Rq(reg));
    //asm.commit().unwrap();
}

pub fn emit_indirect_call(asm: &mut Assembler<X64Relocation>, reg: u8) {
    dynasm!(asm ; call Rq(reg));
    //asm.commit().unwrap();
}

pub fn emit_add_r64_r64(asm: &mut Assembler<X64Relocation>, dst: u8, src: u8) {
    dynasm!(asm; add Rq(dst), Rq(src));
    //asm.commit().unwrap();
}

pub fn emit_nop_n(asm: &mut Assembler<X64Relocation>, n: usize) {
    for _ in 0..n {
        dynasm!(asm; nop);
    }
    //asm.commit().unwrap();
}

pub fn emit_fnop_n(asm: &mut Assembler<X64Relocation>, n: usize) {
    for _ in 0..n {
        dynasm!(asm; fnop);
    }
    //asm.commit().unwrap();
}

pub fn emit_magic_fnop_padding(asm: &mut Assembler<X64Relocation>, n: usize) {
    for _ in 0..n {
        dynasm!(asm; nop);
    }
    dynasm!(asm; fnop);
    //asm.commit().unwrap();
}



pub fn emit_xor_clear(asm: &mut Assembler<X64Relocation>, reg: u8) {
    dynasm!(asm; xor Rq(reg), Rq(reg));
    //asm.commit().unwrap();
}


