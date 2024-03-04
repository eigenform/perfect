use perfect::*;
use perfect::events::*;
use rand::prelude::*;


/// Test 3. Memory renaming relies on displacement bits [9:3].
/// When other bits are set, memory renaming events never occur.
fn emit_renaming_disp_bits(f: &mut X64Assembler) {
    dynasm!(f
        ; mov [0x0000_0008], eax ; mov ebx, [0x0000_0008]
        ; mov [0x0000_0010], eax ; mov ebx, [0x0000_0010]
        ; mov [0x0000_0020], eax ; mov ebx, [0x0000_0020]
        ; mov [0x0000_0040], eax ; mov ebx, [0x0000_0040]
        ; mov [0x0000_0080], eax ; mov ebx, [0x0000_0080]
        ; mov [0x0000_0100], eax ; mov ebx, [0x0000_0100]
        ; mov [0x0000_0200], eax ; mov ebx, [0x0000_0200]
        ; mov [0x0000_03f8], eax ; mov ebx, [0x0000_03f8]
    );
}

/// Test 4. All permutations of displacement bits [9:3].
/// You should observe 127 memory renaming events. 
fn emit_renaming_disp_bits_permute(f: &mut X64Assembler) {
    for addr in (0x0000_0008..=0x0000_03f8).step_by(8) {
        dynasm!(f
            ; mov [addr], eax ; mov ebx, [addr]
        );
    }
}

/// Test 5. Only the youngest six stores are eligible for fowarding thru
/// the memory file? 
fn emit_renaming_window(f: &mut X64Assembler) {
    dynasm!(f
        ; mov rcx, 0x1000
        ; lfence

        // Write some entries into the memory file
        ; .align 4096
        ; mov [0x0000_0008], ecx
        ; mov [0x0000_0010], ecx
        ; mov [0x0000_0020], ecx
        ; mov [0x0000_0040], ecx
        ; mov [0x0000_0080], ecx
        ; mov [0x0000_0100], ecx
        ; mov [0x0000_0200], ecx

        // Wait for the stores to complete/retire
        ; lfence
        ; .align 4096

        ; mov ebx, [0x0000_0008] // Not renamed
        ; mov ebx, [0x0000_0010] // Renamed
        ; mov ebx, [0x0000_0020] // Renamed
        ; mov ebx, [0x0000_0040] // Renamed
        ; mov ebx, [0x0000_0080] // Renamed
        ; mov ebx, [0x0000_0100] // Renamed
        ; mov ebx, [0x0000_0200] // Renamed
    );
}

fn emit_test(emit_content: impl Fn(&mut X64Assembler)) 
    -> X64Assembler 
{
    let mut f = X64Assembler::new().unwrap();

    // NOTE: I imagine that just LFENCE is sufficient for draining the 
    // store queue, however, it doesn't necessarily change the state of 
    // any other underlying storage that might be used for forwarding stores 
    // or predicting memory dependences. Do a bunch of stores with the low
    // bits set to zero, in an attempt to pollute any state that might 
    // outlive the store queue. (this is mostly nonsense)

    for _ in 0..16 {
        dynasm!(f
            ; mov [0x0000_0000], al
            ; mov [0x1000_0000], ah
            ; mov [0x2000_0000], ax
            ; mov [0x3000_0000], eax
            ; mov [0x4000_0000], rax
            ; sfence
            ; mov [0x1000_0000], bl
            ; mov [0x2000_0000], bh
            ; mov [0x3000_0000], bx
            ; mov [0x4000_0000], ebx
            ; mov [0x0000_0000], rbx
            ; sfence
            ; mov [0x2000_0000], cl
            ; mov [0x3000_0000], ch
            ; mov [0x4000_0000], cx
            ; mov [0x0000_0000], ecx
            ; mov [0x1000_0000], rcx
            ; sfence
            ; mov [0x3000_0000], dl
            ; mov [0x4000_0000], dh
            ; mov [0x0000_0000], dx
            ; mov [0x1000_0000], edx
            ; mov [0x2000_0000], rdx
            ; sfence
        );
    }

    dynasm!(f
        ; mov rax, 0xdeadbeef
        ; .align 4096
        ; lfence
    );

    f.emit_rdpmc_start(1, Gpr::R15 as u8);
    emit_content(&mut f);
    f.emit_rdpmc_end(1, Gpr::R15 as u8, Gpr::Rax as u8);
    f.emit_ret();
    f.commit().unwrap();
    f
}

fn main() {
    PerfectEnv::pin_to_core(15);
    let mut harness = HarnessConfig::default().emit();
    let mut events = EventSet::new();
    events.add(Zen2Event::MemFileHit(0x00));
    events.add(Zen2Event::MemRenLdDsp(0x00));
    events.add(Zen2Event::MemRenLdElim(0x00));

    //for (desc, test_emitter) in TESTS {
    //    println!("===============================================");
    //    println!("[*] Running test '{}'", desc);
    //    let asm = emit_test(test_emitter);
    //    let asm_reader = asm.reader();
    //    let asm_tgt_buf = asm_reader.lock();
    //    let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
    //    let asm_fn: MeasuredFn = unsafe { std::mem::transmute(asm_tgt_ptr) };

    //    for event in events.iter() { 
    //        let results = harness.measure(
    //            asm_fn, event.id(), event.mask(), 512, InputMethod::Fixed(0, 0),
    //        ).unwrap();

    //        let dist = results.get_distribution();
    //        let min = results.get_min();
    //        let max = results.get_max();
    //        if max == 0 {
    //            continue;
    //        }

    //        println!("{:03x}:{:02x} {:032}", event.id(), event.mask(), event.name());
    //        println!("\tmin={} max={} dist={:?}", min,max,dist);
    //    }
    //}
}

