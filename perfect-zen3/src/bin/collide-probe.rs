use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use rand::distributions::Uniform;
use std::collections::*;

use perfect::stats::{ RawResults, ResultList };
use perfect::uarch::l1d::ZEN2_L1D_UTAG_FN;
use perfect_zen3::{ Victim, VictimMsg };

pub struct TestResults { 
    desc: &'static str,
    kernel_vaddr: VirtualAddress,
    user_vaddr: VirtualAddress,
    results: RawResults,
}
impl TestResults { 
    pub fn print(&self) {
        println!("[*] Results for '{}'", self.desc);

        let min = self.results.get_min();
        let max = self.results.get_max();
        let mode = self.results.get_mode();
        println!("  Kernel load   @ {:016x}, set={:06b}, utag={:08b}", 
            self.kernel_vaddr.0, 
            self.kernel_vaddr.set(),
            self.kernel_vaddr.utag(),
        );
        println!("  User load     @ {:016x}, set={:06b}, utag={:08b}", 
            self.user_vaddr.0, 
            self.user_vaddr.set(),
            self.user_vaddr.utag(),
        );
        println!("    min={:4} max={:4} mode={:4}", 
            min, max, mode,
        );
        println!();
    }
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

    pub fn with_offset(self, off: usize) -> Self { 
        Self(
            (self.0 & !Self::OFFSET_MASK) | ((off & 0b111111))
        )
    }
    pub fn with_set(self, set: usize) -> Self { 
        Self(
            (self.0 & !Self::SET_MASK) | ((set & 0b111111) << 6)
        )
    }
    pub fn with_utag_input(self, input: usize) -> Self { 
        Self(
            (self.0 & !Self::UTAG_MASK) | ((input & 0xffff) << 12)
        )
    }


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

    // Generate the set of all 256 addresses whose micro-tags are colliding
    // with the micro-tag for this address.
    pub fn generate_collisions(&self) -> Vec<Self> {
        let mut res = Vec::new();
        let map = Self::compute_utag_map();
        let inputs = map.get(&self.utag()).unwrap();
        for input in inputs {
            res.push(VirtualAddress::new(0b000000, 0b000000, *input, 0));
        }
        res
    }

    // Generate a random address whose micro-tag is colliding with the 
    // micro-tag for this address. 
    pub fn random_collision(&self) -> Self { 
        let mut rng = thread_rng();
        let colls = self.generate_collisions();
        let x = rng.gen_range(0..256);
        colls[x]
    }

    /// Build a map of all possible micro-tags and their associated input bits.
    pub fn compute_utag_map() -> BTreeMap<usize, BTreeSet<usize>> {
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
}

/// Demonstrate the Collide+Probe side-channel. 
///
/// Context
/// =======
///
/// The paper[^1] describes a timing side-channel that follows from the 
/// implementation of the L1D way predictor on Family 17h parts. 
/// This experiment tries to reproduce this behavior on Zen 3. 
///
/// For each load, a simple function computes an 8-bit "micro-tag" from 
/// the virtual address. The way predictor consists of a fully-associative 
/// memory which associates each possible utag with an L1D cache way index.
///
/// When a load hits in the L1D cache, the index of the hitting way is tagged
/// with the associated micro-tag. A subsequent load to an address with the 
/// same micro-tag uses this to predict the hitting way, allowing the load to 
/// complete early (before TLB access and L1D tag matching). 
///
/// See [./dcache.rs] for more notes on the way predictor. 
///
/// [^1]: [Take A Way: Exploring the Security Implications of AMD’s Cache Way Predictors](https://gruss.cc/files/takeaway.pdf)
///
/// Configuration
/// =============
///
/// To make this behavior easier to observe, this experiment relies on an 
/// intentionally-vulernable Linux kernel module where
///
/// - The virtual address of a kernel page is intentionally leaked to 
///   userspace at `/sys/kernel/debug/victim/scratch_page_vaddr`
/// - An `ioctl()` can be used to cause the kernel to read a secret value
///
/// This experiment also assumes that: 
///
/// - SMT is disabled
/// - KASLR is disabled
/// - All mitigations are disabled
///
/// Experiment
/// ==========
///
/// 1. Obtain the 'victim address' with micro-tag `V`.
///
/// 2. Generate a 'colliding address' with micro-tag `V`, then 
///    map the address into the address space of the current process.
///
/// 3. Perform a load from the colliding address [in userspace]. 
///    The predictor associates cache way `A` with micro-tag `V`. 
///    
/// 4. Cause the kernel module to load from the victim address. 
///    The predictor associates cache way `B` with micro-tag `V`. 
///
/// 5. Perform a load from the colliding address again and measure. 
///    If the predicted way is correct (cache way `A`), we expect the access 
///    to be fast. Otherwise, the access should be slow. 
///
/// 6. *Assuming that no other aliasing load has occured in-between step #3 
///    and step #5*, we can infer that the load suffered a way misprediction 
///    (a "false completion") caused by an access to the victim address which
///    was hitting in cache way `B`. 
///
/// In order to compare against typical behavior, we test the following  
/// cases: 
///
/// 1. Loads with aliasing micro-tags, to the same cache set. 
/// 2. Loads with aliasing micro-tags, to different cache sets
/// 3. Loads with different micro-tags, to the same cache set
/// 4. Loads with different micro-tags, to different cache sets
///
///
///
/// Results
/// =======
///
/// - In cases #2/#3/#4, the load takes ~6 cycles. 
///   This is the typical case. 
///
/// - For case #1 (aliasing tags, same set), the load takes ~18 cycles. 
///   Presumably, the extra ~12 cycles is caused by the way misprediction.
///
pub struct CollideAndProbe;
impl CollideAndProbe { 
    /// Number of test iterations for each candidate. 
    const ITERS: usize = 128;

