use perfect::*;
use perfect::events::*;
use perfect::uarch::l1d::ZEN2_L1D_UTAG_FN;
use rand::prelude::*;
use rand::distributions::Uniform;
use std::collections::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .arena_alloc(0x0000_0000_0000_0000, 0x0000_0000_4000_0000)
        .pinned_core(Some(15))
        .emit();

    L1DWayPredictorMiss::run_simple(&mut harness);
    //L1DWayPredictorMiss::run_exhaustive(&mut harness);
    //L1DWayPredictorMiss::run_random(&mut harness);
}


#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct VirtualAddress(pub usize);
impl VirtualAddress {
    /// Bits [5:0] map to an offset within the cache line
    const OFFSET_MASK: usize = 0x0000_0000_0000_003f;
    /// Bits [11:6] map to a set in the L1 data cache
    const SET_MASK: usize    = 0x0000_0000_0000_0fc0;
    /// According to the "Take A Way" paper, bits [27:12] constitute input to the utag
    const UTAG_MASK: usize   = 0x0000_0000_0fff_f000;
    /// High bits
    const HI_MASK : usize    = 0x0000_ffff_f000_0000;

    /// Return the set index bits
    pub fn set(&self) -> usize {
        (self.0 & Self::SET_MASK) >> 6
    }

    /// Return the offset bits
    pub fn offset(&self) -> usize {
        self.0 & Self::OFFSET_MASK
    }

    /// Return the micro-tag input bits
    pub fn utag_input(&self) -> usize {
        (self.0 & Self::UTAG_MASK) >> 12
    }

    pub fn utag(&self) -> usize { 
        ZEN2_L1D_UTAG_FN.evaluate(self.0)
    }

    /// Return the 64-bit virtual address as a [`usize`]
    pub fn value(&self) -> usize { 
        self.0
    }

    /// Create a new virtual address from the given offset, set index, 
    /// micro-tag input bits, and high bits. 
    pub fn new(offset: usize, set: usize, utag_input: usize, hi_bits: usize) -> Self {
        let offset = offset & 0x3f;
        let set = set & 0b0011_1111;
        let utag_input = utag_input & 0xffff;
        Self(offset | set << 6 | utag_input << 12 | hi_bits << 28)
    }

}

/// Build a map of all utags and their associated input bits
fn compute_utag_map() -> BTreeMap<usize, BTreeSet<usize>> {
    let mut map: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();

    for input in (0x0000..=0x0ffffusize) {
        let utag = ZEN2_L1D_UTAG_FN.evaluate(input << 12);
        if let Some(inputs) = map.get_mut(&utag) {
            inputs.insert(input);
        } else { 
            let mut s = BTreeSet::new();
            s.insert(input);
            map.insert(utag, s);
        }
    }
    map
}


