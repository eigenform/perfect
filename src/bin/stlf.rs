use perfect::*;
use perfect::zen2::*;
use std::collections::*;

use rand::prelude::*;


/// Test 1. Bits [11:0] determine STLF eligibility. 
/// Aliasing prevents the store from being eligible for forwarding.
fn emit_stlf_eligibility(f: &mut X64Assembler) {
    dynasm!(f
        ; mov rax, 0xdeadbeef
        ; mov [0x0001_0000], al // Store we want to forward
        ; mov [0x0001_0001], al
        ; mov [0x0001_0002], al
        ; mov [0x0001_0004], al
        ; mov [0x0001_0008], al
        ; mov [0x0001_000f], al
        ; mov [0x0001_0010], al
        ; mov [0x0001_0020], al
        ; mov [0x0001_0040], al
        ; mov [0x0001_0080], al
        ; mov [0x0001_00ff], al
        ; mov [0x0001_0100], al
        ; mov [0x0001_0200], al
        ; mov [0x0001_0400], al
        ; mov [0x0001_0800], al
        ; mov [0x0001_0fff], al
        ; mov [0x0001_1000], al // This store is aliasing 
        ; mov bl, [0x0001_0000] // Target load
    );
}

/// Place some number of stores in-between STLF producer and consumer. 
/// At some point, STLF events will not occur due to store queue capacity.
fn emit_stq_capacity(f: &mut X64Assembler, width: usize, depth: usize) {

    // The store we want to forward
    match width {
        1 => { dynasm!(f ; mov [0x0001_0000], al ); },
        2 => { dynasm!(f ; mov [0x0001_0000], ah ); },
        4 => { dynasm!(f ; mov [0x0001_0000], eax ); },
        8 => { dynasm!(f ; mov [0x0001_0000], rax ); },
        _ => unreachable!(),
    }

    // Generate some random non-aliasing padding stores to fill the STQ
    let mut r: Vec<i32> = match width { 
        1 => (0x0001_0001..=0x0001_0fff).collect(),
        2 => (0x0001_0002..=0x0001_0ffe).step_by(2).collect(),
        4 => (0x0001_0004..=0x0001_0ffc).step_by(4).collect(),
        8 => (0x0001_0008..=0x0001_0ff8).step_by(8).collect(),
        _ => unreachable!(),
    };
    let mut rng = rand::thread_rng();
    r.shuffle(&mut rng);
    for addr in &r[1..=depth] {
        match width {
            1 => { dynasm!(f ; mov [*addr], al); },
            2 => { dynasm!(f ; mov [*addr], ah); },
            4 => { dynasm!(f ; mov [*addr], eax); },
            8 => { dynasm!(f ; mov [*addr], rax); },
            _ => unreachable!(),
        }
    }

    // Load whose result we expect to be forwarded
    match width {
        1 => { dynasm!(f ; mov bl, [0x0001_0000] ); },
        2 => { dynasm!(f ; mov bh, [0x0001_0000] ); },
        4 => { dynasm!(f ; mov ebx, [0x0001_0000] ); },
        8 => { dynasm!(f ; mov rbx, [0x0001_0000] ); },
        _ => unreachable!(),
    }
}

// It seems like there can be 48 in-flight stores. 
// After 47 padding stores, no STLF event occurs.
// This matches the figure in the SOG for Family 17h. 
fn emit_stq_capacity_byte(f: &mut X64Assembler) { 
    emit_stq_capacity(f, 1, 47); 
}
fn emit_stq_capacity_half(f: &mut X64Assembler) { 
    emit_stq_capacity(f, 2, 47);
}
fn emit_stq_capacity_word(f: &mut X64Assembler) { 
    emit_stq_capacity(f, 4, 47);
}
fn emit_stq_capacity_quad(f: &mut X64Assembler) { 
    emit_stq_capacity(f, 8, 47);
}


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

    f.emit_rdpmc_start(0, Gpr::R15 as u8);
    emit_content(&mut f);
    f.emit_rdpmc_end(0, Gpr::R15 as u8, Gpr::Rax as u8);
    f.emit_ret();
    f.commit().unwrap();
    f
}

const TESTS: &[(&'static str, fn(&mut X64Assembler))] = &[
    ("STLF eligibility",    emit_stlf_eligibility),
    ("STQ capacity (byte)", emit_stq_capacity_byte),
    ("STQ capacity (half)", emit_stq_capacity_half),
    ("STQ capacity (word)", emit_stq_capacity_word),
    ("STQ capacity (quad)", emit_stq_capacity_quad),
    ("Memory renaming eligibility #1", emit_renaming_disp_bits),
    ("Memory renaming eligibility #2", emit_renaming_disp_bits_permute),
    ("In-flight memory renames", emit_renaming_window),
];

fn main() {
    PerfectEnv::pin_to_core(15);

    // NOTE: `config.sh` changes the appropriate sysctl knob for allowing 
    // allocations that start at virtual address 0x0. 
    //
    // This is just to make it easier to write simple loads and stores inside 
    // JIT'ed code with absolute addresses that are in the very bottom part 
    // of virtual memory. It's also nice because we can guarantee that all  
    // other bits in those addresses are zero. 
    //
    // This call has an effect on the current process, but we don't care about
    // the pointer returned by `mmap_fixed` in this case because we don't have 
    // to pass around any references to this allocation (and it would probably
    // be a bad idea anyway because this is a "valid" pointer to zero). 
    let _ = PerfectEnv::mmap_fixed(0, 0x8000_0000);

    let emap = Zen2EventMap::new();
    let mut harness = HarnessConfig::default().emit();

    let mut events = EventSet::new();
    events.add_event_bits(0x24);
    events.add_event_bits(0x25);
    events.add_event_bits(0x29);
    events.add_event_bits(0x2f);
    events.add_event_nomask(0x35);
    events.add_event_bits(0x36);
    events.add_event_bits(0x37);
    events.add_event_nomask(0x76);
    events.add_event_nomask(0xae);
    events.add_event_nomask(0xaf);
    events.add_event_nomask(0xb1);
    events.add_event_nomask(0xb2);
    events.add_event_nomask(0xb3);

    for (desc, test_emitter) in TESTS {
        println!("===============================================");
        println!("[*] Running test '{}'", desc);
        let asm = emit_test(test_emitter);
        let asm_reader = asm.reader();
        let asm_tgt_buf = asm_reader.lock();
        let asm_tgt_ptr = asm_tgt_buf.ptr(AssemblyOffset(0));
        let asm_fn: MeasuredFn = unsafe { std::mem::transmute(asm_tgt_ptr) };
        for (event, umask) in events.iter() {
            let event_name = if let Some(desc) = emap.lookup(*event) {
                desc.name.to_string()
            } else { format!("unk_{:03x}", event) };

            let results = harness.measure(
                asm_fn, *event, *umask, 512, 0, 0
            ).unwrap();

            let dist = results.get_distribution();
            let min = results.get_min();
            let max = results.get_max();
            if max == 0 {
                continue;
            }

            println!("{:03x}:{:02x} {:032}", event, umask, event_name);
            println!("\tmin={} max={} dist={:?}", min,max,dist);
        }
    }
}

