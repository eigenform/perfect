use perfect::*;
use perfect::zen2::*;
use perfect::util::disas;
use std::collections::*;

fn mysterious_loop(f: &mut X64Assembler) {
    dynasm!(f
        ; mov rcx, 0x4000
        ; mov QWORD [rsp+0x10], 0

        ; ->lp:
        ; add DWORD [rsp+0x10], 0x78563412
        ; loop ->lp
    );
}

fn mysterious_loop_nomem(f: &mut X64Assembler) {
    dynasm!(f
        ; mov rcx, 0x4000
        ; mov rbx, 0
        ; .align 64
        ; ->lp:
        ; add rbx, 0x78563412
        ; loop ->lp
    );
}


const TESTS: &[(&'static str, fn(&mut X64Assembler))] = &[
    ("Mysterious loop",    mysterious_loop),
    ("Mysterious loop (no RSP)", mysterious_loop_nomem),
];



fn emit_test(emit_content: impl Fn(&mut X64Assembler)) 
    -> X64Assembler 
{
    let mut f = X64Assembler::new().unwrap();
    f.emit_rdpmc_start64(1, Gpr::R15 as u8);
    emit_content(&mut f);
    f.emit_rdpmc_end64(1, Gpr::R15 as u8, Gpr::Rax as u8);
    f.emit_ret();
    f.commit().unwrap();
    f
}

fn emit_test_rdtsc(emit_content: impl Fn(&mut X64Assembler)) 
    -> X64Assembler 
{
    let mut f = X64Assembler::new().unwrap();
    f.emit_rdtsc_start(Gpr::R15 as u8);
    emit_content(&mut f);
    f.emit_rdtsc_end(Gpr::R15 as u8, Gpr::Rax as u8);
    f.emit_ret();
    f.commit().unwrap();
    f
}




fn main() {
    PerfectEnv::pin_to_core(15);
    let _ = PerfectEnv::mmap_fixed(0, 0x8000_0000);

    let emap = Zen2EventMap::new();
    let mut harness = HarnessConfig::default().emit();

    //println!("[*] Harness disassembly:");
    //harness.disas();
    //println!();

    let mut events = EventSet::new();
    events.add_event_nomask(0x35); // stlf
    events.add_event_nomask(0x76); // cycles
    events.add_event_nomask(0xc0); // instructions
    events.add_event_nomask(0xc1); // uops
    events.add_event_nomask(0xc2); // branches
    events.add_event_nomask(0xc3); // branch misp

    const TEST_ITERS: usize = 512;
    for (desc, test_emitter) in TESTS {
        println!("===============================================");
        println!("[*] Running test '{}' {} times ...", desc, TEST_ITERS);

        // Just use RDTSC to measure (no rdpmc usage).
        // NOTE: Hacky because this still *enables* the performance counters. 
        // Should probably have a method on the harness dedicated to this.
        let asm = emit_test_rdtsc(test_emitter);
        let asm_reader = asm.reader();
        let asm_tgt_buf = asm_reader.lock();
        let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
        let asm_fn: MeasuredFn = unsafe { std::mem::transmute(asm_tgt_ptr) };
        let results = harness.measure(
            asm_fn, 0, 0, TEST_ITERS, InputMethod::Fixed(0, 0)
        ).unwrap();
        let dist = results.get_distribution();
        let min = results.get_min();
        let max = results.get_max();
        println!("{:03x}:{:02x} {:032}", 0, 0, "rdtsc");
        println!("\tmin={} max={} dist={:?}", min,max,dist);


        let asm = emit_test(test_emitter);
        let asm_reader = asm.reader();
        let asm_tgt_buf = asm_reader.lock();
        //println!("[*] Disassembly:");
        //disas(&asm_tgt_buf);
        let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
        let asm_fn: MeasuredFn = unsafe { std::mem::transmute(asm_tgt_ptr) };

        for (event, umask) in events.iter() {
            let event_name = if let Some(desc) = emap.get_by_event(*event) {
                desc.name.to_string()
            } else { format!("unk_{:03x}", event) };

            let results = harness.measure(
                asm_fn, *event, *umask, TEST_ITERS, InputMethod::Fixed(0, 0)
            ).unwrap();

            let dist = results.get_distribution();
            let min = results.get_min();
            let max = results.get_max();
            println!("{:03x}:{:02x} {:032}", event, umask, event_name);
            println!("\tmin={} max={} dist={:?}", min,max,dist);
        }
    }
}

