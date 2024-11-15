
use dynasmrt::{
    DynasmApi,
    DynasmLabelApi,
    DynasmError,
    AssemblyOffset,
    Assembler,
    dynasm,
    x64::X64Relocation,
    DynamicLabel,
    LabelKind,
    TargetKind,
};
use dynasmrt::components::{
    LabelRegistry,
    RelocRegistry,
    ManagedRelocs,
    StaticLabel,
    PatchLoc,
};
use iced_x86::{ 
    Decoder, DecoderOptions, Instruction, Formatter, IntelFormatter 
};

use nix::sys::mman::{ ProtFlags, MapFlags, mmap, munmap, mprotect };

use rand::Rng;
use rand::distributions::{ Distribution, Standard };

use crate::MeasuredFn;
use crate::HarnessFn;

/// Fallback/default assembler from [dynasmrt]. 
pub type X64Assembler = Assembler<X64Relocation>;

/// This is supposed to mimic the behavior of [dynasmrt::Assembler], 
/// but where the size and address of the backing memory is fixed.
pub struct X64AssemblerFixed {
    /// Pointer to backing allocation
    pub ptr: *const u8,
    /// Size of backing allocation
    pub len: usize,
    /// Temporary buffer for the assembler
    pub ops: Vec<u8>,

    pub labels: LabelRegistry,
    pub relocs: RelocRegistry<X64Relocation>,
    pub managed: ManagedRelocs<X64Relocation>,
    pub error: Option<DynasmError>,
    pub committed: bool,
}
impl X64AssemblerFixed { 

    /// Obtain a anonymous fixed mapping at the requested address. 
    ///
    /// NOTE: Userspace ends at `0000_7fff_ffff_ffff`. 
    /// See https://www.kernel.org/doc/html/latest/arch/x86/x86_64/mm.html
    fn mmap_fixed(req_addr: usize, len: usize) -> *mut u8 {
        assert!(req_addr + len < 0x0000_7fff_ffff_ffff,
            "Requested allocation {:016x}-{:016x} exceeds userspace limits?",
            req_addr, req_addr + len
        );
        let addr = std::num::NonZeroUsize::new(req_addr);
        let len  = std::num::NonZeroUsize::new(len).unwrap();
        let prot = ProtFlags::PROT_READ 
                 | ProtFlags::PROT_WRITE
                 | ProtFlags::PROT_EXEC;
        let flag = MapFlags::MAP_ANONYMOUS
                 | MapFlags::MAP_PRIVATE 
                 | MapFlags::MAP_FIXED;
                 //| MapFlags::MAP_POPULATE;
        let fd   = 0;
        let off  = 0;
        let ptr  = unsafe { 
            mmap(addr, len, prot, flag, fd, off).unwrap() 
        };
        assert!(ptr as usize == req_addr);
        ptr as *mut u8
    }

    /// Unmap the backing allocation.
    fn unmap(&mut self) { 
        unsafe { 
            munmap(self.ptr as *mut std::ffi::c_void, self.len).unwrap();
        }
    }

    fn encode_relocs(&mut self) -> Result<(), DynasmError> {
        for (loc, label) in self.relocs.take_statics() {
            let target = self.labels.resolve_static(&label)?;
            let buf = &mut self.ops[loc.range(0)];
            if loc.patch(buf, self.ptr as usize, target.0).is_err() {
                return Err(DynasmError::ImpossibleRelocation(
                    if label.is_global() {
                        TargetKind::Global(label.get_name())
                    } else {
                        TargetKind::Local(label.get_name())
                    }
                ));
            }
            if loc.needs_adjustment() { 
                self.managed.add(loc) 
            }
        }

        for (loc, id) in self.relocs.take_dynamics() {
            let target = self.labels.resolve_dynamic(id)?;
            let buf = &mut self.ops[loc.range(0)];
            if loc.patch(buf, self.ptr as usize, target.0).is_err() {
                return Err(
                    DynasmError::ImpossibleRelocation(TargetKind::Dynamic(id))
                );
            }
            if loc.needs_adjustment() { 
                self.managed.add(loc) 
            }
        }
        Ok(())
    }
}

