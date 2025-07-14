use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use rand::distributions::Uniform;
use std::collections::*;

//#[repr(transparent)]
//pub struct Cacheline(pub [u8; 64]);
//impl Cacheline {
//    pub fn ptr(&self) -> *const u8 {
//        self.0.as_ptr()
//    }
//}

//#[repr(C)]
//pub struct ReloadArray {
//    line: [Cacheline; 256]
//}
//impl ReloadArray {
//    /// Flush an entry from the cache.
//    pub fn flush(&self, idx: u8) {
//        unsafe {
//            core::arch::x86_64::_mm_clflush(self.line[idx as usize].ptr());
//            core::arch::x86_64::_mm_mfence();
//            core::arch::x86_64::_mm_lfence();
//        }
//    }
//
//    /// Return a pointer to the given entry.
//    pub fn ptr(&self, idx: u8) -> *const u8 {
//        self.line[idx as usize].ptr()
//    }
//}

pub struct VirtualAddress(pub usize);
impl VirtualAddress {
    /// Bits [5:0] map to an offset within the cache line
    const OFFSET_MASK: usize = 0x0000_0000_0000_003f;
    /// Bits [11:6] map to a set in the L1 data cache
    const SET_MASK: usize    = 0x0000_0000_0000_0fc0;
    /// According to the paper, bits [27:12] consitute input to the utag
    const UTAG_MASK: usize   = 0x0000_0000_0fff_f000;
    /// High bits
    const HI_MASK : usize    = 0x0000_ffff_f000_0000;

    pub fn set(&self) -> usize {
        (self.0 & Self::SET_MASK) >> 6
    }

    pub fn offset(&self) -> usize {
        self.0 & Self::OFFSET_MASK
    }

    pub fn value(&self) -> usize { self.0 }

    pub fn new(offset: usize, set: usize, utag_input: usize, hi_bits: usize) -> Self {
        let offset = offset & 0x3f;
        let set = set & 0b0011_1111;
        let utag_input = utag_input & 0xffff;
        Self(offset | set << 6 | utag_input << 12 | hi_bits << 28)
    }

    pub fn utag_input(&self) -> usize {
        (self.0 & Self::UTAG_MASK) >> 12
    }


}



fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .arena_alloc(0x0000_0000_0000_0000, 0x0000_0000_4000_0000)
        .pinned_core(Some(15))
        .emit();

    L1DWayPredictorMiss::run(&mut harness);
}

/// Create conflicting loads that cause L1D way mispredictions.
///
/// Context
/// =======
///
/// In the Zen 2 microarchitecture, the L1 data cache is 32KiB
/// set-associative (8-way, 64 sets, 64B lines), virtually-indexed
/// and physically-tagged (VIPT).
///
/// In VIPT caches [in general], the physical address must be known in order
/// to determine which way matches the tag. After the physical address is
/// resolved, all ways are read in parallel to find the matching tag.
/// This costs power (driving all ways for each load), and also incurs some
/// latency since data cannot be obtained before address translation.
///
/// In the Zen 2 microarchitecture, a "way predictor" attempts to avoid this
/// by predicting which way contains the associated data: bits in the virtual
/// address are hashed to form a "micro-tag" (utag) which is used to lookup
/// the previous way index in a table. The utag for a cacheline is [presumably]
/// updated on every successful load.
///
/// In the best case, this means that:
///
/// - A single way can be read instead of all eight in parallel
///   (which amounts to some dynamic power saving)
///
/// - L1D arrays are organized into seperate banks in order to support multiple
///   accesses in parallel; reading only a single way reduces the chance
///   of conflicts with other loads that may be pending to the same bank
///
/// - Data can be provided *before* the physical address is resolved,
///   allowing loads to [speculatively] complete early
///
/// - A miss can be recognized *before* the physical address is resolved,
///   and the associated fill request can be sent early
///
/// The hash function for Zen 2 is known from previous research[^1] by Lipp,
/// et al. (and it would be nice to reproduce their results here).
///
/// [^1]: [Take A Way: Exploring the Security Implications of AMD's Cache Way Predictors](https://dl.acm.org/doi/10.1145/3320269.3384746)
///
/// Test
/// ====
///
/// 1. Perform a load from address A; way A is tagged with utag A
/// 2. Perform a load from address B, attempting to find way B with utag B
/// 3. If utag A and utag B are colliding (`utag B == utag A`), way A is always
///    predicted [incorrectly], causing a MAB allocation
///
/// ...
///
/// Results
/// =======
///
/// A Miss Address Buffer (MAB) allocation occurs consistently for the second
/// load when any of these conditions is true:
///
/// - `(vaddr_a[15] ^ vaddr_b[20]) == 1`
/// - `(vaddr_a[16] ^ vaddr_b[21]) == 1`
/// - `(vaddr_a[17] ^ vaddr_b[22]) == 1`
/// - `(vaddr_a[18] ^ vaddr_b[23]) == 1`
///
///
pub struct L1DWayPredictorMiss;
impl L1DWayPredictorMiss {

