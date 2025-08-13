
//nix::ioctl_none_bad! { victim_ping, 0 }
nix::ioctl_write_int_bad! { victim_ping, 0 }

nix::ioctl_none_bad! { victim_invd, 1 }
nix::ioctl_write_ptr_bad! { victim_read, 2, VictimMsg }

#[repr(C, packed)]
pub struct VictimMsg { 
    pub ptr: *const u32,
    pub off: usize,
}
impl VictimMsg { 
    pub fn as_ptr(&self) -> *const Self { 
        self
    }
}

/// Wrapper for interacting with an intentionally-vulnerable kernel module.
pub struct Victim { 
    fd: i32,
}
impl Victim { 
    /// Open a handle to the character device
    pub fn open() -> Self { 
        use nix::fcntl:: { open, OFlag };
        use nix::errno::Errno;
        use nix::sys::stat::Mode;
        let fd = match open("/dev/victim", OFlag::O_RDWR, Mode::S_IRWXU) {
            Ok(fd) => fd,
            Err(e) => match e { 
                _ => panic!("{}", e),
            }
        };
        Self { fd } 
    }

    /// Read the address of the scratch page allocated by the module
    pub fn scratch_page(&self) -> usize {
        use std::fs::read_to_string;
        match read_to_string("/sys/kernel/debug/victim/scratch_page") {
            Ok(s) => {
                let x = s[2..].strip_suffix("\n").unwrap();
                usize::from_str_radix(x, 16).unwrap()
            },
            Err(e) => panic!("{}", e),
        }
    }

    /// Ask the kernel module to read a secret value
    pub fn ping(&mut self, offset: i32) { 
        unsafe { victim_ping(self.fd, offset).unwrap(); }
    }

    /// Ask the kernel module to invalidate the L1D cache
    pub fn invd(&mut self) { 
        unsafe { victim_invd(self.fd).unwrap(); }
    }

    /// Ask the kernel module to read from an arbitrary virtual address
    pub fn read(&mut self, msg: &Box<VictimMsg>) { 
        unsafe { victim_read(self.fd, msg.as_ptr()).unwrap(); }
    }

}