/// Try to intentionally create L1D cache way mispredictions. 
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
/// the previous way index in a table. 
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
/// - If a utag is valid, data can be provided *before* the physical address 
///   is resolved, allowing loads to [speculatively] complete early
///
/// - If a utag is invalid (not associated with some way), an L1D miss can be 
///   recognized *before* the physical address is resolved, and the associated 
///   fill request can [speculatively] start early
///
/// On Zen 2 parts, the hash function used to produce the utag is known from 
/// previous research[^1] by Lipp, et al. 
/// See [`perfect::uarch::l1d::ZEN2_L1D_UTAG_FN`] for more details. 
///
/// [^1]: [Take A Way: Exploring the Security Implications of AMD's Cache Way Predictors](https://dl.acm.org/doi/10.1145/3320269.3384746)
///
/// Test
/// ====
///
/// 1. Perform a load from address A; way A is tagged with utag A
///
/// 2. Perform a load from address B, attempting to find way B with utag B
///
/// 3. If utag A and utag B are colliding (`utag B == utag A`), way A should 
///    always predicted [incorrectly] for the load from address B 
///
/// 4. After recognizing that way A *cannot* contain the data used to fulfill 
///    the load from address B, a MAB allocation *must* occur, eventually 
///    causing way B to be filled from the L2 cache
///
/// Since the hash function is already known (and consists of only 8 XOR gates
/// on pairs of independent input bits), we can easily validate this by scanning 
/// over all pairs of addresses whose input bits only contain two differences.
///
/// Results
/// =======
///
/// For all pairs of addresses with two-bit differences in the input, a 
/// Miss Address Buffer (MAB) allocation occurs consistently for the second
/// load when any of these conditions is true:
///
/// - `(vaddr_a[12] ^ vaddr_b[27]) == 1`
/// - `(vaddr_a[13] ^ vaddr_b[26]) == 1`
/// - `(vaddr_a[14] ^ vaddr_b[25]) == 1`
/// - `(vaddr_a[15] ^ vaddr_b[20]) == 1`
/// - `(vaddr_a[16] ^ vaddr_b[21]) == 1`
/// - `(vaddr_a[17] ^ vaddr_b[22]) == 1`
/// - `(vaddr_a[18] ^ vaddr_b[23]) == 1`
/// - `(vaddr_a[19] ^ vaddr_b[24]) == 1`
///
/// This matches the hash function described in the paper[^1].
///
/// Other Notes
/// ===========
///
/// The utag for a cacheline is [presumably] updated on every *retired* load.
///
/// According to the AMD SOG, within a particular set, only a single way can
/// be associated with a particular utag. Slightly paraphrasing:
///
/// > At a given L1D set index (for bits [11:6]), only one cacheline (one way 
/// > in the set) with a given utag is accessible at any time; any cachelines
/// > with matching utags are marked invalid and are not accessible.
///
/// This is the behavior used to build a side-channel in the paper[^1]. 
/// By issuing a load whose utag collides with some victim load, an attacker 
/// can force the victim to suffer an L1D miss. Afterwards, an attacker can 
/// re-issue the load and distinguish whether or not the victim has accessed
/// some line in the set. 
///
/// From my testing, it seems like the hash function is the same on Zen 3
/// as well - however, it seems like the event for MAB allocations has changed
/// on Family 19h parts: using PMC 0x41 mask 0x8 (instead of mask 0x1 used here)
/// appears to give us the same results. 
///
pub struct L1DWayPredictorMiss;
impl L1DWayPredictorMiss {

    /// Generate pairs consisting of address `a1`, and all (255) other 
    /// addresses whose utag is the same
    fn generate_candidates_for(a1: VirtualAddress) 
        -> Vec<(VirtualAddress, VirtualAddress)> 
    {
        let mut res = Vec::new();
        let map = compute_utag_map();
        let inputs = map.get(&a1.utag()).unwrap();
        for input in inputs {
            res.push((
                a1,
                VirtualAddress::new(0b000000, 0b000000, *input, 0),
            ));
        }
        res
    }

    /// Generate pairs of virtual addresses with 1-bit differences.
    fn generate_1bit_candidates() -> Vec<(VirtualAddress, VirtualAddress)> {
        let mut res = Vec::new();
        for idx in 0..16 {
            res.push((
                VirtualAddress::new(0b000000, 0b000000, 1<<idx, 0),
                VirtualAddress::new(0b000000, 0b000000, 0, 0),
            ));
        }
        for idx in 0..16 {
            res.push((
                VirtualAddress::new(0b000000, 0b000000, 0, 0),
                VirtualAddress::new(0b000000, 0b000000, 1<<idx, 0),
            ));
        }

        // NOTE: Same as above, but with unset bits instead of set bits.
        //for idx in 0..16 {
        //    res.push((
        //        VirtualAddress::new(0b000000, 0b000000, !(1<<idx), 0),
        //        VirtualAddress::new(0b000000, 0b000000, !0, 0),
        //    ));
        //}
        //for idx in 0..16 {
        //    res.push((
        //        VirtualAddress::new(0b000000, 0b000000, !0, 0),
        //        VirtualAddress::new(0b000000, 0b000000, !(1<<idx), 0),
        //    ));
        //}

        res
    }

    /// Generate pairs of virtual addresses with 2-bit differences.
    fn generate_2bit_candidates() -> Vec<(VirtualAddress, VirtualAddress)> {
        let mut res = Vec::new();
        for idx1 in 0..16 {
            for idx2 in 0..16 {
                res.push((
                    VirtualAddress::new(0b000000, 0b000000, 1<<idx1, 0),
                    VirtualAddress::new(0b000000, 0b000000, 1<<idx2, 0),
                ));
            }
        }

        // NOTE: Same as above, but with unset bits instead of set bits.
        //for idx1 in 0..16 {
        //    for idx2 in 0..16 {
        //        res.push((
        //            VirtualAddress::new(0b000000, 0b000000, !(1<<idx1), 0),
        //            VirtualAddress::new(0b000000, 0b000000, !(1<<idx2), 0),
        //        ));
        //    }
        //}

        res
    }

