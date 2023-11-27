

use perfect::*;
use perfect::asm::*;

fn main() {
    let mut b = PerfectBuffer::new(0x0000_1337_0000_0000, 0x10_000);

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
}
