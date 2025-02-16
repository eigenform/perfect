
use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use perfect::stats::*;
use perfect::asm::Emitter;

fn main() {
    let mut harness = HarnessConfig::default_zen2().emit();
    LoadQueueCapacity::run(&mut harness);
}

/// Create load queue pressure. 
///
/// Explanation
/// ===========
///
/// The load queue keeps track of the addresses/values of recent stores. 
/// The Family 17h SOG mentions that the store queue capacity is 48 entries. 
///
/// Test
/// ====
///
/// Execute many stores to different addresses and measure stall cycles 
/// (with 'StoreQueueRsrcStall'). When we exceed the store queue capacity,
/// the number of stall cycles will be nonzero. 
///
/// Results
/// =======
///
/// Stall cycles observed when we perform more than 48 stores. 
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

    //pub fn emit(num_loads: usize) -> X64Assembler {
    //    let mut rng = rand::thread_rng();
    //    let mut f = X64Assembler::new().unwrap();

    //    let mut addrs: Vec<i32> = (0x0001_0000..=0x0002_1000)
    //        .step_by(64).collect();
    //    assert!(num_loads < addrs.len());
    //    addrs.shuffle(&mut rng);

    //    for addr in &addrs[0..=num_loads] {
    //        dynasm!(f 
    //            ; mov rax, *addr as _
    //            ; clflush [rax]
    //        );
    //    }

    //    dynasm!(f
    //        ; mov rax, 0x1111_dead_1111_dead
    //        ; vmovq xmm0, rax
    //        ; vpbroadcastq ymm0, xmm0
    //        ; lfence
    //        ; .align 4096
    //        ; lfence
    //        ; mfence
    //    );

    //    f.emit_rdpmc_start(0, Gpr::R15 as u8);

    //    for addr in &addrs[0..=num_loads] {
    //        dynasm!(f ; mov rax, [*addr]); // 8B
    //        //dynasm!(f ; mov eax, [*addr]); // 4B
    //        //dynasm!(f ; mov ax, [*addr]); // 2B
    //        //dynasm!(f ; mov al, [*addr]); // 1B
    //        //dynasm!(f ; movnti rax, [*addr]);
    //        //dynasm!(f ; vmovd xmm0, [*addr]); // 4B
    //        //dynasm!(f ; vmovq xmm0, [*addr]); // 8B
    //        //dynasm!(f ; vmovdqa xmm0, [*addr]); // 16B
    //        //dynasm!(f ; vmovdqa ymm0, [*addr]); // 32B
    //    }

    //    f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
    //    f.emit_ret();
    //    f.commit().unwrap();
    //    f
    //}

    //pub fn run(harness: &mut PerfectHarness) {
    //    let mut events = EventSet::new();
    //    //events.add(Zen2Event::LsDispatch(LsDispatchMask::LdDispatch));
    //    events.add(Zen2Event::DeDisDispatchTokenStalls1(
    //        DeDisDispatchTokenStalls1Mask::LoadQueueRsrcStall
    //    ));

    //    for num_loads in 0..=128 {
    //        let asm = Self::emit(num_loads);
    //        let asm_reader = asm.reader();
    //        let asm_tgt_buf = asm_reader.lock();
    //        let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
    //        let asm_fn: MeasuredFn = unsafe { 
    //            std::mem::transmute(asm_tgt_ptr)
    //        };

    //        println!("[*] num_loads={}", num_loads);
    //        for event in events.iter() { 
    //            let desc = event.as_desc();
    //            let results = harness.measure(asm_fn, 
    //                desc.id(), desc.mask(), 128, InputMethod::Fixed(0, 0),
    //            ).unwrap();

    //            let dist = results.get_distribution();
    //            let min = results.get_min();
    //            let max = results.get_max();
    //            println!("  {:03x}:{:02x} {:032} min={:3} max={:3}",
    //                desc.id(), desc.mask(), desc.name(), min, max);
    //        }
    //    }
    //}

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


