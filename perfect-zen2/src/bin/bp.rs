use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use perfect::util::*;
use perfect::ir::branch::*;
use itertools::*;
use std::collections::*;


fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .arena_alloc(0x0000_0000_0000_0000, 0x0000_0000_1000_0000)
        .emit();
    CorrelatedBranches::run(&mut harness);
    //ConditionalBranch::run(&mut harness);
}

/// [Naively?] try to interfere with two correlated conditional branches. 
///
/// Context
/// =======
///
/// Predicting the *direction* of a branch usually entails the following:
///
/// - Keeping track of a "local" history of outcomes for a particular branch
/// - Keeping track of a "global" history of outcomes for all branches
/// - Using some fancy method of combining these two kinds of information
///
/// Test
/// ====
///
/// 1. Emit a conditional branch with a *random* outcome. 
/// 2. Emit a variable number of always-taken padding branches.
/// 3. Emit a conditional branch with *the same* outcome as the first branch.
///
/// Under normal circumstances (with very few padding branches), we expect 
/// that the branch predictor has learned that the first and last branches
/// are correlated, and that the last branch will be correctly predicted 
/// very close to 100% of the time. 
///
/// After a certain number of padding branches, we expect that the machine
/// will not be able to preserve the correlation between the first and last
/// branches. This is a reasonable assumption because the amount of storage 
/// used for tracking branch history must be finite. 
/// 
/// This should cause the last branch to be correctly predicted only ~50% of 
/// the time (effectively the same as a random guess). 
///
/// Results
/// =======
///
/// Misprediction rate increases to ~50% after 90 padding branches?
///
/// This probably reflects one (or both?) of the following things: 
/// - We've filled up some [global] history register with outcomes
/// - We've created aliasing in some table of tracked branches
///
/// .. although, this test doesn't tell us exactly which of these is the case. 
///
/// The predictors are probably sensitive to the exact target address and 
/// program counter of each branch, and it doesn't help that the placement of 
/// our emitted code in memory (with [`X64Assembler`]) isn't guaranteed to be 
/// the same every time. 
///
pub struct CorrelatedBranches;
impl CorrelatedBranches {
    fn emit(num_padding: usize) -> X64Assembler {
        let mut f = X64Assembler::new().unwrap();

        // Set up the flags for when we execute the conditional branches. 
        // We expect RDI to be a *random* value (either 0 or 1). 
        dynasm!(f
            ; cmp rdi, 1
        );

        // Flush global history with [unconditional] taken branches.
        for _ in 0..2048 {
            dynasm!(f
                ; jmp >wow
                ; wow:
            );
        }
        f.emit_lfence();

        // This branch is *always* predicted locally [assuming that we've really
        // cleared the state of global history before this] and it should not be
        // correlated with the outcome of a previous branch? 
        //
        // NOTE: You can verify this by wrapping this block with RDPMC and 
        // observing that the misprediction rate is always 50%.
        dynasm!(f
            ; je >foo
            ; foo:
        );

        // Emit a variable number of unconditional padding branches.
        for i in 0..num_padding {
            if i == 0 {
                continue;
            } else {
                dynasm!(f ; jmp >wow ; wow:);
            }
        }

        f.emit_rdpmc_start(0, Gpr::R15 as u8);
        dynasm!(f
            ; je >bar
            ; bar:
        );
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {
        let event = Zen2Event::ExRetBrnMisp(0x00);
        let desc = event.as_desc();
        for num_padding in 0..=128 {
            let f = Self::emit(num_padding);
            let buf = f.finalize().unwrap();
            let ptr = buf.ptr(AssemblyOffset(0));
            let func: MeasuredFn = unsafe { std::mem::transmute(ptr) };

            let results = harness.measure(func,
                desc.id(), desc.mask(), 2048, 
                InputMethod::Random(&|rng, _| { 
                    (rng.gen::<bool>() as usize, 0) 
                }),
            ).unwrap();

            let dist = results.get_distribution();
            let min = results.get_min();
            let max = results.get_max();
            println!("padding={:03} min={} max={} dist={:?}", 
                num_padding, min,max,dist);
        }
    }
}


/// Describes the requested placement of a branch instruction. 
#[derive(Clone)]
pub struct Args {
    padding: Vec<BranchDesc>,
    test_brn: BranchDesc,
}

/// A region of code containing only a branch instruction. 
///
pub struct DirectBranchRegion {
    branch: BranchDesc,
    f: X64AssemblerFixed,
}
impl DirectBranchRegion {
    // "JMP with a 32-bit signed offset" is encoded in 5 bytes
    const JMP_ENC_LEN: usize = 5;
    pub fn new(branch: BranchDesc) -> Self { 
        assert!(branch.offset() >= 0x4000);
        let mut f = X64AssemblerFixed::new(
            branch.addr, 
            0x0000_0000_0000_4000
        );

        let offset = branch.offset() - Self::JMP_ENC_LEN;
        assert!(offset <= 0x7fff_ffff);
        dynasm!(f
            ; jmp offset as i32
        );

        f.commit().unwrap();
        //f.disas(AssemblyOffset(0), None);
        Self { branch, f }
    }
}

pub struct ConditionalBranch;
impl ConditionalBranch {
    fn align_down(addr: usize, bits: usize) -> usize {
        let align: usize = (1 << bits);
        let mask: usize  = !(align - 1);
        (addr & mask).wrapping_sub(align)
    }

