#![allow(unused_mut)]
#![allow(unused_parens)]
#![allow(unused_variables)]
#![allow(dead_code)]

pub mod zen2;
pub mod codegen;
pub mod ir;

use perf_event::Builder;
use perf_event::events::*;
use std::collections::*;
use std::marker::*;
use std::sync::RwLockReadGuard;
use iced_x86::{ 
    Decoder, DecoderOptions, Instruction, Formatter, IntelFormatter 
};
use rand::rngs::ThreadRng;


pub use itertools::*;
pub use dynasmrt::{
    dynasm, 
    DynasmApi, 
    DynasmLabelApi, 
    DynamicLabel,
    Assembler, 
    AssemblyOffset, 
    ExecutableBuffer, 
    Executor,
    x64::X64Relocation
};
pub use crate::ir::Gpr;


/// Pin to a particular core.
pub fn pin_to_core(core: usize) {
    let this_pid = nix::unistd::Pid::from_raw(0);
    let mut cpuset = nix::sched::CpuSet::new();
    cpuset.set(core).unwrap();
    nix::sched::sched_setaffinity(this_pid, &cpuset).unwrap();
}

pub fn mmap_fixed(addr: usize, len: usize) -> *mut u8 {
    use nix::sys::mman::{ ProtFlags, MapFlags };

    let ptr = unsafe { 
        nix::sys::mman::mmap(std::num::NonZeroUsize::new(addr),
            std::num::NonZeroUsize::new(len).unwrap(),
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MapFlags::MAP_ANONYMOUS | 
            MapFlags::MAP_PRIVATE | 
            MapFlags::MAP_FIXED, 0, 0).unwrap()
    };
    assert!(ptr as usize == addr);
    ptr as *mut u8

}

pub fn get_distribution(results: &Vec<usize>) -> BTreeMap<usize, usize> {
    let mut dist = std::collections::BTreeMap::new();
    for r in results.iter() {
        if let Some(cnt) = dist.get_mut(r) {
            *cnt += 1;
        } else {
            dist.insert(*r, 1);
        }
    }
    dist
}



/// Print the disassembly for a particular [ExecutableBuffer].
pub fn disas(buf: &ExecutableBuffer) {
    let ptr: *const u8 = buf.ptr(AssemblyOffset(0));
    let addr: u64   = ptr as u64;
    let buf: &[u8]  = unsafe { std::slice::from_raw_parts(ptr, buf.len()) };

    let mut decoder = Decoder::with_ip(64, buf, addr, DecoderOptions::NONE);
    let mut formatter = IntelFormatter::new();
    formatter.options_mut().set_digit_separator("_");
    let _ = formatter.options_mut().first_operand_char_index();
    let mut output = String::new();
    let mut instr  = Instruction::default();

    while decoder.can_decode() {
        decoder.decode_out(&mut instr);
        output.clear();
        formatter.format(&instr, &mut output);

        let start_idx = (instr.ip() - addr) as usize;
        let instr_bytes = &buf[start_idx..start_idx + instr.len()];
        let mut bytestr = String::new();
        for b in instr_bytes.iter() {
            bytestr.push_str(&format!("{:02x}", b));
        }
        println!("{:016x}: {:32} {}", instr.ip(), bytestr, output);
    }
}



#[repr(C, align(0x10000))]
pub struct Stack { data: [u8; 0x8000], }
impl Stack {
    pub fn new() -> Self { Self { data: [0; 0x8000] } }
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { self.data.as_ptr().offset(0x3000) }
    }
}

pub fn commit(asm: &mut Assembler<X64Relocation>) {
    asm.commit().unwrap();
}

