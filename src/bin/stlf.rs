use perfect::*;
use perfect::codegen::*;
use perfect::zen2::*;
use std::collections::*;

fn emit_test(mut emitter: impl FnMut(&mut PerfectFn)) -> PerfectFn {
    let mut f = PerfectFn::new("test");

    // NOTE: I imagine that just LFENCE is sufficient for draining the 
    // store queue, however, it doesn't necessarily change the state of 
    // any other underlying storage that might be used for forwarding stores 
    // or predicting memory dependences. Do a bunch of stores with the low
    // bits set to zero, in an attempt to pollute any state that might 
    // outlive the store queue. (this is mostly nonsense)

    for _ in 0..16 {
        dynasm!(f.asm
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

    dynasm!(f.asm
        ; mov rax, 0xdeadbeef
        ; .align 4096
        ; lfence
    );

    f.emit_rdpmc_start(1, Gpr::R15 as u8);

    emitter(&mut f);

    f.emit_rdpmc_end(1, Gpr::R15 as u8, Gpr::Rax as u8);
    f.emit_ret();
    f.commit();
    f.disas();
    println!();

    f

}

/// Test 1. Bits [11:0] determine STLF eligibility. 
/// Aliasing prevents the store from being eligible for forwarding.
fn emit_stlf_eligibility(f: &mut PerfectFn) {
    dynasm!(f.asm
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

/// Test 2. After 49 in-flight stores, forwarding never occurs. 
/// This probably reflects the store queue capacity.
fn emit_stq_capacity(f: &mut PerfectFn) {
    dynasm!(f.asm
        ; mov [0x0001_0000], al // Store we want to forward
        ; mov [0x0001_0001], al
        ; mov [0x0001_0002], al
        ; mov [0x0001_0003], al
        ; mov [0x0001_0004], al
        ; mov [0x0001_0005], al
        ; mov [0x0001_0006], al
        ; mov [0x0001_0007], al
        ; mov [0x0001_0008], al
        ; mov [0x0001_0009], al
        ; mov [0x0001_0011], al
        ; mov [0x0001_0012], al
        ; mov [0x0001_0013], al
        ; mov [0x0001_0014], al
        ; mov [0x0001_0015], al
        ; mov [0x0001_0016], al
        ; mov [0x0001_0017], al
        ; mov [0x0001_0018], al
        ; mov [0x0001_0019], al
        ; mov [0x0001_0020], al
        ; mov [0x0001_0021], al
        ; mov [0x0001_0022], al
        ; mov [0x0001_0023], al
        ; mov [0x0001_0024], al
        ; mov [0x0001_0025], al
        ; mov [0x0001_0026], al
        ; mov [0x0001_0027], al
        ; mov [0x0001_0028], al
        ; mov [0x0001_0029], al
        ; mov [0x0001_0030], al
        ; mov [0x0001_0031], al
        ; mov [0x0001_0032], al
        ; mov [0x0001_0033], al
        ; mov [0x0001_0034], al
        ; mov [0x0001_0035], al
        ; mov [0x0001_0036], al
        ; mov [0x0001_0037], al
        ; mov [0x0001_0038], al
        ; mov [0x0001_0039], al
        ; mov [0x0001_0040], al
        ; mov [0x0001_0041], al
        ; mov [0x0001_0042], al
        ; mov [0x0001_0043], al
        ; mov [0x0001_0044], al
        ; mov [0x0001_0045], al
        ; mov [0x0001_0046], al
        ; mov [0x0001_0047], al
        ; mov [0x0001_0048], al
        ; mov bl, [0x0001_0000] // Target load
    );
}

/// Test 3. Memory renaming relies on displacement bits [9:3].
/// When other bits are set, memory renaming events never occur.
fn emit_renaming_disp_bits(f: &mut PerfectFn) {
    dynasm!(f.asm
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
fn emit_renaming_disp_bits_permute(f: &mut PerfectFn) {
    dynasm!(f.asm
        ; mov [0x0000_0008], eax ; mov ebx, [0x0000_0008]
        ; mov [0x0000_0010], eax ; mov ebx, [0x0000_0010]
        ; mov [0x0000_0018], eax ; mov ebx, [0x0000_0018]
        ; mov [0x0000_0020], eax ; mov ebx, [0x0000_0020]
        ; mov [0x0000_0028], eax ; mov ebx, [0x0000_0028]
        ; mov [0x0000_0030], eax ; mov ebx, [0x0000_0030]
        ; mov [0x0000_0038], eax ; mov ebx, [0x0000_0038]
        ; mov [0x0000_0040], eax ; mov ebx, [0x0000_0040]
        ; mov [0x0000_0048], eax ; mov ebx, [0x0000_0048]
        ; mov [0x0000_0050], eax ; mov ebx, [0x0000_0050]
        ; mov [0x0000_0058], eax ; mov ebx, [0x0000_0058]
        ; mov [0x0000_0060], eax ; mov ebx, [0x0000_0060]
        ; mov [0x0000_0068], eax ; mov ebx, [0x0000_0068]
        ; mov [0x0000_0070], eax ; mov ebx, [0x0000_0070]
        ; mov [0x0000_0078], eax ; mov ebx, [0x0000_0078]
        ; mov [0x0000_0080], eax ; mov ebx, [0x0000_0080]
        ; mov [0x0000_0088], eax ; mov ebx, [0x0000_0088]
        ; mov [0x0000_0090], eax ; mov ebx, [0x0000_0090]
        ; mov [0x0000_0098], eax ; mov ebx, [0x0000_0098]
        ; mov [0x0000_00a0], eax ; mov ebx, [0x0000_00a0]
        ; mov [0x0000_00a8], eax ; mov ebx, [0x0000_00a8]
        ; mov [0x0000_00b0], eax ; mov ebx, [0x0000_00b0]
        ; mov [0x0000_00b8], eax ; mov ebx, [0x0000_00b8]
        ; mov [0x0000_00c0], eax ; mov ebx, [0x0000_00c0]
        ; mov [0x0000_00c8], eax ; mov ebx, [0x0000_00c8]
        ; mov [0x0000_00d0], eax ; mov ebx, [0x0000_00d0]
        ; mov [0x0000_00d8], eax ; mov ebx, [0x0000_00d8]
        ; mov [0x0000_00e0], eax ; mov ebx, [0x0000_00e0]
        ; mov [0x0000_00e8], eax ; mov ebx, [0x0000_00e8]
        ; mov [0x0000_00f0], eax ; mov ebx, [0x0000_00f0]
        ; mov [0x0000_00f8], eax ; mov ebx, [0x0000_00f8]
        ; mov [0x0000_0100], eax ; mov ebx, [0x0000_0100]
        ; mov [0x0000_0108], eax ; mov ebx, [0x0000_0108]
        ; mov [0x0000_0110], eax ; mov ebx, [0x0000_0110]
        ; mov [0x0000_0118], eax ; mov ebx, [0x0000_0118]
        ; mov [0x0000_0120], eax ; mov ebx, [0x0000_0120]
        ; mov [0x0000_0128], eax ; mov ebx, [0x0000_0128]
        ; mov [0x0000_0130], eax ; mov ebx, [0x0000_0130]
        ; mov [0x0000_0138], eax ; mov ebx, [0x0000_0138]
        ; mov [0x0000_0140], eax ; mov ebx, [0x0000_0140]
        ; mov [0x0000_0148], eax ; mov ebx, [0x0000_0148]
        ; mov [0x0000_0150], eax ; mov ebx, [0x0000_0150]
        ; mov [0x0000_0158], eax ; mov ebx, [0x0000_0158]
        ; mov [0x0000_0160], eax ; mov ebx, [0x0000_0160]
        ; mov [0x0000_0168], eax ; mov ebx, [0x0000_0168]
        ; mov [0x0000_0170], eax ; mov ebx, [0x0000_0170]
        ; mov [0x0000_0178], eax ; mov ebx, [0x0000_0178]
        ; mov [0x0000_0180], eax ; mov ebx, [0x0000_0180]
        ; mov [0x0000_0188], eax ; mov ebx, [0x0000_0188]
        ; mov [0x0000_0190], eax ; mov ebx, [0x0000_0190]
        ; mov [0x0000_0198], eax ; mov ebx, [0x0000_0198]
        ; mov [0x0000_01a0], eax ; mov ebx, [0x0000_01a0]
        ; mov [0x0000_01a8], eax ; mov ebx, [0x0000_01a8]
        ; mov [0x0000_01b0], eax ; mov ebx, [0x0000_01b0]
        ; mov [0x0000_01b8], eax ; mov ebx, [0x0000_01b8]
        ; mov [0x0000_01c0], eax ; mov ebx, [0x0000_01c0]
        ; mov [0x0000_01c8], eax ; mov ebx, [0x0000_01c8]
        ; mov [0x0000_01d0], eax ; mov ebx, [0x0000_01d0]
        ; mov [0x0000_01d8], eax ; mov ebx, [0x0000_01d8]
        ; mov [0x0000_01e0], eax ; mov ebx, [0x0000_01e0]
        ; mov [0x0000_01e8], eax ; mov ebx, [0x0000_01e8]
        ; mov [0x0000_01f0], eax ; mov ebx, [0x0000_01f0]
        ; mov [0x0000_01f8], eax ; mov ebx, [0x0000_01f8]
        ; mov [0x0000_0200], eax ; mov ebx, [0x0000_0200]
        ; mov [0x0000_0208], eax ; mov ebx, [0x0000_0208]
        ; mov [0x0000_0210], eax ; mov ebx, [0x0000_0210]
        ; mov [0x0000_0218], eax ; mov ebx, [0x0000_0218]
        ; mov [0x0000_0220], eax ; mov ebx, [0x0000_0220]
        ; mov [0x0000_0228], eax ; mov ebx, [0x0000_0228]
        ; mov [0x0000_0230], eax ; mov ebx, [0x0000_0230]
        ; mov [0x0000_0238], eax ; mov ebx, [0x0000_0238]
        ; mov [0x0000_0240], eax ; mov ebx, [0x0000_0240]
        ; mov [0x0000_0248], eax ; mov ebx, [0x0000_0248]
        ; mov [0x0000_0250], eax ; mov ebx, [0x0000_0250]
        ; mov [0x0000_0258], eax ; mov ebx, [0x0000_0258]
        ; mov [0x0000_0260], eax ; mov ebx, [0x0000_0260]
        ; mov [0x0000_0268], eax ; mov ebx, [0x0000_0268]
        ; mov [0x0000_0270], eax ; mov ebx, [0x0000_0270]
        ; mov [0x0000_0278], eax ; mov ebx, [0x0000_0278]
        ; mov [0x0000_0280], eax ; mov ebx, [0x0000_0280]
        ; mov [0x0000_0288], eax ; mov ebx, [0x0000_0288]
        ; mov [0x0000_0290], eax ; mov ebx, [0x0000_0290]
        ; mov [0x0000_0298], eax ; mov ebx, [0x0000_0298]
        ; mov [0x0000_02a0], eax ; mov ebx, [0x0000_02a0]
        ; mov [0x0000_02a8], eax ; mov ebx, [0x0000_02a8]
        ; mov [0x0000_02b0], eax ; mov ebx, [0x0000_02b0]
        ; mov [0x0000_02b8], eax ; mov ebx, [0x0000_02b8]
        ; mov [0x0000_02c0], eax ; mov ebx, [0x0000_02c0]
        ; mov [0x0000_02c8], eax ; mov ebx, [0x0000_02c8]
        ; mov [0x0000_02d0], eax ; mov ebx, [0x0000_02d0]
        ; mov [0x0000_02d8], eax ; mov ebx, [0x0000_02d8]
        ; mov [0x0000_02e0], eax ; mov ebx, [0x0000_02e0]
        ; mov [0x0000_02e8], eax ; mov ebx, [0x0000_02e8]
        ; mov [0x0000_02f0], eax ; mov ebx, [0x0000_02f0]
        ; mov [0x0000_02f8], eax ; mov ebx, [0x0000_02f8]
        ; mov [0x0000_0300], eax ; mov ebx, [0x0000_0300]
        ; mov [0x0000_0308], eax ; mov ebx, [0x0000_0308]
        ; mov [0x0000_0310], eax ; mov ebx, [0x0000_0310]
        ; mov [0x0000_0318], eax ; mov ebx, [0x0000_0318]
        ; mov [0x0000_0320], eax ; mov ebx, [0x0000_0320]
        ; mov [0x0000_0328], eax ; mov ebx, [0x0000_0328]
        ; mov [0x0000_0330], eax ; mov ebx, [0x0000_0330]
        ; mov [0x0000_0338], eax ; mov ebx, [0x0000_0338]
        ; mov [0x0000_0340], eax ; mov ebx, [0x0000_0340]
        ; mov [0x0000_0348], eax ; mov ebx, [0x0000_0348]
        ; mov [0x0000_0350], eax ; mov ebx, [0x0000_0350]
        ; mov [0x0000_0358], eax ; mov ebx, [0x0000_0358]
        ; mov [0x0000_0360], eax ; mov ebx, [0x0000_0360]
        ; mov [0x0000_0368], eax ; mov ebx, [0x0000_0368]
        ; mov [0x0000_0370], eax ; mov ebx, [0x0000_0370]
        ; mov [0x0000_0378], eax ; mov ebx, [0x0000_0378]
        ; mov [0x0000_0380], eax ; mov ebx, [0x0000_0380]
        ; mov [0x0000_0388], eax ; mov ebx, [0x0000_0388]
        ; mov [0x0000_0390], eax ; mov ebx, [0x0000_0390]
        ; mov [0x0000_0398], eax ; mov ebx, [0x0000_0398]
        ; mov [0x0000_03a0], eax ; mov ebx, [0x0000_03a0]
        ; mov [0x0000_03a8], eax ; mov ebx, [0x0000_03a8]
        ; mov [0x0000_03b0], eax ; mov ebx, [0x0000_03b0]
        ; mov [0x0000_03b8], eax ; mov ebx, [0x0000_03b8]
        ; mov [0x0000_03c0], eax ; mov ebx, [0x0000_03c0]
        ; mov [0x0000_03c8], eax ; mov ebx, [0x0000_03c8]
        ; mov [0x0000_03d0], eax ; mov ebx, [0x0000_03d0]
        ; mov [0x0000_03d8], eax ; mov ebx, [0x0000_03d8]
        ; mov [0x0000_03e0], eax ; mov ebx, [0x0000_03e0]
        ; mov [0x0000_03e8], eax ; mov ebx, [0x0000_03e8]
        ; mov [0x0000_03f0], eax ; mov ebx, [0x0000_03f0]
        ; mov [0x0000_03f8], eax ; mov ebx, [0x0000_03f8]
    );
}

/// Test 5. Only the youngest six stores are eligible for fowarding thru
/// the memory file? 
fn emit_renaming_window(f: &mut PerfectFn) {
    dynasm!(f.asm
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



fn main() {
    pin_to_core(15);

    // This is just to make it easier to write simple loads and stores
    // inside JIT'ed code. 
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

    //let f = emit_test(emit_stlf_eligibility);
    //let f = emit_test(emit_stq_capacity);
    //let f = emit_test(emit_renaming_disp_bits);
    //let f = emit_test(emit_renaming_disp_bits_permute);
    let mut f = emit_test(emit_renaming_window);

    #[repr(C, align(0x10000))]
    pub struct Storage {
        data: [u8; 0x1000000]
    }

    let mut storage = Box::new(Storage { data: [0; 0x1000000] });
    //let mut storage = vec![0u8; 0x1000000].into_boxed_slice();
    let ptr = storage.data.as_ptr();
    println!("{:?}", ptr);

    let mut events = Vec::new();
    for e in 0x00..=0xdf {
        for umask in [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80] {
            events.push((e, umask));
        }
    }

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

    for (event, umask) in events.iter() {
        let event_name = if let Some(desc) = emap.lookup(*event) {
            desc.name.to_string()
        } else { format!("unk_{:03x}", event) };

        let (results, _) = harness.measure(&mut f, 
            *event, *umask, 512, ptr as usize, 0
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
        println!("\tfloor min={} max={} dist={:?}", floor_min,floor_max,floor_dist);
        println!("\tmin={} max={} dist={:?}", min,max,dist);

        //let norm = if min == max {
        //    if let Some((fmin,fmax,fdist)) = floor_res.get(&(event,umask)) {
        //        Some(min - fmin)
        //    } else { None }
        //} else { None };

        //if let Some(norm_val) = norm {
        //    if norm_val == 0 { continue; }
        //    println!("{:03x}:{:02x} {:032} norm={}", 
        //             event, umask, event_name, norm_val);
        //} else {
        //    if min == 0 && max == 0 { continue; }
        //    println!("{:03x}:{:02x} {:032} min={} max={}", 
        //             event,umask,event_name,min,max);
        //    println!("\tdist={:?}", dist);
        //}

    }

   
}
