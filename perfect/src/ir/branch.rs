
use crate::asm::*;
use crate::util::Align;
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
    x64::X64Relocation,
};

#[derive(Clone)]
pub struct BranchSet {
    pub data: Vec<BranchDesc>,
}
impl BranchSet {

    /// Generate a set of uniformly-spaced aligned branches.
    pub fn gen_uniform(start_addr: usize, align: Align, len: usize) 
        -> Self 
    {
        assert!(align.check(start_addr));
        let mut data = Vec::new();
        let mut addr = start_addr & align.index_mask();
        let mut tgt  = addr + align.value();
        for idx in 0..=len {
            if idx == 0 { continue; }
            data.push(BranchDesc::new(addr, tgt));
            addr += align.value();
            tgt += align.value();
        }
        Self { data }
    }

    /// Generate a set of uniformly-spaced branches sharing the same 
    /// offset bits.
    pub fn gen_uniform_offset(
        start_addr: usize, 
        align: Align,
        offset: usize,
        len: usize,
    ) -> Self 
    {
        assert!(align.check(start_addr));
        assert!(offset & align.index_mask() == 0);
        let mut data = Vec::new();
        let mut addr = (start_addr & align.index_mask()) | offset;
        let mut tgt  = addr + align.value();
        for idx in 0..=len {
            if idx == 0 { continue; }
            data.push(BranchDesc::new(addr, tgt));
            addr += align.value();
            tgt += align.value();
        }
        Self { data }
    }


    pub fn first(&self) -> Option<&BranchDesc> {
        self.data.first()
    }
    pub fn first_mut(&mut self) -> Option<&mut BranchDesc> {
        self.data.first_mut()
    }

    pub fn last(&self) -> Option<&BranchDesc> {
        self.data.last()
    }
    pub fn last_mut(&mut self) -> Option<&mut BranchDesc> {
        self.data.last_mut()
    }


}


#[derive(Clone, Copy, Debug)]
pub struct BranchDesc {
    /// The requested program counter for the branch instruction.
    pub addr: usize,
    /// The requested target address of the branch instruction. 
    pub tgt: usize,
}
impl BranchDesc {
    pub fn new(addr: usize, tgt: usize) -> Self {
        // NOTE: This only handles "forward" facing branches.
        // NOTE: I think the shortest encoding for jump is 2 bytes?
        assert!(tgt >= addr + 2);
        Self { addr, tgt }
    }

    /// Return the number of bytes in-between the address and target.
    pub fn offset(&self) -> usize { 
        self.tgt - self.addr
    }

    pub fn emit_jmp_direct(&self, f: &mut X64AssemblerFixed) {
        f.pad_until(self.addr);
        let lab = f.new_dynamic_label();
        dynasm!(f ; jmp =>lab);
        f.pad_until(self.tgt);
        f.place_dynamic_label(lab);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn branchset_uniform() {
        let align = Align::from_bit(5);
        let mut set = BranchSet::gen_uniform(0x1_0000_0000, align, 32);
        for brn in set.data.iter() {
            assert!(align.check(brn.addr));
            assert!(align.check(brn.tgt));
            println!("{:016x?}", brn);
        }
    }

    #[test]
    fn branchset_offset() {
        let align = Align::from_bit(5);
        let mut set = BranchSet::gen_uniform_offset
            (0x1_0000_0000, align, 0x0_0000_0004, 32);
        for brn in set.data.iter() {
            println!("{:016x?}", brn);
        }
    }

}
