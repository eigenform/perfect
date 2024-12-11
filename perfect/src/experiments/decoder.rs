
use rand::prelude::*;
use rand::Rng;
use rand::distributions::{ Distribution, Standard };
use iced_x86::{ 
    Decoder, DecoderOptions, Instruction, Formatter, IntelFormatter 
};


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyPrefix {
    Group1(LegacyPrefix1),
    Group2(LegacyPrefix2),
    Group3(LegacyPrefix3),
}
impl LegacyPrefix { 
    pub fn as_byte(&self) -> u8 {
        match self { 
            Self::Group1(p) => *p as u8,
            Self::Group2(p) => *p as u8,
            Self::Group3(p) => *p as u8,
        }
    }
}
impl Distribution<LegacyPrefix> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> LegacyPrefix {
        match rng.gen_range(1..=3) {
            1 => LegacyPrefix::Group1(rng.gen()),
            2 => LegacyPrefix::Group2(rng.gen()),
            3 => LegacyPrefix::Group3(rng.gen()),
            _ => unreachable!(),
        }
    }
}


#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyPrefix1 {
    ReptLock    = 0xf0,
    RepRepe     = 0xf3,
    Repne       = 0xf2,
}
impl Distribution<LegacyPrefix1> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> LegacyPrefix1 {
        match rng.gen_range(1..=3) {
            1 => LegacyPrefix1::ReptLock,
            2 => LegacyPrefix1::RepRepe,
            3 => LegacyPrefix1::Repne,
            _ => unreachable!(),
        }
    }
}


#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyPrefix2 {
    Es          = 0x26,
    Cs          = 0x2e,
    Ss          = 0x36,
    Ds          = 0x3e,
    Fs          = 0x64,
    Gs          = 0x65,
}
impl Distribution<LegacyPrefix2> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> LegacyPrefix2 {
        match rng.gen_range(1..=6) {
            1 => LegacyPrefix2::Es,
            2 => LegacyPrefix2::Cs,
            3 => LegacyPrefix2::Ss,
            4 => LegacyPrefix2::Ds,
            5 => LegacyPrefix2::Fs,
            6 => LegacyPrefix2::Gs,
            _ => unreachable!(),
        }
    }
}



#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyPrefix3 {
    OperandSize = 0x66,
    AddressSize = 0x67,
}
impl Distribution<LegacyPrefix3> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> LegacyPrefix3 {
        match rng.gen_range(1..=2) {
            1 => LegacyPrefix3::OperandSize,
            2 => LegacyPrefix3::AddressSize,
            _ => unreachable!(),
        }
    }
}


// 0100_wrxb
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RexPrefix(u8);
impl RexPrefix { pub fn as_byte(&self) -> u8 { self.0 } }
impl Distribution<RexPrefix> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RexPrefix {
        let val: u8 = rng.gen_range(0b0000_0000..=0b0000_1111);
        RexPrefix(0b0100_0000 | val)
    }
}

// ddrrrmmm
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModRm(u8);
impl ModRm { pub fn as_byte(&self) -> u8 { self.0 } }
impl Distribution<ModRm> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ModRm {
        ModRm(rng.gen())
    }
}

