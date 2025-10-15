use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use rand::distributions::Uniform;
use std::collections::*;

use perfect::stats::{ RawResults, ResultList };

fn main() {
    let mut harness = HarnessConfig::default_zen3()
        .pinned_core(Some(5))
        .emit();
    Div::run(&mut harness);
}


/// Record for the result of some test.
#[derive(Clone)]
pub struct TestResult { 
    inp: Input,
    mode: usize,
}

/// Inputs to a DIV operation. 
///
/// - The dividend (input value) is RDX:RAX
/// - The quotient (output value) is in RAX, with remainder in RDX 
/// - The maximum quotient is 0xffff_ffff_ffff_ffff (overwise #DE)
///
#[derive(Clone, Copy, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Input { 
    rdx: usize,
    rax: usize,
    div: i64,
}
impl Input { 
    /// Is this set of inputs "valid?" (ie. is the quotient in-range?)
    fn is_valid(&self) -> bool { 
        let x: u128 = ((self.rdx as u128) << 64) | self.rax as u128;
        let quotient = x / self.div as u128;
        if quotient >= u64::MAX as u128 { return false; }
        if quotient.count_ones() == 0 { return false; }
        true
    }
}

// Purely random inputs
impl Input { 
    fn new_random(rng: &mut ThreadRng) -> Self { 
        Self { 
            rdx: rng.gen(),
            rax: rng.gen(),
            div: rng.gen(),
        }
    }
    fn new_random_valid(rng: &mut ThreadRng) -> Self { 
        loop {
            let res = Self::new_random(rng);
            if res.is_valid() { return res; }
        }
    }
}

// Random inputs constrained to a certain bit-width. 
impl Input { 
    fn new_random_bits(rng: &mut ThreadRng, 
        dividend_bits: usize,
        divisor_bits: usize,
    ) -> Self 
    { 
        let mask = if dividend_bits >= 128 { u128::MAX } 
        else if dividend_bits == 0 { u128::MIN } 
        else { (1 << dividend_bits) - 1 };

        let dividend: u128 = rng.gen::<u128>() & mask;
        let rdx = (
            (dividend & 0xffff_ffff_ffff_ffff_0000_0000_0000_0000) >> 64
        ) as usize;
        let rax = (dividend & 0xffff_ffff_ffff_ffff) as usize;

        let mask = if divisor_bits >= 64 { u64::MAX } 
        else if divisor_bits == 0 { u64::MIN } 
        else { (1 << divisor_bits) - 1 };

        let divisor: u64 = rng.gen();
        let div = (divisor & mask) as i64;

        Self { rdx, rax, div }
    }

    fn new_random_bits_valid(rng: &mut ThreadRng, 
        dividend_bits: usize,
        divisor_bits: usize,
    ) -> Self 
    { 
        loop {
            let res = Self::new_random_bits(rng, dividend_bits, divisor_bits);
            if res.is_valid() { return res; }
        }
    }
}

impl Input { 
    fn new_random_nbits(rng: &mut ThreadRng,
        num_rdx_bits: usize,
        num_rax_bits: usize,
        num_div_bits: usize,
    ) -> Self 
    { 
        fn gen_bits(rng: &mut ThreadRng, num: usize) -> HashSet<usize> {
            let mut set = HashSet::new();
            while set.len() < num {
                set.insert(rng.gen_range(0..64));
            }
            set
        }

        let rdx_bits = gen_bits(rng, num_rdx_bits);
        let rax_bits = gen_bits(rng, num_rdx_bits);
        let div_bits = gen_bits(rng, num_rdx_bits);

        let mut rdx = 0;
        let mut rax = 0;
        let mut div = 0;

        for idx in rdx_bits { rdx |= (1 << idx); }
        for idx in rax_bits { rax |= (1 << idx); }
        for idx in div_bits { div |= (1 << idx); }

        Self { rdx, rax, div }
    }

    fn new_random_nbits_valid(rng: &mut ThreadRng, 
        num_rdx_bits: usize,
        num_rax_bits: usize,
        num_div_bits: usize,
    ) -> Option<Self>
    { 
        let mut iters = 0;
        loop {
            if num_rdx_bits == 0 { 
                return None;
            }
            if iters > 1_000 {
                return None;
            }
            let res = Self::new_random_nbits(rng, 
                num_rdx_bits,
                num_rax_bits,
                num_div_bits
            );
            if res.is_valid() { return Some(res); }
            iters += 1;
        }
    }

}

/// Explore the data-dependent behavior of the x86 DIV instruction. 
///
/// Scan over the space of input values to DIV and record the latency. 
pub struct Div;
impl Div { 

