
use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use perfect::stats::*;
use perfect::asm::Emitter;

fn main() {
    let mut harness = HarnessConfig::default_zen2().emit();
    DispatchTest::run(&mut harness);
}


/// Use PMC events to characterize instruction dispatch. 
///
/// Context
/// ======= 
///
/// According to different revisions of the SOG, there are three ways that an 
/// x86 instruction may be represented internally:
///
/// - "Fastpath single": 1 macro-op
/// - "Fastpath double": 2 macro-ops
/// - "Microcoded":     >2 macro-ops
///
/// Each macro-op [mop] corresponds with up to 2 micro-ops [uop].
/// 
/// Some other relevant facts from the Family 17h SOG: 
///
/// - Up to 4 instructions can be decoded per cycle
/// - Up to 6 macro-ops [mops] can be dispatched per cycle
///
/// Tests
/// =====
///
/// In general, the strategy here is: 
///
/// - Emit some short string of instructions
/// - Measure the number of cycles and the number of ops dispatched each cycle
///
/// Observations
/// ============
///
/// It seems like the size of a dispatch group is constrained by how the 
/// instructions are decomposed into macro-ops? The following permutations 
/// can be dispatched in a single cycle: 
///
/// - 3 fastpath double instructions (6 mops)
/// - 2 fastpath double + 2 fastpath single instructions (6 mops)
/// - 1 fastpath double + 4 fastpath single instructions (6 mops)
/// - 5 fastpath single instructions (5 mops)
///
/// Note that we [seemingly] cannot dispatch 6 fastpath single instructions. 
/// This is easy to observe with NOP: we never observe cycles where the full 
/// 6 NOPs are dispatched simultaneously, and it always occurs over two cycles.
///
/// It seems like at least one fastpath double instruction is required to 
/// achieve full throughput. The location of the fastpath double mops within
/// the dispatch group does not seem to matter, ie. all of the following 
/// cases lead to 6 dispatched mops in a single cycle: 
///
///     add; sub; and; mov; mul
///     add; sub; and; mul; mov
///     add; sub; mul; and; mov
///     add; mul; sub; and; mov
///     mul; add; sub; and; mov
///
/// In summary, I think this means that: 
///
/// - The "dispatch window" (set of instructions at the head of the op queue 
///   which are eligible to be dispatched) consists of 5 instructions 
///
/// - The "dispatch group" (set of macro-ops being removed from the queue) 
///   consists of up to 6 macro-ops from the window
///
pub struct DispatchTest;
impl DispatchTest {

