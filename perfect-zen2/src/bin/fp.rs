use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use rand::prelude::*;
use std::io::Write;

/// Create vector physical register file (PRF) pressure.
///
/// The Family 17h SOG mentions that the PRF has 160 entries.
///
pub struct VectorPrfPressure;
impl MispredictedReturnTemplate<usize> for VectorPrfPressure {}
impl VectorPrfPressure {

    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::DeDisDispatchTokenStalls1(
                DeDisDispatchTokenStalls1Mask::FpRegFileRsrcStall
        ));

        let opts = MispredictedReturnOptions::zen2_defaults()
            .rdpmc_strat(RdpmcStrategy::MemStatic(0x0000_5670))
            .prologue_fn(Some(|mut f, _| {
                dynasm!(f
                    ; mov r8, 0x1111_2222_3333_4444
                    ; vzeroall
                );
                for _ in 0..16 {
                    for idx in 0..=15 {
                        dynasm!(f; vpxor Ry(idx), Ry(idx), Ry(idx));
                    }
                }
            }));

        let cases = StaticEmitterCases::new(&[
            EmitterDesc { desc: "vaddpd (ymm)", 
                func: |f, input| {
                for _ in 0..=input { 
                    dynasm!(f ; vaddpd Ry(0), Ry(0), Ry(0));
                }
            }}, 
            EmitterDesc { desc: "vaddpd (xmm)", 
                func: |f, input| {
                for _ in 0..=input { 
                    dynasm!(f ; vaddpd Rx(0), Rx(0), Rx(0));
                }
            }}, 
            EmitterDesc { desc: "vpxor", 
                func: |f, input| {
                for _ in 0..=input { 
                    dynasm!(f ; vpxor Ry(0), Ry(0), Ry(0));
                }
            }}, 
            EmitterDesc { desc: "vmovq (from r8)", 
                func: |f, input| {
                for _ in 0..=input { 
                    dynasm!(f ; vmovq Rx(0), r8);
                }
            }}, 
        ]);

        let mut exp_results = ExperimentResults::new();
        for testcase in cases.iter() {
            println!("[*] Testcase '{}'", testcase.desc);
            let mut case_res = ExperimentCaseResults::new(testcase.desc);

            for i in 0..=256 {
                let asm = Self::emit(opts, i, testcase.func);

                let asm_reader = asm.reader();
                let asm_tgt_buf = asm_reader.lock();
                let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
                let asm_fn: MeasuredFn = unsafe { 
                    std::mem::transmute(asm_tgt_ptr)
                };

                for event in events.iter() {
                    let desc = event.as_desc();
                    let results = harness.measure(asm_fn, 
                        &desc, 64, InputMethod::Fixed(0, 0)
                    ).unwrap();
                    case_res.record(*event, i, results.data);
                }
            }
            exp_results.push(case_res.clone());
        }

        for case_results in exp_results.data.iter() {
            println!("# Results for case '{}'", case_results.desc);

            for (event, event_results) in case_results.data.iter() {
                let edesc = event.as_desc();
                let case_name = case_results.desc.replace(" ", "_").to_lowercase();
                let event_name = edesc.name().replace(".", "_").to_lowercase();

                let path = format!("/tmp/{}__{:02x}_{:02x}_{}.dat", case_name, 
                    edesc.id(), edesc.mask(), event_name);
                let mut f = std::fs::OpenOptions::new()
                    .write(true) .create(true) .truncate(true)
                    .open(&path).unwrap();

                println!("# Event {:03x}:{:02x} '{}'", 
                    edesc.id(), edesc.mask(), edesc.name());
                println!("writing to {}", path);


                let minmax = event_results.local_minmax();
                let avgs = event_results.local_avg_usize();
                for ((input, avg), (min, max)) in event_results.inputs.iter()
                    .zip(avgs.iter()).zip(minmax.iter()) 
                {
                    writeln!(f, "input={} min={} avg={} max={}", input, min, avg, max)
                        .unwrap();
                    println!("input={} min={} avg={} max={}", input, min, avg, max);
                }

            }
            println!();
        }
    }
}


fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .zero_strategy_fp(ZeroStrategyFp::Vzeroall)
        .emit();
    VectorPrfPressure::run(&mut harness);
}

