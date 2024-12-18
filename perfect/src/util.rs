
use std::io::Read;
use dynasmrt::{
    ExecutableBuffer,
    AssemblyOffset,
};
use iced_x86::{ 
    Decoder, DecoderOptions, Instruction, Formatter, IntelFormatter 
};
use perf_event::{ Builder, Group, Counter };
use perf_event::events::*;
use perf_event::hooks::sys::bindings::perf_event_mmap_page;
use crate::harness::{PerfectHarness, TargetPlatform};
use crate::events::EventDesc;


/// Utilities for controlling the state of the current process.
pub struct PerfectEnv; 
impl PerfectEnv {
    const BOOST_PATH: &'static str = 
        "/sys/devices/system/cpu/cpufreq/boost";
    const ISOLATED_PATH: &'static str = 
        "/sys/devices/system/cpu/isolated";
    const NOHZ_PATH: &'static str = 
        "/sys/devices/system/cpu/nohz_full";
    const RDPMC_PATH: &'static str = 
        "/sys/bus/event_source/devices/cpu/rdpmc";
    const SMT_PATH: &'static str = 
        "/sys/devices/system/cpu/smt/control";
    const MMAP_MIN_PATH: &'static str = 
        "/proc/sys/vm/mmap_min_addr";


    /// Return a string describing the set of isolated cores.
    pub fn sysfs_isolated() -> String { 
        let mut f = std::fs::File::open(Self::ISOLATED_PATH).unwrap();
        let mut res = String::new();
        f.read_to_string(&mut res).unwrap();
        match res.trim() {
            "" => "disabled".to_string(),
            _ => res.trim().to_string(),
        }
    }

    /// Return a string describing the set of 'nohz_full' cores.
    pub fn sysfs_nohz() -> String { 
        let mut f = std::fs::File::open(Self::NOHZ_PATH).unwrap();
        let mut res = String::new();
        f.read_to_string(&mut res).unwrap();
        match res.trim() {
            "" => "disabled".to_string(),
            _ => res.trim().to_string(),
        }
    }


    /// Returns true if cpufreq boost is enabled.
    pub fn sysfs_cpufreq_boost_enabled() -> bool { 
        let mut f = std::fs::File::open(Self::BOOST_PATH).unwrap();
        let mut res = String::new();
        f.read_to_string(&mut res).unwrap();
        match res.trim() {
            "0" => false,
            "1" => true,
            _ => unreachable!("{:02x?}", res.as_bytes()),
        }
    }

    /// Return a string describing the cpufreq scaling strategy for a 
    /// particular core. 
    pub fn sysfs_cpufreq_governor(n: usize) 
        -> Result<String, std::io::ErrorKind> 
    { 
        let path = format!(
            "/sys/devices/system/cpu/cpufreq/policy{}/scaling_governor", n
        );
        let mut f = std::fs::File::open(path).map_err(|e| e.kind())?;
        let mut res = String::new();
        f.read_to_string(&mut res).unwrap();
        Ok(res.trim().to_string())
    }


    /// Return the minimum supported `mmap()` address
    pub fn procfs_mmap_min_addr() -> usize { 
        let mut f = std::fs::File::open(Self::MMAP_MIN_PATH).unwrap();
        let mut res = String::new();
        f.read_to_string(&mut res).unwrap();
        res.trim().parse().unwrap()
    }

    /// Returns true if RDPMC usage is enabled.
    pub fn sysfs_rdpmc_enabled() -> Result<bool, std::io::ErrorKind> {
        use std::io::Error;
        let mut f = std::fs::File::open(Self::RDPMC_PATH)
            .map_err(|e| e.kind())?;
        let mut res = String::new();
        f.read_to_string(&mut res).unwrap();
        match res.trim() {
            "1" => Ok(false),
            "2" => Ok(true),
            _ => unreachable!("{:02x?}", res.as_bytes()),
        }
    }

