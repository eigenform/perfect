use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use rand::prelude::*;

use std::fs::File;
use std::io::Write;

fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .zero_strategy(ZeroStrategy::MovFromZero)
        .emit();
    //RenameResources::run(&mut harness);
    MoveElimination::run(&mut harness);
}


pub struct MoveElimination;
impl Experiment<usize> for MoveElimination {
    fn emit(input: usize) -> X64Assembler {
        let mut f = X64Assembler::new().unwrap();

        let lab = f.new_dynamic_label();

        f.emit_flush_btb(0x4000);

        f.emit_rdpmc_to_addr(0, 0x0000_0280);

        // Save the stack pointer
        dynasm!(f ; mov [0x0000_0380], rsp);

        // Write the indirect branch target to memory somewhere
        dynasm!(f
            ; lea r14, [=>lab]
            ; movnti [0x0001_0004], r14
        );
        f.emit_sfence();

        // Rename all register map entries to a known-zero register
        for _ in 0..2 {
            for i in 0..16 {
                dynasm!(f; mov Rq(i), r9);
            }
            for i in 0..16 {
                dynasm!(f; mov Rd(i), r9d);
            }
        }
        for _ in 0..8 {
            for i in 0..16 {
                dynasm!(f; mov [0x0000_0200], Rq(i));
            }
        }
        f.emit_sfence();

        dynasm!(f 
            ; .align 64
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP5
            ; lfence
            ; jmp QWORD [0x0001_0004]
        );
        for i in 0..input {
            dynasm!(f
                ; mov ax, bx
            );
        }

        f.emit_fnop_sled(1);
        f.emit_nop_sled(4096);
        dynasm!(f
            ; .align 64
            ; =>lab
        );

        dynasm!(f
            ; lfence
            ; mov rcx, 0
            ; lfence
            ; rdpmc
            ; lfence
            ; mov rbx, [0x0000_0280]
            ; sub rax, rbx
        );

        // Restore the stack pointer
        dynasm!(f ; mov rsp, [0x0000_0380]);

        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::DeDisDispatchTokenStalls1(
                DeDisDispatchTokenStalls1Mask::IntPhyRegFileRsrcStall
        ));
        //events.add(Zen2Event::DeDisOpsFromDecoder(DeDisOpsFromDecoderMask::Fp));


        for i in 0..256 {
            let asm = Self::emit(i);
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

                let min = results.get_min();
                let max = results.get_max();
                println!("i={:04}:  {:03x}:{:02x} {:032} min={} max={}",
                    i, desc.id(), desc.mask(), desc.name(), min, max);
            }
        }
        println!();
    }
}

/// Determine what resources [if any] are allocated by zero idioms and 
/// register-to-register moves. 
///
/// Explanation
/// ===========
///
/// Zeroing idioms are instructions whose encoding *unambiguously* means that 
/// the destination register will be set to zero. These instructions are 
/// guaranteed to be dependency-breaking. The main examples on x86 are:
///
/// - `xor rax, rax`
/// - `sub rax, rax`
///
/// Zeroing idioms can [at least in principle] be eliminated in the front-end 
/// of the machine during renaming, similar to how register-to-register moves
/// are eliminated by translating them into operations on a register map.
///
/// Test
/// ====
///
/// Perform a long sequence of back-to-back zero idioms/moves, but prevent
/// them all from retiring. If they are *not* completely eliminated from 
/// the pipeline, we expect to measure stall cycles at dispatch for the 
/// availability of some resource. 
///
/// Results
/// =======
///
/// - Zero idioms on known-nonzero registers stall
/// - Zero idioms on known-zero registers stall
/// - Moves from nonzero registers stall
/// - Moves from known-zero registers *do not* stall 
///
pub struct RenameResources;
impl MispredictedReturnTemplate<usize> for RenameResources {}
impl RenameResources {

    /// The set of zeroed registers (in all cases). 
    /// We expect all registers except for RSP (4) to be zeroed
    const ZEROED_REGS: &'static [u8] = &[ 
        0,1,2,3,5,6,7,8,9,10,11,12,13,14,15 
    ];

    const CASES: StaticEmitterCases<usize> = StaticEmitterCases::new(&[
        EmitterDesc { 
            desc: "Zero idiom on vector register",
            func: |f, input| {
                for _ in 0..=input { 
                    dynasm!(f ; vpxor xmm0, xmm0, xmm0); 
                }
            },
        }, 

        EmitterDesc { 
            desc: "Zero idiom random zeroed register", 
            func: |f, input| {
                let mut rand = thread_rng();
                for _ in 0..=input { 
                    let r = Self::ZEROED_REGS.choose(&mut rand).unwrap();
                    dynasm!(f ; xor Rq(*r), Rq(*r));
                }
            },
        }, 

        EmitterDesc { 
            desc: "Zero idiom on zeroed register",
            func: |f, input| {
                for _ in 0..=input { dynasm!(f ; xor r8, r8); }
            },
        }, 

        EmitterDesc { 
            desc: "Zero idiom on nonzero register",
            func: |f, input| {
                for _ in 0..=input { dynasm!(f ; xor rsp, rsp); }
            },
        }, 

        EmitterDesc { 
            desc: "Move from zeroed register",
            func: |f, input| {
                for _ in 0..=input { dynasm!(f ; mov rax, r8); }
            },
        }, 

        EmitterDesc { 
            desc: "Move from random zeroed register",
            func: |f, input| {
                let mut rand = thread_rng();
                for _ in 0..=input { 
                    let r = Self::ZEROED_REGS.choose(&mut rand).unwrap();
                    dynasm!(f ; mov rax, Rq(*r)); 
                }
            },
        }, 

        EmitterDesc { 
            desc: "Move from nonzero register", 
            func: |f, input| {
                for _ in 0..=input { dynasm!(f ; mov rax, rsp); }
            },
        }, 

        EmitterDesc { 
            desc: "Move from immediate zero to random",
            func: |f, input| {
                let mut rand = thread_rng();
                for _ in 0..=input { 
                    let r = Self::ZEROED_REGS.choose(&mut rand).unwrap();
                    dynasm!(f ; mov Rq(*r), 0x0); 
                }
            },
        }, 

        EmitterDesc { 
            desc: "Move from nonzero register to random",
            func: |f, input| {
                let mut rand = thread_rng();
                for _ in 0..=input { 
                    let r = Self::ZEROED_REGS.choose(&mut rand).unwrap();
                    dynasm!(f ; mov Rq(*r), rsp); 
                }
            },
        }, 
    ]);

    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::DeDisDispatchTokenStalls1(
                DeDisDispatchTokenStalls1Mask::IntPhyRegFileRsrcStall
        ));

        let opts = MispredictedReturnOptions::zen2_defaults()
            .explicit_lfence(true)
            .free_pregs(true)
            .rdpmc_strat(RdpmcStrategy::MemStatic(0x0000_5670));

        let mut exp_results = ExperimentResults::new();
        for testcase in Self::CASES.iter() {
            println!("[*] Running case '{}'", testcase.desc);
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
                        desc.id(), desc.mask(), 256, InputMethod::Fixed(0, 0)
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

                println!("# Event {:03x}:{:02x} '{}'", 
                    edesc.id(), edesc.mask(), edesc.name());

                let minmax = event_results.local_minmax();
                let avgs = event_results.local_avg_usize();
                let iterator = event_results.inputs.iter()
                    .zip(avgs.iter()).zip(minmax.iter());
                for ((input, avg), (min, max)) in iterator {
                    println!("input={} min={} avg={} max={}", 
                        input, min, avg, max
                    );
                }
            }
            println!();
        }
    }
}