impl X64AssemblerFixed { 
    pub fn new(addr: usize, len: usize) -> Self {
        let ptr = Self::mmap_fixed(addr, len);
        Self { 
            ptr, 
            len,
            ops: Vec::new(),
            labels: LabelRegistry::new(),
            relocs: RelocRegistry::new(),
            managed: ManagedRelocs::new(),
            error: None,
            committed: false,
        }
    }

    /// Return the base address for the block of emitted code. 
    pub fn base_addr(&self) -> usize { self.ptr as usize }

    /// Return the last address for the block of emitted code.
    pub fn max_addr(&self) -> usize { self.base_addr() + self.len }

    /// Return the current offset (in bytes). 
    pub fn cursor(&self) -> usize { self.ops.len() }

    /// Return the current virtual address. 
    pub fn cur_addr(&self) -> usize { self.base_addr() + self.cursor() }

    /// Return a function pointer [MeasuredFn] to this block of emitted code.
    pub fn as_fn(&self) -> MeasuredFn {
        assert!(self.committed);
        unsafe { std::mem::transmute(self.ptr) }
    }

    pub fn as_extern_fn(&self) -> extern "C" fn(usize,usize) -> usize { 
        assert!(self.committed);
        unsafe { std::mem::transmute(self.ptr) }
    }

    /// Return a function pointer [HarnessFn] to this block of emitted code.
    pub fn as_harness_fn(&self) -> HarnessFn {
        assert!(self.committed);
        unsafe { std::mem::transmute(self.ptr) }
    }


    pub fn mprotect(&mut self, prot: ProtFlags) {
        unsafe { 
            mprotect(self.ptr as *mut std::ffi::c_void, self.len, prot)
                .unwrap()
        }
    }

    /// Create a new [DynamicLabel]. 
    pub fn new_dynamic_label(&mut self) -> DynamicLabel { 
        self.labels.new_dynamic_label()
    }

    /// Pad with 0xCC until the requested address.
    /// Returns the number of emitted bytes.
    pub fn pad_cc_until(&mut self, addr: usize) -> usize { 
        if self.cur_addr() == addr { 
            return 0;
        }
        assert!(addr > self.cur_addr(),
            "Requested pad target {:016x} must be > {:016x}",
            addr, self.cur_addr(),
        );
        assert!(addr <= self.max_addr(),
            "Requested {:016x} must be <= max addr {:016x}",
            addr, self.max_addr(),
        );
        let mut count = 0;
        let num_padding = addr - self.cur_addr();
        for _ in 0..num_padding {
            dynasm!(self ; .bytes &[0xcc]);
            count += 1;
        }
        count
    }

    /// Pad with NOP (up to the 8-byte encoding) until the requested address. 
    /// Returns the number of emitted bytes. 
    pub fn pad_until(&mut self, addr: usize) -> usize {
        if self.cur_addr() == addr { 
            return 0;
        }

        assert!(addr > self.cur_addr(),
            "Requested pad target {:016x} must be > {:016x}",
            addr, self.cur_addr(),
        );
        assert!(addr <= self.max_addr(),
            "Requested {:016x} must be <= max addr {:016x}",
            addr, self.max_addr(),
        );

        let mut count = 0;
        let num_padding = addr - self.cur_addr();
        let nop8_count = num_padding / 8;
        let rem8 = num_padding % 8;
        for _ in 0..nop8_count {
            dynasm!(self ; .bytes NOP8);
            count += 8;
        }
        match rem8 {
            0 => {},
            1 => dynasm!(self ; nop),
            2 => dynasm!(self ; .bytes NOP2),
            3 => dynasm!(self ; .bytes NOP3),
            4 => dynasm!(self ; .bytes NOP4),
            5 => dynasm!(self ; .bytes NOP5),
            6 => dynasm!(self ; .bytes NOP6),
            7 => dynasm!(self ; .bytes NOP7),
            _ => unreachable!(),
        }
        count += rem8;
        assert_eq!(self.cur_addr(), addr);
        return count;
    }