    /// Run a test which scans over all inputs with two-bit differences 
    /// (yielding collisions for utags where only a single bit is set).
    fn run_simple(harness: &mut PerfectHarness) {
        let pairs = Self::generate_2bit_candidates();
        let collisions = Self::run_with_candidates(harness, pairs);
        for (a1, a2) in collisions.iter() {
            let tag_diff = a1.utag_input() ^ a2.utag_input();
            println!("    inp_diff={:016b} [a1={:016x} inp={:016b} tag={:08b}] [a2={:016x} inp={:016b} tag={:08b}]",
                tag_diff,
                a1.value(), a1.utag_input(), a1.utag(), a2.value(), a2.utag_input(), a2.utag(),
            );
        }
    }

    /// Run a test which exhaustively produces all collisions for all inputs.
    fn run_exhaustive(harness: &mut PerfectHarness) {
        for input in 0x0000..=0xffff {
            let vaddr = VirtualAddress::new(0, 0, input, 0);
            let pairs = Self::generate_candidates_for(vaddr);
            let collisions = Self::run_with_candidates(harness, pairs);
            println!(" Observed {:3}/255 collisions for address {:016x}", collisions.len(), vaddr.value());
        }
    }

    /// Randomly test a handful of inputs against all collisions. 
    fn run_random(harness: &mut PerfectHarness) {
        let mut rng = rand::thread_rng();
        let mut inputs = Vec::new();
        for _ in 0..32 {
            inputs.push(rng.gen_range(0x0000..=0xffffusize));
        }
        for input in inputs {
            let vaddr = VirtualAddress::new(0, 0, input, 0);
            let pairs = Self::generate_candidates_for(vaddr);
            let collisions = Self::run_with_candidates(harness, pairs);
            println!(" Observed {:3}/255 collisions for address {:016x}", collisions.len(), vaddr.value());
        }
    }


    /// Run tests over the given pairs of virtual addresses. 
    ///
    /// If a miss always occurs for the second load, presumably we have 
    /// created a situation where the prediction is *always* incorrect 
    /// because the utags for both addresses are colliding. 
    ///
    /// Returns the set of pairs whose utags are colliding. 
    fn run_with_candidates(
        harness: &mut PerfectHarness, 
        pairs: Vec<(VirtualAddress, VirtualAddress)>,
    ) -> BTreeSet<(VirtualAddress, VirtualAddress)>
    {
        let mut collisions = BTreeSet::new();

        // We're measuring for Miss Address Buffer (MAB) allocations that are 
        // caused by loads.
        // NOTE: On Zen 3 parts, I think this is mask bit 0x8 instead of 0x1
        let event = Zen2Event::LsMabAlloc(LsMabAllocMask::Loads);

        let measure_asm = Self::emit_measure();
        let measure_asm_fn = measure_asm.as_fn();
        let desc = event.as_desc();

        for (a1, a2) in pairs.iter() {
            let results = harness.measure(measure_asm_fn,
                desc.id(), desc.mask(), 1024,
                InputMethod::Fixed(a1.value(), a2.value()),
            ).unwrap();

            let min = results.get_min();
            if min != 0 {
                collisions.insert((*a1, *a2));
            }
        }
        collisions
    }

    /// Emit the experiment. 
    /// Input arguments to this function during runtime are: 
    ///
    /// - `RDI`, virtual address A
    /// - `RSI`, virtual address B
    ///
    fn emit_measure() -> X64AssemblerFixed {
        let base_addr = 0x0000_1000_0000_0000;
        let mut f = X64AssemblerFixed::new(base_addr, 0x0001_0000);

        // Perform the first load (from the address in RDI). 
        //
        // NOTE: I'm not actually sure that the MFENCE and LFENCE are necessary,
        // but this way it's obvious that the first load *will* be complete 
        // and the utag *will* be set to the appropriate way for the first load
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
}


