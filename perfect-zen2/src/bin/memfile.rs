/// Memory renaming experiments. 

use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use rand::distributions::Uniform;
use std::collections::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2().emit();
    MemfileDisplacement::run(&mut harness);
    MemfileWindow::run(&mut harness);
}


/// Determine which [immediate] displacement bits in a memory operand are 
/// used for determining memory renaming eligibility. 
///
/// Context
/// =======
///
/// Memory renaming uses parts of the memory operand (the base register, 
/// an immediate displacement, a scaling factor, etc) to identify/disambiguate 
/// loads and stores to the same address. 
///
/// Test
/// ====
///
/// Execute a store-load pair with a single bit set in the displacement. 
/// If the pair is not eligible for renaming, no events will occur. 
///
/// Results
/// =======
///
/// Eligibility depends on displacement bits [9:3].
///
/// - Renaming does not occur when any of the bits [2:0] are set
///   (the displacement must be a multiple of 8)
/// - Renaming does not occur when any of the bits [16:10] are set
///   (the displacement must be less than or equal to 0x3f8)
///
pub struct MemfileDisplacement;
impl Experiment<usize> for MemfileDisplacement {
    fn emit(bit: usize) -> X64Assembler {
        assert!(bit > 0 && bit < 31);
        let mut f = X64Assembler::new().unwrap();
        let addr = 0x0000_0000 | (1 << bit);

        f.emit_rdpmc_start(0, Gpr::R15 as u8);
        dynasm!(f
            ; mov [addr], eax
            ; mov ebx, [addr]
        );
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::MemFileHit(0x00));
        events.add(Zen2Event::MemRenLdDsp(0x00));
        events.add(Zen2Event::MemRenLdElim(0x00));
        events.add(Zen2Event::LsSTLF(0x00));

        for bit in 1..=16 {
            let asm = Self::emit(bit);
            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };

            let results = harness.measure_events(
                asm_fn, &events, 256, InputMethod::Fixed(0, 0)
            ).unwrap();

            println!("[*] bit={}", bit);
            for result in results { 
                let min = result.get_min();
                let max = result.get_max();
                println!("    {:03x}:{:02x} {:032} min={} max={}",
                    result.event.id(), 
                    result.event.mask(), 
                    result.event.name(), 
                    min, max
                );
            }
        }
        println!();
    }
}

/// Determine if the age of entries in the store queue has any effect on 
/// eligibility for memory renaming. 
///
/// Explanation
/// ===========
///
/// Memory renaming relies on the fact that recent stores are kept in the 
/// store queue. The Family 17h SOG mentions that the store queue capacity 
/// is 48 entries. 
///
/// Test
/// ====
///
/// 1. Emit a store that we expect will be forwarded. 
/// 2. Emit some variable number of padding stores. 
/// 3. Emit a load which matches the initial store. 
///
/// Emit some variable number of stores (up to the store queue capacity), 
/// and then a load that we expect to hit in the memory file. 
///
/// Results
/// =======
///
/// Only the most-recent 6 stores are eligible for memory renaming. 
/// 'MemFileHit', 'MemRenLdDsp', and 'MemRenLdElim' do not occur for loads
/// matching older stores. 
///
pub struct MemfileWindow;
impl Experiment<usize> for MemfileWindow {
    fn emit(idx: usize) -> X64Assembler {
        let mut f = X64Assembler::new().unwrap();

        let mut rng = thread_rng();
        let mut addrs: Vec<i32> = (0x0000_0008..=0x0000_03f8)
            .step_by(8).collect();
        addrs.shuffle(&mut rng);
        assert!(idx <= 47);

        f.emit_rdpmc_start(0, Gpr::R15 as u8);

        // Fill the store queue
        for addr in &addrs[0..=47] {
            dynasm!(f; mov [*addr], rax);
        }

        // Generate a load expected to hit in the memory file
        let addr = addrs[idx];
        dynasm!(f
            ; mov rbx, [addr]
        );
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::MemFileHit(0x00));
        events.add(Zen2Event::MemRenLdDsp(0x00));
        events.add(Zen2Event::MemRenLdElim(0x00));
        events.add(Zen2Event::LsSTLF(0x00));

        for idx in 0..=47 {
            let asm = Self::emit(idx);
            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };

            let results = harness.measure_events(
                asm_fn, &events, 256, InputMethod::Fixed(0, 0)
            ).unwrap();

            println!("[*] idx={}", idx);
            for result in results { 
                let min = result.get_min();
                let max = result.get_max();
                println!("    {:03x}:{:02x} {:032} min={} max={}",
                    result.event.id(), 
                    result.event.mask(), 
                    result.event.name(), 
                    min, max
                );
            }
        }
        println!();
    }
}


