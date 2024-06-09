use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use perfect::ir::branch::*;
use perfect::util::*;
use itertools::*;
use std::collections::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .arena_alloc(0x0000_0000_0000_0000, 0x0000_0000_1000_0000)
        .emit();
    BTBCapacity::run(&mut harness);

    //for index in 0..4096 {
    //    let x = BTBAddress::new(0, index, 0x2000);
    //    println!("{}", x);
    //}

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

pub struct BTBTest {
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

pub struct BTBCapacity;
impl BTBCapacity {
    fn emit(arg: BTBCapacityArgs) -> X64AssemblerFixed {

        let branches = BranchSet::gen_uniform(
            arg.start_addr,
            arg.align,
            arg.num_padding
        );


        let fixed_len = 0x0000_0002_0002_0000;
        let base_addr = align_down(arg.start_addr, 16);
        let end_addr  = base_addr + fixed_len;
        let mut f = X64AssemblerFixed::new(base_addr, fixed_len);
        let head_lab = f.new_dynamic_label();
        dynasm!(f
            ; cmp rdi, 0
        );

        f.place_dynamic_label(head_lab);
        for branch in branches.data {
            if branch.addr >= end_addr || branch.tgt >= end_addr {
                panic!("{:016x?} exceeds limit {:016x}",
                    branch, end_addr);
            }

            let lab = f.new_dynamic_label();
            f.pad_until(branch.addr);
            assert!(f.cur_addr() == branch.addr, "{:016x} != {:016x}", 
                f.cur_addr(), branch.addr
            );

            if branch.offset() < 128 {
                dynasm!(f ; jmp BYTE =>lab);
            } else {
                dynasm!(f ; jmp =>lab);
            }
            f.pad_until(branch.tgt);
            assert!(f.cur_addr() == branch.tgt, "{:016x} != {:016x}", 
                f.cur_addr(), branch.tgt
            );
            f.place_dynamic_label(lab);
        }

        f.emit_rdpmc_start(0, Gpr::R15 as u8);

        let test_addr = (f.cur_addr() & arg.align.index_mask()) 
            + arg.align.value()
            + arg.test_offset;
        f.pad_until(test_addr);
        assert!(f.cur_addr() == arg.test_addr(), "{:016x} != {:016x}", 
            f.cur_addr(), arg.test_addr()
        );
        dynasm!(f
            ; je >end
            ; end:
        );
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {
        //let event = Zen2Event::ExRetBrnMisp(0x00);
        let event = Zen2Event::BpRedirect(BpRedirectMask::Unk(0x80));
        let desc = event.as_desc();
        let mut exp_results = ExperimentResults::new();
        let mut case_res = ExperimentCaseResults::new("foo");

        let mut args = Vec::new();

        for align_bit in 6..=6 {
            for test_bit in 6..=19 {
                //for num_padding in &[15,31,63,127,255,511,1023,2047,4095] {
                for num_padding in &[8192] {
                    //let start_addr_hi = harness.rng.gen_range(0x0001..=0x00ff);
                    //let start_addr = start_addr_hi << 34;
                    
                    let test_offset = if test_bit == 0 { 0 } else { (1 << test_bit) };

                    args.push(BTBCapacityArgs {
                        start_addr: 0x0000_0001_0000_0000,
                        //start_addr,
                        align: Align::from_bit(align_bit),
                        num_padding: *num_padding,
                        test_offset,
                    });
                }
            }
        }

        let mut a = args[0].align;
        for arg in args.iter() {
            if arg.align != a {
                println!();
            }

            let f = Self::emit(*arg);
            let func = f.as_fn();

            //for _ in 0..8 {
            //    let _ = harness.measure(func,
            //        desc.id(), desc.mask(), 512, InputMethod::Fixed(0, 0)
            //    ).unwrap();
            //}
           
            let results = harness.measure(func,
                desc.id(), desc.mask(), 512, InputMethod::Fixed(0, 0)
            ).unwrap();

            let dist = results.get_distribution();
            let min = results.get_min();
            let max = results.get_max();
            println!("{:016x} num={:5} pad_align={:08x?} offset={:08x} min={} max={} dist={:?}", 
                arg.start_addr, arg.num_padding, arg.align, arg.test_offset, min, max, dist);
            //for chunk in results.data.chunks(32) {
            //    println!("{:?}", chunk);
            //}

            case_res.record(event, arg.num_padding, results);
            a = arg.align;
        }
        exp_results.push(case_res);

        //exp_results.write_results_freq();

    }

}


