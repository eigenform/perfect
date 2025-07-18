
use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use perfect::stats::*;
use perfect::asm::Emitter;

fn main() {
    let mut harness = HarnessConfig::default_zen2().emit();
    ReorderBufferCapacity::run(&mut harness);
}

/// Measure the capacity of the reorder buffer. 
///
/// Context
/// =======
///
/// The reorder buffer (ROB) tracks in-flight instructions that have been 
/// dispatched into the back-end of the machine but have not yet retired.
///
/// - In Zen 2, the documented reorder buffer capacity is 224 entries
/// - Presumably, each entry corresponds to a single in-flight macro-op (mop) 
/// - Presumably, a NOP instruction occupies a single ROB entry
///
/// Test
/// ====
///
/// According to Family 19h PPRs, the "RetireTokenStall" event (0x0af, 0x20) 
/// counts "cycles where a dispatch group is valid but does not get dispatched 
/// due to a token stall". Presumably, this means that dispatch is stalled 
/// for the availability of a reorder buffer entry. 
///
/// 0. Assume that a NOP instruction occupies a reorder buffer entry.
///    Ensure that the reorder buffer is drained placing LFENCE very close
///    to the start of measured code.
///
/// 1. While measuring "retire token" stall cycles, speculatively dispatch 
///    `N` NOP instructions [which will not stall for any other resources]
///
/// 2. If we observe stall cycles, the reorder buffer capacity must be at least 
///    `N` entries.
///
/// Alternatively, we can use the PMC event for "speculatively-dispatched ops"
/// to determine the capacity: 
///
/// 0. Assume that a NOP instruction occupies a reorder buffer entry.
///
/// 1. While measuring speculatively dispatched FP ops, speculatively dispatch
///    `N` NOP instructions [which will not stall for other resources], and
///    then a single FNOP instruction.
///
/// 2. If we observe that FNOP has been speculatively dispatched, the reorder
///    buffer capacity must be at least `N+1` entries.
///
///
/// Results
/// =======
///
/// When measuring with the stall cycles, we needed to emit a serializing 
/// instruction [to be speculatively dispatched] immediately after the string 
/// of tested instructions. Otherwise, no stall cycles would be recorded. 
///
/// It seems like using MFENCE instead of LFENCE causes us to measure a couple
/// more stall cycles, but I don't exactly understand why:
///
/// - With LFENCE, stall cycles occur after 217 1-mop instructions
/// - With LFENCE, stall cycles occur after 108 2-mop instructions
///
/// - With MFENCE, stall cycles occur after 221 1-mop instructions
/// - With MFENCE, stall cycles occur after 110 2-mop instructions
///
/// When measuring for the FNOP marker (and MFENCE), we observe that: 
///
/// - FNOP is not dispatched after 222 1-mop instructions
/// - FNOP is not dispatched after 111 2-mop instructions
///
pub struct ReorderBufferCapacity;
impl MispredictedReturnTemplate<usize> for ReorderBufferCapacity {}
impl ReorderBufferCapacity {

    const CASES: StaticEmitterCases<usize> = StaticEmitterCases::new(&[
        EmitterDesc { desc: "nop [1 mop]; lfence", 
            func: |f, input| {
            if input == 0 { return; }
            for _ in 0..=input { dynasm!(f ; nop) }
            dynasm!(f; lfence);
        }}, 
        EmitterDesc { desc: "nop15 [1 mop]; lfence", 
            func: |f, input| {
            if input == 0 { return; }
            for _ in 0..=input { dynasm!(f ; .bytes NOP15) }
            dynasm!(f; lfence);
        }}, 
        EmitterDesc { desc: "xchg rax,rbx [2 mops]; lfence", 
            func: |f, input| {
            if input == 0 { return; }
            for _ in 0..=input { dynasm!(f ; xchg rax,rbx) }
            dynasm!(f; lfence);
        }}, 

        EmitterDesc { desc: "nop [1 mop]; mfence", 
            func: |f, input| {
            if input == 0 { return; }
            for _ in 0..=input { dynasm!(f ; nop) }
            dynasm!(f; mfence);
        }}, 
        EmitterDesc { desc: "nop15 [1 mop]; mfence", 
            func: |f, input| {
            if input == 0 { return; }
            for _ in 0..=input { dynasm!(f ; .bytes NOP15) }
            dynasm!(f; mfence);
        }}, 
        EmitterDesc { desc: "xchg rax,rbx [2 mops]; mfence", 
            func: |f, input| {
            if input == 0 { return; }
            for _ in 0..=input { dynasm!(f ; xchg rax,rbx) }
            dynasm!(f; mfence);
        }}, 

        EmitterDesc { desc: "nop [1 mop]; fnop; mfence", 
            func: |f, input| {
            if input == 0 { return; }
            for _ in 0..=input { dynasm!(f ; nop) }
            dynasm!(f; fnop; mfence );
        }}, 
        EmitterDesc { desc: "nop15 [1 mop]; fnop; mfence", 
            func: |f, input| {
            if input == 0 { return; }
            for _ in 0..=input { dynasm!(f ; .bytes NOP15) }
            dynasm!(f; fnop; mfence);
        }}, 
        EmitterDesc { desc: "xchg rax,rbx [2 mops]; fnop; mfence", 
            func: |f, input| {
            if input == 0 { return; }
            for _ in 0..=input { dynasm!(f ; xchg rax,rbx) }
            dynasm!(f; fnop; mfence);

        }}, 


    ]);

