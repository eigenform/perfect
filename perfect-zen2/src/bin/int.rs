
use std::io::Write;
use perfect::*;
use perfect::events::*;
use perfect::asm::*;
use perfect::stats::*;
use rand::prelude::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .zero_strategy(ZeroStrategy::XorIdiom)
        .arena_alloc(0, 0x2000_0000)
        .emit();
    IntPrfPressure::run(&mut harness);
}

/// Create integer physical register file (PRF) pressure.
///
/// Context
/// =======
///
/// Each time we execute an instruction with some register result (apart from 
/// ones that are eliminated by renaming), the machine needs to allocate a 
/// physical register for it. When instructions are retired, the physical 
/// register can be freed for use.
///
/// If a physical register cannot be allocated, an instruction *must* remain
/// stalled at dispatch until one is available - this is required for the
/// dynamic scheduling in out-of-order machines.
///
/// Pressure in the physical register file occurs when many instructions are
/// in-flight (meaning "dispatched but not-yet-retired") simultaneously. 
///
/// If we wanted to create pressure, we'd need to dispatch many instructions
/// but somehow prevent them from retiring.
///
/// Test
/// ====
///
/// Speculatively allocate a lot of physical registers. 
/// Eventually, we expect to measure stall cycles ('IntPhyRegFileRsrcStall') 
/// because we cannot dispatch instructions without the availability of free 
/// physical registers.
///
/// Results
/// =======
///
/// The Family 17h SOG mentions that the PRF is:
/// > "[...] 180 registers, with up to 38 per thread mapped to architectural 
/// > state or micro-architectural temporary state." 
///
/// Inconsistent, but this is probably just due to our lack of control over 
/// the exact state of the physical register file before the measurement
/// (which is totally expected). The maximum observed number of instructions 
/// dispatched before seeing stall cycles is 159. 
///
/// Zeroing Idioms?
/// ===============
///
/// Strangely, if you use `xor rax,rax`/`sub rax,rax`, these also allocate 
/// and eventually stall. This is sort of counterintuitive because we expect
/// that zeroing idioms are eliminated (ie. handled with renaming by setting 
/// some zero bit on the integer register map), and that there's be no reason
/// for them to actually allocate? 
///
/// See the experiment in `src/bin/rename.rs` for more information. 
///
pub struct IntPrfPressure;
impl MispredictedReturnTemplate<usize> for IntPrfPressure {}
impl IntPrfPressure {
    const CASES: StaticEmitterCases<usize> = StaticEmitterCases::new(&[
        EmitterDesc { desc: "add reg", 
            func: |f, input| {
            for _ in 0..=input { 
                dynasm!(f ; add Rq(0), Rq(0));
            }
        }}, 
        EmitterDesc { desc: "add QWORD imm (one)", 
            func: |f, input| {
            for _ in 0..=input { 
                dynasm!(f ; add Rq(0), 1);
            }
        }}, 
        EmitterDesc { desc: "inc", 
            func: |f, input| {
            for _ in 0..=input { 
                dynasm!(f ; inc Rq(0));
            }
        }}, 
        EmitterDesc { desc: "mov qword imm (random)", 
            func: |f, input| {
            let mut rng = thread_rng();
            for i in 0..=input { 
                let val: i64 = rng.gen_range(
                    0x1000_0000_0000_0000..=0x2000_0000_0000_0000
                );
                let reg = (i % 16) as u8;
                dynasm!(f ; mov Rq(reg), QWORD val);
            }
        }}, 
        EmitterDesc { desc: "mov qword imm (zero)", 
            func: |f, input| {
            for i in 0..=input { 
                let reg = (i % 16) as u8;
                dynasm!(f ; mov Rq(reg), QWORD 0);
            }
        }}, 
        EmitterDesc { desc: "lahf", 
            func: |f, input| {
            for _ in 0..=input { 
                dynasm!(f ; lahf);
            }
        }}, 
        EmitterDesc { desc: "cmp imm", 
            func: |f, input| {
            for i in 0..=input { 
                let reg = (i % 16) as u8;
                dynasm!(f ; cmp Rq(reg), 0);
            }
        }}, 
        EmitterDesc { desc: "zero idiom", 
            func: |f, input| {
            for i in 0..=input { 
                let reg = (i % 16) as u8;
                dynasm!(f ; xor Rq(reg), Rq(reg));
            }
        }}, 
    ]);


    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::DeDisDispatchTokenStalls1(
                DeDisDispatchTokenStalls1Mask::IntPhyRegFileRsrcStall
        ));

        let opts = MispredictedReturnOptions::zen2_defaults()
            .free_pregs(true)
            .rdpmc_strat(RdpmcStrategy::MemStatic(0x0000_5670));

        let mut exp_results = ExperimentResults::new();
        for testcase in Self::CASES.iter() {
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
                        desc.id(), desc.mask(), 64, InputMethod::Fixed(0, 0)
                    ).unwrap();
                    case_res.record(*event, i, results);
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

                //let path = format!("/tmp/{}__{:02x}_{:02x}_{}.dat", case_name, 
                //    edesc.id(), edesc.mask(), event_name);
                //let mut f = std::fs::OpenOptions::new()
                //    .write(true) .create(true) .truncate(true)
                //    .open(&path).unwrap();

                println!("# Event {:03x}:{:02x} '{}'", 
                    edesc.id(), edesc.mask(), edesc.name());
                //println!("writing to {}", path);


                let minmax = event_results.local_minmax();
                let avgs = event_results.local_avg_usize();
                for ((input, avg), (min, max)) in event_results.inputs.iter()
                    .zip(avgs.iter()).zip(minmax.iter()) 
                {
                    //writeln!(f, "input={} min={} avg={} max={}", 
                    //  input, min, avg, max).unwrap();
                    println!("input={} min={} avg={} max={}", input, min, avg, max);
                }

            }
            println!();
        }
    }
}