    /// Number of tests to run on a single set of inputs
    const ITERS: usize = 64;

    /// Run some tests
    fn run(harness: &mut PerfectHarness) {
        // Single-bit inputs
        Self::run_single_bit_inputs(harness);

        // Random inputs
        //Self::run_random_search(harness);

        // n-bit inputs
        //Self::run_nbit_inputs(harness);
    }

    /// Emitter measuring a DIV instruction
    fn emit_div(divisor: i64) -> X64AssemblerFixed {
        let mut f = X64AssemblerFixed::new(0x4000_0000, 0x0001_0000);
        dynasm!(f
            ; mov r9, QWORD divisor
        );
        f.emit_aperf_start(Gpr::R8 as u8);
        dynasm!(f
            ; mov rdx, rdi 
            ; mov rax, rsi
            ; div r9
            ; xor rax, rax
            ; xor rdx, rdx
        );
        f.emit_aperf_end(Gpr::R8 as u8, Gpr::Rax as u8);

        f.emit_ret();
        f.commit().unwrap();
        f
    }

    /// Emitter measuring the APERF floor
    fn emit_floor() -> X64AssemblerFixed {
        let mut f = X64AssemblerFixed::new(0x4000_0000, 0x0001_0000);
        f.emit_aperf_start(Gpr::R8 as u8);
        f.emit_aperf_end(Gpr::R8 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    /// Run a test with the given inputs. 
    fn run_test(
        harness: &mut PerfectHarness,
        floor: usize,
        inp: Input,
    ) -> TestResult 
    {
        let div_func = Self::emit_div(inp.div);
        let mut raw = RawResults(vec![0; Self::ITERS]);
        for idx in 0..Self::ITERS { 
            let t = harness.call(inp.rdx, inp.rax, div_func.as_fn());
            raw.0[idx] = t - floor;
        }
        TestResult { inp, mode: raw.get_mode() }
    }

    /// Measure the floor (associated with the use of RDPRU/APERF). 
    fn run_floor(harness: &mut PerfectHarness) -> RawResults {
        let floor_func = Self::emit_floor();
        let mut res = RawResults(vec![0; Self::ITERS]);
        for idx in 0..Self::ITERS { 
            res.0[idx] = harness.call(0, 0, floor_func.as_fn());
        }
        res
    }

    /// Explore the space of inputs with only a single set/unset bit. 
    fn run_single_bit_inputs(harness: &mut PerfectHarness) {
        let floor_func = Self::emit_floor();
        let mut floor_results = RawResults(vec![0; Self::ITERS]);
        for idx in 0..Self::ITERS { 
            floor_results.0[idx] = harness.call(0, 0, floor_func.as_fn());
        }
        let floor = floor_results.get_mode();

        let mut inputs = Vec::new();

        for i in 0..64 {
            for j in 0..64 { 
                for k in 0..64 { 
                    let inp_set = Input { 
                        rdx: (1 << i),
                        rax: (1 << j),
                        div: (1 << k),
                    };
                    let inp_unset = Input { 
                        rdx: !(1 << i),
                        rax: !(1 << j),
                        div: !(1 << k),
                    };

                    if inp_set.is_valid() { inputs.push(inp_set); }
                    if inp_unset.is_valid() { inputs.push(inp_unset); }

                    let inp_set = Input { rdx: 0, rax: (1 << j), div: (1 << k) };
                    if inp_set.is_valid() { inputs.push(inp_set); }
                    let inp_set = Input { rdx: (1<<i), rax: 0, div: (1 << k) };
                    if inp_set.is_valid() { inputs.push(inp_set); }
                }
            }
        }
        let mut best = TestResult { 
            inp: Input { rdx: 0, rax: 0, div: 0 },
            mode: usize::MAX,
        };
        let mut worst = TestResult { 
            inp: Input { rdx: 0, rax: 0, div: 0 },
            mode: usize::MIN,
        };

        let mut hist = BTreeMap::new();
        let mut log: BTreeMap<usize, BTreeSet<Input>> = BTreeMap::new();

        for inp in inputs { 
            let res = Self::run_test(harness, floor, inp);
            if let Some(cnt) = hist.get_mut(&res.mode) {
                *cnt += 1;
            } else { 
                hist.insert(res.mode, 1);
            }

            if let Some(inps) = log.get_mut(&res.mode) { 
                inps.insert(res.inp);
            } else { 
                let mut v = BTreeSet::new();
                v.insert(res.inp);
                log.insert(res.mode, v);
            }

            if res.mode < best.mode {
                println!("best:  {:016x?} {}", res.inp, res.mode);
                best = res.clone();
            }

            if res.mode > worst.mode {
                println!("worst: {:016x?} {}", res.inp, res.mode);
                worst = res.clone();
            }
        }

        for (lat, cnt) in hist.iter() { 
            println!("  lat={:4} cnt={}", lat, cnt);
        }

        for (lat, inputs) in log { 
            println!("[*] Lat {} inputs:", lat);
            for inp in inputs { 
                println!("  {:016x?}", inp);
            }
        }


    }


    /// Randomly search [in an infinite loop] over the space of inputs and 
    /// record the worst/best observed cases. 
    fn run_random_search(harness: &mut PerfectHarness) {

        let floor = Self::run_floor(harness).get_mode();

        let mut best = TestResult { 
            inp: Input { rdx: 0, rax: 0, div: 0 },
            mode: usize::MAX,
        };
        let mut worst = TestResult { 
            inp: Input { rdx: 0, rax: 0, div: 0 },
            mode: usize::MIN,
        };

        let mut hist = BTreeMap::new();
        let mut iter = 0;
        loop {
            //let inp = Input::new_random_valid(&mut harness.rng);
            let inp = Input::new_random_bits_valid(&mut harness.rng, 31, 30);
            let res = Self::run_test(harness, floor, inp);

            if let Some(cnt) = hist.get_mut(&res.mode) {
                *cnt += 1;
            } else { 
                hist.insert(res.mode, 1);
            }

            if res.mode < best.mode {
                println!("best:  {:016x?} {}", res.inp, res.mode);
                best = res.clone();
            }

            if res.mode > worst.mode {
                println!("worst: {:016x?} {}", res.inp, res.mode);
                worst = res.clone();
            }

            if iter % 1_000_000 == 0 {
                println!("[*] iter={}", iter);
                for (lat, cnt) in hist.iter() { 
                    println!("  lat={:4} cnt={}", lat, cnt);
                }
            }

            iter += 1;
        }
    }

    fn run_nbit_inputs(harness: &mut PerfectHarness) {
        let floor_func = Self::emit_floor();
        let mut floor_results = RawResults(vec![0; Self::ITERS]);
        for idx in 0..Self::ITERS { 
            floor_results.0[idx] = harness.call(0, 0, floor_func.as_fn());
        }
        let floor = floor_results.get_mode();

        let mut best = TestResult { 
            inp: Input { rdx: 0, rax: 0, div: 0 },
            mode: usize::MAX,
        };
        let mut worst = TestResult { 
            inp: Input { rdx: 0, rax: 0, div: 0 },
            mode: usize::MIN,
        };

        let mut hist = BTreeMap::new();
        let mut iter = 0;
        let mut rdx_bits = harness.rng.gen_range(0..64);
        let mut rax_bits = harness.rng.gen_range(0..64);
        let mut div_bits = harness.rng.gen_range(1..64);

        println!("[*] rdx_bits={} rax_bits={} div_bits={}", 
            rdx_bits, rax_bits, div_bits,
        );

        'l: loop {
            //let inp = Input::new_random_valid(&mut harness.rng);
            
            let mut inp: Input;
            match Input::new_random_nbits_valid(&mut harness.rng, 
                rdx_bits, rax_bits, div_bits
            ) 
            {
                Some(res) => { 
                    inp = res; 
                },
                None => {
                    rdx_bits = harness.rng.gen_range(0..64);
                    rax_bits = harness.rng.gen_range(0..64);
                    div_bits = harness.rng.gen_range(1..64);
                    println!("[*] rdx_bits={} rax_bits={} div_bits={}", 
                        rdx_bits, rax_bits, div_bits,
                    );
                    continue 'l;
                },
            }

            let res = Self::run_test(harness, floor, inp);

            if let Some(cnt) = hist.get_mut(&res.mode) {
                *cnt += 1;
            } else { 
                hist.insert(res.mode, 1);
            }

            if res.mode < best.mode {
                println!("best:  {:016x?} {}", res.inp, res.mode);
                best = res.clone();
            }

            if res.mode > worst.mode {
                println!("worst: {:016x?} {}", res.inp, res.mode);
                worst = res.clone();
            }

            if iter % 10_000 == 0 {
                println!("[*] iter={}", iter);
                for (lat, cnt) in hist.iter() { 
                    println!("  lat={:4} cnt={}", lat, cnt);
                }
                rdx_bits = harness.rng.gen_range(0..64);
                rax_bits = harness.rng.gen_range(0..64);
                div_bits = harness.rng.gen_range(1..64);
            println!("[*] rdx_bits={} rax_bits={} div_bits={}", 
                rdx_bits, rax_bits, div_bits,
        );


            }

            iter += 1;
        }


    }


}