/// Saved general-purpose register state. 
#[repr(C)]
#[derive(Copy, Clone)]
pub struct GprState(pub [usize; 16]);
impl GprState {
    pub fn new() -> Self { Self([0; 16]) }
    pub fn clear(&mut self) { self.0 = [0; 16]; }
}
impl std::fmt::Debug for GprState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GprState")
         .field("rax", &self.0[0])
         .field("rcx", &self.0[1])
         .field("rdx", &self.0[2])
         .field("rbx", &self.0[3])
         .field("rsp", &self.0[4])
         .field("rbp", &self.0[5])
         .field("rsi", &self.0[6])
         .field("rdi", &self.0[7])
         .field("r8",  &self.0[8])
         .field("r9",  &self.0[9])
         .field("r10", &self.0[10])
         .field("r11", &self.0[11])
         .field("r12", &self.0[12])
         .field("r13", &self.0[13])
         .field("r14", &self.0[14])
         .field("r15", &self.0[15])
         .finish()
    }
}

pub struct PerfectHarness {
    /// Harness for jumping into JIT'ed code
    pub harness: Option<Assembler<X64Relocation>>,

    pub harness_buf: Option<ExecutableBuffer>,
    pub harness_fn: Option<fn(usize, usize, usize) -> usize>,

    /// Saved stack pointer
    pub harness_state: Box<[u64; 16]>,

    /// Scratchpad memory for JIT'ed code
    pub harness_stack: Box<Stack>,

    /// Scratchpad memory for saving GPR state when JIT'ed code exits
    pub gpr_state: Box<GprState>,

    /// Toggleable: dump GPRs after return from measured code
    dump_gpr: bool,
}
impl PerfectHarness {
    pub fn new() -> Self { 
        let mut harness = Assembler::<X64Relocation>::new().unwrap();
        let mut harness_state = Box::new([0; 16]);
        let mut harness_stack = Box::new(Stack::new());
        let mut gpr_state = Box::new(GprState::new());

        Self { 
            harness: Some(harness),
            harness_buf: None,
            harness_fn: None,
            dump_gpr: false,
            harness_state: Box::new([0; 16]),
            harness_stack: Box::new(Stack::new()),
            gpr_state: Box::new(GprState::new()),
        }
    }

    pub fn set_dump_gpr(mut self, x: bool) -> Self { self.dump_gpr = x; self }

    pub fn emit(mut self) -> Self {
        let mut harness = self.harness.take().unwrap();

        dynasm!(harness
            ; .arch     x64

            // Save nonvolatile registers 
            ; push      rbp
            ; push      rbx
            ; push      rdi
            ; push      rsi
            ; push      r12
            ; push      r13
            ; push      r14
            ; push      r15

            // Pointer to measured code
            ; mov r15, rdx

            // Save the stack pointer
            ; mov rax, QWORD self.harness_state.as_ptr() as _
            ; mov [rax], rsp

            // Set the stack pointer
            ; mov rsp, QWORD self.harness_stack.as_ptr() as _

            // Zero most of the GPRs before we enter measured code:
            //  - RSI and RDI are passed through as arguments
            //  - R15 is necessarily polluted (for the indirect call)

            ; xor rax, rax
            ; xor rcx, rcx
            ; xor rdx, rdx
            ; xor rbx, rbx
            //; xor rsi, rsi
            //; xor rdi, rdi
            ; xor rbp, rbp
            ; xor  r8, r8
            ; xor  r9, r9
            ; xor r10, r10
            ; xor r11, r11
            ; xor r12, r12
            ; xor r13, r13
            ; xor r14, r14
            //; xor r15, r15
        );

        // Indirectly call the tested function
        dynasm!(harness
            ; call r15
            ; lfence
        );

        if self.dump_gpr {
            dynasm!(harness
                ; mov r15, QWORD self.gpr_state.0.as_ptr() as _
                ; mov [r15 + 0x00], rax
                ; mov [r15 + 0x08], rcx
                ; mov [r15 + 0x10], rdx
                ; mov [r15 + 0x18], rbx
                ; mov [r15 + 0x20], rsp
                ; mov [r15 + 0x28], rbp
                ; mov [r15 + 0x30], rsi
                ; mov [r15 + 0x38], rdi
                ; mov [r15 + 0x40], r8
                ; mov [r15 + 0x48], r9
                ; mov [r15 + 0x50], r10
                ; mov [r15 + 0x58], r11
                ; mov [r15 + 0x60], r12
                ; mov [r15 + 0x68], r13
                ; mov [r15 + 0x70], r14
                ; mov [r15 + 0x78], r15
                ; sfence
            );
        }

        dynasm!(harness
            // Restore the stack pointer
            ; mov rcx, QWORD self.harness_state.as_ptr() as _
            ; mov rsp, [rcx]

            ; pop r15
            ; pop r14
            ; pop r13
            ; pop r12
            ; pop rsi
            ; pop rdi
            ; pop rbx
            ; pop rbp
            ; ret
        );

        let buf = harness.finalize().unwrap();
        disas(&buf);
        println!();

        self.harness_fn = unsafe { 
            std::mem::transmute(buf.ptr(AssemblyOffset(0)))
        };
        self.harness_buf = Some(buf);
        self
    }

