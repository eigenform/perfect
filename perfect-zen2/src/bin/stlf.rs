//! Store-to-load forwarding

use perfect::*;
use perfect::events::*;
use rand::prelude::*;

fn main() {
    let args = ExperimentArgs::parse();
    let mut harness = match HarnessConfig::from_cmdline_args(&args) {
        Some(cfg) => cfg.emit(),
        None => HarnessConfig::default_zen2().emit()
    };

    StlfStoreQueuePressure::run(&mut harness);
    StlfEligibility::run_imm(&mut harness);
    StlfEligibility::run_reg(&mut harness);
}

/// Observe the effects of store queue pressure on store-to-load forwarding
/// (STLF) eligibility.
///
/// Context
/// =======
///
/// STLF relies on an earlier matching store being present in the store queue. 
/// The Family 17h SOG mentions that the store queue capacity is 48 entries. 
/// The Family 19h SOG mentions that the store queue capacity is 64 entries. 
///
/// In Family 19h, STLF can also occur speculatively depending on the state 
/// of memory dependency predictors. 
///
/// Test
/// ====
///
/// 1. Create a store and load pair that we expect to be subject to forwarding.
///
/// 2. Place a variable number of *non-aliasing* stores in-between the pair.
///    (Note that 
///
/// 3. After a certain number of padding stores, we expect that the original 
///    store in the pair has been removed from the store queue [and that an 
///    STLF event will *never* occur].
///
/// Results
/// =======
///
/// On Zen 2 platforms: 
///
/// - STLF events only occur *reliably* with 39 padding stores
/// - STLF events only occur once *during the first test iteration* when
///   there are more than 40 in-flight stores
/// - After 48 in-flight stores, an STLF event *never* occurs. 
///
/// On Zen 3 platforms (with PSF and SSB disabled): 
///
/// - STLF events only occur *reliably* with ~16 padding stores. 
/// - STLF events only occur once *during the first test iteration* when
///   there are more than 16 in-flight stores.
/// - After 64 in-flight stores, an STLF event *never* occurs. 
///
pub struct StlfStoreQueuePressure;
impl StlfStoreQueuePressure {
    /// Address shared by the eligible store and load pair
    const STLF_ADDR: i32 = 0x0100_0000;

    fn emit(num_stores: usize) -> X64Assembler {
        let mut rng = rand::thread_rng();
        let mut f = X64Assembler::new().unwrap();

        // Random addresses for padding stores
        //let mut addrs: Vec<i32> = (0x0100_0008..=0x0100_0ff8)
        //    .step_by(8).collect();
        //assert!(num_stores < addrs.len());
        //addrs.shuffle(&mut rng);

        dynasm!(f
            ; mov rax, 0xdead_beef
            ; sfence
            ; lfence
            ; mfence
        );

        f.emit_rdpmc_start(0, Gpr::R15 as u8);

        // Store we expect to be forwarded
        dynasm!(f ; mov [Self::STLF_ADDR], rax );

        // Emit some number of repeated padding stores
        for store_num in 0..num_stores { 
            //let addr = addrs[store_num];
            dynasm!(f ; mov [0x0100_0040], rcx );
        }

        // Target load whose result we expect to be forwarded
        dynasm!(f ; mov rax, [Self::STLF_ADDR]);

        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    pub fn run(harness: &mut PerfectHarness) {
        println!("[*] STLF store queue pressure");
        let mut events = EventSet::new();
        events.add(Zen2Event::LsSTLF(0x00));
        //events.add(Zen2Event::LsDispatch(LsDispatchMask::StDispatch));

        for num_stores in 0..=68 {
            let asm = Self::emit(num_stores);
            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };

            //println!("  Padding stores: {}", num_stores);
            for event in events.iter() {
                let desc = event.as_desc();
                let results = harness.measure(asm_fn, 
                    desc.id(), desc.mask(), 512, InputMethod::Fixed(0, 0)
                ).unwrap();

                let dist = results.get_distribution();
                let min = results.get_min();
                let max = results.get_max();
                println!("  {:2} stores: {:03x}:{:02x} {} min={} max={} dist={:?}", 
                    num_stores,
                    desc.id(), desc.mask(), desc.name(), min, max, dist);
            }
        }
        println!();
    }
}


