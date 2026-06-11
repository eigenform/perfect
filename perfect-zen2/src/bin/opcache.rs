use perfect::*;
use perfect::events::*;
use std::collections::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .arena_alloc(0x0000_0000_0000_0000, 0x0000_0000_4000_0000)
        .pinned_core(Some(5))
        .emit();

    OpCacheCapacity::run(&mut harness);
}

/// [Try to] measure the capacity of the op cache. 
///
/// Context
/// =======
///
/// In many x86 implementations, an op cache (or "micro-op cache") is the 
/// lowest level in the instruction cache hierarchy. Instead of fetching 
/// instruction bytes from the L1I cache and decoding them, recent previously
/// decoded instructions can be provided directly from the op cache. 
///
/// Reviewing the AMD Software Optimization Guides for various Zen parts, 
/// there are some important general points: 
///
/// 1. In IC mode, decoded instructions are built into the op cache. 
/// 2. Transitions to OC mode occur only at branch targets. 
/// 3. OC mode persists until an op-cache miss occurs. 
///
/// Additionally, there are some details specific to Zen2/Zen3 parts: 
///
/// 1. An op cache entry has the following constraints: 
/// 
///     - All instructions belong to the same 64B L1I cacheline
///     - Up to eight instructions can belong to an op cache entry
///     - (Plus some other constraints that don't matter to us here)
///
/// 2. The op cache consists of 64 sets and 8 ways. 
///    The documented capacity is 4096 instructions. 
///
/// As far as I'm aware, the performance impact from an op cache comes from 
/// the ability to speed up loops in the instruction stream. 
/// The problem here can be rephrased as the following: 
///
/// > "What is the largest loop that can be served from the op cache?"
///
/// In order to answer this, we ideally want to write a single loop that 
/// will somehow measure the capacity while also potentially *occupying* the 
/// entire op cache.
///
/// Test
/// ====
///
/// The loop here consists of the following parts: 
///
/// 1. Take sample A from the performance counter. 
/// 2. Perform N padding instructions.
/// 3. Take sample B from the performance counter.
/// 4. Obtain a measurement by taking the difference between B and A. 
/// 5. Write the measurement to memory and increment the pointer.  
/// 6. Decrement the loop counter. 
///    If the counter is nonzero, branch to step (1).
///    Otherwise, return to caller. 
///
/// This loop must be repeated many times in order to guarantee that 
/// all of these instructions reside in the L1I cache, and that all of 
/// these instructions have been built into op cache entries. 
///
/// In general, we expect that the first few iterations may not be served
/// from the op cache, and that later iterations are guaranteed to be 
/// served from the op cache. 
///
/// For large-enough values of N (the number of padding instructions), we 
/// expect this loop will not be able to fit into the op cache, and that 
/// this fact is recorded by our measurements. 
///
/// Results (Zen 2)
/// ===============
///
/// Differences occur when N=4072. 
/// This is very close to the expected capacity (4096). 
///
/// Looking at different PMC events, we see that at this threshold: 
///
/// - The running time increases by ~200 cycles
///
/// - The number of fetched L1I cachelines becomes non-zero
///
/// - The number of macro-ops dispatched from the op cache becomes 
///   inconsistent, and then eventually drops to zero as N becomes larger
///
/// This is consistent with what we'd expect when the loop cannot fit 
/// into the op cache. 
///
///
/// Additional Notes
/// ================
///
/// Note that on Zen 3, although PMC0xAA:01 and PMC0xAA:02 ("DeSrcOpDisp") are 
/// supposed to count the number of ops dispatched from either the decoders or 
/// the op cache, these counters seemingly do not work (or at least, not on 
/// Model 51h parts).
///
/// This is partially documented by errata #1287 ("PMC0xAA [Source of Op 
/// Dispatched From Decoder] Events Will Not Be Counted") in the revision 
/// guides for Family 19h parts. Interestingly, the description of PMC0xAA 
/// ("DeSrcOpDisp") for some Family 19h models in AMD uProf includes slightly 
/// more context about this: 
///
/// > [...] this PMC event counts events from any thread. A microcode patch 
/// > has been written to disable this PMC event for non-secure parts. 
///
/// Presumably, the actual issue here is that the event does not count for 
/// individual hardware threads. Despite this, we can still attempt to 
/// measure the capacity for a single hardware thread when SMT is disabled.
/// The original behavior can seemingly be restored by loading a patch which 
/// does not disable this event. 
///

