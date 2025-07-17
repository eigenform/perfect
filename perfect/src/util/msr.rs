//! Module for interacting with `/dev/cpu/N/msr`. 

use std::os::fd::{AsRawFd, BorrowedFd};

/// Helper for interacting with `/dev/cpu/N/msr`.
pub struct Msr; 
impl Msr { 
    fn open<'a>(cpu: usize) -> Result<BorrowedFd<'a>, &'static str> {
        let filename = format!("/dev/cpu/{}/msr", cpu);
        let raw_fd = nix::fcntl::open(filename.as_str(), 
            nix::fcntl::OFlag::O_RDWR,
            nix::sys::stat::Mode::S_IRUSR
        ).map_err(|_| "Couldn't open /dev/cpu/<cpu>/msr")?;
        unsafe { 
            Ok(std::os::fd::BorrowedFd::borrow_raw(raw_fd))
        }
    }

    fn close<'a>(fd: BorrowedFd<'a>) {
        nix::unistd::close(fd.as_raw_fd()).unwrap();
    }

    /// Read an MSR on the given CPU. 
    pub fn rdmsr(msr: u32, cpu: usize) -> Result<u64, &'static str> { 
        let fd = Self::open(cpu)?;
        let mut buf = [0u8; 8];
        nix::sys::uio::pread(fd.as_raw_fd(), &mut buf, msr as i64).map_err(|_| { 
            "Failed to read MSR" 
        })?;

        Self::close(fd);
        Ok(u64::from_le_bytes(buf))
    }

    /// Write an MSR on the given CPU. 
    pub fn wrmsr(msr: u32, cpu: usize, value: u64) 
        -> Result<(), String>
    {
        let fd = Self::open(cpu)?;
        let buf = u64::to_le_bytes(value);
        nix::sys::uio::pwrite(fd.as_raw_fd(), &buf, msr as i64).map_err(|e| {
            format!("{:?}", e)
        })?;
        Self::close(fd);
        Ok(())
    }

    pub fn wrmsr_toggle(msr: u32, cpu: usize, bit: usize, val: bool)
        -> Result<(), String>
    {
        assert!(bit <= 63);
        let prev = Self::rdmsr(msr, cpu)?;
        let next = if val { prev | (1 << bit) } else { prev & !(1 << bit) };
        Self::wrmsr(msr, cpu, next)?;
        println!("[*] MSR {:08x}: {:016x} => {:016x}", msr, prev, next);
        Ok(())
    }

}



