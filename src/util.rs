
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