    /// Returns true if SMT is enabled.
    pub fn sysfs_smt_enabled() -> bool { 
        let mut f = std::fs::File::open(Self::SMT_PATH).unwrap();
        let mut res = String::new();
        f.read_to_string(&mut res).unwrap();
        match res.trim() {
            "off" => false,
            "on" => true,
            _ => unreachable!("{:02x?}", res.as_bytes()),
        }
    }

    /// Pin to a particular core.
    pub fn pin_to_core(core: usize) {
        let this_pid = nix::unistd::Pid::from_raw(0);
        let mut cpuset = nix::sched::CpuSet::new();
        cpuset.set(core).unwrap();
        nix::sched::sched_setaffinity(this_pid, &cpuset).unwrap();
    }

    /// Migrate the current PID to a dedicated cpuset. 
    pub fn pin_to_cpuset() {
        use std::io::prelude::*;

        let pid = std::process::id();
        let pid = format!("{}\n", pid);

        // Just unwrap errors for now ..
        let mut f = std::fs::File::options().write(true)
            .open("/sys/fs/cgroup/cpuset/perfect/tasks").unwrap();
        f.write(pid.as_bytes()).unwrap();
    }

    /// Explicitly map some region into the current virtual address space.
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
}

pub fn disas_single(buf: &ExecutableBuffer, offset: AssemblyOffset)
    -> (String, String)
{
    let ptr: *const u8 = buf.ptr(offset);
    let addr: u64   = ptr as u64;
    let buf: &[u8]  = unsafe { 
        std::slice::from_raw_parts(ptr, buf.len() - offset.0)
    };

    let mut decoder = Decoder::with_ip(64, buf, 0, DecoderOptions::NONE);
    let mut formatter = IntelFormatter::new();
    formatter.options_mut().set_digit_separator("_");
    let _ = formatter.options_mut().first_operand_char_index();

    let mut instr  = Instruction::default();
    let mut bstr = String::new();
    let mut istr = String::new();
    if decoder.can_decode() {
        decoder.decode_out(&mut instr);
        formatter.format(&instr, &mut istr);

        let start_idx = (instr.ip() - 0) as usize;
        let instr_bytes = &buf[start_idx..start_idx + instr.len()];
        for b in instr_bytes.iter() {
            bstr.push_str(&format!("{:02x}", b));
        }
    }
    (istr, bstr)
}

pub fn disas_chunk(buf: &ExecutableBuffer, 
    start_offset: AssemblyOffset,
    end_offset: AssemblyOffset,
) -> Vec<(String, String, bool)>
{
    let ptr: *const u8 = buf.ptr(start_offset);
    let addr: u64   = ptr as u64;
    let buf: &[u8]  = unsafe { 
        std::slice::from_raw_parts(ptr, end_offset.0 - start_offset.0)
    };

    let mut decoder = Decoder::with_ip(64, buf, 0, DecoderOptions::NONE);
    let mut formatter = IntelFormatter::new();
    formatter.options_mut().set_digit_separator("_");
    let _ = formatter.options_mut().first_operand_char_index();

    let mut res = Vec::new();
    while decoder.can_decode() {
        let mut instr  = Instruction::default();
        let mut bstr = String::new();
        let mut istr = String::new();
        decoder.decode_out(&mut instr);
        formatter.format(&instr, &mut istr);

        let start_idx = (instr.ip() - 0) as usize;
        let instr_bytes = &buf[start_idx..start_idx + instr.len()];
        for b in instr_bytes.iter() {
            bstr.push_str(&format!("{:02x}", b));
        }
        res.push((istr, bstr, instr.is_invalid()));
    }
    res
}

pub fn disas_bytes(buf: &[u8]) -> Vec<(String, String, bool)>
{
    let mut decoder = Decoder::with_ip(64, buf, 0, DecoderOptions::NO_INVALID_CHECK);
    let mut formatter = IntelFormatter::new();
    formatter.options_mut().set_digit_separator("_");
    let _ = formatter.options_mut().first_operand_char_index();

    let mut res = Vec::new();

    while decoder.can_decode() {
        let mut instr  = Instruction::default();
        let mut bstr = String::new();
        let mut istr = String::new();
        decoder.decode_out(&mut instr);

        formatter.format(&instr, &mut istr);
        let start_idx = (instr.ip() - 0) as usize;
        let instr_bytes = &buf[start_idx..start_idx + instr.len()];
        for b in instr_bytes.iter() {
            bstr.push_str(&format!("{:02x}", b));
        }
        res.push((istr, bstr, instr.is_invalid()));
    }

    res
}


