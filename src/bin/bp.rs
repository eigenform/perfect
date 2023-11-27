use perfect::*;
use perfect::codegen::*;
use perfect::zen2::*;
use std::collections::*;

fn random_input(rng: &mut ThreadRng) -> (usize, usize) {
    let dir: bool = rng.gen();
    (dir as usize, 0)
}

/// Two branches with a shared, random outcome. 
/// Correlation not preserved after 90 taken branches?
fn emit_correlated_branches(num_padding: usize) -> PerfectFn {
    let mut f = PerfectFn::new("test");

    // I guess this isn't really necessary, but whatever
    for _ in 0..1024 {
        dynasm!(f.asm
            ; jmp >wow
            ; wow:
        );
    }

    // We expect RDI to be a *random* value (either 0 or 1)
    dynasm!(f.asm
        ; cmp rdi, 1
    );

    // This branch is *always* predicted locally [assuming that we've really
    // cleared the state of global history before this] and it should not be
    // correlated with the outcome of a previous branch. 
    //
    // You can verify this by wrapping this block with RDPMC and observing 
    // that the misprediction rate is always 50%. (Incidentally, this always
    // seems to be the case regardless of whether or not we insert any number 
    // of unconditional padding branches beforehand?)

    dynasm!(f.asm
        ; je >foo
        ; foo:
    );

    // A variable number of unconditional padding branches.
    for _ in 0..num_padding {
        dynasm!(f.asm
            ; jmp >wow
            ; wow:
        );
    }

    // We only care about measuring the misprediction rate for this branch.
    // After a certain number of padding branches, we expect that the machine
    // will not be able to keep track of the correlation between this branch
    // and the first branch.

    f.emit_rdpmc_start(1, Gpr::R15 as u8);
    dynasm!(f.asm
        ; je >bar
        ; bar:
    );
    f.emit_rdpmc_end(1, Gpr::R15 as u8, Gpr::Rax as u8);

    f.emit_ret();
    f.commit();
    f
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

    let mut events = EventSet::new();
    events.add_event_nomask(0xc3);

    for num_padding in 0..=100 {

        let mut f = emit_correlated_branches(num_padding);

        for (event, umask) in events.iter() {
            let event_name = if let Some(desc) = emap.lookup(*event) {
                desc.name.to_string()
            } else { format!("unk_{:03x}", event) };

            let (results, _) = harness.measure_vary(&mut f, 
                *event, *umask, 1024, random_input,
            ).unwrap();


            let dist = get_distribution(&results);
            let min = *results.iter().min().unwrap() as i64;
            let max = *results.iter().max().unwrap() as i64;

            //println!("{:03x}:{:02x} {:032}", event, umask, event_name);
            println!("\tpad={:03} min={} max={} dist={:?}", 
                num_padding, min,max,dist);
        }

    }

}

