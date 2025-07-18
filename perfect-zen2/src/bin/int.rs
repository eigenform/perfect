
use std::io::Write;
use perfect::*;
use perfect::events::*;
use perfect::asm::*;
use perfect::stats::*;
use rand::prelude::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .zero_strategy(ZeroStrategy::MovFromZero)
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
/// If we wanted to create pressure, we'd need to dispatch many instructions
/// but somehow prevent them from retiring.
///
/// > "SOG for AMD Family 17h Models 30h and Greater Processors",
/// > Document #56305, Rev 3.02 (dated March 2020):
/// > 
/// > *"The integer physical register file (PRF) consists of 180 registers, 
/// > with up to 38 per thread mapped to architectural state or 
/// > microarchitectural temporary state."*
///
/// Test
/// ====
///
/// In the shadow of a costly mispredicted branch, speculatively allocate as 
/// many physical registers as we can (ie. by repeating an instruction which 
/// allocates). Eventually, after emitting a certain number of instructions, 
/// we expect to measure stall cycles (ie. with `IntPhyRegFileRsrcStall`)
/// because we cannot dispatch more instructions without the availability of 
/// free physical registers.
///
/// If we assume that no other conditions are preventing instructions from 
/// being dispatched, this means that the number of free physical registers 
/// available during the test must be [approximately] 
/// `(# of instructions) * (# of allocations per instruction)`.
///
/// Results
/// =======
///
/// Observations are mostly consistent when running this executable multiple
/// times. 
///
/// - For instructions which allocate a single result, the maximum observed 
///   number of instructions dispatched before seeing stall cycles is usually 
///   160. 
///
/// - MUL starts stalling after 80 instructions; this is consistent with our
///   expectation that it allocates twice (for the result in RAX and RDX).
///
/// - The LAHF test starts stalling after 40 instructions. You could interpret
///   this to mean that the implementation in microcode allocates three times,
///   plus once for the architecturally-visible result in AH? 
///
/// I don't have a way to reproduce this, but it *is* possible to see a 
/// maximum of 168. I've seen the results suddenly switch to consistently 
/// being 168 - but I don't have a good explanation for why. 
///
/// - It never happens shortly after rebooting the machine
/// - After observing it, it seems stable (I never see it return to 160)
///
/// Presumably this means there are typically `(180 - 160) = 20` physical 
/// registers that are still being used when we run this test. 
///

pub struct IntPrfPressure;
impl MispredictedReturnTemplate<usize> for IntPrfPressure {}
impl IntPrfPressure {

