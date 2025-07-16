
use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use perfect::stats::*;
use perfect::asm::Emitter;

fn main() {
    let args = ExperimentArgs::parse();
    let mut harness = match HarnessConfig::from_cmdline_args(&args) {
        Some(cfg) => cfg.emit(),
        None => HarnessConfig::default_zen2().emit()
    };
    LoadQueueCapacity::run(&mut harness);
}

/// Create load queue pressure. 
///
/// Context
/// =======
///
/// In the Zen microarchitecture, a load queue (LDQ) keeps track of pending 
/// loads. The capacity of the load queue reflects the maximum number of  
/// in-flight loads. 
///
/// The Family 17h SOG mentions that the load queue capacity is 44 entries. 
/// The Family 19h SOG omits the load queue capacity, but mentions that 
/// "the load-store unit can process up to 72 out-of-order loads". 
///
/// Test
/// ====
///
/// Execute repeated loads to the same address and measure stall cycles 
/// (with 'LoadQueueRsrcStall'). When we exceed the load queue capacity,
/// we expect the number of stall cycles to be nonzero. 
///
/// Results
/// =======
///
/// For Zen 2: 
///     - Stall cycles are observed when we perform more than 43 loads
///       (16, 32, or 64-bit loads). 
///     - Stall cycles are observed when we perform more than 44 
///       prefetch instructions.
///
/// For Zen 3: 
///     - Stall cycles are observed when we perform more than 42 loads
///       (16, 32, or 64-bit loads)
///     - Stall cycles are observed when we perform more than ~115
///       prefetch instructions
///
pub struct LoadQueueCapacity;
impl MispredictedReturnTemplate<usize> for LoadQueueCapacity {}
impl LoadQueueCapacity {

    const CASES: StaticEmitterCases<usize> = StaticEmitterCases::new(&[
        EmitterDesc { desc: "mov r64, [imm]", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; mov Rq(0), [0x1000]); }
        }}, 
        EmitterDesc { desc: "mov r32, [imm]", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; mov Rd(0), [0x1000]); }
        }}, 
        EmitterDesc { desc: "mov r16, [imm]", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; mov Rw(0), [0x1000]); }
        }}, 
        EmitterDesc { desc: "prefetch [imm]", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; prefetch [0x1000]); }
        }}, 
        EmitterDesc { desc: "prefetchnta [imm]", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; prefetchnta [0x1000]); }
        }}, 



    ]);

    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::DeDisDispatchTokenStalls1(
                DeDisDispatchTokenStalls1Mask::LoadQueueRsrcStall
        ));

        let opts = MispredictedReturnOptions::zen2_defaults()
            .prologue_fn(Some(|f, input| { 
                dynasm!(f
                );
            }))
            .rdpmc_strat(RdpmcStrategy::Gpr(Gpr::R15));

        let mut exp_results = ExperimentResults::new();
        for testcase in Self::CASES.iter() {
            println!("[*] Testcase '{}'", testcase.desc);
            let mut case_res = ExperimentCaseResults::new(testcase.desc);

            for input in 0..=128 {
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
                    case_res.record(*event, input, results.data);
                }
            }
            exp_results.push(case_res.clone());
        }

        for case_results in exp_results.data.iter() {
            for (event, event_results) in case_results.data.iter() {
                let edesc = event.as_desc();
                let minmax = event_results.local_minmax();

                // Find the first test where the minimum observed number of 
                // events is non-zero 
                let limit = minmax.iter().enumerate()
                    .filter(|(idx,x)| x.0 > 0)
                    .next()
                    .unwrap_or((0, &(0, 0)));

                println!("{:03x}:{:02x}, limit={:4} ({})",
                    edesc.id(), edesc.mask(), limit.0, case_results.desc
                );

            }

        }

    }



}


