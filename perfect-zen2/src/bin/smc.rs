
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
    SmcSimple::run(&mut harness);
}

/// Observe the speculative window created by the latency associated with 
/// re-synchronizing the pipeline after a store which modifies the current 
/// instruction stream.
///
/// Context
/// =======
///
/// Within a single hardware thread, self-modifying code is supported by 
/// monitoring for writes that hit in the L1I cache. 
///
/// The AMD64 Architecture Programmer's Manual (Volume 2, Section 7.6.1)
/// mentions:
///
/// > [...] AMD64 processors will flush any lines from the instruction cache 
/// > that such stores hit, and will additionally check whether an instruction
/// > being modified is already in decode or execution behind the store 
/// > instruction. If so, it will flush the pipeline and restart instruction 
/// > fetch to acquire and re-decode the updated instruction bytes.
///
/// Test
/// ====
///
/// 1. Emit a 64-bit aligned store whose target is an FNOP instruction that 
///    occurs sometime after the store; during runtime, replace the FNOP 
///    instruction with a different instruction (ie. NOP)
///
/// 2. Emit a variable number of single-byte padding NOPs in-between the store
///    and the patched instruction bytes
///
/// 3. With no padding NOPs, we expect that FNOP will be [speculatively]
///    fetched/dispatched/executed, and then eventually flushed from the 
///    pipeline after the store has completed and the state of the L1I cache
///    is coherent. We expect that FNOP will *never* retire. 
///
/// 4. After a certain number of padding NOPs, we expect that FNOP will never
///    be speculatively dispatched, and that the instruction stream has become
///    re-coherent before FNOP is fetched/decoded
///
/// Results
/// =======
///
/// FNOP is never observed to retire. 
///
/// After ~220 padding NOPs, FNOP is never observed to be dispatched. 
/// This is [presumably] because we've filled up the retire queue entirely 
/// with NOPs, and that the latency [either associated with the completion of 
/// the store, or the resync, or both] must be longer than the time it takes 
/// to speculatively fetch/decode/dispatch/complete them. 
///
/// NOTE: Measuring with 'LsNotHaltedCyc', there seems to be something like 
/// ~142 cycles of latency. This code measures ~238 cycles, and when emitting 
/// this function *without* the store, we measure only ~97 cycles.
///
pub struct SmcSimple;
impl SmcSimple {
    fn emit(padding: usize) -> X64AssemblerFixed
    {
        let mut rng = rand::thread_rng();
        let mut f = X64AssemblerFixed::new(
            0x0000_1000_0000_0000,
            0x0000_0000_0001_0000,
        );

        let target = f.new_dynamic_label();
        let exit = f.new_dynamic_label();
        let fnop = f.new_dynamic_label();

        dynasm!(f
            ; lea r8, [=>target]
            ; mfence
        );

        f.emit_rdpmc_start(0, Gpr::R15 as u8);

        // Drain the pipeline.
        // The store begins on the next-sequential cacheline. 
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
        );

        // 64-bit store to 'target', writing over bytes that will be fetched
        // (or have already been fetched) into the instruction stream
        dynasm!(f
            ; mov QWORD [r8], rdi
        );

        // Emit a variable number of single-byte padding NOPs in-between 
        // the store and the patched instruction. 
        //f.emit_nop_sled(padding);
        for _ in 0..padding { 
            dynasm!(f
                //; nop
                //; pause
            );
        }

        // Unconditionally jump to the patched instruction
        dynasm!(f
            ; jmp =>target
        );

        // Target instruction (patched during runtime)
        f.pad_until(0x0000_1000_0000_0400);
        f.place_dynamic_label(target);
        dynasm!(f 
            ; fnop
            //; vpxor xmm0, xmm0, xmm0
        );
        f.pad_until(0x0000_1000_0000_0440);

        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();

        f
    }

    pub fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::DeDisOpsFromDecoder(DeDisOpsFromDecoderMask::Fp));

        //events.add(Zen2Event::FpSseAvxOpsRetired(0xff));

        //events.add(Zen2Event::LsNotHaltedCyc(0x00));

        // This probably counts pipeline-resynchronizing events
        //events.add(Zen2Event::BpRedirect(BpRedirectMask::Unk(0x00)));

        for padding in 0..=256 {
            for event in events.iter() {
                let mut results = RawResults(Vec::new());

                // Re-emit measured code each iteration
                let mut asm = Self::emit(padding);

                for iter in 0..512 {
                    asm.commit().unwrap();
                    let result = harness.measure_event(asm.as_fn(), 
                        *event, 1,
                        InputMethod::Fixed(0x90909090_90909090, 0)
                    ).unwrap();
                    results.0.extend_from_slice(&result.data.0);
                }

                let dist = results.histogram();
                let min = results.get_min();
                let max = results.get_max();
                let desc = event.as_desc();
                let mode = results.get_mode();

                println!("  pad={:3} {:03x}:{:02x} {:32} min={} max={} {:?}",
                    padding,
                    desc.id(), desc.mask(), desc.name(), 
                    min, max,
                    dist,
                );
            }
            println!();
        }
    }
}




