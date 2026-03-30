use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use perfect::ir::branch::*;
use perfect::util::*;
use itertools::*;
use std::collections::*;
use perfect::uarch::btb::*;

const USERSPACE_RANGE: std::ops::Range<usize> = {
    0x0000_0000_0000..0x0000_7fff_ffff_ffff
};
const KIND: [IndBrn; 4] = [ 
    IndBrn::CallMem, IndBrn::CallReg, IndBrn::JmpMem, IndBrn::JmpReg
];


fn main() {
    PerfectEnv::pin_to_core(15);
    let mut rng = thread_rng();

    // Test different indirect branches. 
    // FIXME: This will sometimes fail if we generate addresses that are 
    // invalid (or aliasing with some other existing allocation).
    for brn_kind in KIND {
        let arg_aliasing = BTBAliasArgs { 
            desc: "aliasing",
            victim_addr: rng.gen_range(USERSPACE_RANGE) & !0x1f,
            attacker_addr: None,
            kind: brn_kind,
        };

        let arg_nonaliasing = BTBAliasArgs { 
            desc: "non-aliasing",
            victim_addr: rng.gen_range(USERSPACE_RANGE) & !0x1f,
            attacker_addr: Some(rng.gen_range(USERSPACE_RANGE) & !0x1f),
            kind: brn_kind,
        };

        BTBAliasing::run(arg_nonaliasing);
        BTBAliasing::run(arg_aliasing);
    }
}


#[derive(Clone, Copy, Debug)]
pub enum IndBrn { 
    /// CALL (register operand)
    CallReg, 
    /// CALL (memory operand)
    CallMem,
    /// JMP (register operand)
    JmpReg,
    /// JMP (memory operand)
    JmpMem,
}

pub struct BTBAliasTest { 
    /// Containing the victim indirect branch
    victim: X64AssemblerFixed,
    /// Containing the attacker's indirect branch
    attacker: X64AssemblerFixed,

    /// Target for the victim's indirect branch
    victim_tgt: X64AssemblerFixed,
    /// Target for the attackers's indirect branch
    attacker_tgt: X64AssemblerFixed,
}

pub struct BTBAliasArgs { 
    desc: &'static str,
    /// Victim indirect branch address
    victim_addr: usize,
    /// Attacker indirect branch address
    attacker_addr: Option<usize>,
    /// Branch type/encoding
    kind: IndBrn,
}


/// Demonstrate the effects of aliasing BTB entries. 
///
/// Context
/// =======
///
/// A "branch target buffer" (BTB) is a structure that maintains information
/// about previously-resolved branches and their target addresses. 
///
/// When encountering a branch in the instruction stream, modern machines
/// attempt to predict the target address by trying to find a BTB entry for
/// the branch. In order to distinguish a particular branch, implementations
/// typically mix together the current program counter with information about 
/// recent control-flow (ie. the addresses and/or targets of previous 
/// recently-taken branches). The resulting bits are then used as an index to 
/// access the BTB. 
///
/// In general, it's possible that an implementation *cannot* distinguish 
/// between certain branches, and that distinct branches may be aliasing 
/// with one another in the BTB. 
///
/// "Branch Target Injection" (BTI)
/// ===============================
///
/// This is the idea behind "branch target injection" (BTI) attacks
/// (which are sometimes also called "Spectre Variant 2"). 
///
/// If an attacker can intentionally create a branch which is aliasing in the 
/// BTB with a victim branch, the attacker may be able to cause the victim 
/// branch to suffer mispredictions. Since the attacker controls the target 
/// address of the aliasing branch, the victim branch may suffer incorrect 
/// speculation at a target chosen by the attacker.  
///
/// Creating Aliasing BTB Entries
/// =============================
///
/// In our case, the RETBLEED paper[^1] describes the hash function for Zen 2 
/// and Zen 3 parts. This uses the program counter of the branch to create a 
/// 12-bit BTB index (see [`perfect::uarch::btb::ZEN2_BTB_INDEX_FN`]). 
///
/// [^1]: [RETBLEED: Arbitrary Speculative Code Execution with Return Instructions](https://comsec.ethz.ch/wp-content/files/retbleed_sec22.pdf)
///
/// Test
/// ====
///
/// This setup consists of the following parts: 
///
/// - A "victim" indirect branch, and the victim's target
/// - An "attacker" indirect branch, and the attacker's target
/// - A "probe" cacheline used to determine whether or not the victim's 
///   indirect branch was mispredicted 
///
/// In order to distinguish cases where a misprediction occurs, we rely on the
/// fact that a load instruction in the attacker's target may affect the state 
/// of the L1D cache. 
///
/// In this case, [`Self::PROBE_ADDR`] is the address of a cacheline used to 
/// make the distinction. This address is propagated into the victim's target, 
/// which *does not* perform any load. After the test, we measure a load to 
/// determine whether or not incorrect speculation occurred. 
///
/// 1. Train the predictor by performing the attacker's indirect branch with 
///    the attacker's target (while avoiding a load to the probe cacheline).
///
/// 2. Explicitly flush the probe cacheline. 
///
/// 3. Perform the victim's indirect branch with the victim's target
///    (while passing the address of the probe cacheline).
///
/// 4. Use RDPRU and APERF to measure a load from the probe cacheline. 
///
/// If we observe that the load to the probe cacheline completed quickly, we 
/// expect that the following has occurred:
///
/// 1. The victim's branch was mispredicted using the BTB entry associated 
///    with the attacker's branch, and incorrect speculation occured at
///    'attacker_tgt'
///
/// 2. The address of the probe cacheline passed to the victim's branch was 
///    propagated to the load instruction in the attacker's target.
///
/// 3. The probe cacheline was [speculatively, incorrectly] brought into the 
///    L1D cache before the branch misprediction was resolved. 
///
pub struct BTBAliasing;
impl BTBAliasing {