    /// Write assembler to backing memory.
    pub fn commit(&mut self) -> Result<(), &str> {
        if self.cursor() > self.len {
            return Err("Assembled code doesn't fit into backing allocation");
        }
        if let Err(e) = self.encode_relocs() {
            return Err("Failed to encode relocations");
        }

        let buf: &mut [u8]  = unsafe { 
            std::slice::from_raw_parts_mut(self.ptr as *mut u8, self.cursor())
        };
        self.committed = true;
        buf.copy_from_slice(&self.ops);
        Ok(())
    }

    /// Disassemble bytes in the backing allocation. 
    pub fn disas(&self, offset: AssemblyOffset, max_inst: Option<usize>) {
        let ptr: *const u8 = unsafe { 
            self.ptr.offset(offset.0 as isize)
        };
        let addr: u64   = self.ptr as u64;
        let buf_len = self.cursor() - offset.0;
        let buf: &[u8]  = unsafe { 
            std::slice::from_raw_parts(ptr, buf_len)
        };

        let mut decoder = Decoder::with_ip(64, buf, addr, DecoderOptions::NONE);
        let mut formatter = IntelFormatter::new();
        formatter.options_mut().set_digit_separator("_");
        let _ = formatter.options_mut().first_operand_char_index();
        let mut output = String::new();
        let mut instr  = Instruction::default();

        let mut num_inst = 0;
        while decoder.can_decode() {
            if let Some(max) = max_inst {
                if num_inst >= max { break; }
            }
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
            num_inst += 1;
        }
    }
}

/// Presumably we want to call `munmap` when this object is destroyed. 
impl Drop for X64AssemblerFixed { 
    fn drop(&mut self) {
        self.unmap();
    }
}

// Required for implementing [DynasmApi].
impl Extend<u8> for X64AssemblerFixed {
    fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item=u8> {
        self.ops.extend(iter)
    }
}
// Required for implementing [DynasmApi].
impl <'a> Extend<&'a u8> for X64AssemblerFixed {
    fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item=&'a u8> {
        self.ops.extend(iter)
    }
}

// NOTE: [DynasmApi] kind of assumes that the size of backing memory is going 
// to be extensible (like a [Vec]); we probably want to just panic in all of 
// the cases where the requested assembly would become larger than the size of 
// backing memory. 
impl DynasmApi for X64AssemblerFixed {
    fn offset(&self) -> AssemblyOffset {
        AssemblyOffset(self.ops.len())
    }

    fn push(&mut self, byte: u8) {
        if (self.ops.len() + 1) > self.len { 
            panic!("Assembled code would overflow backing allocation");
        }
        self.ops.push(byte);
    }

    fn align(&mut self, alignment: usize, with: u8) {
        let misalign = self.offset().0 % alignment;
        if (self.ops.len() + (misalign..alignment).len()) > self.len {
            panic!("Assembled code would overflow backing allocation");
        }

        if misalign != 0 {
            for _ in misalign..alignment {
                self.push(with);
            }
        }
    }
}

impl DynasmLabelApi for X64AssemblerFixed {
    type Relocation = X64Relocation;

    fn local_label(&mut self, name: &'static str) {
        let offset = self.offset();
        self.labels.define_local(name, offset);
    }

    fn global_label( &mut self, name: &'static str) {
        let offset = self.offset();
        if let Err(e) = self.labels.define_global(name, offset) {
            self.error = Some(e)
        }
    }

    fn dynamic_label(&mut self, id: DynamicLabel) {
        let offset = self.offset();
        if let Err(e) = self.labels.define_dynamic(id, offset) {
            self.error = Some(e)
        }
    }

    fn global_relocation(&mut self, name: &'static str, 
        target_offset: isize, field_offset: u8, ref_offset: u8, 
        kind: Self::Relocation) 
    {
        let location = self.offset();
        let label = StaticLabel::global(name);
        self.relocs.add_static(label, 
            PatchLoc::new(location, 
                target_offset, field_offset, ref_offset, kind
            )
        );
    }