/// Determine which *displacement* bits are relevant for STLF eligibility.
///
/// Explanation
/// ===========
///
/// When a load occurs, STLF eligibility depends on whether or not a matching
/// store is present in the store queue. Matching is done with bits from the 
/// memory operands/addressing mode contained in the instruction encoding 
/// (ie. a base register, an immediate displacement, a scaling factor, etc).
///
/// At any point, the store queue *may* contain more than one matching entry.
/// In this case, STLF cannot occur because the memory operands themselves 
/// cannot be used to disambiguate the source value for the load. 
/// In this case, the machine must wait for the address to be resolved.
///
/// Test
/// ====
///
/// 1. Perform a store and load where [most of] the address bits are zero
///
/// 2. In-between the target store and load, perform another store where some 
///    bit is set. 
///
/// 3. If this store is aliasing with the original store, we expect that STLF 
///    will *not* occur.
///
/// Results
/// =======
///
/// For Zen 2: 
///
/// - An STLF event is reliably observed when the padding store has a set bit 
///   in the range [11:3] (implying that bits [11:3] are used to distinguish 
///   STLF eligibility)
///
///
pub struct StlfEligibility;
impl StlfEligibility {

    const STLF_ADDR: usize = 0x0100_0000;
    const MAX_BIT: usize = 23;

    /// Emit the test [with immediate addressing]
    fn emit_imm(disp: usize) -> X64Assembler {
        let mut f = X64Assembler::new().unwrap();
        let addr = (Self::STLF_ADDR | disp) as i32;

        dynasm!(f 
            ; mov rax, 0xdeadbeef
            ; sfence
            ; lfence
            ; mfence
        );
        f.emit_rdpmc_start(0, Gpr::R15 as u8);

        dynasm!(f 
            ; mov [Self::STLF_ADDR as _], rax
            ; mov [addr], rax
            ; mov rbx, [Self::STLF_ADDR as _]
        );

        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    /// Emit the test [with base register addressing]
    fn emit_reg() -> X64Assembler {
        let mut f = X64Assembler::new().unwrap();

        dynasm!(f 
            ; mov rax, 0xdeadbeef
            ; sfence
            ; lfence
            ; mfence
        );
        f.emit_rdpmc_start(0, Gpr::R15 as u8);

        dynasm!(f 
            ; mov [rdi], rax
            ; mov [rsi], rax
            ; mov rbx, [rdi]
        );

        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    pub fn run_reg(harness: &mut PerfectHarness) {
        println!("[*] STLF eligibility (base register addressing)");
        let event = Zen2Event::LsSTLF(0x00);
        let desc = event.as_desc();

        // Build arguments to the test (values of RDI and RSI). 
        // The first argument (RDI) is the address for the eligible store/load
        // pair, and the second argument (RSI) is the address for the 
        // [potentially-aliasing] padding store.
        let mut pairs = Vec::new();
        for bit in 0..=23 {
            pairs.push((0x0100_0000, 0x0100_0000 | (1<< bit)));
        }

        for (addr, alias_addr) in pairs {
            let asm = Self::emit_reg();
            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };
            let results = harness.measure(asm_fn, 
                desc.id(), desc.mask(), 512, 
                InputMethod::Fixed(addr, alias_addr)
            ).unwrap();

            let dist = results.get_distribution();
            let min = results.get_min();
            let max = results.get_max();
            if min == 0 && max == 0 { continue; }
            println!("  ({:08x}): [{:03x}:{:02x} {} min={} max={} dist={:?}]", 
                alias_addr, 
                desc.id(), desc.mask(), desc.name(), min, max, dist);
        }
        println!();
    }

    pub fn run_imm(harness: &mut PerfectHarness) {
        println!("[*] STLF eligibility (immediate addressing)");
        let event = Zen2Event::LsSTLF(0x00);
        let desc = event.as_desc();

        for bit in 0..=23 {
            let disp = (1 << bit); 
            let asm = Self::emit_imm(disp);
            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };
            let results = harness.measure(asm_fn, 
                desc.id(), desc.mask(), 512, InputMethod::Fixed(0, 0)
            ).unwrap();

            let dist = results.get_distribution();
            let min = results.get_min();
            let max = results.get_max();
            if min == 0 && max == 0 { continue; }
            println!("  Bit {:02} ({:08x}): [{:03x}:{:02x} {} min={} max={} dist={:?}]", 
                bit, (Self::STLF_ADDR | disp),
                desc.id(), desc.mask(), desc.name(), min, max, dist);
        }
        println!();
    }

}


