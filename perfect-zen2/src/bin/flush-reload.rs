use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use rand::distributions::Uniform;
use std::collections::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2().emit();
    FlushReload::run(&mut harness);
}

/// Simple synthetic example of cache timing with FLUSH+RELOAD. 
///
/// Context
/// =======
///
/// This is an example of how speculative loads can leave measurable 
/// side-effects on the cache (and how you can observe them with RDTSC). 
///
/// Test
/// ====
///
/// 1. Assume a single shared virtual address space. 
///
/// 2. Emit a 'victim' function with two loads [to different addresses] which 
///    occur depending on the outcome of a branch instruction, and where the 
///    outcome of the branch instruction is heavily biased in one direction.
///
/// 3. Emit an 'attacker' function whose branch instruction has a BTB entry 
///    which would be aliasing with the entry for the victim's branch.
///
/// 4. Run the victim function, which creates a BTB entry for the branch. 
///
/// 5. Flush the cachelines that would be occupied by the victim loads.
///
/// 6. Run the attacker function, which creates an aliasing BTB entry for 
///    the victims branch.
///
/// 7. Run the victim function again. 
///
/// 8. Probe the cache by repeating both victim loads and measuring the access 
///    time. If the victim's branch was mispredicted, we expect the probe for 
///    the speculatively-loaded address to be fast. 
///
/// Results
/// =======
///
/// Probes for both loads are fast, indicating that the attacker caused 
/// the victim to perform a speculative load. 
///
pub struct FlushReload;
impl FlushReload {
    const PROBE_ADDR:       usize = 0x0000_0000_4001_0000;
    const VICTIM_ADDR:      usize = 0x0000_0000_4002_0000;
    const ATTACKER_ADDR:    usize = 0x0000_1001_4002_0000;
    const ARR_ADDR:         usize = 0x0000_0000_0a00_0000;

    /// Emit a gadget for timing a single load with RDTSC (which returns the 
    /// observed value in RAX). 
    fn emit_probe() -> X64AssemblerFixed {
        let mut f = X64AssemblerFixed::new(Self::PROBE_ADDR, 0x4000);
        dynasm!(f
            ; xor r8, r8
            ; lfence
            ; rdtsc
            ; sub r8, rax

            ; mov r9, [rdi]

            ; lfence
            ; rdtsc
            ; add rax, r8
        );
        dynasm!(f
            ; ret
        );
        f.commit().unwrap();
        f
    }

    /// Variation 1. Two paths with different loads to different addresses
    fn emit_victim_v1(f: &mut X64AssemblerFixed) {
        dynasm!(f
            ; ->branch:
            ; jz ->taken

            // Speculative case
            ; mov rbx, r11
            ; shl rbx, 12
            ; or r8, rbx
            ; mov rdx, [r8]
            ; .align 64
            ; ret

            // Architectural case
            ; .align 64
            ; ->taken:
            ; mov rbx, r10
            ; shl rbx, 12
            ; or r8, rbx
            ; mov rdx, [r8]
            ; ret
        );
    }

    /// Variation 2. Two paths leading to the same load, but different 
    /// addresses [depending on different registers]
    fn emit_victim_v2(f: &mut X64AssemblerFixed) {
        dynasm!(f
            // Architectural case
            ; mov rbx, r10

            ; ->branch:
            ; jz ->taken

            // Speculative case
            ; mov rbx, r11

            ; ->taken:
            ; shl rbx, 12
            ; or r8, rbx
            ; mov rdx, [r8]
        );
    }

    /// Variation 3. Two paths leading to the same load, but different 
    /// addresses [depending on another load eligible for renaming/forwarding]
    fn emit_victim_v3(f: &mut X64AssemblerFixed) {
        dynasm!(f
            // Architectural case (eligible store for forwarding)
            ; mov [0x3f0], r10

            ; nop 
            ; nop

            ; ->branch:
            ; jz ->taken

            // Speculative case (younger, aliasing, eligible store)
            ; mov [0x3f0], r11

            ; ->taken:
            ; mov rbx, [0x3f0]
            ; shl rbx, 12
            ; or r8, rbx
            ; mov rdx, [r8]
        );
    }