    /// Set of test cases. 
    ///
    /// Each of these cases consists of a single repeated instruction. 
    /// The instructions in each case are only executed speculatively.
    const CASES: StaticEmitterCases<usize> = StaticEmitterCases::new(&[

        EmitterDesc { desc: "add r64, r64", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; add Rq(0), Rq(0)); }
        }}, 
        EmitterDesc { desc: "add r32, r32", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; add Rd(0), Rd(0)); }
        }}, 
        EmitterDesc { desc: "add r64, 0x1", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; add Rq(0), 1); }
        }}, 

        EmitterDesc { desc: "vmovq r64, xmm", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; vmovq Rq(0), xmm0); }
        }}, 
        EmitterDesc { desc: "movd r32, xmm", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; movd Rd(0), xmm0); }
        }}, 
        EmitterDesc { desc: "movmskpd r64, xmm", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; movmskpd Rq(0), xmm0) }
        }}, 
        EmitterDesc { desc: "movmskps r64, xmm", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; movmskps Rq(0), xmm0) }
        }}, 

        // NOTE: These don't stall for physical registers (probably because
        // we're stalling for floating-point resources?)
        EmitterDesc { desc: "cvtsd2si r64, xmm", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; cvtsd2si Rq(0), xmm0); }
        }}, 
        EmitterDesc { desc: "cvtss2si r64, xmm", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; cvtss2si Rq(0), xmm0); }
        }}, 
        EmitterDesc { desc: "cvttsd2si r64, xmm", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; cvttsd2si Rq(0), xmm0); }
        }}, 
        EmitterDesc { desc: "cvttss2si r64, xmm", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; cvttss2si Rq(0), xmm0); }
        }}, 
        EmitterDesc { desc: "extractps r64, xmm, 0", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; extractps Rq(0), xmm0, 0) }
        }}, 






        EmitterDesc { desc: "inc r64", 
            func: |f, input| {
            for i in 0..=input { dynasm!(f ; inc Rq(0)); }
        }}, 
        EmitterDesc { desc: "inc r32", 
            func: |f, input| {
            for i in 0..=input { dynasm!(f ; inc Rd(0)); }
        }}, 
        EmitterDesc { desc: "inc r16", 
            func: |f, input| {
            for i in 0..=input { dynasm!(f ; inc Rh(0)); }
        }}, 
        EmitterDesc { desc: "inc r8", 
            func: |f, input| {
            for i in 0..=input { dynasm!(f ; inc Rb(0)); }
        }}, 

        EmitterDesc { desc: "mov r64, imm (random)", 
            func: |f, input| {
            let mut rng = thread_rng();
            for i in 0..=input { 
                let val: i64 = rng.gen_range(
                    0x1000_0000_0000_0000..=0x2000_0000_0000_0000
                );
                dynasm!(f ; mov Rq(0), QWORD val);
            }
        }}, 
        EmitterDesc { desc: "mov r64, imm (zero)", 
            func: |f, input| {
            for i in 0..=input { dynasm!(f ; mov Rq(0), QWORD 0); }
        }}, 


        EmitterDesc { desc: "cmp r64, imm (zero)", 
            func: |f, input| {
            for i in 0..=input { dynasm!(f ; cmp Rq(0), 0); }
        }}, 

        EmitterDesc { desc: "cmp r64, imm (random)", 
            func: |f, input| {
            let mut rng = thread_rng();
            for i in 0..=input { 
                let val: i32 = rng.gen_range(
                    0x1000_0000..=0x2000_0000
                );
                dynasm!(f ; cmp Rq(0), DWORD val);
            }
        }}, 


        EmitterDesc { desc: "cmp r64, rax", 
            func: |f, input| {
            for i in 0..=input { dynasm!(f ; cmp Rq(0), rax); }
        }}, 

        EmitterDesc { desc: "lea r64, [rip]", 
            func: |f, input| {
            for i in 0..=input { dynasm!(f ; lea Rq(0), [rip]); }
        }}, 

        EmitterDesc { desc: "lea r64, [rip + imm] (random)", 
            func: |f, input| {
            let mut rng = thread_rng();
            for i in 0..=input { 
                let val: i32 = rng.gen_range(
                    0x1000_0000..=0x2000_0000
                );
                //let reg = (i % 16) as u8;
                dynasm!(f ; lea Rq(0), [rip + val]);
            }
        }}, 


        EmitterDesc { desc: "xor r64, r64 (zero idiom)", 
            func: |f, input| {
            for i in 0..=input { dynasm!(f ; xor Rq(0), Rq(0)); }
        }}, 


        EmitterDesc { desc: "mul r64", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; mul Rq(0)); }
        }}, 

        EmitterDesc { desc: "lahf", 
            func: |f, input| {
            for _ in 0..=input { dynasm!(f ; lahf); }
        }}, 


    ]);


    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::DeDisDispatchTokenStalls1(
                DeDisDispatchTokenStalls1Mask::IntPhyRegFileRsrcStall
        ));

        let opts = MispredictedReturnOptions::zen2_defaults()
            .free_pregs(true)
            .prologue_fn(Some(|f, input| { 
                dynasm!(f; mov rax, 0xdeadbeef; vmovq xmm0, rax)
            }))
            .rdpmc_strat(RdpmcStrategy::MemStatic(0x0000_5670));

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
                        &desc, 256, InputMethod::Fixed(0, 0)
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

                if limit.0 != 0 {
                    println!("{:03x}:{:02x}, limit={:4} ({})",
                        edesc.id(), edesc.mask(), limit.0, case_results.desc
                    );
                }

            }

        }

    }
}


