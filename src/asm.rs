
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

/// This is supposed to mimic the behavior of [dynasmrt::Assembler], 
/// but where the size and address of the backing memory is fixed.
pub struct PerfectAsm {
    /// Pointer to backing allocation
    pub ptr: *const u8,
    /// Size of backing allocation
    pub len: usize,
    /// Number of bytes written to backing allocation
    pub cursor: usize,
    /// Temporary buffer for the assembler
    pub ops: Vec<u8>,

    pub labels: LabelRegistry,
    pub relocs: RelocRegistry<X64Relocation>,
    pub managed: ManagedRelocs<X64Relocation>,
    pub error: Option<DynasmError>,
}
impl PerfectAsm { 

    /// Obtain a anonymous fixed mapping at the requested address. 
    fn mmap_fixed(req_addr: usize, len: usize) -> *mut u8 {
        let addr = std::num::NonZeroUsize::new(req_addr);
        let len  = std::num::NonZeroUsize::new(len).unwrap();
        let prot = ProtFlags::PROT_READ 
                 | ProtFlags::PROT_WRITE
                 | ProtFlags::PROT_EXEC;
        let flag = MapFlags::MAP_ANONYMOUS
                 | MapFlags::MAP_PRIVATE 
                 | MapFlags::MAP_FIXED;
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
            let buf = &mut self.ops[loc.range(self.cursor)];
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
            let buf = &mut self.ops[loc.range(self.cursor)];
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

impl PerfectAsm { 
    pub fn new(addr: usize, len: usize) -> Self {
        let ptr = Self::mmap_fixed(addr, len);
        Self { 
            ptr, 
            len,
            cursor: 0,
            ops: Vec::new(),
            labels: LabelRegistry::new(),
            relocs: RelocRegistry::new(),
            managed: ManagedRelocs::new(),
            error: None,
        }
    }

    pub fn mprotect(&mut self, prot: ProtFlags) {
        unsafe { 
            mprotect(self.ptr as *mut std::ffi::c_void, self.len, prot)
                .unwrap()
        }
    }

    pub fn new_dynamic_label(&mut self) -> DynamicLabel { 
        self.labels.new_dynamic_label()
    }

    /// Write assembler to backing memory.
    pub fn commit(&mut self) -> Result<(), &str> {
        if self.ops.len() > self.len {
            return Err("Assembled code doesn't fit into backing allocation");
        }
        if let Err(e) = self.encode_relocs() {
            return Err("Failed to encode relocations");
        }

        self.cursor = self.ops.len();
        let buf: &mut [u8]  = unsafe { 
            std::slice::from_raw_parts_mut(self.ptr as *mut u8, self.cursor)
        };
        buf.copy_from_slice(&self.ops);
        Ok(())
    }

    /// Disassemble bytes in the backing allocation. 
    pub fn disas(&self) {
        let ptr: *const u8 = self.ptr;
        let addr: u64   = self.ptr as u64;
        let buf: &[u8]  = unsafe { 
            std::slice::from_raw_parts(ptr, self.cursor)
        };

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
}

/// Presumably we want to call `munmap` when this object is destroyed. 
impl Drop for PerfectAsm { 
    fn drop(&mut self) {
        self.unmap();
    }
}

// Required for implementing [DynasmApi].
impl Extend<u8> for PerfectAsm {
    fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item=u8> {
        self.ops.extend(iter)
    }
}
// Required for implementing [DynasmApi].
impl <'a> Extend<&'a u8> for PerfectAsm {
    fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item=&'a u8> {
        self.ops.extend(iter)
    }
}

// NOTE: [DynasmApi] kind of assumes that the size of backing memory is going 
// to be extensible (like a [Vec]); we probably want to just panic in all of 
// the cases where the requested assembly would become larger than the size of 
// backing memory. 
impl DynasmApi for PerfectAsm {
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

impl DynasmLabelApi for PerfectAsm {
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
        let buf = &mut self.ops[loc.range(self.cursor)];
        if loc.patch(buf, self.ptr as usize, target).is_err() {
            self.error = Some(
                DynasmError::ImpossibleRelocation(TargetKind::Extern(target))
            )
        } else if loc.needs_adjustment() {
            self.managed.add(loc)
        }
    }
}

/// Utility functions that you might want on something implementing [DynasmApi].
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
        dynasm!(self ; lea Rq(dst), [ =>lab ]);
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
}

impl Emitter for Assembler<X64Relocation> {}
impl Emitter for PerfectAsm {}