    fn make_cfg(event: u16, mask: u8) -> u64 {
        let event_num = event as u64 & 0b1111_1111_1111;
        let event_lo  = event_num & 0b0000_1111_1111;
        let event_hi  = (event_num & 0b1111_0000_0000) >> 8;
        let mask_num  = mask as u64;
        (event_hi << 32) | (mask_num << 8) | event_lo
    }

    pub fn measure(&mut self, 
        f: &mut PerfectFn, 
        event: u16,
        mask: u8,
        iters: usize, 
        rdi: usize,
        rsi: usize
    ) -> Result<(Vec<usize>, Option<Vec<GprState>>), &str>
    {
        let harness_func = if let Some(f) = self.harness_fn { f } 
        else { 
            return Err("harness not emitted");
        };

        let reader = f.asm.reader();
        let tgt_buf = reader.lock();
        let tgt_ptr = tgt_buf.ptr(AssemblyOffset(0));
        //println!("{:?}", tgt_ptr);

        let mut results = Vec::new();
        let mut gpr_dumps = if self.dump_gpr { Some(Vec::new()) } else { None };

        let cfg = Self::make_cfg(event, mask);
        let mut ctr = Builder::new()
            .kind(Event::Raw(cfg))
            .build().unwrap();

        for i in 0..iters {
            self.gpr_state.clear();
            ctr.reset().unwrap();
            ctr.enable().unwrap();

            let res = harness_func(rdi, rsi, tgt_ptr as usize);

            ctr.disable().unwrap();
            results.push(res);
            if let Some(ref mut dumps) = gpr_dumps {
                dumps.push(*self.gpr_state);
            }
        }

        Ok((results, gpr_dumps))
    }

    pub fn measure_vary(&mut self, 
        f: &mut PerfectFn, 
        event: u16,
        mask: u8,
        iters: usize, 
        mut func: impl FnMut(&mut ThreadRng) -> (usize, usize),
    ) -> Result<(Vec<usize>, Option<Vec<GprState>>), &str>
    {
        let harness_func = if let Some(f) = self.harness_fn { f } 
        else { 
            return Err("harness not emitted");
        };

        let mut rng = rand::thread_rng();
        let reader = f.asm.reader();
        let tgt_buf = reader.lock();
        let tgt_ptr = tgt_buf.ptr(AssemblyOffset(0));

        let mut results = Vec::new();
        let mut gpr_dumps = if self.dump_gpr { Some(Vec::new()) } else { None };

        let cfg = Self::make_cfg(event, mask);
        let mut ctr = Builder::new()
            .kind(Event::Raw(cfg))
            .build().unwrap();

        for i in 0..iters {
            self.gpr_state.clear();
            ctr.reset().unwrap();
            ctr.enable().unwrap();

            let (rdi, rsi) = func(&mut rng);
            let res = harness_func(rdi, rsi, tgt_ptr as usize);

            ctr.disable().unwrap();
            results.push(res);
            if let Some(ref mut dumps) = gpr_dumps {
                dumps.push(*self.gpr_state);
            }
        }

        Ok((results, gpr_dumps))
    }



}