    fn dynamic_relocation(&mut self, id: DynamicLabel, 
        target_offset: isize, field_offset: u8, ref_offset: u8, 
        kind: Self::Relocation) 
    {
        let location = self.offset();
        self.relocs.add_dynamic(id, 
            PatchLoc::new(
                location, target_offset, field_offset, ref_offset, kind
            )
        );
    }

    fn forward_relocation(&mut self, name: &'static str, 
        target_offset: isize, field_offset: u8, ref_offset: u8, 
        kind: Self::Relocation) 
    {
        let location = self.offset();
        let label = match self.labels.place_local_reference(name) {
            Some(label) => label.next(),
            None => StaticLabel::first(name),
        };
        self.relocs.add_static(label, 
            PatchLoc::new(
                location, target_offset, field_offset, ref_offset, kind
            )
        );
    }

    fn backward_relocation(&mut self, name: &'static str, 
        target_offset: isize, field_offset: u8, ref_offset: u8, 
        kind: Self::Relocation) 
    {
        let location = self.offset();
        let label = match self.labels.place_local_reference(name) {
            Some(label) => label,
            None => {
                self.error = Some(
                    DynasmError::UnknownLabel(LabelKind::Local(name))
                );
                return;
            }
        };
        self.relocs.add_static(label, 
            PatchLoc::new(
                location, target_offset, field_offset, ref_offset, kind
            )
        );
    }

    fn bare_relocation(&mut self, 
        target: usize, field_offset: u8, ref_offset: u8, 
        kind: Self::Relocation) 
    {
        let location = self.offset();
        let loc = PatchLoc::new(location, 0, field_offset, ref_offset, kind);
        //let cursor = self.ops.len();
        //let buf = &mut self.ops[loc.range(cursor)];
        let buf = &mut self.ops[loc.range(0)];
        if loc.patch(buf, self.ptr as usize, target).is_err() {
            self.error = Some(
                DynasmError::ImpossibleRelocation(TargetKind::Extern(target))
            )
        } else if loc.needs_adjustment() {
            self.managed.add(loc)
        }
    }
}

/// Utility functions you might want on something implementing [DynasmApi].
pub trait Emitter: DynasmLabelApi<Relocation=X64Relocation> {
    fn place_dynamic_label(&mut self, lab: DynamicLabel) {
        dynasm!(self ; =>lab);
    }

    fn emit_lfence(&mut self) { dynasm!(self ; lfence ); }
    fn emit_mfence(&mut self) { dynasm!(self ; mfence); }
    fn emit_sfence(&mut self) { dynasm!(self ; sfence); }
    fn emit_clflush_base(&mut self, base: u8) {
        dynasm!(self ; clflush [ Rq(base) ]);
    }
    fn emit_clflush_base_imm(&mut self, base: u8, imm: i32) {
        dynasm!(self ; clflush [ Rq(base) + imm ]);
    }


    fn emit_load_r64_base(&mut self, dst: u8, base: u8) {
        dynasm!(self ; mov Rq(dst), [ Rq(base) ]);
    }
    fn emit_load_r64_base_imm(&mut self, dst: u8, base: u8, imm: i32) {
        dynasm!(self ; mov Rq(dst), [ Rq(base) + imm ]);
    }
    fn emit_store_base_r64(&mut self, base: u8, src: u8) {
        dynasm!(self ; mov [ Rq(base) ], Rq(src) );
    }
    fn emit_store_base_imm_r64(&mut self, base: u8, imm: i32, src: u8) {
        dynasm!(self ; mov [ Rq(base) + imm ], Rq(src) );
    }

