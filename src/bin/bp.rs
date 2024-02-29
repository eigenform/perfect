use perfect::*;

/// Two branches with a shared, random outcome. 
/// Correlation not preserved after 90 taken branches?
fn emit_correlated_branches(num_padding: usize) -> X64Assembler {
    let mut f = X64Assembler::new().unwrap();

    // I guess this isn't really necessary, but whatever
    //for _ in 0..1024 {
    //    dynasm!(f
    //        ; jmp >wow
    //        ; wow:
    //    );
    //}

    // We expect RDI to be a *random* value (either 0 or 1)
    dynasm!(f
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

    dynasm!(f
        ; je >foo
        ; foo:
    );

    // A variable number of unconditional padding branches.
    for _ in 0..num_padding {
        dynasm!(f
            ; jmp >wow
            ; wow:
        );
    }

    // We only care about measuring the misprediction rate for this branch.
    // After a certain number of padding branches, we expect that the machine
    // will not be able to keep track of the correlation between this branch
    // and the first branch.

    f.emit_rdpmc_start(0, Gpr::R15 as u8);
    dynasm!(f
        ; je >bar
        ; bar:
    );
    f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);

    f.emit_ret();
    f.commit().unwrap();
    f
}

fn test_correlated_branches() {
    let mut harness = HarnessConfig::default().emit();

    for num_padding in 0..=100 {
        let f = emit_correlated_branches(num_padding);
        let buf = f.finalize().unwrap();
        let ptr = buf.ptr(AssemblyOffset(0));
        let func: MeasuredFn = unsafe { std::mem::transmute(ptr) };

        // Run with random branch outcomes.
        // Measuring mispredicted branches.
        let results = harness.measure(func,
            0xc3, 0x00, 1024, 
            InputMethod::Random(&|rng, iters| { 
                (rng.gen::<bool>() as usize, 0) 
            }),
        ).unwrap();

        let dist = results.get_distribution();
        let min = results.get_min();
        let max = results.get_max();
        println!("pad={:03} min={} max={} dist={:?}", 
            num_padding, min,max,dist);

    }
}

fn main() {
    PerfectEnv::pin_to_core(15);
    test_correlated_branches();
}

