
use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use perfect::asm::Emitter;

/// Perform a variable number of stores. 
/// When we exceed the store queue capacity, we expect the number of stall 
/// cycles (from 'StoreQueueRsrcStall') to be non-zero. 
fn emit_stq_test(num_stores: usize) -> X64Assembler {
    let mut rng = rand::thread_rng();
    let mut f = X64Assembler::new().unwrap();

    // Generate some random addresses for stores
    let mut addrs: Vec<i32> = (0x0001_0008..=0x0001_0ff8)
        .step_by(8).collect();
    assert!(num_stores < addrs.len());
    addrs.shuffle(&mut rng);

    dynasm!(f
        ; mov rax, 0xdeadbeef
        ; sfence
        ; lfence
        ; .align 4096
        ; lfence
    );

    f.emit_rdpmc_start(1, Gpr::R15 as u8);
    // Insert stores
    for store_num in 0..num_stores { 
        let addr = addrs[store_num];
        dynasm!(f ; mov [addr], rax );
    }
    f.emit_rdpmc_end(1, Gpr::R15 as u8, Gpr::Rax as u8);
    f.emit_ret();
    f.commit().unwrap();
    f
}

fn main() {
    PerfectEnv::pin_to_core(15);
    let _ = PerfectEnv::mmap_fixed(0, 0x8000_0000);
    let mut harness = HarnessConfig::default().emit();
    let mut events = EventSet::new();
    events.add(Zen2Event::LsDispatch(LsDispatchMask::StDispatch));
    events.add(Zen2Event::DeDisDispatchTokenStalls1(
        DeDisDispatchTokenStalls1Mask::StoreQueueRsrcStall
    ));

    for num_stores in 0..=49 {
        let asm = emit_stq_test(num_stores);
        let asm_reader = asm.reader();
        let asm_tgt_buf = asm_reader.lock();
        let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
        let asm_fn: MeasuredFn = unsafe { std::mem::transmute(asm_tgt_ptr) };

        println!("[*] num_stores={}", num_stores);
        for event in events.iter() { 
            let results = harness.measure(
                asm_fn, event.id(), event.mask(), 16, InputMethod::Fixed(0, 0),
            ).unwrap();

            let dist = results.get_distribution();
            let min = results.get_min();
            let max = results.get_max();

            println!("  {:03x}:{:02x} {:032} min={:3} max={:3} dist={:?}", 
                event.id(), event.mask(), event.name(), min, max, dist);
        }
    }
}