    /// Emit some kind of indirect branch at virtual address `addr`. 
    fn emit_indir_brn(addr: usize, kind: IndBrn) -> X64AssemblerFixed {
        let base = addr & 0xffff_ffff_ffff_e000;
        let mut f = X64AssemblerFixed::new(
            base,
            0x0000_0000_0000_4000,
        );
        f.emit_push_nonvolatile_gprs();

        // If we're using a memory operand, use a non-temporal store to 
        // write the target address into memory somewhere. 
        // This should expose the branch to extreme latency. 
        match kind { 
            IndBrn::CallMem | IndBrn::JmpMem => {
                dynasm!(f
                    ; mov r15, QWORD Self::TGT_STORAGE as i64
                    ; movnti QWORD [r15], rdi
                );
            },
            _ => {},
        }

        // Pad to the requested address, emit the requested indirect branch
        f.pad_until(addr);
        match kind { 
            IndBrn::JmpMem => {
                dynasm!(f ; jmp QWORD [r15]);
            },
            IndBrn::CallMem => { 
                dynasm!(f ; call QWORD [r15]);
            },
            IndBrn::JmpReg => { 
                dynasm!(f ; jmp rdi);
            },
            IndBrn::CallReg => { 
                dynasm!(f ; call rdi);
            },
        }

        f.emit_nop_sled(2048);

        // If we're testing a CALL instruction, emit RET. 
        // Otherwise, for JMP instructions, we expect the branch targets will 
        // contain RET and return immediately to our caller. 
        match kind { 
            IndBrn::CallReg | IndBrn::CallMem => { 
                f.emit_pop_nonvolatile_gprs();
                f.emit_ret();
                f.emit_lfence();
            },
            _ => {},
        }

        f.commit().unwrap();
        f
    }

    /// Emit a target function for an indirect branch. 
    fn emit_target(addr: usize, probe: bool, pop: bool) -> X64AssemblerFixed { 
        let mut f = X64AssemblerFixed::new(
            addr,
            0x0000_0000_0000_4000
        );

        // Load from RSI (which may or may not be the probed cacheline)
        if probe { 
            dynasm!(f ; mov rax, QWORD [rsi]);
        }

        f.emit_nop_sled(2048);

        // Restore nonvolatile GPRs before returning
        if pop { 
            f.emit_pop_nonvolatile_gprs();
        }

        f.emit_ret();
        f.emit_lfence();
        f.commit().unwrap();
        f
    }

    /// Emit the code for a particular test. 
    /// FIXME: For now, the target locations are fixed
    fn emit_test(
        victim_addr: usize,
        attacker_addr: usize,
        kind: IndBrn,
    ) -> BTBAliasTest {
        let victim = Self::emit_indir_brn(victim_addr, kind);
        let attacker = Self::emit_indir_brn(attacker_addr, kind);

        let (victim_tgt, attacker_tgt) = match kind { 
            IndBrn::CallReg | IndBrn::CallMem => { 
                (
                    Self::emit_target(0x2222_4000, false, false), 
                    Self::emit_target(0x2222_8000, true, false), 
                )
            },
            IndBrn::JmpReg | IndBrn::JmpMem => { 
                (
                    Self::emit_target(0x2223_4000, false, true), 
                    Self::emit_target(0x2223_8000, true, true), 
                )
            },
        };

        BTBAliasTest { 
            victim, attacker, victim_tgt, attacker_tgt
        }

    }

}

impl BTBAliasing {
    const PROBE_ADDR: usize = 0x1234_1234_1000;
    const TGT_STORAGE: usize = Self::PROBE_ADDR + 0x4c0;