    fn emit_mov_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self ; mov Rq(dst), Rq(src));
    }
    fn emit_mov_r64_i32(&mut self, dst: u8, imm: i32) {
        dynasm!(self ; mov Rq(dst), imm);
    }
    fn emit_mov_r64_i64(&mut self, dst: u8, qword: i64) {
        dynasm!(self ; mov Rq(dst), QWORD qword);
    }

    fn emit_add_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self ; add Rq(dst), Rq(src));
    }
    fn emit_sub_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self ; sub Rq(dst), Rq(src));
    }
    fn emit_and_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self ; and Rq(dst), Rq(src));
    }
    fn emit_or_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self ; or Rq(dst), Rq(src));
    }
    fn emit_xor_r64_r64(&mut self, dst: u8, src: u8) {
        dynasm!(self ; xor Rq(dst), Rq(src));
    }

    fn emit_dec_r64(&mut self, dst: u8) {
        dynasm!(self ; dec Rq(dst));
    }
    fn emit_inc_r64(&mut self, dst: u8) {
        dynasm!(self ; dec Rq(dst));
    }


    fn emit_cmp_r64_imm(&mut self, dst: u8, imm: i32) {
        dynasm!(self ; cmp Rq(dst), imm);
    }
    fn emit_add_r64_imm(&mut self, dst: u8, imm: i32) {
        dynasm!(self ; add Rq(dst), imm);
    }
    fn emit_sub_r64_imm(&mut self, dst: u8, imm: i32) {
        dynasm!(self ; sub Rq(dst), imm);
    }
    fn emit_and_r64_imm(&mut self, dst: u8, imm: i32) {
        dynasm!(self ; and Rq(dst), imm);
    }
    fn emit_or_r64_imm(&mut self, dst: u8, imm: i32) {
        dynasm!(self ; or Rq(dst), imm);
    }


    fn emit_ret(&mut self) { 
        dynasm!(self ; ret); 
    }
    fn emit_jmp_indirect(&mut self, reg: u8) {
        dynasm!(self ; jmp Rq(reg));
    }
    fn emit_call_indirect(&mut self, reg: u8) {
        dynasm!(self ; call Rq(reg));
    }
    fn emit_jmp_label(&mut self, lab: DynamicLabel) {
        dynasm!(self ; jmp =>lab);
    }
    fn emit_je_label(&mut self, lab: DynamicLabel) {
        dynasm!(self ; je =>lab);
    }
    fn emit_jne_label(&mut self, lab: DynamicLabel) {
        dynasm!(self ; jne =>lab);
    }
    fn emit_jz_label(&mut self, lab: DynamicLabel) {
        dynasm!(self ; jz =>lab);
    }
    fn emit_jnz_label(&mut self, lab: DynamicLabel) {
        dynasm!(self ; jnz =>lab);
    }
    fn emit_jge_label(&mut self, lab: DynamicLabel) {
        dynasm!(self ; jge =>lab);
    }
    fn emit_jle_label(&mut self, lab: DynamicLabel) {
        dynasm!(self ; jle =>lab);
    }
    fn emit_call_label(&mut self, lab: DynamicLabel) {
        dynasm!(self ; call =>lab);
    }

    fn emit_lea_r64_label(&mut self, dst: u8, lab: DynamicLabel) {
        dynasm!(self ; lea Rq(dst), [=>lab]);
    }



    fn emit_nop_sled(&mut self, n: usize) {
        for _ in 0..n { dynasm!(self ; nop) }
    }
    fn emit_fnop_sled(&mut self, n: usize) {
        for _ in 0..n { dynasm!(self ; fnop) }
    }
    fn emit_jmp_sled(&mut self, n: usize) {
        for _ in 0..n {
            dynasm!(self
                ; jmp >label
                ; label:
            );
        }
    }

    fn emit_dis_pad_i5(&mut self) {
        dynasm!(self ; nop ; nop ; nop ; nop ; nop);
    }
    fn emit_dis_pad_i1f4(&mut self) {
        dynasm!(self ; nop ; fnop ; fnop ; fnop ; fnop);
    }

    fn emit_rdtsc_start(&mut self, scratch: u8) {
        dynasm!(self 
            ; lfence
            ; rdtsc
            ; lfence
            ; xor Rq(scratch), Rq(scratch)
            ; sub Rq(scratch), rax
            ; xor rax, rax
            ; xor rdx, rdx
            ; lfence
        );
    }

    fn emit_rdtsc_end(&mut self, scratch: u8, result: u8) {
        dynasm!(self 
            ; lfence
            ; rdtsc
            ; lfence
            ; add Rq(result), Rq(scratch)
            ; lfence
        );
    }


    /// Start a measurement by emitting RDPMC, then moving the result into 
    /// some scratch register which is expected to live at least until 
    /// the second measurement (which must be emitted with `emit_rdpmc_end`).
    ///
    /// NOTE: You should avoid implementing this with instructions that might 
    /// change the state of the flags, or the state of any other register 
    /// apart from the provided scratch register. RAX, RCX, and the scratch
    /// register are necessarily clobbered here (unless you want to assume 
    /// the value of RCX at some point - maybe something to think about
    /// later if you want to measure with multiple counters). 
    ///
    /// NOTE: This block of code is 0x18 bytes. 
    ///
    /// NOTE: This [presumably] allocates two physical registers: one for RCX, 
    /// and one for the result of RDPMC in RAX.
    ///
    fn emit_rdpmc_start(&mut self, counter: i32, scratch: u8) {
        dynasm!(self 
            ; lfence
            ; mov rcx, counter
            ; lfence
            ; rdpmc
            ; lfence
            ; mov Rq(scratch), rax
            ; lfence
        );
    }

    /// End the measurement by emitting RDPMC, taking the difference with a 
    /// previous measurement held in some scratch register, and placing the
    /// result in the given result register.
    fn emit_rdpmc_end(&mut self, counter: i32, scratch: u8, result: u8) {
        dynasm!(self 
            ; lfence
            ; mov rcx, counter
            ; lfence
            ; rdpmc
            ; lfence
            ; sub Rq(result), Rq(scratch)
            ; lfence
        );
    }

    fn emit_rdpmc_to_addr(&mut self, counter: i32, addr: i32) {
        dynasm!(self
            ; lfence
            ; mov rcx, counter
            ; lfence
            ; rdpmc
            ; lfence
            ; mov [addr], rax
            ; lfence
        );
    }

    fn emit_rdpmc_start64(&mut self, counter: i32, scratch: u8) {
        dynasm!(self
            ; lfence
            ; mov rcx, counter
            ; lfence
            ; rdpmc
            ; lfence
            ; shl rdx, 32
            ; or rax, rdx
            ; mov Rq(scratch), rax
            ; lfence
        )
    }
    fn emit_rdpmc_end64(&mut self, counter: i32, scratch: u8, result: u8) {
        dynasm!(self 
            ; lfence
            ; mov rcx, counter
            ; lfence
            ; rdpmc
            ; lfence
            ; shl rdx, 32
            ; or rax, rdx
            ; sub rax, Rq(scratch)
            ; mov Rq(result), rax
            ; lfence
        );
    }

    fn emit_flush_btb(&mut self, iter: usize) {
        for _ in 0..iter { 
            dynasm!(self ; jmp >flush_next; flush_next: );
        }
    }
}

