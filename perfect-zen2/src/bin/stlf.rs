//! Store-to-load forwarding

use perfect::*;
use perfect::events::*;
use rand::prelude::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2().emit();
    StlfStoreQueueCapacity::run(&mut harness);
    StlfEligibility::run(&mut harness);
}

/// Observe the effects of store queue pressure on store-to-load forwarding
/// (STLF) eligibility.
///
/// Explanation
/// ===========
///
/// STLF relies on the store queue, which has limited capacity.
/// The Family 17h SOG mentions that the store queue capacity is 48 entries. 
/// When a store is pushed out of the queue, we cannot forward the value to
/// a dependent load. 
///
/// Test
/// ====
///
/// Place a variable number of padding stores in-between a store and load 
/// that we expect will be subject to forwarding. After a certain number of
/// padding stores, we expect that the original store will be evicted from
/// the store queue [and that an STLF event will *never* occur].
///
/// Results
/// =======
///
/// 1. STLF events only occur *reliably* with 39 padding stores (40 in-flight).
///
/// 2. STLF events only occur once *during the first test iteration* when
///    there are more than 40 in-flight stores. (I'm not sure why?)
///
/// 3. After 48 in-flight stores, an STLF event *never* occurs. 
///    This is the capacity of the store queue.
///
pub struct StlfStoreQueueCapacity;
impl StlfStoreQueueCapacity {
    fn emit(num_stores: usize) -> X64Assembler {
        let mut rng = rand::thread_rng();
        let mut f = X64Assembler::new().unwrap();

        // Random addresses for padding stores
        let mut addrs: Vec<i32> = (0x0001_0008..=0x0001_0ff8)
            .step_by(8).collect();
        assert!(num_stores < addrs.len());
        addrs.shuffle(&mut rng);

        dynasm!(f
            ; mov rax, 0xdead_beef
            ; mov rbx, 0x0001_0000
            ; sfence
            ; lfence
            ; .align 4096
        );

        f.emit_rdpmc_start(1, Gpr::R15 as u8);

        // Store we expect to be forwarded
        dynasm!(f ; mov [rbx], rax );

        // Padding stores
        for store_num in 0..num_stores { 
            let addr = addrs[store_num];
            dynasm!(f ; mov [addr], rax );
            //dynasm!(f ; mov rcx, addr as _ );
            //dynasm!(f ; mov [rcx], rax);
        }

        // Target load whose result we expect to be forwarded
        dynasm!(f ; mov rax, [0x0001_0000]);

        f.emit_rdpmc_end(1, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    pub fn run(harness: &mut PerfectHarness) {
        println!("[*] STLF store queue pressure");
        let mut events = EventSet::new();
        events.add(Zen2Event::LsSTLF(0x00));
        events.add(Zen2Event::LsDispatch(LsDispatchMask::StDispatch));

        for num_stores in 0..=49 {
            let asm = Self::emit(num_stores);
            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };

            println!("  Padding stores: {}", num_stores);
            for event in events.iter() {
                let desc = event.as_desc();
                let results = harness.measure(asm_fn, 
                    desc.id(), desc.mask(), 512, InputMethod::Fixed(0, 0)
                ).unwrap();

                let dist = results.get_distribution();
                let min = results.get_min();
                let max = results.get_max();
                println!("    {:03x}:{:02x} {:032} min={} max={} dist={:?}", 
                    desc.id(), desc.mask(), desc.name(), min, max, dist);
            }
        }
        println!();
    }
}


/// Determine which displacement bits are relevant for STLF eligibility.
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
/// Perform a store and load where [most of] the displacement bits are zero.
/// In-between the target store and load, perform another store where some 
/// bit is set in the displacement. If this store is aliasing with the
/// original store, we expect that STLF will *not* occur.
///
/// Results
/// =======
///
/// Displacement bits [11:0] (0xfff) are used for STLF eligibility.
/// An STLF event is only observed when the padding store has one of the low 
/// 12 bits set (in this example, 0x0001_0000 is aliasing with 0x0001_1000).
///
pub struct StlfEligibility;
impl StlfEligibility {

    fn emit(disp: usize) -> X64Assembler {
        let mut f = X64Assembler::new().unwrap();
        let addr = (0x0001_0000 | (disp & 0xffff)) as i32;

        dynasm!(f 
            ; mov rax, 0xdeadbeef
            ; sfence
            ; lfence
        );
        f.emit_rdpmc_start(1, Gpr::R15 as u8);

        dynasm!(f 
            ; mov [0x0001_0000], al // Store
            ; mov [addr], al        // Potentially aliasing store
            ; mov bl, [0x0001_0000] // Load
        );

        f.emit_rdpmc_end(1, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    pub fn run(harness: &mut PerfectHarness) {
        println!("[*] STLF eligibility");
        let event = Zen2Event::LsSTLF(0x00);
        let desc = event.as_desc();

        for bit in 0..=15 {
            let disp = (1 << bit); 
            let asm = Self::emit(disp);
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
            println!("  Bit: {:02} ({:08x})", bit, 0x0001_0000 | disp);
            println!("    {:03x}:{:02x} {:032} min={} max={} dist={:?}", 
                desc.id(), desc.mask(), desc.name(), min, max, dist);
        }
        println!();
    }

}


