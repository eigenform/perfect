
use perfect::*;
use itertools::*;
use std::time::Instant;

fn emit_test(num_nops: usize, num_iters: usize) -> PerfectAsm {
    let mut asm = PerfectAsm::new(
        0x0000_1000_0000_0000, 
        0x0000_0000_0010_0000
    );

    // NOTE: When the NMI watchdog is disabled on this core for whatever 
    // reason, 'perf' will allocate the event to PMC0 (instead of PMC1). 
    asm.emit_rdpmc_start64(0, Gpr::R15 as u8);

    let label = asm.new_dynamic_label();
    dynasm!(asm
        ; mov rax, num_iters as i32
        ; =>label
    );
    asm.emit_nop_sled(num_nops);
    dynasm!(asm
        ; dec rax
        ; jnz =>label
    );
    asm.emit_rdpmc_end64(0, Gpr::R15 as u8, Gpr::Rax as u8);
    asm.emit_ret();
    asm.commit().unwrap();
    asm
}

// Spin in a loop and use the PMCs to listen for interrupts. 
// We want as much uninterrupted time on-core as we can manage. 
//
// When booting with 'isolcpus=' and 'nohz_full=', it seems like we can be
// scheduled for many billions of cycles.

fn main() {
    //PerfectEnv::pin_to_core(15);
    PerfectEnv::pin_to_cpuset();
    let cpu = nix::sched::sched_getcpu().unwrap();
    //std::thread::sleep(std::time::Duration::from_secs(1));
    println!("[*] Running on CPU{}", cpu);

    let mut harness = PerfectHarness::new().emit(HarnessConfig::default());

    let num_nops = (1 << 10);
    for num_iters in &[(1<<24)] {
        let s = Instant::now();
        let asm = emit_test(num_nops, *num_iters as usize);
        println!("Emitted in {:?}", s.elapsed());
        let asm_fn = asm.as_fn();

        // How many cycles does this take?
        let results = harness.measure(asm_fn, 0x76, 0x00, 1, 0, 0).unwrap();
        let cycles = results.get_max();

        // Listen for interrupts
        let runs = 1;
        let results = harness.measure(asm_fn, 0x2c, 0x01, runs, 0, 0).unwrap();
        let dist = results.get_distribution();
        let min  = results.get_min();
        let max  = results.get_max();
        println!("Loop params:  num_nops={}, iters={}, total_cycles={}", 
            num_nops, num_iters, cycles);
        println!("Observations: runs={} min={} max={} dist={:?}", 
            runs, min,max,dist);
        println!();

        //let list = results.find(1);
        //let list = results.filter(|x| x != 0);
        //for (x,y) in list.iter().tuple_windows() {
        //    println!("{}", y-x);
        //}

    }
}
