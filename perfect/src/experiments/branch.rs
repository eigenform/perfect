
use crate::asm::*;
use dynasmrt::{
    DynasmApi,
    DynasmLabelApi,
    DynasmError,
    AssemblyOffset,
    Assembler,
    dynasm,
    x64::X64Relocation,
    DynamicLabel,
    LabelKind,
    TargetKind,
};

fn align_down(addr: usize, bits: usize) -> usize {
    let align: usize = (1 << bits);
    let mask: usize  = !(align - 1);
    (addr & mask).wrapping_sub(align)
}


pub trait AsPattern {
    fn to_pattern(&self) -> BranchPattern;
}
impl AsPattern for u8 {
    fn to_pattern(&self) -> BranchPattern {
        let mut res = vec![0; 8];
        for idx in 0..8 {
            res[idx] = ((self & (1 << idx)) != 0) as u8;
        }
        BranchPattern(res)
    }
}
impl AsPattern for u16 {
    fn to_pattern(&self) -> BranchPattern {
        let mut res = vec![0; 16];
        for idx in 0..16 {
            res[idx] = ((self & (1 << idx)) != 0) as u8;
        }
        BranchPattern(res)
    }
}
impl AsPattern for u32 {
    fn to_pattern(&self) -> BranchPattern {
        let mut res = vec![0; 32];
        for idx in (0..32) {
            res[32 - idx - 1] = ((self & (1 << idx)) != 0) as u8;
            //res[idx] = ((this & (1 << (31-idx))) != 0) as u8;
        }
        BranchPattern(res)
    }
}




pub struct BranchPattern(pub Vec<u8>);
impl BranchPattern {
    pub fn new<T: Copy + Into<u8>>(slice: &[T]) -> Self {
        let mut res = Vec::new();
        for val in slice.iter() {
            res.push((*val).into());
        }
        Self(res)
    }
    pub fn from_number<T: AsPattern>(num: T) -> Self {
        num.to_pattern()
    }
}

pub struct BranchOutcomes {
    data: Vec<usize>,
}
impl BranchOutcomes {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
        }
    }
    pub fn from_pattern<T: Clone + Into<usize>>(size: usize, pattern: &[T]) -> Self
    {
        let mut data = vec![0; size];
        for (idx, val) in data.iter_mut().enumerate() {
            let x = pattern[idx % pattern.len()].clone();
            *val = x.into();
        }
        Self { data }
    }
    pub fn append_from_slice<T: Copy + Into<usize>>(&mut self, slice: &[T]) {
        for val in slice.iter() {
            self.data.push((*val).into());
        }
    }

    pub fn to_rdi_inputs(&self, size: usize) -> Vec<(usize, usize)> {
        let mut res = vec![(0, 0); size];
        for (idx, x) in self.data.iter().enumerate() {
            res[idx].0 = *x as usize;
        }
        res
    }

}


pub struct ConditionalBranch;
impl ConditionalBranch {
    pub fn emit_je_nopad(brn_addr: usize, tgt_addr: usize) 
        -> X64AssemblerFixed
    {
        assert!(brn_addr < 0x0000_7000_0000_0000);
        assert!(tgt_addr > brn_addr);

        let base_addr = align_down(brn_addr, 16);

        let tgt_off = tgt_addr - brn_addr;
        assert!(tgt_off < 0x0000_0001_0000_0000);

        let mut asm = X64AssemblerFixed::new(base_addr, 0x0000_0001_8000_0000);

        asm.pad_until(brn_addr - 0x18);
        asm.emit_rdpmc_start(0, Gpr::R15 as u8);
        assert_eq!(asm.cur_addr(), brn_addr);
        let tgt = asm.new_dynamic_label();
        if tgt_off < 128 {
            dynasm!(asm ; je BYTE =>tgt);
        } else {
            dynasm!(asm ; je =>tgt);
        }

        asm.pad_until(tgt_addr);
        assert_eq!(asm.cur_addr(), tgt_addr);
        asm.place_dynamic_label(tgt);
        asm.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        asm.emit_ret();
        asm.commit().unwrap();
        asm
    }


}

#[cfg(test)]
mod test {
    use super::*;
}