pub struct PerfectFn {
    pub asm: Assembler<X64Relocation>,
    pub name: &'static str,
}
impl PerfectFn {
    pub fn new(name: &'static str) -> Self { 
        let mut asm = Assembler::<X64Relocation>::new().unwrap();
        Self { asm , name }
    }

    pub fn commit(&mut self) {
        self.asm.commit().unwrap();
    }

    pub fn disas(&mut self) {
        println!("[*] Disassembly for PerfectFn '{}'", self.name);
        self.asm.commit().unwrap();
        let rdr = self.asm.reader();
        let buf = rdr.lock();
        disas(&buf);
    }
}

impl PerfectFn {

    pub fn new_dynamic_label(&mut self) -> DynamicLabel {
        self.asm.new_dynamic_label()
    }
    pub fn place_dynamic_label(&mut self, lab: DynamicLabel) {
        dynasm!(self.asm ; =>lab);
    }

    pub fn emit_lfence(&mut self) { dynasm!(self.asm ; lfence); }
    pub fn emit_mfence(&mut self) { dynasm!(self.asm ; mfence); }
    pub fn emit_sfence(&mut self) { dynasm!(self.asm ; sfence); }
    pub fn emit_clflush_base(&mut self, base: u8) {
        dynasm!(self.asm ; clflush [ Rq(base) ]);
    }
    pub fn emit_clflush_base_imm(&mut self, base: u8, imm: i32) {
        dynasm!(self.asm ; clflush [ Rq(base) + imm ]);
    }


    pub fn emit_load_r64_base(&mut self, dst: u8, base: u8) {
        dynasm!(self.asm ; mov Rq(dst), [ Rq(base) ]);
    }
    pub fn emit_load_r64_base_imm(&mut self, dst: u8, base: u8, imm: i32) {
        dynasm!(self.asm ; mov Rq(dst), [ Rq(base) + imm ]);
    }
    pub fn emit_store_base_r64(&mut self, base: u8, src: u8) {
        dynasm!(self.asm ; mov [ Rq(base) ], Rq(src) );
    }
    pub fn emit_store_base_imm_r64(&mut self, base: u8, imm: i32, src: u8) {
        dynasm!(self.asm ; mov [ Rq(base) + imm ], Rq(src) );
    }