// Implement [Emitter] for all of the JIT assemblers we care about
impl Emitter for X64Assembler {}
impl Emitter for X64AssemblerFixed {}


// Various flavors of NOP encoding (these are from the AMD Family 17h SOG). 
pub const NOP2:  [u8; 2] = [ 0x66, 0x90 ];
pub const NOP3:  [u8; 3] = [ 0x0f, 0x1f, 0x00 ];
pub const NOP4:  [u8; 4] = [ 0x0f, 0x1f, 0x40, 0x00 ];
pub const NOP5:  [u8; 5] = [ 0x0f, 0x1f, 0x44, 0x00, 0x00 ];
pub const NOP6:  [u8; 6] = [ 0x66, 0x0f, 0x1f, 0x44, 0x00, 0x00 ];
pub const NOP7:  [u8; 7] = [ 0x0f, 0x1f, 0x80, 0x00, 0x00, 0x00, 0x00 ];
pub const NOP8:  [u8; 8] = [ 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00 ];
pub const NOP9:  [u8; 9] = [ 
    0x66, 
    0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00 
];
pub const NOP10:  [u8; 10] = [ 
    0x66, 0x66, 
    0x0F, 0x1F, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00
];
pub const NOP11:  [u8; 11] = [ 
    0x66, 0x66, 0x66, 
    0x0F, 0x1F, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00
];
pub const NOP12:  [u8; 12] = [ 
    0x66, 0x66, 0x66, 0x66, 
    0x0F, 0x1F, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00
];
pub const NOP13:  [u8; 13] = [ 
    0x66, 0x66, 0x66, 0x66, 0x66, 
    0x0F, 0x1F, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00
];
pub const NOP14:  [u8; 14] = [ 
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 
    0x0F, 0x1F, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00
];
pub const NOP15: [u8; 15] = [ 
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 
    0x0F, 0x1F, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00
];

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum VectorGpr {
    YMM0 = 0,
    YMM1 = 1,
    YMM2 = 2,
    YMM3 = 3,
    YMM4 = 4,
    YMM5 = 5,
    YMM6 = 6,
    YMM7 = 7,
    YMM8 = 8,
    YMM9 = 9,
    YMM10 = 10,
    YMM11 = 11,
    YMM12 = 12,
    YMM13 = 13,
    YMM14 = 14,
    YMM15 = 15,
}
impl VectorGpr {
    pub fn as_usize(&self) -> usize { 
        *self as usize
    }
}
impl From<u8> for VectorGpr {
    fn from(x: u8) -> Self {
        match x {
            0  => Self::YMM0,
            1  => Self::YMM1,
            2  => Self::YMM2,
            3  => Self::YMM3,
            4  => Self::YMM4,
            5  => Self::YMM5,
            6  => Self::YMM6,
            7  => Self::YMM7,
            8  => Self::YMM8,
            9  => Self::YMM9,
            10 => Self::YMM10,
            11 => Self::YMM11,
            12 => Self::YMM12,
            13 => Self::YMM13,
            14 => Self::YMM14,
            15 => Self::YMM15,
            _ => unreachable!(),
        }
    }
}



