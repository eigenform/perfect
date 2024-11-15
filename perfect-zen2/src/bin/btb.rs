use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use perfect::ir::branch::*;
use perfect::util::*;
use itertools::*;
use std::collections::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .emit();
    BTBCapacity::run(&mut harness);
}



/// A hypothetical BTB addressing scheme. 
///
/// NOTE: The Family 17h SOG mentions the following: 
///
/// - There are 8 L0 entries (that's 3-bit)
/// - There are 256 L1 entries (that's 8-bit)
/// - There are 4096 L2 entries (that's 12-bit)
/// - An entry can hold two branches in the same 64-byte cacheline
/// - An entry can hold two branches if the first branch is conditional
/// - Branches whose *target* crosses a 19-bit boundary cannot share a BTB
///   entry with other branches
///

pub struct BTBConfig {
    pub offset_mask: usize,
    pub index_mask: usize,
    pub tag_mask: usize,
}
pub struct BTBAddress(pub usize);
impl BTBAddress {
    // NOTE: Just sketching *something* out...
    const OFFSET_MASK: usize = 0x0000_0000_0000_003f;
    const INDEX_MASK: usize  = 0x0000_0000_0003_ffc0;
    const TAG_MASK: usize    = 0xffff_ffff_fffc_0000;
    pub fn offset_bits(&self) -> usize {
        self.0 & Self::OFFSET_MASK
    }
    pub fn index_bits(&self) -> usize {
        (self.0 & Self::INDEX_MASK) >> 6
    }
    pub fn tag_bits(&self) -> usize {
        (self.0 & Self::TAG_MASK) >> 19
    }

    pub fn from_usize(x: usize) -> Self {
        Self(x)
    }

    const OFFSET_MASK2: usize = 0x0000_0000_0000_003f;
    const INDEX_MASK2: usize  = 0x0000_0000_0000_0fff;
    const TAG_MASK2: usize    = 0x0000_3fff_ffff_ffff;
    pub fn new(offset: usize, index: usize, tag: usize) -> Self {
        let offset = (offset & Self::OFFSET_MASK2);
        let index = (index & Self::INDEX_MASK2) << 6;
        let tag = (tag & Self::TAG_MASK2) << 19;
        Self(tag | index | offset)
    }
}
impl std::fmt::Display for BTBAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:016x}:{:010x}:{:04x}:{:02x}", 
            self.0, self.tag_bits(), self.index_bits(), self.offset_bits()
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BTBCapacityArgs {
    start_addr: usize,
    num_padding: usize,
    align: Align,
    test_offset: usize,
}
impl BTBCapacityArgs {
    pub fn test_addr(&self) -> usize { 
        self.start_addr + 
        (self.num_padding * self.align.value()) + 
        self.align.value() +
        self.test_offset
    }
}


/// Determine BTB capacity.
///
/// Context
/// =======
///
/// Test
/// ====
///
/// 1. Ensure that the BTB is polluted in the harness. 
///
///
/// Results
/// =======
///

pub struct BTBCapacity;
impl BTBCapacity {
    const TEST_ADDR: usize  = 0x0000_0000_2000_0000;

    fn emit_measure() -> X64AssemblerFixed {

        let mut f = X64AssemblerFixed::new(
            Self::TEST_ADDR | 0x0000_1001_0000_0000,
            0x0000_0000_0008_0000
        );
        dynasm!(f
            ; mov r8, QWORD Self::TEST_ADDR as _
        );
        f.emit_rdpmc_start(0, Gpr::R15 as u8);
        dynasm!(f
            ; call r8
        );
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn emit(len: usize, align: usize) -> X64AssemblerFixed {
       let mut set = BranchSet::gen_uniform_offset(
            Self::TEST_ADDR,
            Align::from_bit(align), 
            0x0000_0000_0000_0000,
            len,
        );

        let mut f = X64AssemblerFixed::new(
            Self::TEST_ADDR,
            0x0000_0000_0008_0000
        );

        f.pad_until(set.first().unwrap().addr);
        for branch in set.data {
            let lab = f.new_dynamic_label();
            f.pad_until(branch.addr);
            assert!(f.cur_addr() == branch.addr, "{:016x} != {:016x}", 
                f.cur_addr(), branch.addr);
            if branch.offset() < 128 {
                dynasm!(f ; jmp BYTE =>lab);
            } else {
                dynasm!(f ; jmp =>lab);
            }
            f.pad_until(branch.tgt);
            assert!(f.cur_addr() == branch.tgt, "{:016x} != {:016x}", 
                f.cur_addr(), branch.tgt);
            f.place_dynamic_label(lab);
        }


        f.emit_ret();

        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {
        //let event = Zen2Event::ExRetBrnMisp(0x00);
        let mut events = EventSet::new();
        events.add_list(&[
            Zen2Event::LsNotHaltedCyc(0x00),

            Zen2Event::IcFetchStallCyc(IcFetchStallCycMask::Any),
            Zen2Event::IcFw32(0x00),
            Zen2Event::IcFw32Miss(0x00),
            //Zen2Event::IcCacheFillL2(0x00),
            //Zen2Event::IcCacheFillSys(0x00),

            //Zen2Event::Unk(0xa6, 0x01),
            //Zen2Event::Unk(0xa6, 0x02),
            //Zen2Event::Unk(0xa6, 0x04),
            //Zen2Event::Unk(0xa6, 0x08),
            //Zen2Event::Unk(0xa6, 0x10),
            //Zen2Event::Unk(0xa6, 0x20),
            //Zen2Event::Unk(0xa6, 0x40),
            //Zen2Event::Unk(0xa6, 0x80),

            Zen2Event::BpRedirect(BpRedirectMask::Unk(0x01)),
            Zen2Event::BpL0BTBHit(0x00),
            Zen2Event::BpL1BTBCorrect(0x00),
            Zen2Event::BpL2BTBCorrect(0x00),
            Zen2Event::BpDeReDirect(0x01),
            //Zen2Event::ExRetBrnMisp(0x00),
            Zen2Event::ExRetBrnIndMisp(0x00),
            //Zen2Event::ExRetBrnTakenMisp(0x00),
            //Zen2Event::ExRetBrn(0x00),
            //Zen2Event::ExRetMsprdBrnchInstrDirMsmtch(0x00),
            Zen2Event::Bp1RetBrUncondMisp(0x00),
        ]);

        for len in &[1,2,4,8,16,32,64,128,256,512,1024,2048,4096] {
            println!("{} branches", len);
            let f = Self::emit(*len, 5);
            let m = Self::emit_measure();
            let func = m.as_fn();

            for event in events.iter() {
                let desc = event.as_desc();
                let results = harness.measure(func,
                    desc.id(), desc.mask(), 512, 
                    InputMethod::Fixed(Self::TEST_ADDR, 0),
                ).unwrap();
                let dist = results.get_distribution();
                let min = results.get_min();
                let max = results.get_max();
                println!("{:03x}:{:02x} ({:36}): min={:5} max={:5}",
                    desc.id(),desc.mask(),desc.name(),
                    min, max,
                );
            }
            println!();
        }

    }

}


