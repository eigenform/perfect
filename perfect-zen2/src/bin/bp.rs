use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use perfect::util::*;
use perfect::ir::branch::*;
use itertools::*;
use std::collections::*;
use bitvec::prelude::*;

fn gen_random_addr() -> usize { 
    let r = thread_rng().gen_range(0x2000..=0x3fff);
    0x0000_0000_0000_0000usize | (r << 32)
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
/// 1. Emit a conditional branch with a *random* outcome. 
/// 2. Emit a variable number of always-taken padding branches.
/// 3. Emit a conditional branch with *the same* outcome as the first branch.
///
/// Under normal circumstances (with very few padding branches), we expect 
/// that the branch predictor has learned that the first and last branches
/// are correlated, and that the last branch will be correctly predicted 
/// very close to 100% of the time. 
///
/// After a certain number of padding branches, we expect that the machine
/// will not be able to preserve the correlation between the first and last
/// branches. This is a reasonable assumption because the amount of storage 
/// used for tracking branch history must be finite. 
/// 
/// This should cause the last branch to be correctly predicted only ~50% of 
/// the time (effectively the same as a random guess). 
///
/// Results
/// =======
///
/// Misprediction rate increases to ~50% after exactly 90 padding branches?
///
/// This probably reflects one (or both?) of the following things: 
/// - We've filled up some [global] history register with taken outcomes
/// - We've created aliasing in some table of tracked branches
///
/// .. although, this test doesn't tell us exactly which of these is the case. 
/// The predictors are probably sensitive to the exact target address and 
/// program counter of each branch (which we are sort of ignoring here). 
///
pub struct CorrelatedBranches;
impl CorrelatedBranches {
    fn emit(num_padding: usize) -> X64AssemblerFixed {
        let mut f = X64AssemblerFixed::new(
            gen_random_addr(),
            0x0000_0000_0080_0000
        );

        // Set up the flags for when we execute the conditional branches. 
        // We expect RDI to be a *random* value (either 0 or 1). 
        dynasm!(f
            ; cmp rdi, 1
        );

        // Flush global history with [unconditional] taken branches.
        for _ in 0..2048 {
            dynasm!(f
                ; jmp >wow
                ; wow:
            );
        }
        f.emit_lfence();

        // NOTE: Since we're using RDPMC to measure the last branch, 
        // the alignment must be at least 5 bits (RDPMC uses 0x18 bytes).
        let abit = 5;

        let next = AlignedAddress::new(f.cur_addr(), Align::from_bit(abit))
            .aligned().next().value();
        f.pad_until(next);

        // This branch is *always* predicted locally [assuming that we've really
        // cleared the state of global history before this] and it should not be
        // correlated with the outcome of a previous branch? 
        //
        // NOTE: You can verify this by wrapping this block with RDPMC and 
        // observing that the misprediction rate is always 50%.
        dynasm!(f
            ; je >foo
            ; foo:
        );

        // Emit a variable number of unconditional padding branches.
        let next = AlignedAddress::new(f.cur_addr(), Align::from_bit(abit))
            .aligned().next().value();
        let set = BranchSet::gen_uniform(
            next,
            Align::from_bit(abit), 
            num_padding
        );
        f.pad_until(next);
        for branch in set.data {
            branch.emit_jmp_direct(&mut f);
        }

        let next = AlignedAddress::new(f.cur_addr(), Align::from_bit(abit))
            .aligned().next().value();
        f.pad_until(next - 0x18);

        // Measure this branch. 
        // If a correlation with the first branch can be maintained, we expect
        // this to be correctly-predicted. 
        f.emit_rdpmc_start(0, Gpr::R15 as u8);
        dynasm!(f
            ; ->brn:
            ; je >bar
            ; bar:
        );
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {
        let event = Zen2Event::ExRetBrnMisp(0x00);
        let desc = event.as_desc();
        for num_padding in 0..=128 {
            let f = Self::emit(num_padding);
            let func = f.as_fn();

            let results = harness.measure(func,
                desc.id(), desc.mask(), 4096, 
                InputMethod::Random(&|rng, _| { 
                    (rng.gen::<bool>() as usize, 0) 
                }),
            ).unwrap();

            let dist = results.get_distribution();
            let min = results.get_min();
            let max = results.get_max();
            println!("padding={:03} min={} max={} dist={:?}", 
                num_padding, min,max,dist);
        }
    }
}

fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .cmp_rdi(1)
        .arena_alloc(0x0000_0000_0000_0000, 0x0000_0000_1000_0000)
        .emit();

    CorrelatedBranches::run(&mut harness);
}