#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum Gpr {
    Rax = 0,
    Rcx = 1,
    Rdx = 2,
    Rbx = 3,
    Rsp = 4,
    Rbp = 5,
    Rsi = 6,
    Rdi = 7,
    R8  = 8,
    R9  = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
}
impl Gpr {
    pub fn as_usize(&self) -> usize { 
        *self as usize
    }
}
impl From<u8> for Gpr {
    fn from(x: u8) -> Self {
        match x {
            0 => Self::Rax,
            1 => Self::Rcx,
            2 => Self::Rdx,
            3 => Self::Rbx,
            4 => Self::Rsp,
            5 => Self::Rbp,
            6 => Self::Rsi,
            7 => Self::Rdi,
            8 => Self::R8,
            9 => Self::R9,
            10 => Self::R10,
            11 => Self::R11,
            12 => Self::R12,
            13 => Self::R13,
            14 => Self::R14,
            15 => Self::R15,
            _ => unreachable!(),
        }
    }
}

impl Distribution<Gpr> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Gpr {
        // assume r15 is reserved
        let r = rng.gen_range(0..=15);
        match r {
            0 => Gpr::Rax,
            1 => Gpr::Rcx,
            2 => Gpr::Rdx,
            3 => Gpr::Rbx,
            4 => Gpr::Rax,
            5 => Gpr::Rcx,
            6 => Gpr::Rsi,
            7 => Gpr::Rdi,
            8 => Gpr::R8,
            9 => Gpr::R9,
            10 => Gpr::R10,
            11 => Gpr::R11,
            12 => Gpr::R12,
            13 => Gpr::R13,
            14 => Gpr::R14,
            15 => Gpr::R15,
            _ => unreachable!(),
        }
    }
}


