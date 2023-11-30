
use dynasmrt::{
    ExecutableBuffer,
    AssemblyOffset,
};
use iced_x86::{ 
    Decoder, DecoderOptions, Instruction, Formatter, IntelFormatter 
};


/// Utilities for controlling the state of the current process.
pub struct PerfectEnv; 
impl PerfectEnv {
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

pub fn disas(buf: &ExecutableBuffer) {
    let ptr: *const u8 = buf.ptr(AssemblyOffset(0));
    let addr: u64   = ptr as u64;
    let buf: &[u8]  = unsafe { 
        std::slice::from_raw_parts(ptr, buf.len())
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

