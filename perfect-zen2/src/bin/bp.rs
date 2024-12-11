use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use perfect::util::*;
use perfect::ir::branch::*;
use itertools::*;
use std::collections::*;
use bitvec::prelude::*;

fn gen_random_addr() -> usize { 
    let r = thread_rng().gen_range(0x2000..=0x4fff);
    0x0000_0000_0000_0000usize | (r << 32)
}

#[derive(Clone, Copy)]
pub struct BranchRecord<const FOOTPRINT: usize> { 
    base_addr: usize,
    idx: u8,
    brn_off: usize,
    tgt_off: usize,
}
impl <const FOOTPRINT: usize> BranchRecord<FOOTPRINT> { 
    const IDX_SHIFT: usize = 64 - (FOOTPRINT.leading_zeros() as usize) - 1;

    pub fn new(base_addr: usize, idx: u8, brn_off: usize, tgt_off: usize) 
        -> Self 
    {

        assert!(FOOTPRINT <= (1 << 25),
            "BranchRecord footprint {:016x} is unreasonably large", 
            FOOTPRINT
        );

        // Just to make things less confusing..
        assert!(FOOTPRINT.count_ones() == 1,
            "BranchRecord footprint {:016x} has more than one set bit", 
            FOOTPRINT
        );

        assert!(idx <= 0xc0);

        // Constrain placement within the footprint
        assert!((brn_off & !(FOOTPRINT - 1) == 0), 
                "Branch offset {:016x} must be < {:016x}", brn_off,
                FOOTPRINT
        );
        assert!((tgt_off & !(FOOTPRINT - 1) == 0), 
                "Branch target offset {:016x} must be < {:016x}", tgt_off,
                FOOTPRINT,
        );
        
        // Only forward targets are allowed
        assert!(tgt_off > brn_off, 
            "Branch target offset {:016x} must be < branch offset {:016x}",
            tgt_off, brn_off
        );

        // The smallest branch/jmp encoding is two bytes
        assert!((tgt_off - brn_off) >= 2,
            "Difference between the branch target offset ({:016x}) and the \
            branch offset ({:016x}) must be at least 2 bytes",
            tgt_off, brn_off
        );


        Self { 
            base_addr,
            idx,
            brn_off: brn_off & (FOOTPRINT - 1),
            tgt_off: tgt_off & (FOOTPRINT - 1),
        }
    }
    pub fn addr(&self) -> usize { 
        self.base_addr | (self.idx as usize) << Self::IDX_SHIFT | self.brn_off
    }
    pub fn tgt(&self) -> usize { 
        self.base_addr | (self.idx as usize) << Self::IDX_SHIFT | self.tgt_off
    }
}

pub struct BranchGroup<const FOOTPRINT: usize> { 
    base_addr: usize,
    idx: u8,
    padding: Vec<BranchRecord<FOOTPRINT>>,
    a: Option<BranchRecord<FOOTPRINT>>,
    b: Option<BranchRecord<FOOTPRINT>>,
}
impl <const FOOTPRINT: usize> BranchGroup<FOOTPRINT> { 
    pub fn new(base_addr: usize) -> Self { 
        Self { 
            base_addr,
            idx: 0,
            padding: Vec::new(),
            a: None,
            b: None,
        }
    }
    pub fn add_padding_brn(&mut self, brn_off: usize, tgt_off: usize) {
        //assert!(self.a.is_none() && self.b.is_none());
        self.padding.push(BranchRecord::new(
            self.base_addr, self.idx, brn_off, tgt_off
        ));
        self.idx += 1;
    }
    pub fn def_brn_a(&mut self, brn_off: usize, tgt_off: usize) {
        self.a = Some(BranchRecord::new(
            self.base_addr, self.idx, brn_off, tgt_off
        ));
        self.idx += 1;
    }
    pub fn def_brn_b(&mut self, brn_off: usize, tgt_off: usize) {
        self.b = Some(BranchRecord::new(
            self.base_addr, self.idx, brn_off, tgt_off
        ));
        self.idx += 1;
    }

    pub fn pad_iter(&self) -> impl Iterator<Item=&BranchRecord<FOOTPRINT>> {
        self.padding.iter()
    }

