
use rand::prelude::*;
use rand::Rng;
use rand::distributions::{Distribution, Standard};

use crate::asm::Gpr;

impl Distribution<Gpr> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Gpr {
        // assume r15 is reserved
        let r = rng.gen_range(0..=14);
        match r {
            0 => Gpr::Rax,
            1 => Gpr::Rcx,
            2 => Gpr::Rdx,
            3 => Gpr::Rbx,
            4 => Gpr::Rax,
            5 => Gpr::Rcx,
            6 => Gpr::Rsi,
            7 => Gpr::Rdi,
            8 => Gpr::R8,
            9 => Gpr::R9,
            10 => Gpr::R10,
            11 => Gpr::R11,
            12 => Gpr::R12,
            13 => Gpr::R13,
            14 => Gpr::R14,
            //15 => Gpr::Rdx,
            _ => unreachable!(),
        }
    }
}


// Distribution over GPRs *excluding* R15. 
pub struct GprDist;
impl Distribution<Gpr> for GprDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Gpr {
        Gpr::from(rng.gen_range(0..=14))
    }
}

// Distribution over GPRs *excluding* RSP, RBP, R15. 
pub struct GprNoStackDist;
impl GprNoStackDist {
    const SET: [Gpr; 13] = [
        Gpr::Rax, Gpr::Rcx, Gpr::Rdx, Gpr::Rbx,
        Gpr::Rsi, Gpr::Rdi, Gpr::R8, Gpr::R9,
        Gpr::R10, Gpr::R11, Gpr::R12, Gpr::R13,
        Gpr::R14,
    ];
}
impl Distribution<Gpr> for GprNoStackDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Gpr {
        Self::SET.choose(rng).unwrap().clone()
    }
}

// Distribution over GPRs *excluding* RSI, RDI, RSP, RBP, R15.
pub struct GprNoArgDist;
impl GprNoArgDist {
    const SET: [Gpr; 11] = [
        Gpr::Rax, Gpr::Rcx, Gpr::Rdx, Gpr::Rbx,
        Gpr::R8, Gpr::R9, Gpr::R10, Gpr::R11, 
        Gpr::R12, Gpr::R13, Gpr::R14,
    ];
}
impl Distribution<Gpr> for GprNoArgDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Gpr {
        Self::SET.choose(rng).unwrap().clone()
    }
}

pub fn random_gpr() -> Gpr {
    rand::random::<Gpr>()
}

pub struct Reg64(pub Gpr);
pub struct Reg32(pub Gpr);
pub struct Reg16(pub Gpr);
pub struct Reg8(pub Gpr);

pub enum Reg1Op {
    Mul, Neg, Not
}
pub enum Reg2Op {
    Mov, Add, Sub, And, Or, Xor
}

pub enum IRMovReg {
    Movsx16_8(Reg16, Reg8),
    Movsx32_8(Reg32, Reg8),
    Movsx32_16(Reg32, Reg16),
    Movsx64_8(Reg64, Reg8),
    Movsx64_16(Reg64, Reg16),

    Movsxd16_16(Reg16, Reg16),
    Movsxd32_32(Reg32, Reg32),
    Movsxd64_32(Reg64, Reg32),

    Movzx16_8(Reg16, Reg8),
    Movzx32_8(Reg32, Reg8),
    Movzx64_8(Reg64, Reg8),
    Movzx32_16(Reg32, Reg16),
    Movzx64_16(Reg64, Reg16),

    Mov64(Reg64, Reg64),
    Mov32(Reg32, Reg32),
    Mov16(Reg16, Reg16),
    Mov8(Reg8, Reg8),
}
