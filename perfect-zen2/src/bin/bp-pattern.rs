use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use perfect::util::*;
use perfect::ir::branch::*;
use itertools::*;
use std::collections::*;
use bitvec::prelude::*;

/// Create an exhaustive list of patterns for the given length. 
fn generate_patterns_exhaustive(plen: usize) -> Vec<BitVec<usize, Msb0>> {
    let mut patterns = Vec::new();
    for val in 0usize..=(1<<plen)-1 {
        let bits = &val.view_bits::<Msb0>()[64-plen..];
        patterns.push(bits.to_bitvec());
    }
    patterns
}


fn to_string_chunks(vec: &Vec<usize>, len: usize) -> Vec<String> {
    let mut res = Vec::new();
    for chunk in vec.chunks(len) {
        let s = chunk.iter().map(|v| 
            std::char::from_digit(*v as u32, 10).unwrap()
        ).collect::<String>();
        res.push(s);
    }
    res
}

/// Convert a bitslice to a flat string of 1's and 0's. 
///
/// NOTE: This only exists because, when using .chunks(), the binary formatter 
/// in `bitvec` seems to include spaces at the boundaries between the backing 
/// words in memory (.. or maybe I'm just using it incorrectly). 
///
fn bvfmt(bv: &BitSlice<usize, Msb0>) -> String {
    bv.iter().map(|b| std::char::from_digit(*b as u32, 2).unwrap()).collect()
}

/// Generate an address where bits [45:32] are randomized
fn gen_random_addr() -> usize { 
    let r = thread_rng().gen_range(0x2000..=0x3fff);
    0x0000_0000_0000_0000usize | (r << 32)
}

/// Representing a pattern of branch outcomes and the observed response from 
/// the predictor. 
pub struct PatternResults { 
    pattern: BitVec<usize, Msb0>,
    outcomes: BitVec<usize, Msb0>,
    misses: BitVec<usize, Msb0>,
    predictions: BitVec<usize, Msb0>,
}
impl PatternResults {
    pub fn new(pattern: BitVec<usize, Msb0>,
        outcomes: BitVec<usize, Msb0>,
        predictions: BitVec<usize, Msb0>,
        misses: BitVec<usize, Msb0>,
    ) -> Self { 

        Self { pattern, outcomes, predictions, misses }
    }
}


/// Test branch direction predictor response against patterns of outcomes. 
pub struct PatternStimulus;
impl PatternStimulus {
    /// The event used to measure the branch
    const EVENT: Zen2Event = Zen2Event::ExRetMsprdBrnchInstrDirMsmtch(0x00);
    /// The number of times each pattern is tested
    const PATTERN_ITERS: usize = 32;