    pub fn brn_a(&self) -> BranchRecord<FOOTPRINT> { 
        if let Some(x) = self.a { x } 
        else { panic!("Branch 'A' is undefined"); }
    }
    pub fn brn_b(&self) -> BranchRecord<FOOTPRINT> { 
        if let Some(x) = self.b { x } 
        else { panic!("Branch 'B' is undefined"); }
    }

    pub fn print(&self) {
        let first = self.padding.first().unwrap();
        let last = self.padding.last().unwrap();
        println!("pad_f: {:016x} {:016x}", first.addr(), first.tgt());
        println!("pad_l: {:016x} {:016x}", last.addr(), last.tgt());
        if let Some(a) = self.a { 
            println!("brn_a: {:016x} {:016x}", a.addr(), a.tgt());
        }
        if let Some(b) = self.b { 
            println!("brn_b: {:016x} {:016x}", b.addr(), b.tgt());
        }


    }
}

#[derive(Copy, Clone)]
pub struct ClearGhistArgs {
    num_padding: usize,
    pad_addr_off: usize,
    pad_tgt_off:  usize,
    a_addr_off:   usize,
    a_tgt_off:    usize,
    b_addr_off:   usize,
    b_tgt_off:    usize,
}

pub struct ClearGhist;
impl ClearGhist {
    const NUM_ITER: usize = 1024;
    const EVENT: Zen2Event = Zen2Event::ExRetMsprdBrnchInstrDirMsmtch(0x00);