    /// Emit the 'victim' function and the address of the victim's branch.
    fn emit_victim() -> (X64AssemblerFixed, usize) {
        let mut f = X64AssemblerFixed::new(Self::VICTIM_ADDR, 0x4000);

        dynasm!(f
            ; mov r8, QWORD Self::ARR_ADDR as _
            // Architecturally-used value
            ; mov r10, 0xbb
            // Speculatively-used value
            ; mov r11, 0xcc

            // Ensure that the condition for the next branch is static
            // (ie. a subsequent JZ would be always-taken) 
            ; xor rcx, rcx
            ; cmp rcx, 0

            ; lfence
        );

        //Self::emit_victim_v1(&mut f);
        Self::emit_victim_v2(&mut f);
        //Self::emit_victim_v3(&mut f);

        dynasm!(f
            ; .align 64
            ; ret
        );

        // Resolve the virtual address of the victim's branch
        // (so we can build an aliasing one in the 'attacker' function).
        let jz_off = f.labels.resolve_static(&StaticLabel::global("branch"))
            .unwrap();
        let victim_jz_addr = Self::VICTIM_ADDR + jz_off.0;

        f.commit().unwrap();
        (f, victim_jz_addr)
    }

    /// Emit the 'attacker' function.
    fn emit_attacker(victim_jz_addr: usize) -> X64AssemblerFixed {
        let mut f = X64AssemblerFixed::new(Self::ATTACKER_ADDR, 0x4000);
        dynasm!(f
            ; mov r8, QWORD Self::ARR_ADDR as _
            ; mov r10, 0xaa
            ; mov r11, 0xaa
            ; xor rcx, rcx
            ; cmp rcx, 0
            ; lfence
        );

        // Pad until we reach address matching the victim's branch
        let brn_addr = victim_jz_addr | 0x0000_1001_0000_0000;
        f.pad_until(brn_addr);
        
        // Emit a branch which is aliasing with the victim's branch
        dynasm!(f
            ; jz ->taken
            ; ->taken:
            ; mov rbx, r10
            ; shl rbx, 12
            ; or r8, rbx
            ; mov rdx, [r8]
            ; .align 64
            ; ret
        );
        f.commit().unwrap();
        f
    }

    /// Probe all of the addresses that the victim might have accessed.
    ///
    /// NOTE: We cannot *sequentially* probe this array without polluting 
    /// the state of the cache, since we might expect the data prefetcher
    /// to access all of the lines we are trying to measure. 
    /// Instead, perform accesses on each element in a random order.
    fn run_probe(harness: &mut PerfectHarness, probe_fn: MeasuredFn) 
        -> [usize; 256]
    {
        let mut results = [0usize; 256];
        let mut indexes = (0..=255).collect_vec();
        indexes.shuffle(&mut harness.rng);
        for idx in indexes {
            let addr = (Self::ARR_ADDR | (idx << 12)) as usize;
            let res = harness.call(addr, 0, probe_fn);
            results[idx] = res;
        }
        results
    }

    fn run(harness: &mut PerfectHarness) {
        let probe  = Self::emit_probe();
        let (victim, victim_jz_addr) = Self::emit_victim();
        let attacker = Self::emit_attacker(victim_jz_addr);

        //victim.disas(AssemblyOffset(0), None);
        //attacker.disas(AssemblyOffset(0), None);

        // Victim is running. 
        // Since the victim's branch is always-taken, we expect it to be 
        // installed in the BTB. 
        for _ in 0..128 { 
            let res = harness.call(0, 0, victim.as_fn());
        }

        // Attacker runs and inserts a BTB entry which should be aliasing with
        // the entry for the victim branch
        let res = harness.call(0, 0, attacker.as_fn());

        // Flush lines from the cache
        unsafe { 
            for _ in 0..64 {
                for idx in 0..=255 {
                    let addr = (Self::ARR_ADDR | (idx << 12)) as *const u8;
                    core::arch::x86_64::_mm_clflush(addr);
                    core::arch::x86_64::_mm_mfence();
                    core::arch::x86_64::_mm_lfence();
                }
            }
        }

        // Victim runs again and is forced to mispredict "not-taken"
        // [because of the aliasing BTB entry from the attacker's branch]. 
        harness.call(1, 0, victim.as_fn());

        // Probe the cache state. 
        // We expect that the time is low for both the architectural and 
        // speculative accesses in the victim function.
        let results = Self::run_probe(harness, probe.as_fn());
        for idx in 0..=255 {
            if results[idx] < 300 {
                println!("Test 2, {:02x}: {}", idx, results[idx]);
            }
        }
    }
}