    fn emit(padding_brns: Option<usize>) -> X64AssemblerFixed {
        // NOTE: In order to avoid creating interference between tests for 
        // different patterns, we should randomize some of the high bits in 
        // the location for this code. 
        //
        // This means that any state associated with the branch in *this* test 
        // is somewhat less likely to be used by the predictor during another
        // test in the future.

        let mut f = X64AssemblerFixed::new(
            gen_random_addr(),
            0x0000_0000_0080_0000
        );

        // Optionally emit some number of unconditional jumps. 
        //
        // This pollutes the global history of outcomes [and any notion of a 
        // "path history" that the machine might maintain] being used by the 
        // predictor when it encounters our branch. 

        if let Some(cnt) = padding_brns { 
            f.emit_jmp_byte_sled(cnt);
        }

        // Measure a single conditional branch with RDPMC.
        // (ie. using the event for branch mispredictions)

        f.emit_rdpmc_start(0, Gpr::R15 as u8);
        dynasm!(f
            ; je BYTE >bar
            ; bar:
        );
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    /// Test the predictor's response to a single pattern of branch outcomes.
    /// The pattern is repeated [`Self::PATTERN_ITERS`] times. 
    fn run_pattern(
        harness: &mut PerfectHarness, 
        pattern: &BitVec<usize, Msb0>,
        padding: Option<usize>,
        )
        -> PatternResults
    {
        let edesc = Self::EVENT.as_desc();

        // Emit a new copy of our test
        let f = Self::emit(padding);
        let func = f.as_fn();

        // Build a list of all branch outcomes during this test, repeating
        // the input pattern some number of times
        let mut outcomes = bitvec![usize, Msb0;];
        for _ in 0..Self::PATTERN_ITERS { 
            outcomes.extend(pattern);
        }

        // Generate the values for RDI/RSI that are passed to the harness
        let inputs: Vec<(usize,usize)> = outcomes.iter()
            .map(|bit| (*bit as usize, 0))
            .collect();

        // Try to eliminate the possibility of any older BTB entries 
        // interfering with the predictor state used during the test.
        // We expect our branch to miss in the BTB when first encountered.
        flush_btb::<8192>();

        // Call our function in a loop and collect the results from RDPMC.
        let results = harness.measure(func,
            &edesc, outcomes.len(),
            InputMethod::List(&inputs)
        ).unwrap();

        // Since we're measuring a single branch, we expect to measure at 
        // most one misprediction per iteration. Otherwise, our test was 
        // probably not reliable and something is very wrong with our 
        // setup (ie. SMT is not disabled, or we have been pre-empted 
        // while running the test?)
        assert!(results.get_max() <= 1);

        // Use observed mispredictions to recover the predicted outcomes
        // (ie. when we mispredict, the predicted outcome must be the 
        // opposite of the input we provided)
        let mut predictions = bitvec![usize, Msb0;];
        let mut misses = bitvec![usize, Msb0;];
        for (misp, outcome) in results.data.iter().zip(outcomes.iter()) {
            misses.push(*misp != 0);
            if *misp == 0 {
                predictions.push(*outcome);
            } else { 
                predictions.push(!*outcome);
            }
        }

        PatternResults::new(pattern.clone(), outcomes, predictions, misses)
    }

    /// Test the predictor's response to one or more patterns of branch 
    /// outcomes. 
    fn run(
        harness: &mut PerfectHarness, 
        patterns: Vec<BitVec<usize, Msb0>>,
        padding: Option<usize>,
    ) -> Vec<PatternResults>
    {
        println!("[*] Testing {} patterns ...", patterns.len());
        let mut passes = 0;

        let mut results = Vec::new();

        // Test the predictor's response to each pattern
        for pattern in patterns.iter() {

            let res = Self::run_pattern(harness, pattern, padding);
            results.push(res);

        }
        results
    }

    fn print_results(results: Vec<PatternResults>) {
        for res in results.iter() {
            // If a branch is newly-discovered (ie. not already tracked in the 
            // predictor), we expect the default prediction is "not-taken."
            // This means that, if we find the first prediction was "taken",
            // we should probably disregard the results for this test because
            // the predictor is not in a clean-enough state for us. 
            let dirty_test = (
                // If the first outcome is T and we correctly predict taken
                ((res.misses[0] == false) && (res.pattern[0] == true)) |

                // If the first outcome is N and we mispredict taken 
                ((res.misses[0] == true) && (res.pattern[0] == false))
            );
                
            // For each iteration of the pattern, consider the pattern to be
            // perfectly "captured" by the predictor when no mispredictions 
            // have occurred 
            let iter_captured = res.misses.chunks(res.pattern.len())
                .map(|c| c.not_any())
                .collect::<BitVec<usize, Msb0>>();

            let num_miss = res.misses.count_ones();

            if iter_captured.count_ones() >= iter_captured.len() / 2 {
                continue;
            }

            println!("[*] Pattern {:b}", res.pattern);
            println!("    Captured iterations: {}/{} {:b}", 
                iter_captured.count_ones(), iter_captured.len(),
                iter_captured);
            for (idx,chunk) in res.misses.chunks_exact(res.pattern.len())
                .enumerate() 
            {
                println!("      Iteration #{:2}: {}", idx, bvfmt(chunk));

            }
            println!("    Miss count: {}/{}", num_miss, res.misses.len());
            if dirty_test {
                println!("    Test might be invalid...");
            }
            println!();
        }
    }
}


fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .cmp_rdi(1)
        .arena_alloc(0x0000_0000_0000_0000, 0x0000_0000_1000_0000)
        .emit();

    // Test all N-bit patterns of branch outcomes
    let mut patterns = generate_patterns_exhaustive(10);
    patterns.shuffle(&mut thread_rng());

    let res = PatternStimulus::run(&mut harness, patterns, None);
    PatternStimulus::print_results(res);

}


