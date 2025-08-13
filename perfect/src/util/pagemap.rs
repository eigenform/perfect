//! Module for interacting with `/proc/self/pagemap`. 

/// An entry in '/proc/self/pagemap'.
#[derive(Clone, Copy, Debug)]
pub struct PageMapEntry(pub u64);
impl PageMapEntry {
    const NUM_BYTES: usize = 8;
    pub fn present(&self) -> bool { (self.0 & (1 << 63)) != 0 }
    pub fn swapped(&self) -> bool { (self.0 & (1 << 62)) != 0 }
    pub fn exclusive(&self) -> bool { (self.0 & (1 << 56)) != 0 }
    pub fn soft_dirty(&self) -> bool { (self.0 & (1 << 55)) != 0 }
    pub fn pfn(&self) -> usize {
        self.0 as usize & ((1 << 55) - 1)
    }
}

/// Wrapper for interacting with '/proc/self/pagemap'.
pub struct PageMap;
impl PageMap { 
    // NOTE: Assumes 4KiB page size
    const NUM_OFFSET_BITS: usize = 12;

    /// Resolve the given virtual address into a physical address. 
    pub fn resolve_paddr(vaddr: usize) -> Result<usize, &'static str> { 
        use std::io::prelude::*;
        let mut f = std::fs::File::open("/proc/self/pagemap").map_err(|_| { 
            "Couldn't open /proc/self/pagemap (do you have permission?)"
        })?;

        // Seek to the appropriate pagemap entry and read it
        let mut buf = [0u8; 8];
        let vfn  = vaddr >> Self::NUM_OFFSET_BITS;
        let foff = (vfn * PageMapEntry::NUM_BYTES) as u64;
        f.seek(std::io::SeekFrom::Start(foff)).unwrap();
        f.read_exact(&mut buf).unwrap();
        let entry = PageMapEntry(u64::from_le_bytes(buf));

        if !entry.present() {
            return Err("Couldn't find entry in page map");
        }
        if entry.pfn() == 0 {
            return Err("Got PFN 0 from page map (do you have permission?)");
        }

        // Compute the physical address
        let paddr = (
            (entry.pfn() << Self::NUM_OFFSET_BITS) | 
            (vaddr & ((1 << Self::NUM_OFFSET_BITS) - 1))
        );

        Ok(paddr)
    }
}