    const CASES: StaticEmitterCases<usize> = StaticEmitterCases::new(&[

        // 3 double (6 dispatched mops)
        EmitterDesc { desc: "xchg (3)", 
            func: |f, input| {
            dynasm!(f
                ; xchg rax, rbx
                ; xchg rax, rbx
                ; xchg rax, rbx
            );
        }}, 

        // 2 double, two single (6 dispatched mops)
        EmitterDesc { desc: "xchg (2); nop (2)", 
            func: |f, input| {
            dynasm!(f
                ; xchg rax, rbx
                ; xchg rax, rbx
                ; nop
                ; nop
            );
        }}, 


        // 3 fastpath double (6 dispatched mops)
        EmitterDesc { desc: "mul (3)", 
            func: |f, input| {
            dynasm!(f
                ; mul rdx
                ; mul rdx
                ; mul rdx
            );
        }}, 


        // 4 single, 1 double (6 dispatched mops)
        EmitterDesc { desc: "nop (4); mul", 
            func: |f, input| {
            dynasm!(f
                ; nop
                ; nop
                ; nop
                ; nop
                ; mul rdx
            );
        }}, 

        // 4 single, 1 double (6 dispatched mops)
        EmitterDesc { desc: "nop (3); mul; nop", 
            func: |f, input| {
            dynasm!(f
                ; nop
                ; nop
                ; nop
                ; mul rdx
                ; nop
            );
        }}, 

 
        // 4 single, 1 double (6 dispatched mops)
        EmitterDesc { desc: "add; sub; and; load; mul", 
            func: |f, input| {
            dynasm!(f
                ; add rax, 1
                ; sub rbx, 1
                ; and rcx, 1
                ; mov rdi, [0x1000]
                ; mul rax
            );
        }}, 

        // 6 single (4 dispatched mops; 2 dispatched mops)
        EmitterDesc { desc: "load (6)",
            func: |f, input| {
            dynasm!(f
                ; mov rdi, [0x1000]
                ; mov rsi, [0x1100]
                ; mov rax, [0x1200]
                ; mov rbx, [0x1300]
                ; mov rcx, [0x1400]
                ; mov rcx, [0x1500]
            );
        }}, 

        // 6 single (4 dispatched mops; 4 dispatched mops)
        EmitterDesc { desc: "lea r64, [imm] (6)",
            func: |f, input| {
            dynasm!(f
                ; lea rsi, [0x1100]
                ; lea rax, [0x1200]
                ; lea rbx, [0x1300]
                ; lea rcx, [0x1400]
                ; lea rdx, [0x1500]
                ; lea rdi, [0x1600]
            );
        }}, 


        // 6 single (5 dispatched mops; 1 dispatched mop)
        EmitterDesc { desc: "lea r64, [r64+imm] (1); lea r64, [imm] (5)",
            func: |f, input| {
            dynasm!(f
                ; lea rdi, [rip + 0x200]
                ; lea rsi, [0x1100]
                ; lea rax, [0x1200]
                ; lea rbx, [0x1300]
                ; lea rcx, [0x1400]
                ; lea rdx, [0x1500]
            );
        }}, 

        // 6 single (4 dispatched mops; 2 dispatched mops)
        // NOTE: I think we can only dispatch 4 ALU ops per cycle
        // (one for each ALSQ) 
        EmitterDesc { desc: "add; add; add; add; add; add",
            func: |f, input| {
            dynasm!(f
                ; add rax, rax
                ; add rbx, rcx
                ; add rdx, rcx
                ; add rdi, rcx
                ; add rsi, rcx
                ; add rdi, rbx
            );
        }}, 

        // 6 single (5 dispatched mops; 1 dispatched mops)
        // NOTE: I think we can only dispatch 4 ALU ops per cycle
        // (one for each ALSQ) 
        EmitterDesc { desc: "add; add; add; add; load; load",
            func: |f, input| {
            dynasm!(f
                ; add rax, [0x1000]
                ; add rbx, [0x2000]
                ; add rdx, [0x4000]
                ; add rdi, rcx
                ; mov [0x1000], rax
            );
        }}, 

       // 1 microcoded instruction (3, 2, 1 dispatched mop) (unknown order)
       EmitterDesc { desc: "bsr", 
            func: |f, input| {
            dynasm!(f
                ; bsr rax, rbx
            );
        }}, 

        // 6 single (5 dispatched mops; 1 dispatched mop)
        EmitterDesc { desc: "nop (6)",
            func: |f, input| {
            dynasm!(f
                ; nop
                ; nop
                ; nop
                ; nop
                ; nop
                ; nop
            );
       }}, 

    ]);

