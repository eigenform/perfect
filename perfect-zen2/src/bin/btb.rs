use perfect::*;
use perfect::events::*;
use perfect::stats::*;
use perfect::ir::branch::*;
use perfect::util::*;
use itertools::*;
use std::collections::*;

fn main() {
    let mut harness = HarnessConfig::default_zen2()
        .no_arena_alloc()
        .emit();
    BTBCapacity::run(&mut harness);
}



#[derive(Clone, Copy, Debug)]
pub struct BTBCapacityArgs {
    start_addr: usize,
    num_padding: usize,
    align: Align,
    test_offset: usize,
}
impl BTBCapacityArgs {
    pub fn test_addr(&self) -> usize { 
        self.start_addr + 
        (self.num_padding * self.align.value()) + 
        self.align.value() +
        self.test_offset
    }
}


/// Determine BTB capacity.
///
/// Context
/// =======
///
/// Test
/// ====
///
/// 1. Ensure that the BTB is polluted in the harness. 
///
///
/// Results
/// =======
///

pub struct BTBCapacity;
impl BTBCapacity {

    /// Emit a gadget used to measure our test code.
    fn emit_measure() -> X64AssemblerFixed {
        let mut f = X64AssemblerFixed::new(
            0x0000_1000_0000_0000,
            0x0000_0000_0000_1000,
        );
        f.emit_rdpmc_start(0, Gpr::R15 as u8);
        dynasm!(f ; call rsi);
        f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
        f.emit_ret();
        f.commit().unwrap();
        f.commit().unwrap();
        f
    }

    fn emit_empty_test() -> X64AssemblerFixed { 
        let mut f = X64AssemblerFixed::new(
            0x0000_2000_0000_0000,
            0x0000_0000_0000_1000
        );
        f.emit_ret();
        f.commit().unwrap();
        f
    }

    /// Emit a test. 
    fn emit_test(brn_addr: usize) -> X64AssemblerFixed {
        assert!(brn_addr < 0x0000_7fff_ffff_ffff);

        let mut f = X64AssemblerFixed::new(
            0x0000_0000_0000_0000,
            0x0000_0000_0008_0000
        );

        dynasm!(f
            ; jmp ->tgt
            ; ->tgt:
        );

        f.emit_ret();
        f.commit().unwrap();
        f
    }

    fn run(harness: &mut PerfectHarness) {
        //let event = Zen2Event::ExRetBrnMisp(0x00);
        let mut events = EventSet::new();
        events.add_list(&[
            //Zen2Event::BpRedirect(BpRedirectMask::Unk(0x01)),
            //Zen2Event::BpL0BTBHit(0x00),
            Zen2Event::BpL1BTBCorrect(0x00),
            //Zen2Event::BpL2BTBCorrect(0x00),
            Zen2Event::BpDeReDirect(0x01),
            //Zen2Event::ExRetBrnMisp(0x00),
            Zen2Event::ExRetBrnIndMisp(0x00),
            //Zen2Event::ExRetBrnTakenMisp(0x00),
            //Zen2Event::ExRetBrn(0x00),
            //Zen2Event::ExRetMsprdBrnchInstrDirMsmtch(0x00),
            //Zen2Event::Bp1RetBrUncondMisp(0x00),
        ]);

        let measure = Self::emit_measure();
        let measure_fn = measure.as_fn();

        let empty_test = Self::emit_empty_test();
        let empty_fn = empty_test.as_fn();

        let test = Self::emit_test(1);
        let test_fn = test.as_fn();

        for _ in 0..128 { 
            harness.call(empty_fn as usize, 0, measure_fn);
        }

        let results = harness.measure_events(measure_fn,
            &events, 512, 
            InputMethod::Fixed(empty_fn as usize, 0),
        ).unwrap();

        for result in results {
            let dist = result.get_distribution();
            let min = result.get_min();
            let max = result.get_max();
            println!("{:03x}:{:02x} ({:36}): min={:5} max={:5}",
                result.event_id(), result.event_mask(), result.event.name(),
                min, max,
            );
        }
        println!();
    }

}