pub fn disas(
    buf: &ExecutableBuffer, 
    offset: AssemblyOffset, 
    max_inst: Option<usize>,
)
{
    let ptr: *const u8 = buf.ptr(offset);
    let addr: u64   = ptr as u64;
    let buf: &[u8]  = unsafe { 
        std::slice::from_raw_parts(ptr, buf.len() - offset.0)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Align(pub usize);
impl Align {
    pub const fn from_value(value: usize) -> Self {
        assert!(value.is_power_of_two());
        Self(value)
    }
    pub const fn from_bit(bit: usize) -> Self { 
        assert!(bit <= 63);
        Self(1 << bit)
    }
    pub const fn offset_mask(&self) -> usize { self.0 - 1 }
    pub const fn index_mask(&self) -> usize { !self.offset_mask() }
    pub const fn value(&self) -> usize { self.0 }
    pub const fn check(&self, value: usize) -> bool {
        (value & self.offset_mask()) == 0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AlignedAddress { 
    addr: usize,
    align: Align,
}
impl AlignedAddress {
    pub const fn new(addr: usize, align: Align) -> Self {
        Self { addr, align }
    }

    pub const fn next(&self) -> Self {
        let addr = self.addr + self.align.value();
        let align = self.align;
        Self { addr, align }
    }

    pub const fn prev(&self) -> Self {
        let addr = self.addr - self.align.value();
        let align = self.align;
        Self { addr, align }
    }

    pub const fn aligned(&self) -> Self {
        let addr = self.index_bits();
        let align = self.align;
        Self { addr, align }
    }

    pub const fn index_bits(&self) -> usize { 
        self.addr & self.align.index_mask()
    }

    pub const fn offset_bits(&self) -> usize { 
        self.addr & self.align.offset_mask()
    }

    pub fn set_bits(&mut self, val: usize) {
        let offset_bits = val & self.align.offset_mask();
        self.addr = self.index_bits() | offset_bits;
    }

    pub const fn value(&self) -> usize { 
        self.addr
    }
}

pub fn align_down(addr: usize, bits: usize) -> usize {
    let align: usize = (1 << bits);
    let mask: usize  = !(align - 1);
    (addr & mask).wrapping_sub(align)
}

#[inline(always)]
pub fn flush_btb<const CNT: usize>() {
    unsafe { 
        core::arch::asm!(r#"
        .rept {cnt}
        jmp 2f
        2:
        .endr
        "#, cnt = const CNT,
        );
    }
}

// NOTE: Quick hack for building this outside of [PerfectHarness]
pub fn build_pmc_counter(p: TargetPlatform, desc: &EventDesc) -> Counter { 
        let mut ctr = match p {
            TargetPlatform::Zen2 => {
                let cfg = PerfectHarness::make_cfg_amd(desc.id(), desc.mask());
                Builder::new()
                .kind(Event::Raw(cfg))
                .build().unwrap()
            },
            TargetPlatform::Tremont => {
                let cfg = PerfectHarness::make_cfg_intel(desc.id() as u8, desc.mask());
                Builder::new()
                .kind(Event::Raw(cfg))
                .build().unwrap()
            },
        };
        ctr
}




#[cfg(test)]
mod test { 
    use super::*;

    #[test]
    fn align() {
        let x = AlignedAddress::new(0x0000_0001_0000_0000, Align::from_bit(5));
        assert_eq!(x.value(), 0x0000_0001_0000_0000);
        assert_eq!(x.next().value(), 0x0000_0001_0000_0020);
        assert_eq!(x.prev().value(), 0x0000_0000_ffff_ffe0);
    }
}