    fn emit(case_emitter: fn(&mut X64Assembler, usize)) -> X64Assembler {
        let mut f = X64Assembler::new().unwrap();

        dynasm!(f
            ; mov r9, 0
            ; sub r9, 0x5a5a5a59
            ; cmp r9, 0
            ; vmovq xmm0, r9
            ; mov r10, 0x2
            ; mov [0x1000], r9
            ; mfence
            ; lfence
        );

        dynasm!(f
            ; lfence
            ; mov rcx, 0
            ; lfence
            ; rdpmc
            ; lfence
            ; mov [0x2000], rax
            ; mov rax, 0xdead_beef
            ; xor rdx, rdx
            ; lfence

        );

        case_emitter(&mut f, 0);
        dynasm!(f
            ; mfence
            ; lfence
        );
        dynasm!(f
            ; lfence
            ; mov rcx, 0
            ; lfence
            ; rdpmc
            ; lfence
            ; mov rbx, [0x2000]
            ; sub rax, rbx
        );

        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();

        events.add(Zen2Event::LsNotHaltedCyc(0x00));
        events.add(Zen2Event::DeDisUopQueueEmpty(0x00));
        events.add(Zen2Event::DsTokStall3(DsTokStall3Mask::NonZero));
        events.add(Zen2Event::DsTokStall3(DsTokStall3Mask::Zero));
        events.add(Zen2Event::DsTokStall3(DsTokStall3Mask::Cop1Disp));
        events.add(Zen2Event::DsTokStall3(DsTokStall3Mask::Cop2Disp));
        events.add(Zen2Event::DsTokStall3(DsTokStall3Mask::Cop3Disp));
        events.add(Zen2Event::DsTokStall3(DsTokStall3Mask::Cop4Disp));
        events.add(Zen2Event::DsTokStall3(DsTokStall3Mask::Cop5Disp));
        events.add(Zen2Event::DsTokStall3(DsTokStall3Mask::Cop6Disp));
        events.add(Zen2Event::ExRetCops(0x00));
        events.add(Zen2Event::ExRetInstr(0x00));

        //events.add(Zen2Event::Dsp0Stall(0x01));
        //events.add(Zen2Event::DeDisDispatchTokenStalls0(DeDisDispatchTokenStalls0Mask::ALUTokenStall));
        //events.add(Zen2Event::DeDisDispatchTokenStalls0(DeDisDispatchTokenStalls0Mask::ALSQ1RsrcStall));
        //events.add(Zen2Event::DeDisDispatchTokenStalls0(DeDisDispatchTokenStalls0Mask::ALSQ2RsrcStall));
        //events.add(Zen2Event::DeDisDispatchTokenStalls0(DeDisDispatchTokenStalls0Mask::ALSQ3_0_TokenStall));
        //events.add(Zen2Event::DeDisDispatchTokenStalls0(DeDisDispatchTokenStalls0Mask::AGSQTokenStall));

        //events.add(Zen2Event::DeMsStall(DeMsStallMask::Serialize));

        //events.add_unknown(0xa7);
        //events.add_unknown(0xac);
        //events.add_unknown(0xad);
        //events.add(Zen2Event::Unk(0xd5, 0x00));
        //events.add(Zen2Event::Unk(0x1d6, 0x00));
        //events.add(Zen2Event::Unk(0xa8, 0x01));
        //events.add(Zen2Event::Unk(0xa8, 0x02));
        //events.add(Zen2Event::Unk(0xa8, 0x80));

        events.add(Zen2Event::DeDisOpsFromDecoder(
                DeDisOpsFromDecoderMask::Unk(0xff)
        ));

        // Measure the floor
        let mut floor_res: ExperimentCaseResults<Zen2Event, usize> = 
            ExperimentCaseResults::new("floor");
        let floor_asm = Self::emit(|f, input| {});
        let floor_asm_reader = floor_asm.reader();
        let floor_asm_tgt_buf = floor_asm_reader.lock();
        let floor_asm_tgt_ptr = floor_asm_tgt_buf.ptr(AssemblyOffset(0));
        let floor_fn: MeasuredFn = unsafe { 
            std::mem::transmute(floor_asm_tgt_ptr)
        };

        for testcase in Self::CASES.iter() {
            println!("[*] Testcase '{}'", testcase.desc);

            let asm = Self::emit(testcase.func);

            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };
            for event in events.iter() {
                let desc = event.as_desc();
                let floor_results = harness.measure(floor_fn, 
                    desc.id(), desc.mask(), 1024, InputMethod::Fixed(0, 0)
                ).unwrap();
                let results = harness.measure(asm_fn, 
                    desc.id(), desc.mask(), 1024, InputMethod::Fixed(0, 0)
                ).unwrap();

                let fmin = floor_results.get_min();
                let fmax = floor_results.get_max();


                let rmin = results.get_min();
                let rmax = results.get_max();
                let norm_min = (rmin as i32 - fmin as i32);

                if norm_min == 0 { continue; }

                println!("norm_min={:4} (fmin={:4} fmax={:4}) (rmin={:4} rmax={:4}) {:03x}:{:02x} {}",
                    norm_min,
                    fmin,fmax,rmin,rmax,
                    desc.id(), desc.mask(), desc.name()
                );
            }
            println!();
        }
    }
}