    fn parse_results(exp_results: &ExperimentResults<Zen2Event, usize>) {
        pub enum Strat { NonZero, Zero, None }
        for case_results in exp_results.data.iter() {
            println!("[*] Test case '{}'", case_results.desc);
            for (event, event_results) in case_results.data.iter() {
                let edesc = event.as_desc();
                let minmax = event_results.local_minmax();

                let (gmin, min_idx) = event_results.global_min();
                let (gmax, max_idx) = event_results.global_max();

                let strat = match event { 
                    Zen2Event::Dsp0Stall(0x2) |
                    Zen2Event::DeDisDispatchTokenStalls0(
                        DeDisDispatchTokenStalls0Mask::RetireTokenStall
                    ) => Strat::NonZero,
                    Zen2Event::DeDisOpsFromDecoder(DeDisOpsFromDecoderMask::Fp) 
                      => Strat::Zero,
                    _ => Strat::None,
                };

                match strat {
                    // Find the first test where the minimum observed value 
                    // is non-zero
                    Strat::NonZero => {
                        let limit = minmax.iter().enumerate()
                            .filter(|(idx,x)| x.0 > 0 && *idx != 0)
                            .next()
                            .unwrap_or((0, &(0, 0)));
                        println!("limit={:4} {:03x}:{:02x} {}",
                            limit.0, 
                            edesc.id(), edesc.mask(), 
                            edesc.name()
                        );
                    },
                    Strat::Zero => {
                        let limit = minmax.iter().enumerate()
                            .filter(|(idx,x)| x.0 == 0 && *idx != 0)
                            .next()
                            .unwrap_or((0, &(0, 0)));
                        println!("limit={:4} {:03x}:{:02x} {}",
                            limit.0,
                            edesc.id(), edesc.mask(), 
                            edesc.name()
                        );

                    },
                    Strat::None => {
                        println!("min={:4} (idx={}) max={:4} (idx={}) {:03x}:{:02x} {}",
                            gmin,
                            min_idx,
                            gmax,
                            max_idx,
                            edesc.id(), edesc.mask(), 
                            edesc.name()
                        );
                    },
                }
            }
            println!();

        }

    }

    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();

        // Stall cycles
        events.add(Zen2Event::DeDisDispatchTokenStalls0(
                DeDisDispatchTokenStalls0Mask::RetireTokenStall
        ));

        // NOTE: It seems like this coincides with the retire stall event?
        // (What is this even?)
        events.add(Zen2Event::Dsp0Stall(0x2));

        // Speculatively-dispatched FP ops
        events.add(Zen2Event::DeDisOpsFromDecoder(
                DeDisOpsFromDecoderMask::Fp
        ));

        let mut opts = MispredictedReturnOptions::zen2_defaults()
            .rdpmc_strat(RdpmcStrategy::Gpr(Gpr::R15));
        opts.speculative_epilogue_fn = Some(|f, input| {
            dynasm!(f 
            );
        });

        // Measure all of the testcases
        let mut exp_results = ExperimentResults::new();
        for testcase in Self::CASES.iter() {
            let mut case_res = ExperimentCaseResults::new(testcase.desc);
            for input in 0..=256 {
                let asm = Self::emit(opts, input, testcase.func);
                let asm_reader = asm.reader();
                let asm_tgt_buf = asm_reader.lock();
                let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
                let asm_fn: MeasuredFn = unsafe { 
                    std::mem::transmute(asm_tgt_ptr)
                };
                for event in events.iter() {
                    let desc = event.as_desc();
                    let results = harness.measure(asm_fn, 
                        &desc, 256, InputMethod::Fixed(0, 0)
                    ).unwrap();
                    case_res.record(*event, input, results.data);
                }
            }
            exp_results.push(case_res.clone());
        }
        Self::parse_results(&exp_results);
    }
}
