use perfect::*;
use perfect::events::*;
use rand::prelude::*;
use rand::distributions::Uniform;
use std::collections::*;

#[repr(transparent)]
pub struct Cacheline(pub [u8; 64]);
impl Cacheline { 
    pub fn ptr(&self) -> *const u8 {
        self.0.as_ptr() 
    }
}

#[repr(C)]
pub struct ReloadArray {
    line: [Cacheline; 256]
}
impl ReloadArray {
    /// Flush an entry from the cache. 
    pub fn flush(&self, idx: u8) {
        unsafe { 
            core::arch::x86_64::_mm_clflush(self.line[idx as usize].ptr());
            core::arch::x86_64::_mm_mfence();
            core::arch::x86_64::_mm_lfence();
        }
    }

    /// Return a pointer to the given entry. 
    pub fn ptr(&self, idx: u8) -> *const u8 { 
        self.line[idx as usize].ptr()
    }
}

fn main() {
    let mut harness = HarnessConfig::default_zen2().emit();
}


