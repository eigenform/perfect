//! Harness configuration.

use crate::experiments::ExperimentArgs;
use crate::harness::PerfectHarness;
use crate::util::*;

/// The target platform for generated code. 
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum TargetPlatform {
    Zen2,
    Zen3,
    Tremont,
}

/// A strategy used by [`PerfectHarness`] for zeroing out the integer 
/// general-purpose registers before entering measured code. 
#[derive(Clone, Copy)]
pub enum ZeroStrategy {
    None,
    XorIdiom,
    MovFromZero,
}

/// A strategy used by [`PerfectHarness`] for zeroing out the vector 
/// general-purpose registers before entering measured code. 
#[derive(Clone, Copy)]
pub enum ZeroStrategyFp {
    None,
    Vzeroall,
    XorIdiom,
    MovFromZero,
}

/// Configuration passed to [PerfectHarness::emit].
#[derive(Clone, Copy)]
pub struct HarnessConfig {
    /// The target platform
    pub platform: TargetPlatform,

    pub harness_addr: usize,
    pub harness_size: usize,

    /// Optionally dump the state of the integer general-purpose registers 
    /// after running measured code.
    pub dump_gpr: bool,

    /// Optionally dump the state of the vector general-purpose registers
    /// after running measured code.
    pub dump_vgpr: bool,

    /// Optionally allow the harness to automatically execute RDPMC
    /// immediately before/after calling into measured code. 
    pub auto_rdpmc: Option<usize>,

    /// Optionally compare RDI to a constant value before entering measured
    /// code. 
    pub cmp_rdi: Option<i32>,

    /// Optionally pin the caller to a specific hardware thread. 
    pub pinned_core: Option<usize>,

    /// Optionally allocate a fixed memory region for use by measured code.
    pub arena_alloc: Option<(usize, usize)>, 

    /// Optionally [try to] flush the BTB. 
    pub flush_btb: Option<usize>,

    /// The strategy for zeroing integer general-purpose registers before 
    /// entering measured code.
    pub zero_strat: ZeroStrategy,

    /// The strategy for zeroing vector general-purpose registers before 
    /// entering measured code.
    pub zero_strat_fp: ZeroStrategyFp,
}

impl HarnessConfig {

    /// Default base address for the harness. 
    const DEFAULT_ADDR: usize = 0x0000_1337_0000_0000;

    /// Default allocation size for the harness (64MiB)
    const DEFAULT_SIZE: usize = 0x0000_0000_0400_0000;

    pub fn from_cmdline_args(args: &ExperimentArgs) -> Option<Self> { 
        if let Some(p) = args.platform {
            match p { 
                TargetPlatform::Zen2 => Some(Self::default_zen2()),
                TargetPlatform::Zen3 => Some(Self::default_zen3()),
                _ => unimplemented!("{:?}", p),
            }
        } else {
            None
        }
    }

    pub fn default_zen2() -> Self { 
        Self {
            pinned_core: Some(15),
            harness_addr: Self::DEFAULT_ADDR,
            harness_size: Self::DEFAULT_SIZE,
            arena_alloc: Some((0x0000_0000, 0x1000_0000)),
            dump_gpr: false,
            dump_vgpr: false,
            auto_rdpmc: None,
            cmp_rdi: None,
            flush_btb: None,
            platform: TargetPlatform::Zen2,
            zero_strat: ZeroStrategy::MovFromZero,
            zero_strat_fp: ZeroStrategyFp::None,
        }
    }

    pub fn default_zen3() -> Self { 
        Self {
            pinned_core: Some(5),
            harness_addr: Self::DEFAULT_ADDR,
            harness_size: Self::DEFAULT_SIZE,
            arena_alloc: Some((0x0000_0000, 0x1000_0000)),
            dump_gpr: false,
            dump_vgpr: false,
            auto_rdpmc: None,
            cmp_rdi: None,
            flush_btb: None,
            platform: TargetPlatform::Zen3,
            zero_strat: ZeroStrategy::MovFromZero,
            zero_strat_fp: ZeroStrategyFp::None,
        }
    }


    pub fn default_tremont() -> Self { 
        Self {
            pinned_core: Some(15),
            harness_addr: Self::DEFAULT_ADDR,
            harness_size: Self::DEFAULT_SIZE,
            arena_alloc: Some((0x0000_0000, 0x1000_0000)),
            dump_gpr: false,
            dump_vgpr: false,
            auto_rdpmc: None,
            cmp_rdi: None,
            flush_btb: None,
            platform: TargetPlatform::Tremont,
            zero_strat: ZeroStrategy::MovFromZero,
            zero_strat_fp: ZeroStrategyFp::None,
        }
    }
}

impl HarnessConfig {
    pub fn harness_addr(mut self, addr: usize) -> Self { 
        self.harness_addr = addr;
        self
    }
    pub fn harness_size(mut self, size: usize) -> Self { 
        self.harness_size = size;
        self
    }

    pub fn dump_gpr(mut self, x: bool) -> Self {
        self.dump_gpr = x;
        self
    }

    pub fn dump_vgpr(mut self, x: bool) -> Self {
        self.dump_vgpr = x;
        self
    }

    pub fn auto_rdpmc(mut self, x: Option<usize>) -> Self {
        self.auto_rdpmc = x;
        self
    }

    pub fn pinned_core(mut self, x: Option<usize>) -> Self {
        self.pinned_core = x;
        self
    }

    pub fn no_arena_alloc(mut self) -> Self { 
        self.arena_alloc = None;
        self
    }

    pub fn arena_alloc(mut self, base: usize, len: usize) -> Self {
        self.arena_alloc = Some((base, len));
        self
    }

    pub fn cmp_rdi(mut self, x: i32) -> Self {
        self.cmp_rdi = Some(x);
        self
    }

    pub fn platform(mut self, x: TargetPlatform) -> Self {
        self.platform = x;
        self
    }

    pub fn flush_btb(mut self, x: usize) -> Self {
        self.flush_btb = Some(x);
        self
    }

    pub fn zero_strategy(mut self, x: ZeroStrategy) -> Self { 
        self.zero_strat = x;
        self
    }

    pub fn zero_strategy_fp(mut self, x: ZeroStrategyFp) -> Self { 
        self.zero_strat_fp = x;
        self
    }

}

impl HarnessConfig {
    /// Create a [PerfectHarness] using this configuration.
    pub fn emit(self) -> PerfectHarness {
        if let Some(pinned_core) = self.pinned_core {
            PerfectEnv::pin_to_core(pinned_core);
            //println!("[*] Pinned to core {}", pinned_core);
        }
        if let Some((base, len)) = self.arena_alloc {
            let mmap_min_addr = PerfectEnv::procfs_mmap_min_addr();
            if base < mmap_min_addr {
                println!("[!] The harness arena is set to {:016x}, \
                    but vm.mmap_min_addr is set to {:016x}", 
                    base, mmap_min_addr);
                panic!("[!] Cannot mmap for harness arena at {:016x}", base);
            }
            let _ = PerfectEnv::mmap_fixed(base, len);
        }

        let mut res = PerfectHarness::new(self);
        res
    }
}


