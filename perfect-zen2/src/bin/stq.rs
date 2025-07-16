
use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use rand::prelude::*;
use perfect::asm::Emitter;

fn main() {
    let args = ExperimentArgs::parse();
    let mut harness = match HarnessConfig::from_cmdline_args(&args) {
        Some(cfg) => cfg.emit(),
        None => HarnessConfig::default_zen2().emit()
    };
    StoreQueueCapacity::run(&mut harness);
}

/// Create store queue pressure. 
///
/// Context
/// =======
///
/// In the Zen microarchitecture, a store queue (STQ) tracks the addresses 
/// and values of pending stores. The capacity of the store queue reflects
/// the maximum number of in-flight stores. 
///
/// The Family 17h SOG mentions that the store queue capacity is 48 entries. 
/// The Family 19h SOG mentions that the store queue capacity is 64 entries. 
///
/// Test
/// ====
///
/// Execute repeated stores to the same address and measure stall cycles 
/// (with 'StoreQueueRsrcStall'). When we exceed the store queue capacity,
/// the number of stall cycles will be nonzero. 
///
/// Results
/// =======
///
/// On Zen 2, stall cycles observed when we perform more than 48 stores. 
/// On Zen 3, stall cycles observed when we perform more than 64 stores. 
///
pub struct StoreQueueCapacity;
impl MispredictedReturnTemplate<usize> for StoreQueueCapacity {}
impl StoreQueueCapacity {

    const CASES: StaticEmitterCases<usize> = StaticEmitterCases::new(&[
        EmitterDesc { desc: "mov [imm], r64", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; mov [0x1000], Rq(0)); }
        }}, 
        EmitterDesc { desc: "mov [imm], r32", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; mov [0x1000], Rd(0)); }
        }}, 
        EmitterDesc { desc: "mov [imm], r16", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; mov [0x1000], Rw(0)); }
        }}, 
        EmitterDesc { desc: "movnti [imm], r64", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; movnti [0x1000], Rq(0)); }
        }}, 
        EmitterDesc { desc: "movnti [imm], r32", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; movnti [0x1000], Rd(0)); }
        }}, 
        EmitterDesc { desc: "sfence", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; sfence) }
        }}, 


    ]);

    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::DeDisDispatchTokenStalls1(
                DeDisDispatchTokenStalls1Mask::StoreQueueRsrcStall
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