    /// Emit the function used to measure the colliding load. 
    fn emit_measure() -> X64AssemblerFixed { 
        let mut f = X64AssemblerFixed::new(0x4000_0000, 0x0001_0000);
        f.emit_aperf_start(Gpr::Rsi as u8);
        dynasm!(f ; mov rax, [rdi]);
        f.emit_aperf_end(Gpr::Rsi as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn emit_measure_floor() -> X64AssemblerFixed { 
        let mut f = X64AssemblerFixed::new(0x4100_0000, 0x0001_0000);
        f.emit_aperf_start(Gpr::Rsi as u8);
        f.emit_aperf_end(Gpr::Rsi as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }


    /// Run the Collide+Probe test where: 
    ///
    /// - 'kernel_vaddr' is the address of the kernel load
    /// - 'user_vaddr' is the address of the measured load [in userspace]
    ///
    fn run_collide_and_probe(
        harness: &mut PerfectHarness,
        victim: &mut Victim,
        probe_func: MeasuredFn,
        floor_func: MeasuredFn,
        user_vaddr: VirtualAddress,
        kernel_vaddr: VirtualAddress,
    ) -> RawResults 
    { 
        let mut floor_results = RawResults(vec![0; Self::ITERS]);
        for i in 0..Self::ITERS { 
            let t = (floor_func)(0, 0);
            floor_results.0[i] = t;
        }
        let floor = floor_results.get_min();

        let user_ptr = user_vaddr.0 as *const u32;
        let kernel_offset = (kernel_vaddr.0 & 0xfff) as i32;

        let mut results = RawResults(vec![0; Self::ITERS]);
        unsafe { 
            victim.invd();
            for i in 0..Self::ITERS { 
                let _ = core::ptr::read_volatile(user_ptr);
                core::arch::x86_64::_mm_mfence();
                victim.ping(kernel_offset);
                let t = (probe_func)(user_ptr as usize, 0);
                //let t = harness.call(user_ptr as usize, 0, probe_func);
                results.0[i] = (t - floor);
            }
        }
        results
    }
}

impl CollideAndProbe { 
    fn run_aliasing_test(
        harness: &mut PerfectHarness,
        victim: &mut Victim,
        probe_func: MeasuredFn,
        floor_func: MeasuredFn,
        kernel_base_vaddr: VirtualAddress,
    ) -> TestResults
    {
        // Pick a random colliding address
        let user_base_vaddr = kernel_base_vaddr.random_collision();
        // Pick a random cache set used by both loads
        let set = harness.rng.gen_range(0..64);

        // Map the colliding address into our process' address space
        let user_base_ptr = PerfectEnv::mmap_fixed(
            user_base_vaddr.0, 0x4000
        );

        let user_vaddr   = VirtualAddress(user_base_vaddr.0)
            .with_set(set);
        let kernel_vaddr = VirtualAddress(kernel_base_vaddr.0)
            .with_set(set);

        let results = Self::run_collide_and_probe(
            harness, 
            victim, 
            probe_func,
            floor_func,
            user_vaddr,
            kernel_vaddr,
        );

        TestResults { 
            desc: "aliasing utag, same set",
            kernel_vaddr,
            user_vaddr, 
            results
        }

    }

    fn run_aliasing_cross_set_test(
        harness: &mut PerfectHarness,
        victim: &mut Victim,
        probe_func: MeasuredFn,
        floor_func: MeasuredFn,
        kernel_base_vaddr: VirtualAddress,
    ) -> TestResults
    {
        // Pick a random colliding address
        let user_base_vaddr = kernel_base_vaddr.random_collision();

        // Pick two random *distinct* cache sets
        let user_set = harness.rng.gen_range(0..64);
        let kernel_set = user_set ^ 0b111111; 

        // Map the colliding address into our process' address space
        let user_base_ptr = PerfectEnv::mmap_fixed(
            user_base_vaddr.0, 0x4000
        );

        let user_vaddr   = VirtualAddress(user_base_vaddr.0)
            .with_set(user_set);
        let kernel_vaddr = VirtualAddress(kernel_base_vaddr.0)
            .with_set(kernel_set);

        let results = Self::run_collide_and_probe(
            harness, 
            victim, 
            probe_func,
            floor_func,
            user_vaddr,
            kernel_vaddr,
        );

        TestResults { 
            desc: "aliasing utag, different set",
            kernel_vaddr,
            user_vaddr, 
            results
        }

    }

    fn run_nonaliasing_cross_set_test(
        harness: &mut PerfectHarness,
        victim: &mut Victim,
        probe_func: MeasuredFn,
        floor_func: MeasuredFn,
        kernel_base_vaddr: VirtualAddress,
    ) -> TestResults
    {
        // Pick two random *distinct* cache sets
        let user_set = harness.rng.gen_range(0..64);
        let kernel_set = user_set ^ 0b111111; 

        let input_bits = harness.rng.gen_range(0x0000..=0xffff);
        let user_base_vaddr = VirtualAddress(0x1dea_0000_0000)
            .with_utag_input(input_bits);

        // Map the colliding address into our process' address space
        let user_base_ptr = PerfectEnv::mmap_fixed(
            user_base_vaddr.0, 0x4000
        );

        let user_vaddr   = VirtualAddress(user_base_vaddr.0)
            .with_set(user_set);
        let kernel_vaddr = VirtualAddress(kernel_base_vaddr.0)
            .with_set(kernel_set);

        let results = Self::run_collide_and_probe(
            harness, 
            victim, 
            probe_func,
            floor_func,
            user_vaddr,
            kernel_vaddr,
        );

        TestResults { 
            desc: "non-aliasing utag, different set",
            kernel_vaddr,
            user_vaddr, 
            results
        }
    }

    fn run_nonaliasing_test(
        harness: &mut PerfectHarness,
        victim: &mut Victim,
        probe_func: MeasuredFn,
        floor_func: MeasuredFn,
        kernel_base_vaddr: VirtualAddress,
    ) -> TestResults
    {
        // Pick a random cache set
        let user_set = harness.rng.gen_range(0..64);
        let kernel_set = user_set;

        let input_bits = harness.rng.gen_range(0x0000..=0xffff);
        let user_base_vaddr = VirtualAddress(0x1dea_0000_0000)
            .with_utag_input(input_bits);


        // Map the colliding address into our process' address space
        let user_base_ptr = PerfectEnv::mmap_fixed(
            user_base_vaddr.0, 0x4000
        );

        let user_vaddr   = VirtualAddress(user_base_vaddr.0)
            .with_set(user_set);
        let kernel_vaddr = VirtualAddress(kernel_base_vaddr.0)
            .with_set(kernel_set);

        let results = Self::run_collide_and_probe(
            harness, 
            victim, 
            probe_func,
            floor_func,
            user_vaddr,
            kernel_vaddr,
        );

        TestResults { 
            desc: "non-aliasing utag, same set",
            kernel_vaddr,
            user_vaddr, 
            results
        }
    }




    fn run(harness: &mut PerfectHarness) {
        // Open a handle to the victim kernel module
        let mut victim = Victim::open();

        // Emit gadget for measuring our access
        let probe_func = Self::emit_measure();
        let floor_func = Self::emit_measure_floor();

        // Find the virtual address of the page allocated by the kernel
        let kernel_base_vaddr = VirtualAddress(victim.scratch_page());
        println!("Kernel page @ {:016x}, utag={:08b}", 
            kernel_base_vaddr.0,
            kernel_base_vaddr.utag(),
        );
        println!();

        let res = Self::run_aliasing_test(
            harness, 
            &mut victim, 
            probe_func.as_fn(), 
            floor_func.as_fn(),
            kernel_base_vaddr
        );
        res.print();

        let res = Self::run_aliasing_cross_set_test(
            harness, 
            &mut victim, 
            probe_func.as_fn(), 
            floor_func.as_fn(),
            kernel_base_vaddr
        );
        res.print();

        let res = Self::run_nonaliasing_test(
            harness, 
            &mut victim, 
            probe_func.as_fn(), 
            floor_func.as_fn(),
            kernel_base_vaddr
        );
        res.print();

        let res = Self::run_nonaliasing_cross_set_test(
            harness, 
            &mut victim, 
            probe_func.as_fn(), 
            floor_func.as_fn(),
            kernel_base_vaddr
        );
        res.print();




    }
}


fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .pinned_core(Some(5))
        .emit();

    CollideAndProbe::run(&mut harness);

}


