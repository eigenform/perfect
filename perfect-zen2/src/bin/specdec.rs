
use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use perfect::asm::Emitter;
use perfect::experiments::decoder::*;
use perfect::util::disas_bytes;

use rand::prelude::*;
use rand::Rng;
use rand::distributions::{ Distribution, Standard };

use iced_x86::{ 
    Decoder, DecoderOptions, Instruction, Formatter, IntelFormatter 
};

fn main() {
    let mut harness = HarnessConfig::default_zen2().emit();
    SpeculativeDecodeFuzz::run(&mut harness);
}

/// Try to speculatively evaluate random x86_64 instruction encodings.
///
/// Test
/// ====
///
/// - Perform a random instruction in the shadow of a costly misprediction.
///
/// - If we observe that a marker instruction (ie. PREFETCH) after the tested 
///   instruction is speculatively dispatched, this means that the encoding
///   is probably valid
///
/// - If we observe that a marker instruction is *not* dispatched, this 
///   probably means that the speculative path was cancelled due to some 
///   kind of exception
///
///
pub struct SpeculativeDecodeFuzz;
const SIZE: usize = 16;
impl MispredictedReturnTemplate<[u8; SIZE]> for SpeculativeDecodeFuzz {}
impl SpeculativeDecodeFuzz {

    fn emit_random_instr(f: &mut X64Assembler, input: [u8; SIZE]) {
        dynasm!(f ; .bytes input );
    }

    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::LsPrefInstrDisp(0x1));

        let opts = MispredictedReturnOptions::zen2_defaults()
            .speculative_epilogue_fn(Some(|f, input| { 
                dynasm!(f 
                    ; nop
                    ; nop
                    ; nop
                    ; nop
                    ; prefetch [rax]
                );
                for _ in 0..128 { 
                    dynasm!(f; int3);
                }
            }))
            .post_prologue_fn(Some(|f, input| {
                dynasm!(f ; mov rcx, 2);
            }))
            .prologue_fn(Some(|f, input| { 
                dynasm!(f
                );
            }))
            .rdpmc_strat(RdpmcStrategy::Gpr(Gpr::R15));

        let mut cases = Vec::new();
        let mut rng = thread_rng();

        // Generate some random-ish x86 instruction encodings
        for _ in 0..1024 { 
            let enc: RandomEncoding<16> = rng.gen();
            cases.push(enc.as_bytes());
        }


        for case in cases.iter() { 
            let asm = Self::emit(opts, *case, Self::emit_random_instr);
            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };

            for event in events.iter() {
                let desc = event.as_desc();
                let results = harness.measure(asm_fn, 
                    desc.id(), desc.mask(), 8, InputMethod::Fixed(0, 0)
                ).unwrap();

                // Ignore cases which are completely garbage
                if results.get_min() == 0 {
                    continue;
                }

                // Create a buffer with NOP padding at the end (reflecting
                // the actual bytes we speculatively decoded).
                let mut buf = [0x90u8; 20];
                let mut buf = Vec::new();
                buf.extend_from_slice(case);
                buf.extend_from_slice(&[0x90; 4]);
                //buf[..16].copy_from_slice(case);

                let dis = disas_bytes(&buf);
                let maybe_invalid = dis.iter().filter(|x| x.2).count() != 0;

                if !maybe_invalid { 
                    continue;
                }

                println!("input={:02x?}", buf);
                for (istr, bstr, invalid) in dis.iter() {
                    println!("  {:32} {} ", bstr, istr);
                }
                println!();
            }
        } 
    }
}