    fn emit_trampoline() -> X64AssemblerFixed {
        let base_addr = gen_random_addr();
        let mut f = X64AssemblerFixed::new(
            base_addr,
            0x0000_0000_0010_0000
        );


        f.emit_rdpmc_start(0, Gpr::R15 as u8);
        dynasm!(f
            ; call rsi
        );
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn emit<const FOOTPRINT: usize>(arg: ClearGhistArgs) -> X64AssemblerFixed 
    {
        let base_addr = gen_random_addr();

        let mut grp: BranchGroup<FOOTPRINT> = BranchGroup::new(base_addr);
        for idx in 1..=arg.num_padding {
            grp.add_padding_brn(
                arg.pad_addr_off,
                arg.pad_tgt_off,
            );
        }

        grp.def_brn_a(
            arg.a_addr_off,
            arg.a_tgt_off,
        );


        grp.def_brn_b(
            arg.b_addr_off,
            arg.b_tgt_off,
        );
        //println!("FOOTPRINT:    {:032b}", FOOTPRINT);
        //println!("pad_addr_off: {:032b}", arg.pad_addr_off);
        //println!("pad_tgt_off:  {:032b}", arg.pad_tgt_off);
        //println!("a_addr_off:   {:032b}", arg.a_addr_off);
        //println!("a_tgt_off:    {:032b}", arg.a_tgt_off);
        //println!("b_addr_off:   {:032b}", arg.b_addr_off);
        //println!("b_tgt_off:    {:032b}", arg.b_tgt_off);


        //grp.print();
        let mut f = X64AssemblerFixed::new(
            base_addr,
            0x0000_0000_c000_0000
        );

        let brn_a = grp.brn_a();
        let brn_b = grp.brn_b();

        for pad_brn in grp.pad_iter() {
            f.pad_until(pad_brn.addr());
            let lab = f.new_dynamic_label();
            let span = pad_brn.tgt() - pad_brn.addr();
            if span < 128 {
                dynasm!(f ; jmp BYTE =>lab);
            } else {
                dynasm!(f ; jmp =>lab);
            }
            f.pad_until(pad_brn.tgt());
            f.place_dynamic_label(lab);
        }


        f.pad_until(brn_a.addr());
        let lab_a = f.new_dynamic_label();
        let span = brn_a.tgt() - brn_a.addr();
        if span < 128 {
            dynasm!(f ; je BYTE =>lab_a);
        } else {
            dynasm!(f ; je =>lab_a);
        }
        f.pad_until(brn_a.tgt());
        f.place_dynamic_label(lab_a);


        f.pad_until(brn_b.addr());
        let lab_b = f.new_dynamic_label();
        let span = brn_b.tgt() - brn_b.addr();
        if span < 128 {
            dynasm!(f ; je BYTE =>lab_b);
        } else {
            dynasm!(f ; je =>lab_b);
        }
        f.pad_until(brn_b.tgt());
        f.place_dynamic_label(lab_b);

        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {
        let desc = Self::EVENT.as_desc();

        const FTPT: usize   = 0b0_0000_0010_0000_0000_0000_0000;
        let mut cases: Vec<ClearGhistArgs> = Vec::new();

        //cases.push(ClearGhistArgs {
        //    num_padding: 91,
        //    pad_addr_off: 0b0_0000_0000_0000_0000_0000_0000,
        //    pad_tgt_off:  0b0_0000_0001_0000_0000_0000_0000,
        //    a_addr_off:   0b0_0000_0000_0000_0000_0000_0000,
        //    a_tgt_off:    0b0_0000_0001_0000_0000_0000_0000,
        //    b_addr_off:   0b0_0000_0000_0000_0000_0000_0000,
        //    b_tgt_off:    0b0_0000_0001_0000_0000_0000_0000,
        //});

        //for i in 0..=15 {
        //        cases.push(ClearGhistArgs {
        //            num_padding:  94,
        //            pad_addr_off: 0b0_0000_0000_0000_0000_0000_0000,
        //            pad_tgt_off:  0b0_0000_0001_0000_0000_0000_0000,

        //            a_addr_off:   0b0_0000_0000_0000_0000_0000_0000 | (1<<i),
        //            a_tgt_off:    0b0_0000_0001_0000_0000_0000_0000,

        //            b_addr_off:   0b0_0000_0000_0000_0000_0000_0000 | (1<<i),
        //            b_tgt_off:    0b0_0000_0001_0000_0000_0000_0000,
        //        });
        //}

        //for i in 0..=15 {
        //        cases.push(ClearGhistArgs {
        //            num_padding:  94,
        //            pad_addr_off: 0b0_0000_0000_0000_0000_0000_0000,
        //            pad_tgt_off:  0b0_0000_0001_0000_0000_0000_0000,

        //            a_addr_off:   0b0_0000_0000_0000_0000_0000_0000,
        //            a_tgt_off:    0b0_0000_0001_0000_0000_0000_0000 | (1<<i),

        //            b_addr_off:   0b0_0000_0000_0000_0000_0000_0000,
        //            b_tgt_off:    0b0_0000_0001_0000_0000_0000_0000 | (1<<i),
        //        });
        //}


        let trampoline = Self::emit_trampoline();
        let trampoline_fn = trampoline.as_fn();


        for (caseno, case) in cases.iter().enumerate() {
            let f = Self::emit::<FTPT>(*case);
            let func = f.as_fn();

            let mut inputs: Vec<(usize,usize)> = (0..Self::NUM_ITER)
                .map(|i| (thread_rng().gen::<bool>() as usize, func as usize))
                .collect();

            flush_btb::<8192>();

            let results = harness.measure(trampoline_fn,
                desc.id(), desc.mask(), Self::NUM_ITER, 
                InputMethod::List(&inputs)
            ).unwrap();

            let dist = results.get_distribution();
            let min = results.get_min();
            let max = results.get_max();
            let hitrate = results.count(0) as f32 / Self::NUM_ITER as f32;
            println!("  hitrate={:.3} num_padding={:02} pad_addr_off={:016x} pad_tgt_off={:016x} \
                a_addr_off={:016x} a_tgt_off={:016x} b_addr_off={:016x} b_tgt_off={:016x} dist={:?}", 
                hitrate,
                case.num_padding,
                case.pad_addr_off, 
                case.pad_tgt_off, 
                case.a_addr_off, 
                case.a_tgt_off, 
                case.b_addr_off, 
                case.b_tgt_off, 
                dist
            );
        }
    }

}



/// [Naively?] try to interfere with two correlated conditional branches. 
///
/// Context
/// =======
///
/// Predicting the *direction* of a branch usually entails the following:
///
/// - Keeping track of a "local" history of outcomes for a particular branch
/// - Keeping track of a "global" history of outcomes for all branches
/// - Using some fancy method of combining these two kinds of information
///
/// Test
/// ====
///
/// 1. Emit a branch 'A' with a *random* outcome. 
/// 2. Emit a variable number of always-taken padding branches/jumps.
/// 3. Emit a branch 'B' with *the same* random outcome as branch 'A'.
///
/// Under normal circumstances (with very few padding branches in-between), we 
/// expect that the branch predictor establishes a correlation between the 
/// outcome of 'A' and the outcome of 'B'. This means that 'B' should be 
/// correctly predicted very close to 100% of the time. 
///
/// However, after a certain number of padding branches, we expect that the 
/// machine will not be able to preserve the correlation between 'A' and 'B'.
///
/// This is a reasonable assumption because the amount of storage used for 
/// tracking branch history must be finite. Eventually, the outcome of 'A' 
/// will be lost [overwritten by the outcomes of the padding branches].
///
/// This should cause 'B' to be correctly predicted only ~50% of the time
/// (ie. the best you can do at predicting random outcomes, assuming you're
/// only using a "local" history of outcomes, or using a fixed prediction).
///
/// Results
/// =======
///
/// The prediction hit rate decreases to ~50% after 90 padding branches. 
///
/// This probably reflects one (or both?) of the following things: 
///
/// - We've filled up some [global] history register with taken outcomes,
///   which prevents the predictor from accessing the outcome of branch A.
/// - We've created aliasing in some table of tracked branches
///
/// .. although, this test doesn't tell us exactly which of these is the case. 
/// The predictors are probably sensitive to the exact target address and 
/// program counter of each branch (which we are sort of ignoring here). 
///
pub struct CorrelatedBranches;
impl CorrelatedBranches {

    const NUM_ITER: usize = 2048;
    const EVENT: Zen2Event = Zen2Event::ExRetMsprdBrnchInstrDirMsmtch(0x00);

    /// Emit the test. 
    ///
    /// Arguments
    /// =========
    ///
    /// - `num_padding`: The number of padding jumps
    /// - `abit`: Requested alignment (in bits) for all branch addresses
    ///           and targets
    /// - `a_brn_off`: Offset added to the branch address of 'A' 
    /// - `a_brn_tgt`: Offset added to the target address of 'A' 
    /// - `b_brn_off`: Offset added to the branch address of 'B' 
    /// - `b_brn_tgt`: Offset added to the target address of 'B' 
    ///
    fn emit(
        num_padding: usize, 
        abit: usize,
        a_brn_off: usize,
        a_tgt_off: usize,
        b_brn_off: usize,
        b_tgt_off: usize,
    ) -> X64AssemblerFixed 
    {
        let align = Align::from_bit(abit);

        let mut f = X64AssemblerFixed::new(
            gen_random_addr(),
            0x0000_0000_8000_0000
        );

        // Emit branch 'A'.
        //
        // This branch is *always* predicted locally [assuming that we've 
        // successfully cleared the state of global history before this] and 
        // it should not be correlated with the outcome of a previous branch? 
        //
        // NOTE: You can verify this by wrapping this block with RDPMC and 
        // observing that the misprediction rate is always 50%.

        let cursor = AlignedAddress::new(f.cur_addr(), align);
        let brn_a = BranchDesc::new(
            cursor.aligned().value() + a_brn_off,
            cursor.aligned().next().value() + a_tgt_off,
        );
        brn_a.emit_je_direct(&mut f);

        // Emit a variable number of unconditional padding jumps.

        let cursor = AlignedAddress::new(f.cur_addr(), align).next();
        let set = BranchSet::gen_uniform_offset(
            cursor.aligned().value(),
            align,
            0x0000_0000_0000_0000,
            num_padding
        );
        for jmp in &set.data {
            jmp.emit_jmp_direct(&mut f);
        }

        // Emit branch 'B', which will be measured with RDPMC. 
        //
        // When a correlation with the first branch can be maintained, we 
        // should expect this to be correctly-predicted most if not all the 
        // time. 
        //
        // NOTE: Since `emit_rdpmc_start()` occupies 0x18 bytes, we need the 
        // requested alignment to be at least 5 bits. Would be nice to find
        // a way around this... 

        assert!(abit >= 5); 
        let cursor = AlignedAddress::new(f.cur_addr(), align)
            .aligned()
            .next();
        let brn_b = BranchDesc::new(
            cursor.value() + b_brn_off,
            cursor.aligned().next().value() + b_tgt_off,
            //cursor.value() + 2,
        );

        f.pad_until(brn_b.addr - 0x18);
        f.emit_rdpmc_start(0, Gpr::R15 as u8);
        brn_b.emit_je_direct(&mut f);
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();

        //println!("  brn_a: {:016x} => {:016x}", brn_a.addr, brn_a.tgt);

        //for jmp in &set.data {
        //    println!("  pad  : {:016x} => {:016x}", jmp.addr, jmp.tgt);
        //}

        ////let last = set.last().unwrap();
        ////println!("  pad_t: {:016x} => {:016x}", last.addr, last.tgt);
        //println!("  brn_b: {:016x} => {:016x}", brn_b.addr, brn_b.tgt);

        f
    }

    fn run(harness: &mut PerfectHarness) {
        let desc = Self::EVENT.as_desc();
        let abit = 16;
        let mut offsets = vec![0];
        let mut x: Vec<usize> = (0..=abit-1).map(|x| 1 << x).collect();
        offsets.append(&mut x);


        for off in offsets {
            for num_padding in 1..=96 {
                let f = Self::emit(num_padding, abit, 0, 0, 0, off);
                let func = f.as_fn();

                // Try to reset the state of the predictor before entering each 
                // instance of the test. 
                flush_btb::<8192>();

                let results = harness.measure(func,
                    desc.id(), desc.mask(), Self::NUM_ITER, 
                    InputMethod::Random(&|rng, _| { 
                        (rng.gen::<bool>() as usize, 0) 
                    }),
                ).unwrap();

                let dist = results.get_distribution();
                let min = results.get_min();
                let max = results.get_max();

                let hitrate = results.count(0) as f32 / Self::NUM_ITER as f32;
                println!("abit={:02} b_brn_off={:016x} padding={:03} hitrate={:.3}", 
                    abit, off, num_padding, hitrate
                );

            }
        }
    }

    fn run_abit_scan(harness: &mut PerfectHarness) {
        let desc = Self::EVENT.as_desc();

        // NOTE: Why does using a random order change the results? 
        let mut paddings = (1..=96).collect_vec();
        //paddings.shuffle(&mut thread_rng());

        for abit in 5..=19 {

            let mut res = Vec::new();
            //for num_padding in 1..=96 {
            for num_padding in &paddings {
                let f = Self::emit(*num_padding, abit, 0, 0, 0, 0);
                let func = f.as_fn();

                // Try to reset the state of the predictor before entering each 
                // instance of the test. 
                flush_btb::<8192>();

                let results = harness.measure(func,
                    desc.id(), desc.mask(), Self::NUM_ITER, 
                    InputMethod::Random(&|rng, _| { 
                        (rng.gen::<bool>() as usize, 0) 
                    }),
                ).unwrap();

                let dist = results.get_distribution();
                let min = results.get_min();
                let max = results.get_max();

                let hitrate = results.count(0) as f32 / Self::NUM_ITER as f32;
                res.push(hitrate);

                println!("abit={:02} padding={:03} hitrate={:.3}", 
                    abit, num_padding, hitrate
                );
            }
        }
    }
}

/// Alternate version of [`CorrelatedBranches`], but with a single padding 
/// branch taken in a loop some variable number of times. 
///
/// In this case, the correlation is lost after repeating the branch 91 times. 
/// Since the last outcome in the loop is necessarily "not-taken", this is 
/// the exact same result as the test in [`CorrelatedBranches`], where we've
/// inserted 90 "taken" outcomes in-between branch 'A' and branch 'B'.
///
pub struct CorrelatedBranchesSimple;
impl CorrelatedBranchesSimple {
    fn emit() -> X64AssemblerFixed {
        let mut f = X64AssemblerFixed::new(
            gen_random_addr(),
            0x0000_0000_1000_0000
        );

        dynasm!(f
            // Branch A
            ; ->brn_a:
            ; je ->brn_a_tgt

            // Branch A target
            ; ->brn_a_tgt:

            // Padding branch target
            ; ->pad_tgt:
            ; dec rsi

            // Padding branch
            ; ->pad:
            ; jnz ->pad_tgt

            ; lfence
            ; mov rcx, 0
            ; rdpmc 
            ; mov r15, rax
            ; lfence
            ; cmp rdi, 1

            // Branch B
            ; ->brn_b:
            ; je ->brn_b_tgt

            // Branch B target
            ; ->brn_b_tgt:
            ; lfence

            ; rdpmc 
            ; lfence
            ; sub rax, r15
            ; ret
        );

        let brn_a = f.labels.resolve_static(
            &StaticLabel::global("brn_a")
        ).unwrap();
        let brn_a_tgt = f.labels.resolve_static(
            &StaticLabel::global("brn_a_tgt")
        ).unwrap();
        let pad = f.labels.resolve_static(
            &StaticLabel::global("pad")
        ).unwrap();
        let pad_tgt = f.labels.resolve_static(
            &StaticLabel::global("pad_tgt")
        ).unwrap();
        let brn_b = f.labels.resolve_static(
            &StaticLabel::global("brn_b")
        ).unwrap();
        let brn_b_tgt = f.labels.resolve_static(
            &StaticLabel::global("brn_b_tgt")
        ).unwrap();


        println!("brn_a: {:016x} => {:016x}", 
            brn_a.0 + f.base_addr(),
            brn_a_tgt.0 + f.base_addr(),
        );
        println!("pad:   {:016x} => {:016x}", 
            pad.0 + f.base_addr(),
            pad_tgt.0 + f.base_addr(),
        );
        println!("brn_b: {:016x} => {:016x}", 
            brn_b.0 + f.base_addr(),
            brn_b_tgt.0 + f.base_addr(),
        );


        f.commit().unwrap();
        f

    }

    const NUM_ITER: usize = 4096;
    const EVENT: Zen2Event = Zen2Event::ExRetMsprdBrnchInstrDirMsmtch(0x00);
    fn run(harness: &mut PerfectHarness) {
        let desc = Self::EVENT.as_desc();

        let mut res = Vec::new();

        for num_padding in 1..=96 {
            let f = Self::emit();
            let func = f.as_fn();

            // Try to reset the state of the predictor before entering each 
            // instance of the test. 
            flush_btb::<8192>();

            let inputs: Vec<(usize,usize)> = (0..Self::NUM_ITER)
                .map(|i| (thread_rng().gen::<bool>() as usize, num_padding))
                .collect();

            let results = harness.measure(func,
                desc.id(), desc.mask(), Self::NUM_ITER, 
                InputMethod::List(&inputs)
            ).unwrap();

            let dist = results.get_distribution();
            let min = results.get_min();
            let max = results.get_max();

            let hitrate = results.count(0) as f32 / Self::NUM_ITER as f32;
            res.push(hitrate);

            println!("num_padding={:2} min={} max={} hitrate={:.3} dist={:?}", 
                num_padding, min,max,hitrate,dist
            );
        }

        // Form a bitstring representing each instance of the test, where '1' 
        // means that the correlation between the two branches was captured. 
        let pass: BitVec<usize, Msb0> = res.iter().map(|r| *r > 0.75)
            .collect();

        let chunk_sz = 8;
        println!("[*] Test results (groups of {} tests)", chunk_sz);
        for chunk in pass.chunks(chunk_sz) {
            println!("{:0format$b}", chunk.load::<usize>(), format = chunk_sz);
        }

    }
}




fn main() {
    let r = thread_rng().gen_range(0x1000..=0x1fff);
    let harness_addr = 0x0000_0000_0000_0000usize | (r << 32);

    let mut harness = HarnessConfig::default_zen2()
        //.harness_addr(harness_addr)
        .cmp_rdi(1)
        .arena_alloc(0x0000_0000_0000_0000, 0x0000_0000_1000_0000)
        .emit();

    //CorrelatedBranchesSimple::run(&mut harness);
    //CorrelatedBranches::run_abit_scan(&mut harness);
    //CorrelatedBranches::run(&mut harness);

    ClearGhist::run(&mut harness);
}

