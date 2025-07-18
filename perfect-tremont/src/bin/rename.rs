use perfect::*;
use perfect::events::*;
//use rand::prelude::*;


pub struct ZeroIdiomElim;
impl MispredictedReturnTemplate<usize> for ZeroIdiomElim {}
impl ZeroIdiomElim { 
    pub fn emit_body(f: &mut X64Assembler, i: usize) {
        for _ in 0..=i {
            dynasm!(f ; xor rax, rax);
        }
    }
    pub fn run(harness: &mut PerfectHarness) {
        let mut events = EventSet::new();
        events.add(TremontEvent::TopdownBeBound(TopdownBeBoundMask::AllocRestrictions));
        events.add(TremontEvent::TopdownBeBound(TopdownBeBoundMask::Register));
        events.add(TremontEvent::TopdownBeBound(TopdownBeBoundMask::NonMemScheduler));
        events.add(TremontEvent::TopdownBeBound(TopdownBeBoundMask::MemScheduler));
        events.add(TremontEvent::TopdownBeBound(TopdownBeBoundMask::ReorderBuffer));
        events.add(TremontEvent::TopdownBeBound(TopdownBeBoundMask::Serialization));

        let opts = MispredictedReturnOptions::tremont_defaults()
            .rdpmc_strat(RdpmcStrategy::Gpr(Gpr::R15));

        'top: for i in 0..=256 {
            let asm = Self::emit(opts, i, Self::emit_body);
            let asm_reader = asm.reader();
            let asm_tgt_buf = asm_reader.lock();
            let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
            let asm_fn: MeasuredFn = unsafe { 
                std::mem::transmute(asm_tgt_ptr)
            };

            println!("[*] num_alc={}", i);
            for event in events.iter() {
                let desc = event.as_desc();
                let results = harness.measure(asm_fn, 
                    &desc, 64, InputMethod::Fixed(0, 0)
                ).unwrap();
                let dist = results.get_distribution();
                let min = results.get_min();
                let max = results.get_max();

                //println!("    {:03x}:{:02x} {:032} min={} max={} dist={:?}", 
                //    event.id(), event.mask(), event.name(), min, max, dist);
                
                println!("    {:03x}:{:02x} {:032} min={} max={}",
                    desc.id(), desc.mask(), desc.name(), min, max);


                //if event.id() == 0xcd && max == 0 {
                //    break 'top;
                //}
            }
        }

    }
}

fn main() {
    let mut harness = HarnessConfig::default_tremont()
        .pinned_core(Some(3))
        .arena_alloc(0, 0x1000_0000)
        .emit();
    ZeroIdiomElim::run(&mut harness);
}