    pub fn emit(args: Args) -> X64AssemblerFixed {
        //assert!(brn_addr < 0x0000_7000_0000_0000);
        //assert!(tgt_addr > brn_addr);

        // Compute the base address of emitted code (default to 0xffff_0000).
        let base_addr = if args.padding.len() != 0 {
            let first_brn = &args.padding[0];
            Self::align_down(first_brn.addr, 16)
        } else {
            0x0_ffff_0000
        };

        let mut asm = X64AssemblerFixed::new(
            base_addr, 
            0x0000_0001_0000_0000
        );

        //println!("[*] Starting at {:016x}", asm.cur_addr());

        for brn in args.padding.iter() {
            let tgt = asm.new_dynamic_label();

            asm.pad_until(brn.addr);
            assert_eq!(asm.cur_addr(), brn.addr);
            if brn.offset() < 128 {
                dynasm!(asm ; jmp BYTE =>tgt);
            } else {
                dynasm!(asm ; jmp =>tgt);
            }
            asm.pad_until(brn.tgt);
            asm.place_dynamic_label(tgt);
            assert_eq!(asm.cur_addr(), brn.tgt);
        }

        asm.pad_until(args.test_brn.addr - 0x18);
        asm.emit_rdpmc_start(0, Gpr::R15 as u8);
        assert_eq!(asm.cur_addr(), args.test_brn.addr);

        let tgt = asm.new_dynamic_label();
        if args.test_brn.offset() < 128 {
            dynasm!(asm ; je BYTE =>tgt);
        } else {
            dynasm!(asm ; je =>tgt);
        }
        asm.pad_until(args.test_brn.tgt);
        asm.place_dynamic_label(tgt);
        assert_eq!(asm.cur_addr(), args.test_brn.tgt);

        asm.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        asm.emit_ret();
        asm.commit().unwrap();
        asm
    }

    pub fn measure(
        harness: &mut PerfectHarness,
        args: &Args,
    ) -> bool
    {
        println!("[*] test_brn={:016x} pad_brn={:016x}", 
            args.test_brn.addr,
            args.padding[0].addr,
        );
        let asm = Self::emit(args.clone());
        //asm.disas(AssemblyOffset(0), None);
        let func = asm.as_fn();
        let event = Zen2Event::ExRetBrnMisp(0x00);
        let desc = event.as_desc();
        let results = harness.measure(func,
            desc.id(), desc.mask(), 256, 
            InputMethod::Random(&|rng, _| { 
                (rng.gen::<bool>() as usize, 0) 
            }),
        ).unwrap();

        let dist = results.get_distribution();
        let min = results.get_min();
        let max = results.get_max();
        println!("    {:03x}:{:02x} {:032} min={} max={} dist={:?}",
            desc.id(), desc.mask(), desc.name(), min, max, dist);
        min == 1
    }

    pub fn run(harness: &mut PerfectHarness) {
        let mut cases: Vec<Args> = Vec::new();
        for test_idx in 0..=1 {
            let test_addr: usize = if test_idx == 0 {
                0x1_1000_0000
            } else {
                0x1_1000_0000 | (1 << test_idx)
            };
            let test_tgt: usize = test_addr + 2;
            let test_brn = BranchDesc::new(test_addr, test_tgt);

            for pad_idx in 0..=23 {
                let pad_addr: usize = if pad_idx == 0 {
                    0x1_0000_0000
                } else {
                    0x1_0000_0000 | (1 << pad_idx)
                };
                let pad_tgt: usize = 0x1_0100_0000;
                let pad_brn = BranchDesc::new(pad_addr, pad_tgt);
                let mut padding = Vec::new();
                padding.push(pad_brn);
                cases.push(Args { padding, test_brn });
            }
        }

        let mut res: BTreeMap<usize, BTreeMap<usize, bool>> = BTreeMap::new();
        for arg in cases.iter() {
            let miss = Self::measure(harness, arg);
            if let Some(map) = res.get_mut(&arg.test_brn.addr) {
                map.insert(arg.padding[0].addr, miss);
            } else {
                res.insert(arg.test_brn.addr, BTreeMap::new());
            }
        }

        for (test_addr, map) in res.iter() {
            println!("[*] test_brn={:016X}", test_addr);
            for (pad_addr, miss) in map.iter() {
                println!("      pad_brn={:016X} miss?={}", pad_addr, miss);
            }
        }

    }
}




