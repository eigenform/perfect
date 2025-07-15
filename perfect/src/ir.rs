
pub mod branch;
pub mod bf; 

use crate::asm::*;

use rand::prelude::*;
use rand::Rng;
use rand::distributions::{Distribution, Standard};
pub use dynasmrt::{
    dynasm, 
    DynasmApi, 
    DynasmLabelApi, 
    DynamicLabel,
    components::StaticLabel,
    Assembler, 
    AssemblyOffset, 
    ExecutableBuffer, 
    Executor,
};


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IRRegOperand {
    Rax = 0,
    Rcx = 1,
    Rdx = 2,
    Rbx = 3,
}
impl Distribution<IRRegOperand> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> IRRegOperand {
        let r = rng.gen_range(0..=3);
        match r {
            00 => IRRegOperand::Rax,
            01 => IRRegOperand::Rcx,
            02 => IRRegOperand::Rdx,
            03 => IRRegOperand::Rbx,
            _ => unreachable!(),
        }
    }
}


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IRImmOperand {
    Imm32(i32),
    Imm64(i64),
}
impl Distribution<IRImmOperand> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> IRImmOperand {
        let r = rng.gen_range(0..=1);
        match r {
            00 => {
                let imm = rng.gen_range(0x0000_0000..=0x1fff_ffff);
                IRImmOperand::Imm32(imm)
            },
            01 => {
                let imm = rng.gen_range(
                    0x0000_0000_0000_0000..=0x1fff_ffff_ffff_ffff
                );
                IRImmOperand::Imm64(imm)
            },
            _ => unreachable!(),
        }
    }
}


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IRMemOperand {
    Base,
    BaseImm32(i32),
    MemImm32(i32),
}
impl Distribution<IRMemOperand> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> IRMemOperand {
        let r = rng.gen_range(0..=2);
        match r {
            00 => {
                IRMemOperand::Base
            },
            01 => {
                let imm = rng.gen_range(0..=0x1000);
                IRMemOperand::BaseImm32(imm)
            },
            02 => {
                let imm = rng.gen_range(8..=0x3f8) & 0b111;
                IRMemOperand::MemImm32(imm)
            },
            _ => unreachable!(),
        }
    }
}


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IRAnyOperand {
    Reg(IRRegOperand),
    Mem(IRMemOperand),
    Imm(IRImmOperand),
}
impl Distribution<IRAnyOperand> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> IRAnyOperand {
        let r = rng.gen_range(0..=2);
        match r {
            00 => IRAnyOperand::Reg(rng.gen()),
            01 => IRAnyOperand::Mem(rng.gen()),
            02 => IRAnyOperand::Imm(rng.gen()),
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PerfectOpWidth { Qword, Dword, Word }
impl Distribution<PerfectOpWidth> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> PerfectOpWidth {
        let r = rng.gen_range(0..=2);
        match r {
            00 => PerfectOpWidth::Qword,
            01 => PerfectOpWidth::Dword,
            02 => PerfectOpWidth::Word,
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PerfectOp {
    Nop,

    MovImm(IRRegOperand, IRImmOperand, PerfectOpWidth),

    Mov(IRRegOperand, IRRegOperand, PerfectOpWidth),
    Add(IRRegOperand, IRRegOperand, PerfectOpWidth),
    Xor(IRRegOperand, IRRegOperand, PerfectOpWidth),
    ZeroIdiom(IRRegOperand, PerfectOpWidth),

    Xchg64(IRRegOperand, IRRegOperand),
    Xchg32(IRRegOperand, IRRegOperand),

    Load(IRRegOperand, IRMemOperand, PerfectOpWidth),
    Store(IRMemOperand, IRRegOperand, PerfectOpWidth),

    Movzx64_16(IRRegOperand, IRRegOperand),
    Movzx64_8(IRRegOperand, IRRegOperand),
    Movzx32_16(IRRegOperand, IRRegOperand),
    Movzx32_8(IRRegOperand, IRRegOperand),
    Movzx16_8(IRRegOperand, IRRegOperand),
}
impl Distribution<PerfectOp> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> PerfectOp {
        let r = rng.gen_range(0..=8);
        let dst_reg: IRRegOperand = rng.gen();
        let src_reg: IRRegOperand = rng.gen();
        let mem: IRMemOperand = rng.gen();
        let width: PerfectOpWidth = rng.gen();
        match r {
            00 => PerfectOp::Nop,
            01 => PerfectOp::Mov(dst_reg, src_reg, width),
            02 => PerfectOp::Add(dst_reg, src_reg, width),
            03 => PerfectOp::Xor(dst_reg, src_reg, width),
            04 => PerfectOp::ZeroIdiom(dst_reg, width),
            05 => PerfectOp::Xchg64(dst_reg, src_reg),
            06 => PerfectOp::Xchg32(dst_reg, src_reg),
            07 => PerfectOp::Load(dst_reg, mem, width),
            08 => PerfectOp::Store(mem, src_reg, width),
            09 => PerfectOp::Movzx64_16(dst_reg, src_reg),
            10 => PerfectOp::Movzx64_8(dst_reg, src_reg),
            11 => PerfectOp::Movzx32_16(dst_reg, src_reg),
            12 => PerfectOp::Movzx32_8(dst_reg, src_reg),
            13 => PerfectOp::Movzx16_8(dst_reg, src_reg),

            _ => unreachable!(),
        }
    }
}



impl PerfectOp {
    const ARENA_REG: Gpr = Gpr::R10;
    pub fn emit(&self, f: &mut X64Assembler) {
        match self { 
            Self::MovImm(dst, imm, width) => { match (imm, width) {
                (IRImmOperand::Imm32(val), PerfectOpWidth::Qword) => {
                    dynasm!(f ; mov Rq(*dst as u8), QWORD *val as i64);
                },
                (IRImmOperand::Imm32(val), PerfectOpWidth::Dword) => {
                    dynasm!(f ; mov Rd(*dst as u8), DWORD *val);
                },
                (IRImmOperand::Imm32(val), PerfectOpWidth::Word) => {
                    let v = (*val as u32 & 0x7fff) as i16;
                    dynasm!(f ; mov Rw(*dst as u8), WORD v);
                },
                (IRImmOperand::Imm64(val), PerfectOpWidth::Qword) => {
                    dynasm!(f ; mov Rq(*dst as u8), QWORD *val);
                },
                (IRImmOperand::Imm64(val), PerfectOpWidth::Dword) => {
                    dynasm!(f ; mov Rd(*dst as u8), DWORD *val as i32);
                },
                (IRImmOperand::Imm64(val), PerfectOpWidth::Word) => {
                    let v = (*val as u32 & 0x7fff) as i16;
                    dynasm!(f ; mov Rw(*dst as u8), WORD v);
                },
            }},

            Self::Mov(dst, src, width) => { match width {
                PerfectOpWidth::Qword => {
                    dynasm!(f ; mov Rq(*dst as u8), Rq(*src as u8));
                },
                PerfectOpWidth::Dword => {
                    dynasm!(f ; mov Rd(*dst as u8), Rd(*src as u8));
                },
                PerfectOpWidth::Word => {
                    dynasm!(f ; mov Rw(*dst as u8), Rw(*src as u8));
                },
            }},

            Self::Add(dst, src, width) => { match width {
                PerfectOpWidth::Qword => {
                    dynasm!(f ; add Rq(*dst as u8), Rq(*src as u8));
                },
                PerfectOpWidth::Dword => {
                    dynasm!(f ; add Rd(*dst as u8), Rd(*src as u8));
                },
                PerfectOpWidth::Word => {
                    dynasm!(f ; add Rw(*dst as u8), Rw(*src as u8));
                },
            }},

            Self::Xor(dst, src, width) => { match width {
                PerfectOpWidth::Qword => {
                    dynasm!(f ; xor Rq(*dst as u8), Rq(*src as u8));
                },
                PerfectOpWidth::Dword => {
                    dynasm!(f ; xor Rd(*dst as u8), Rd(*src as u8));
                },
                PerfectOpWidth::Word => {
                    dynasm!(f ; xor Rw(*dst as u8), Rw(*src as u8));
                },
            }},

            Self::ZeroIdiom(src, width) => { match width {
                PerfectOpWidth::Qword => {
                    dynasm!(f ; xor Rq(*src as u8), Rq(*src as u8));
                },
                PerfectOpWidth::Dword => {
                    dynasm!(f ; xor Rd(*src as u8), Rd(*src as u8));
                },
                PerfectOpWidth::Word => {
                    dynasm!(f ; xor Rw(*src as u8), Rw(*src as u8));
                },
            }},

            Self::Xchg64(dst, src) => {
                dynasm!(f; xchg Rq(*dst as u8), Rq(*src as u8));
            },
            Self::Xchg32(dst, src) => {
                dynasm!(f; xchg Rd(*dst as u8), Rd(*src as u8));
            },

            Self::Load(dst, src, width) => { match (src, width) {
                (IRMemOperand::Base, PerfectOpWidth::Qword) => {
                    dynasm!(f ; mov Rq(*dst as u8), [Rq(Self::ARENA_REG as u8)]);
                },
                (IRMemOperand::Base, PerfectOpWidth::Dword) => {
                    dynasm!(f ; mov Rd(*dst as u8), [Rq(Self::ARENA_REG as u8)]);
                },
                (IRMemOperand::Base, PerfectOpWidth::Word) => {
                    dynasm!(f ; mov Rw(*dst as u8), [Rq(Self::ARENA_REG as u8)]);
                },
                (IRMemOperand::BaseImm32(disp), PerfectOpWidth::Qword) => {
                    dynasm!(f ; mov Rq(*dst as u8), [Rq(Self::ARENA_REG as u8) + *disp]);
                },
                (IRMemOperand::BaseImm32(disp), PerfectOpWidth::Dword) => {
                    dynasm!(f ; mov Rd(*dst as u8), [Rq(Self::ARENA_REG as u8) + *disp]);
                },
                (IRMemOperand::BaseImm32(disp), PerfectOpWidth::Word) => {
                    dynasm!(f ; mov Rw(*dst as u8), [Rq(Self::ARENA_REG as u8) + *disp]);
                },

                (IRMemOperand::MemImm32(disp), PerfectOpWidth::Qword) => {
                    dynasm!(f ; mov Rq(*dst as u8), [*disp]);
                },
                (IRMemOperand::MemImm32(disp), PerfectOpWidth::Dword) => {
                    dynasm!(f ; mov Rd(*dst as u8), [*disp]);
                },
                (IRMemOperand::MemImm32(disp), PerfectOpWidth::Word) => {
                    dynasm!(f ; mov Rw(*dst as u8), [*disp]);
                },
            }},

            Self::Store(dst, src, width) => { match (dst, width) {
                (IRMemOperand::Base, PerfectOpWidth::Qword) => {
                    dynasm!(f ; mov [Rq(Self::ARENA_REG as u8)], Rq(*src as u8));
                },
                (IRMemOperand::Base, PerfectOpWidth::Dword) => {
                    dynasm!(f ; mov [Rq(Self::ARENA_REG as u8)], Rd(*src as u8));
                },
                (IRMemOperand::Base, PerfectOpWidth::Word) => {
                    dynasm!(f ; mov [Rq(Self::ARENA_REG as u8)], Rw(*src as u8));
                },
                (IRMemOperand::BaseImm32(disp), PerfectOpWidth::Qword) => {
                    dynasm!(f ; mov [Rq(Self::ARENA_REG as u8) + *disp], Rq(*src as u8));
                },
                (IRMemOperand::BaseImm32(disp), PerfectOpWidth::Dword) => {
                    dynasm!(f ; mov [Rq(Self::ARENA_REG as u8) + *disp], Rd(*src as u8));
                },
                (IRMemOperand::BaseImm32(disp), PerfectOpWidth::Word) => {
                    dynasm!(f ; mov [Rq(Self::ARENA_REG as u8) + *disp], Rw(*src as u8));
                },
                (IRMemOperand::MemImm32(disp), PerfectOpWidth::Qword) => {
                    dynasm!(f ; mov [*disp], Rq(*src as u8));
                },
                (IRMemOperand::MemImm32(disp), PerfectOpWidth::Dword) => {
                    dynasm!(f ; mov [*disp], Rd(*src as u8));
                },
                (IRMemOperand::MemImm32(disp), PerfectOpWidth::Word) => {
                    dynasm!(f ; mov [*disp], Rw(*src as u8));
                },
            }},

            Self::Movzx64_16(dst, src) => {
                dynasm!(f ; movzx Rq(*dst as u8), Rw(*src as u8));
            },
            Self::Movzx64_8(dst, src) => {
                dynasm!(f ; movzx Rq(*dst as u8), Rb(*src as u8));
            },
            Self::Movzx32_16(dst, src) => {
                dynasm!(f ; movzx Rd(*dst as u8), Rw(*src as u8));
            },
            Self::Movzx32_8(dst, src) => {
                dynasm!(f ; movzx Rd(*dst as u8), Rb(*src as u8));
            },
            Self::Movzx16_8(dst, src) => {
                dynasm!(f ; movzx Rw(*dst as u8), Rb(*src as u8));
            },

            _ => todo!(),
        }
    }
}

pub struct PerfectVm {
    pub gpr: [usize; 4],
}
impl PerfectVm {
    pub fn new(init: &[usize; 4]) -> Self {
        Self { gpr: *init }
    }
    pub fn clear(&mut self, init: &[usize; 4]) {
        self.gpr = *init;
    }
    pub fn read_reg(&self, gpr: IRRegOperand) -> usize { 
        self.gpr[gpr as usize]
    }
    pub fn write_reg(&mut self, gpr: IRRegOperand, value: usize) {
        self.gpr[gpr as usize] = value;
    }
    pub fn evaluate_program(&mut self, prog: &PerfectProg) {
        for op in prog.data.iter() {
        }
    }
}

#[derive(Clone, Debug)]
pub struct PerfectProg {
    pub data: Vec<PerfectOp>,
}
impl PerfectProg {
    pub fn len(&self) -> usize { self.data.len() }
    pub fn gen(len: usize) -> Self { 
        let mut data = Vec::new();
        for _ in 0..len {
            data.push(thread_rng().gen());
        }
        Self { data }
    }

    pub fn emit(&self, f: &mut X64Assembler) {
        for irop in &self.data {
            irop.emit(f);
        }
    }

    pub fn apply_to_vm(&self, vm: &mut PerfectVm) {
        vm.evaluate_program(self);
    }
}


pub struct PerfectIR; 
impl PerfectIR {
}