    pub fn emit_mov_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self.asm ; mov Rq(dst), Rq(src));
    }
    pub fn emit_mov_r64_i32(&mut self, dst: u8, imm: i32) {
        dynasm!(self.asm ; mov Rq(dst), imm);
    }
    pub fn emit_mov_r64_i64(&mut self, dst: u8, qword: i64) {
        dynasm!(self.asm ; mov Rq(dst), QWORD qword);
    }

    pub fn emit_add_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self.asm ; add Rq(dst), Rq(src));
    }
    pub fn emit_sub_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self.asm ; sub Rq(dst), Rq(src));
    }
    pub fn emit_and_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self.asm ; and Rq(dst), Rq(src));
    }
    pub fn emit_or_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self.asm ; or Rq(dst), Rq(src));
    }
    pub fn emit_xor_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self.asm ; xor Rq(dst), Rq(src));
    }

    pub fn emit_dec_r64(&mut self, dst: u8) {
        dynasm!(self.asm ; dec Rq(dst));
    }
    pub fn emit_inc_r64(&mut self, dst: u8) {
        dynasm!(self.asm ; dec Rq(dst));
    }


    pub fn emit_cmp_r64_imm(&mut self, dst: u8, imm: i32) {
        dynasm!(self.asm ; cmp Rq(dst), imm);
    }
    pub fn emit_add_r64_imm(&mut self, dst: u8, imm: i32) {
        dynasm!(self.asm ; add Rq(dst), imm);
    }
    pub fn emit_sub_r64_imm(&mut self, dst: u8, imm: i32) {
        dynasm!(self.asm ; sub Rq(dst), imm);
    }
    pub fn emit_and_r64_imm(&mut self, dst: u8, imm: i32) {
        dynasm!(self.asm ; and Rq(dst), imm);
    }
    pub fn emit_or_r64_imm(&mut self, dst: u8, imm: i32) {
        dynasm!(self.asm ; or Rq(dst), imm);
    }


    pub fn emit_ret(&mut self) { 
        dynasm!(self.asm ; ret); 
    }
    pub fn emit_jmp_indirect(&mut self, reg: u8) {
        dynasm!(self.asm ; jmp Rq(reg));
    }
    pub fn emit_call_indirect(&mut self, reg: u8) {
        dynasm!(self.asm ; call Rq(reg));
    }
    pub fn emit_jmp_label(&mut self, lab: DynamicLabel) {
        dynasm!(self.asm ; jmp =>lab);
    }
    pub fn emit_je_label(&mut self, lab: DynamicLabel) {
        dynasm!(self.asm ; je =>lab);
    }
    pub fn emit_jne_label(&mut self, lab: DynamicLabel) {
        dynasm!(self.asm ; jne =>lab);
    }
    pub fn emit_jz_label(&mut self, lab: DynamicLabel) {
        dynasm!(self.asm ; jz =>lab);
    }
    pub fn emit_jnz_label(&mut self, lab: DynamicLabel) {
        dynasm!(self.asm ; jnz =>lab);
    }
    pub fn emit_jge_label(&mut self, lab: DynamicLabel) {
        dynasm!(self.asm ; jge =>lab);
    }
    pub fn emit_jle_label(&mut self, lab: DynamicLabel) {
        dynasm!(self.asm ; jle =>lab);
    }
    pub fn emit_call_label(&mut self, lab: DynamicLabel) {
        dynasm!(self.asm ; call =>lab);
    }

    pub fn emit_lea_r64_label(&mut self, dst: u8, lab: DynamicLabel) {
        dynasm!(self.asm ; lea Rq(dst), [ =>lab ]);
    }



    pub fn emit_nop_sled(&mut self, n: usize) {
        for _ in 0..n { dynasm!(self.asm ; nop) }
    }
    pub fn emit_fnop_sled(&mut self, n: usize) {
        for _ in 0..n { dynasm!(self.asm ; fnop) }
    }
    pub fn emit_jmp_sled(&mut self, n: usize) {
        for _ in 0..n {
            dynasm!(self.asm
                ; jmp >label
                ; label:
            );
        }
    }

    pub fn emit_dis_pad_i5(&mut self) {
        dynasm!(self.asm ; nop ; nop ; nop ; nop ; nop);
    }
    pub fn emit_dis_pad_i1f4(&mut self) {
        dynasm!(self.asm ; nop ; fnop ; fnop ; fnop ; fnop);
    }

    pub fn emit_rdtsc_start(&mut self, scratch: u8) {
        dynasm!(self.asm 
            ; lfence
            ; rdtsc
            ; lfence
            ; sub Rq(scratch), rax
            ; xor rax, rax
            ; xor rdx, rdx
            ; lfence
        );
    }

    pub fn emit_rdtsc_end(&mut self, scratch: u8, result: u8) {
        dynasm!(self.asm 
            ; lfence
            ; rdtsc
            ; lfence
            ; add Rq(result), Rq(scratch)
            ; lfence
        );
    }




    pub fn emit_rdpmc_start(&mut self, counter: i32, scratch: u8) {
        dynasm!(self.asm 
            ; lfence
            ; mov rcx, counter
            ; xor Rq(scratch), Rq(scratch)
            ; lfence
            ; rdpmc
            ; lfence
            ; sub Rq(scratch), rax
            ; xor rax, rax
            ; xor rdx, rdx
            ; xor rcx, rcx
            ; lfence
        );
    }

    pub fn emit_rdpmc_end(&mut self, counter: i32, scratch: u8, result: u8) {
        dynasm!(self.asm 
            ; lfence
            ; mov rcx, counter
            ; lfence
            ; rdpmc
            ; lfence
            ; add Rq(result), Rq(scratch)
            ; lfence
        );
    }

}


