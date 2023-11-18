use perfect::*;
use perfect::codegen::*;
use perfect::zen2::*;
use std::collections::*;

fn emit_test(mut emitter: impl FnMut(&mut PerfectFn)) -> PerfectFn {
    let mut f = PerfectFn::new("test");

    for _ in 0..1024 {
        dynasm!(f.asm
            ; jmp >wow
            ; wow:
        );
    }

    f.emit_rdpmc_start(1, Gpr::R15 as u8);
    emitter(&mut f);
    f.emit_rdpmc_end(1, Gpr::R15 as u8, Gpr::Rax as u8);
    f.emit_ret();
    f.commit();
    f.disas();
    println!();
    f
}

fn random_input(rng: &mut ThreadRng) -> (usize, usize) {
    let dir: bool = rng.gen();
    (dir as usize, 0)
}

/// Two branches with a shared, random outcome. 
/// Correlation not preserved after 90 taken branches?
fn emit_correlated_branches(f: &mut PerfectFn) {
    dynasm!(f.asm
        ; cmp rdi, 1
    );
    dynasm!(f.asm
        ; je >foo
        ; foo:
    );
    for _ in 1..=90 {
        dynasm!(f.asm
            ; jmp >wow
            ; wow:
        );
    }
    dynasm!(f.asm
        ; je >bar
        ; bar:
    );
}

fn main() {
    pin_to_core(15);

    let mut arena = mmap_fixed(0, 0x8000_0000);
    let mut rng = thread_rng();
    let mut emap = Zen2EventMap::new();

    // Emit harness and measure the function
    let mut harness = PerfectHarness::new()
        .set_dump_gpr(false)
        .emit();

    let mut floor = PerfectFn::new("floor");
    floor.emit_rdpmc_start(1, Gpr::R15 as u8);
    floor.emit_rdpmc_end(1, Gpr::R15 as u8, Gpr::Rax as u8);
    floor.emit_ret();
    floor.commit();
    floor.disas();
    println!();

    let mut events = EventSet::new();
    events.add_event_nomask(0xc3);

    let mut floor_res = BTreeMap::new();
    for (event, umask) in events.iter() {
        let (results, _) = harness.measure(&mut floor, 
            *event, *umask, 512, 0, 0
        ).unwrap();
        let dist = get_distribution(&results);
        let min = *results.iter().min().unwrap() as i64;
        let max = *results.iter().max().unwrap() as i64;
        floor_res.insert((event, umask), (min, max, dist));
    }

    let mut f = emit_test(emit_correlated_branches);

    for (event, umask) in events.iter() {
        let event_name = if let Some(desc) = emap.lookup(*event) {
            desc.name.to_string()
        } else { format!("unk_{:03x}", event) };

        let (results, _) = harness.measure_vary(&mut f, 
            *event, *umask, 512, random_input,
        ).unwrap();


        let dist = get_distribution(&results);
        let min = *results.iter().min().unwrap() as i64;
        let max = *results.iter().max().unwrap() as i64;

        let (floor_min, floor_max, floor_dist) = floor_res.get(&(event,umask))
            .unwrap();

        if max == 0 && *floor_max == 0 { 
            continue;
        }

        println!("{:03x}:{:02x} {:032}", event, umask, event_name);
        println!("\tfloor min={} max={} dist={:?}", 
            floor_min,floor_max,floor_dist);
        println!("\tmin={} max={} dist={:?}", min,max,dist);
    }

}