// ssiiibbb
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sib(u8);
impl Sib { pub fn as_byte(&self) -> u8 { self.0 } }
impl Distribution<Sib> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Sib {
        Sib(rng.gen())
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegacyOpcode(u8);
impl Distribution<LegacyOpcode> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> LegacyOpcode {
        LegacyOpcode(rng.gen())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opcode38(u8);
impl Distribution<Opcode38> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Opcode38 {
        Opcode38(rng.gen())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opcode3a(u8);
impl Distribution<Opcode3a> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Opcode3a {
        Opcode3a(rng.gen())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpcodeVex3(u8, u8);
impl Distribution<OpcodeVex3> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OpcodeVex3 {
        OpcodeVex3(rng.gen(), rng.gen())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpcodeVex2(u8);
impl Distribution<OpcodeVex2> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OpcodeVex2 {
        OpcodeVex2(rng.gen())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpcodeXop3(u8, u8);
impl Distribution<OpcodeXop3> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OpcodeXop3 {
        OpcodeXop3(rng.gen(), rng.gen())
    }
}




#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode { 
    Op(LegacyOpcode),
    Op38(Opcode38),
    Op3a(Opcode3a),
    OpVex3(OpcodeVex3),
    OpVex2(OpcodeVex2),
    OpXop3(OpcodeXop3),

}
impl Distribution<Opcode> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Opcode {
        match rng.gen_range(1..=6) { 
            1 => Opcode::Op(rng.gen()),
            2 => Opcode::Op38(rng.gen()),
            3 => Opcode::Op3a(rng.gen()),
            4 => Opcode::OpVex3(rng.gen()),
            5 => Opcode::OpVex2(rng.gen()),
            6 => Opcode::OpXop3(rng.gen()),
            _ => unreachable!(),
        }
    }
}
impl Opcode { 
    pub fn as_bytes(&self) -> Vec<u8> {
        match self { 
            Self::Op(op) => [op.0].to_vec(),
            Self::Op38(op) => [0x0f, 0x38, op.0].to_vec(),
            Self::Op3a(op) => [0x0f, 0x3a, op.0].to_vec(),
            Self::OpVex3(op) => [0xc4, op.0, op.1].to_vec(),
            Self::OpVex2(op) => [0xc5, op.0].to_vec(),
            Self::OpXop3(op) => [0x8f, op.0, op.1].to_vec(),
        }
    }
}

#[derive(Clone)]
pub struct RandomEncoding<const MAX: usize>(pub Vec<u8>);
impl <const MAX: usize> RandomEncoding<MAX> { 
    pub fn as_bytes(&self) -> [u8; MAX] {
        let mut buf = [0x90; MAX];
        let mut enc_len = if self.0.len() < MAX { self.0.len() } else { MAX };
        assert!(enc_len <= MAX);
        let slice = &mut buf[0..enc_len];
        assert!(slice.len() == enc_len);
        slice.copy_from_slice(&self.0[0..enc_len]);
        buf
    }
}

impl <const MAX: usize> Distribution<RandomEncoding<MAX>> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RandomEncoding<MAX> {
        const IMM_LENS: [usize; 5] = [0, 1, 2, 4, 8];

        let mut bytes = Vec::new();

        let has_rex: bool = rng.gen();
        let num_legacy_prefixes: usize = rng.gen_range(0..=4);
        let opcode_bytes = rng.gen::<Opcode>().as_bytes();

        let mut rlen = 16;



        // Variable number of random legacy prefixes
        for _ in 0..num_legacy_prefixes {
            bytes.push(rng.gen::<LegacyPrefix>().as_byte());
        }
        rlen = rlen - num_legacy_prefixes;

        // Random REX prefix
        if has_rex { 
            bytes.push(rng.gen::<RexPrefix>().as_byte());
            rlen = rlen - 1;
        }

        // Random opcode bytes
        bytes.extend_from_slice(&opcode_bytes);
        rlen = rlen - opcode_bytes.len();

        // Random ModR/M byte
        bytes.push(rng.gen::<ModRm>().as_byte());
        rlen = rlen - 1;

        // Random SIB byte
        bytes.push(rng.gen::<Sib>().as_byte());
        rlen = rlen - 1;

        // Fill the rest of the encoding with random disp/imm bytes
        for _ in 0..rlen { 
            bytes.push(rng.gen::<u8>());
        }

        //// Random displacement?
        //let size = IMM_LENS.choose(rng).unwrap();
        //for _ in 0..*size {
        //    bytes.push(rng.gen::<u8>());
        //}

        //// Random immediate?
        //let size = IMM_LENS.choose(rng).unwrap();
        //for _ in 0..*size {
        //    bytes.push(rng.gen::<u8>());
        //}

        RandomEncoding(bytes)
    }
}

