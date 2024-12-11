
use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use rand::prelude::*;
use perfect::asm::Emitter;

fn main() {
    let mut harness = HarnessConfig::default_zen2().emit();
    TbbCapacitySpec::run(&mut harness);
}

/// Measure the capacity of the "taken branch buffer". 
///
/// Context
/// =======
///
/// The function of the "taken branch buffer" is completely undocumented and 
/// not mentioned in any AMD documentation, apart from the description of the
/// PMC event ([`DeDisDispatchTokenStalls0Mask::TakenBrnchBufferRsrc`]). 
///
/// Test
/// ====
///
/// 1. Speculatively dispatch `N` always-taken control flow instructions
/// 2. If we measure stall cycles, `N` must be the number of available entries
///    in the taken branch buffer? 
///
/// Results
/// =======
///
/// We always measure stall cycles after 31 taken control-flow instructions.
/// Stall cycles never occur for never-taken control-flow instructions.
///
/// If we assume that the capacity is actually 32 entries, this is some 
/// evidence that the taken branch buffer tracks taken control-flow 
/// instructions that have been speculatively completed but not-yet-retired.
/// Since the RET in our gadget is also pending, we would expect to only 
/// measure the capacity at 31. 
///
pub struct TbbCapacitySpec;
impl MispredictedReturnTemplate<usize> for TbbCapacitySpec {}
impl TbbCapacitySpec {

    const CASES: StaticEmitterCases<usize> = StaticEmitterCases::new(&[
        EmitterDesc { desc: "jmp 0x0", 
            func: |f, input| {
            for _ in 0..=input { 
                dynasm!(f ; jmp >next ; next:) 
            }
        }}, 
        EmitterDesc { desc: "lea rdi, [>next]; jmp rdi", 
            func: |f, input| {
            for _ in 0..=input { 
                dynasm!(f 
                    ; lea rdi, [>next]
                    ; jmp rdi
                    ; next:
                )
            }
        }}, 
        EmitterDesc { desc: "call 0x0", 
            func: |f, input| {
            for _ in 0..=input { 
                dynasm!(f ; call >next ; next:) 
            }
        }}, 

        EmitterDesc { desc: "jz 0x0 (always-taken)", 
            func: |f, input| {
            for _ in 0..=input { 
                dynasm!(f ; jz >next ; next:) 
            }
        }}, 

        EmitterDesc { desc: "jnz 0x0 (never-taken)", 
            func: |f, input| {
            for _ in 0..=input { 
                dynasm!(f ; jnz >next ; next:) 
            }
        }}, 




    ]);

    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::DeDisDispatchTokenStalls1(
                DeDisDispatchTokenStalls1Mask::TakenBrnchBufferRsrc
        ));

        //events.add(Zen2Event::LsPrefInstrDisp(0x01));

        let opts = MispredictedReturnOptions::zen2_defaults()
            .post_prologue_fn(Some(|f, input| { 
                dynasm!(f
                    ; cmp rcx, 0
                );
            }))
            .speculative_epilogue_fn(Some(|f, input| {
                dynasm!(f ; prefetch [rax])
            }))
            .rdpmc_strat(RdpmcStrategy::Gpr(Gpr::R15));

        let mut exp_results = ExperimentResults::new();
        for testcase in Self::CASES.iter() {
            println!("[*] Testcase '{}'", testcase.desc);
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
                        desc.id(), desc.mask(), 256, InputMethod::Fixed(0, 0)
                    ).unwrap();
                    case_res.record(*event, input, results);
                }
            }
            exp_results.push(case_res.clone());
        }

        pub enum Strat { NonZero, Zero, None }
        for case_results in exp_results.data.iter() {
            for (event, event_results) in case_results.data.iter() {
                let edesc = event.as_desc();
                let minmax = event_results.local_minmax();

                let (gmin, min_idx) = event_results.global_min();
                let (gmax, max_idx) = event_results.global_max();

                let strat = match event { 
                    Zen2Event::DeDisDispatchTokenStalls1(
                        DeDisDispatchTokenStalls1Mask::TakenBrnchBufferRsrc
                    ) => Strat::NonZero,
                    Zen2Event::LsPrefInstrDisp(_) => Strat::Zero,
                    _ => Strat::None,
                };

                match strat {
                    Strat::NonZero => {
                        let limit = minmax.iter().enumerate()
                            .filter(|(idx,x)| x.0 > 0)
                            .next()
                            .unwrap_or((0, &(0, 0)));
                        println!("{:03x}:{:02x}, limit={:4} ({}) {}",
                            edesc.id(), edesc.mask(), limit.0, case_results.desc,
                            edesc.name()
                        );

                    },
                    Strat::Zero => {
                        let limit = minmax.iter().enumerate()
                            .filter(|(idx,x)| x.0 == 0)
                            .next()
                            .unwrap_or((0, &(0, 0)));
                        println!("{:03x}:{:02x}, limit={:4} ({}) {}",
                            edesc.id(), edesc.mask(), limit.0, case_results.desc,
                            edesc.name()
                        );

                    },
                    Strat::None => {
                        println!("{:03x}:{:02x}, min={:4} (idx={}) max={:4} (idx={}) ({}) {}",
                            edesc.id(), edesc.mask(), 
                            gmin,
                            min_idx,
                            gmax,
                            max_idx,
                            case_results.desc,
                            edesc.name()
                        );


                    },
                }
            }

        }

    }


}


