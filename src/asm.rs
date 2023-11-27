
use dynasmrt::{
    DynasmApi,
    DynasmLabelApi,
    DynasmError,
    AssemblyOffset,
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

pub struct PerfectBuffer {
    /// Pointer to backing allocation
    pub ptr: *const u8,
    /// Size of backing allocation
    pub len: usize,
    /// Number of bytes written to backing allocation
    pub cursor: usize,

    pub ops: Vec<u8>,

    pub labels: LabelRegistry,
    pub relocs: RelocRegistry<X64Relocation>,
    pub managed: ManagedRelocs<X64Relocation>,
    pub error: Option<DynasmError>,
}
impl PerfectBuffer { 

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

impl PerfectBuffer { 
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

    /// Unmap the backing allocation.
    pub fn unmap(&mut self) { 
        unsafe { 
            munmap(self.ptr as *mut std::ffi::c_void, self.len).unwrap();
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

impl Drop for PerfectBuffer { 
    fn drop(&mut self) {
        self.unmap();
    }
}

impl Extend<u8> for PerfectBuffer {
    fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item=u8> {
        self.ops.extend(iter)
    }
}
impl <'a> Extend<&'a u8> for PerfectBuffer {
    fn extend<T>(&mut self, iter: T) where T: IntoIterator<Item=&'a u8> {
        self.ops.extend(iter)
    }
}

impl DynasmApi for PerfectBuffer {
    fn offset(&self) -> AssemblyOffset {
        AssemblyOffset(self.ops.len())
    }
    fn push(&mut self, byte: u8) {
        self.ops.push(byte);
    }

    fn align(&mut self, alignment: usize, with: u8) {
        let misalign = self.offset().0 % alignment;
        if misalign != 0 {
            for _ in misalign..alignment {
                self.push(with);
            }
        }
    }
}

impl DynasmLabelApi for PerfectBuffer {
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


