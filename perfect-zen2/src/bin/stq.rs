
use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use perfect::asm::Emitter;

fn main() {
    let mut harness = HarnessConfig::default_zen2().emit();
    StoreQueueCapacity::run(&mut harness);
}

/// Create store queue pressure. 
///
/// Explanation
/// ===========
///
/// The store queue keeps track of the addresses/values of recent stores. 
/// The Family 17h SOG mentions that the store queue capacity is 48 entries. 
///
/// Test
/// ====
///
/// Execute many stores to different addresses and measure stall cycles 
/// (with 'StoreQueueRsrcStall'). When we exceed the store queue capacity,
/// the number of stall cycles will be nonzero. 
///
/// Results
/// =======
///
/// Stall cycles observed when we perform more than 48 stores. 
///
pub struct StoreQueueCapacity;
impl StoreQueueCapacity {
    pub fn emit(num_stores: usize) -> X64Assembler {
        let mut rng = rand::thread_rng();
        let mut f = X64Assembler::new().unwrap();

        // Generate random addresses for the stores
        let mut addrs: Vec<i32> = (0x0001_0000..=0x0001_1000)
            .step_by(64).collect();
        assert!(num_stores < addrs.len());
        addrs.shuffle(&mut rng);

        dynasm!(f
            ; mov rax, 0x1111_dead_1111_dead
            ; vmovq xmm0, rax
            ; vpbroadcastq ymm0, xmm0
            ; lfence
            ; .align 4096
            ; lfence
        );

        f.emit_rdpmc_start(1, Gpr::R15 as u8);

        // Insert stores.
        // The width of the store doesn't seem to matter. 
        for addr in &addrs[0..=num_stores] {
            dynasm!(f ; mov [*addr], rax ); // 8B
            //dynasm!(f ; mov [*addr], eax ); // 4B
            //dynasm!(f ; mov [*addr], ax ); // 2B
            //dynasm!(f ; mov [*addr], al ); // 1B
            //dynasm!(f ; movnti [*addr], rax );
            //dynasm!(f ; vmovd [*addr], xmm0); // 4B
            //dynasm!(f ; vmovq [*addr], xmm0); // 8B
            //dynasm!(f ; vmovdqa [*addr], xmm0); // 16B
            //dynasm!(f ; vmovdqa [*addr], ymm0); // 32B
        }
        dynasm!(f ; sfence);

        f.emit_rdpmc_end(1, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    pub fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(Zen2Event::LsDispatch(LsDispatchMask::StDispatch));
        events.add(Zen2Event::LsNotHaltedCyc(0x00));
        events.add(Zen2Event::DeDisDispatchTokenStalls1(
            DeDisDispatchTokenStalls1Mask::StoreQueueRsrcStall
        ));

        for num_stores in 0..=49 {
            let asm = Self::emit(num_stores);
            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };

            println!("[*] num_stores={}", num_stores);
            for event in events.iter() { 
                let desc = event.as_desc();
                let results = harness.measure(asm_fn, 
                    desc.id(), desc.mask(), 16, InputMethod::Fixed(0, 0),
                ).unwrap();

                let dist = results.get_distribution();
                let min = results.get_min();
                let max = results.get_max();
                println!("  {:03x}:{:02x} {:032} min={:3} max={:3} dist={:?}", 
                    desc.id(), desc.mask(), desc.name(), min, max, dist);
            }
        }
    }
}