    /// Generate a virtual address whose BTB index is colliding with the 
    /// BTB index for virtual address `addr`. 
    fn generate_collision_for(addr: usize) -> usize { 
        let mut rng = thread_rng();
        let coll = zen2_btb_collisions(addr, 1)
            .into_iter()
            .filter(|x| { 
                *x < 0x7fff_ffff_ffffusize && 
                *x != addr
            })
            .collect_vec();
        let alias = coll.choose(&mut rng).unwrap().clone();
        alias
    }

    /// Perform 'N' back-to-back indirect jumps. 
    #[inline(never)]
    fn clear_ghist_indir<const N: usize>() {
        unsafe { 
            core::arch::asm!(r#"
            .rept {cnt}
            lea rax, [rip + 2f]
            jmp rax
            2:
            .endr
            lfence
            "#, cnt = const N,
            out("rax") _,
            );
        }
    }

    /// Perform 'N' back-to-back indirect jumps. 
    #[inline(never)]
    fn clear_ghist_dir<const N: usize>() {
        unsafe { 
            core::arch::asm!(r#"
            .rept {cnt}
            jmp 2f
            2:
            .endr
            lfence
            "#, cnt = const N,
            );
        }
    }

    /// Flush the probe cacheline at [`Self::PROBE_ADDR`].
    #[inline(always)]
    fn flush_probe() { 
        unsafe { 
            let ptr = Self::PROBE_ADDR as *mut u8;
            core::arch::x86_64::_mm_clflush(ptr);
            core::arch::x86_64::_mm_mfence();
            core::arch::x86_64::_mm_lfence();
        }
    }
}

impl BTBAliasing {

    /// Run the test, then measure the probe cacheline and return the result
    fn run_test(
        victim_fn: MeasuredFn,
        attacker_fn: MeasuredFn,
        victim_tgt: usize, 
        attacker_tgt: usize,
    ) -> usize 
    {
        // NOTE: Try to make the state of the ITA consistent by taking some 
        // number of back-to-back indirect JMPs (?)
        //
        // Doing this seems to make the signal stronger and significantly more 
        // repeatable across test iterations. Note that this doesn't seem to be
        // the case if we use back-to-back direct JMPs here instead. 
        //
        // Presumably, apart from the fact that BTB entries might be aliasing
        // here, the indirect predictor also has some internal state, and is 
        // also probably very confused about the target address. When we 
        // predict incorrectly with an aliasing entry, presumably the ITA is
        // still being trained with the correct target (?)
        //
        // In any case, this seems to reliably untrain whatever effects have 
        // been left by the previous test iteration. 

        Self::clear_ghist_indir::<8192>();

        // Call the attacker, training the attacker's branch into the BTB
        // (while avoiding an access to the probe cacheline)
        (attacker_fn)(attacker_tgt, Self::PROBE_ADDR + 0x8c0);

        // Explicitly flush the probe cacheline
        Self::flush_probe();

        // Call the victim, invoke the victim's indirect branch. 
        (victim_fn)(victim_tgt, Self::PROBE_ADDR);

        // Measure a load to the probe cacheline
        let t0 = rdpru();
        unsafe { 
            core::ptr::read_volatile::<u64>(Self::PROBE_ADDR as _);
        }
        let t1 = rdpru();
        t1 - t0
    }

    /// Emit and run a test.
    ///
    /// If `attacker_call_addr` is not provided, generate an address which 
    /// is aliasing in the BTB with `victim_call_addr`. 
    fn run(arg: BTBAliasArgs) { 

        // Try to determine the noise floor for reading APERF with RDPRU. 
        let mut aperf_floor = usize::MAX;
        for _ in 0..1024 { 
            let t0 = rdpru();
            let t1 = rdpru();
            if (t1 - t0) < aperf_floor { 
                aperf_floor = t1 - t0;
            }
        }

        let attacker_addr = if let Some(x) = arg.attacker_addr { 
            x
        } else { 
            Self::generate_collision_for(arg.victim_addr)
        };

        let test = Self::emit_test(arg.victim_addr, attacker_addr, arg.kind);

        let mut results = RawResults(vec![0; 64]);
        let probe_base = PerfectEnv::mmap_fixed(Self::PROBE_ADDR, 0x1000);

        println!("[*] Testing with {:?} ({})", arg.kind, arg.desc);
        println!("  victim_brn:   {:016x}", arg.victim_addr);
        println!("  attacker_brn: {:016x}", attacker_addr);
        println!("  victim_tgt:   {:016x}", test.victim_tgt.ptr as usize);
        println!("  attacker_tgt: {:016x}", test.attacker_tgt.ptr as usize);

        // Run the test a couple of times
        for i in 0..64 { 
            let res = Self::run_test(
                test.victim.as_fn(),
                test.attacker.as_fn(),
                test.victim_tgt.ptr as _,
                test.attacker_tgt.ptr as _
            );
            results.0[i] = res - aperf_floor;
        }
        println!("  measurements:");
        for line in results.0.chunks(16) {
            println!("    {:?}", line);
        }
        println!();

    }

}