pub struct OpCacheCapacity;
impl OpCacheCapacity {
    const BASE_ADDR: usize  = 0x0000_1000_0000_0000;
    const CASE_ADDR: usize = 0x0000_2000_8000_0000;
    const ITERS: usize = 32;

    /// Emit the tested loop. 
    ///
    /// When calling emitted code, be aware that:
    ///
    /// - RSI the number of loop iterations
    /// - RDI is a pointer to an array for storing measurements
    /// - RCX will be clobbered
    /// - R8 will be clobbered
    ///
    fn emit_measure(num_nops: usize) -> X64AssemblerFixed {
        let mut f = X64AssemblerFixed::new(Self::BASE_ADDR, 0x0002_0000);
        let rdpmc_scratch = Gpr::R8 as u8;

        let head = f.new_dynamic_label();
        f.place_dynamic_label(head);

        // Take the first sample (saved in r8). 
        //
        // NOTE: This chunk of instructions occupies 32 bytes
        dynasm!(f
            ; .bytes NOP8
            ; lfence
            ; mov rcx, 1
            ; lfence
            ; rdpmc
            ; lfence
            ; mov r8, rax
            ; lfence
        );

        /// Emit some variable number of single-byte padding NOPs
        for _ in 0..num_nops { 
            dynasm!(f; nop);
        }

        // Take the second sample, then take the difference. 
        //
        // NOTE: This chunk of instructions occupies 32 bytes
        dynasm!(f
            ; lfence
            ; mov rcx, 1
            ; lfence
            ; rdpmc
            ; lfence
            ; sub rax, r8
            ; lfence
            ; .bytes NOP8
        );

        // Write the difference to memory and increment the pointer. 
        // Decrement the loop counter, and repeat if non-zero. 
        //
        // NOTE: This chunk of instructions occupies 16 bytes 
        dynasm!(f
            ; mov QWORD [rdi], rax
            ; add rdi, 8
            ; dec rsi
            ; jnz =>head
            ; ret
        );

        f.commit().unwrap();
        println!("[*] emit_measure():");
        f.disas(AssemblyOffset(0), None);
        f
    }

    fn flush_cache() { 
        unsafe { 
            for addr in (Self::BASE_ADDR..Self::BASE_ADDR+0x10000).step_by(64) { 
                core::arch::x86_64::_mm_clflush(addr as _);
            }
        } 
    }

    #[inline(always)]
    fn run_measurement(
        harness: &mut PerfectHarness, 
        event: &Zen2Event,
        results: &mut perfect::stats::ExperimentCaseResults<Zen2Event, usize>,
    ) 
    { 
        let mut ctr = PerfectHarness::make_perf_cfg(
            harness.cfg.platform, 
            &event.as_desc()
        );
        ctr.reset().unwrap();
        ctr.enable().unwrap();

        let mut data = Box::new([0; Self::ITERS]);
            
        for num_nops in (0..8192).step_by(8) { 

            let measure_asm = Self::emit_measure(num_nops);
            let measure_asm_fn = measure_asm.as_fn();
            //Self::flush_cache();

            let res = measure_asm_fn(data.as_ptr() as _, Self::ITERS);

            results.record(
                *event, 
                num_nops, 
                perfect::stats::RawResults(data.to_vec())
            );
        }


    }

    fn run(harness: &mut PerfectHarness) { 
        let mut all_results = perfect::stats::ExperimentCaseResults::new("opcache_capacity");

        Self::run_measurement(harness, &Zen2Event::LsNotHaltedCyc(0x00), &mut all_results);
        Self::run_measurement(harness, &Zen2Event::DeSrcOpDisp(DeSrcOpDispMask::OpCache), &mut all_results);
        Self::run_measurement(harness, &Zen2Event::IcFw32(0x00), &mut all_results);

        //Self::run_measurement(harness, &Zen2Event::Unk(0x28f, 0xff), &mut all_results);
        //Self::run_measurement(harness, &Zen2Event::IfDqBytesFetched(0x00), &mut all_results);
        //Self::run_measurement(harness, &Zen2Event::IcFetchStallCyc(IcFetchStallCycMask::Any), &mut all_results);

        all_results.write_csv("/tmp/opcache_capacity.csv");

    }

}


