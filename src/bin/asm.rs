

use perfect::*;
use perfect::asm::*;
use perfect::zen2::*;

fn main() {
    let mut b = PerfectAsm::new(0x0000_1337_0000_0000, 0x10_000);

    let lab = b.new_dynamic_label();
    dynasm!(b
        ; nop
        ; je ->foo
        ; nop
        ; nop
        ; ->foo:
        ; nop
        ; jmp >next
        ; next:
        ; nop
        ; nop
        ; jmp =>lab
    );

    dynasm!(b
        ; nop
        ; nop
        ; nop
        ; nop
    );
    b.emit_lfence();
    dynasm!(b
        ; nop
        ; nop
        ; =>lab
        ; mov rax, 0xdead
        ; ret
    );

    b.commit().unwrap();
    b.disas();

    let f: fn() -> usize = unsafe { std::mem::transmute(b.ptr) };
    let x = f();
    assert_eq!(x, 0xdead);

    //let mut harness = PerfectHarness::new().emit();
    //let mut events = EventSet::new();
    //events.add_event_nomask(0xc3);
    //let (results, _) = harness.measure_vary(&mut f, 
    //    *event, *umask, 1024, 0,
    //).unwrap();



}