    /// Emit the experiment. 
    /// Input arguments to this function during runtime are: 
    ///
    /// - `RDI`, virtual address A
    /// - `RSI`, virtual address B
    ///
    fn emit_measure() -> X64AssemblerFixed {
        let base_addr = 0x0000_1000_0000_0000;
        let mut f = X64AssemblerFixed::new(base_addr, 0x0001_0000);

        // Perform the first load (from the address in RDI)
        dynasm!(f
            ; mov rax, QWORD [rdi]
            ; mfence
            ; lfence
        );

        // Perform the second load (from the address in RSI).
        // If the utag for this address collides with the utag for the
        // previous address in RDI, we expect the way is mispredicted.
        f.emit_rdpmc_start(0, Gpr::R15 as _);
        dynasm!(f
            ; mov rax, QWORD [rsi]
        );
        f.emit_rdpmc_end(0, Gpr::R15 as _, Gpr::Rax as _);

        f.emit_ret();
        f.commit().unwrap();
        f
    }


    fn run(harness: &mut PerfectHarness) {
        let mut rng = thread_rng();

        let mut events = EventSet::new();

        // Miss Address Buffer (MAB) allocations resulting from loads.
        // (This should indicate an L1D miss?)
        events.add(Zen2Event::LsMabAlloc(LsMabAllocMask::Loads));


        let measure_asm = Self::emit_measure();
        let measure_asm_fn = measure_asm.as_fn();

        // Scan over single-bit differences between addresses A and B. 
        let mut pairs = BTreeSet::new();
        for idx in 0..12 {
            pairs.insert((1<<idx, 0));
            pairs.insert((0, 1<<idx));
            pairs.insert((!(1<<idx), !0));
            pairs.insert((!0, !(1<<idx)));
        }
        for idx1 in 0..12 {
            for idx2 in 0..12 {
                pairs.insert((1<<idx1, 1<<idx2));
                pairs.insert((!(1<<idx1), !(1<<idx2)));
            }
        }

        for event in events.iter() {
            let desc = event.as_desc();
            for (utag1, utag2) in pairs.iter() {
                let a1 = VirtualAddress::new(0b000000, 0b000000, *utag1, 0);
                let a2 = VirtualAddress::new(0b000000, 0b000000, *utag2, 0);
                let results = harness.measure(measure_asm_fn,
                    desc.id(), desc.mask(), 1024,
                    InputMethod::Fixed(
                        a1.value(),
                        a2.value(),
                    ),
                ).unwrap();

                let min = results.get_min();
                let max = results.get_max();

                // If a miss always occurs for the second load, presumably we 
                // have created a situation where the utag is *always* incorrect
                if min != 0 {
                    println!("    {:03x}:{:02x} {:032} [a1={:016x} (set={:2},inp={:016b})] [a2={:016x} (set={:2},inp={:016b})] min={} max={}",
                        desc.id(), desc.mask(), desc.name(),
                        a1.value(), a1.set(), a1.utag_input(), a2.value(), a2.set(), a2.utag_input(),
                        min, max,
                    );
                }
            }
        }
    }
}


