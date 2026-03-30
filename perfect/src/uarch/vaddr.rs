
use std::collections::*;
use crate::uarch::l1d::ZEN2_L1D_UTAG_FN;

/// Wrapper around a virtual address. 
///
/// NOTE: This is only relevant to experiments for Family 17h/19h parts. 
///
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct VirtualAddress(pub usize);
impl VirtualAddress {
    /// Bits [5:0] map to an offset within the cache line
    const OFFSET_MASK: usize = 0x0000_0000_0000_003f;
    /// Bits [11:6] map to a set in the L1 data cache
    const SET_MASK: usize    = 0x0000_0000_0000_0fc0;
    /// According to the "Take A Way" paper, bits [27:12] constitute input to the utag
    const UTAG_MASK: usize   = 0x0000_0000_0fff_f000;
    /// High bits
    const HI_MASK : usize    = 0x0000_ffff_f000_0000;

    /// Return a new address with the requested offset bits.
    pub const fn with_offset(self, off: usize) -> Self { 
        Self(
            (self.0 & !Self::OFFSET_MASK) | ((off & 0b111111))
        )
    }

    /// Return a new address with the requested cache set bits.
    pub const fn with_set(self, set: usize) -> Self { 
        Self(
            (self.0 & !Self::SET_MASK) | ((set & 0b111111) << 6)
        )
    }

    /// Return a new address with the requested utag input bits.
    pub const fn with_utag_input(self, input: usize) -> Self { 
        Self(
            (self.0 & !Self::UTAG_MASK) | ((input & 0xffff) << 12)
        )
    }

    /// Return a new address with the requested offset bits.
    pub const fn with_hibits(self, hibits: usize) -> Self { 
        Self(
            (self.0 & !Self::HI_MASK) | ((hibits & 0xfffff) << 28)
        )
    }


    /// Return the set index bits
    pub fn set(&self) -> usize {
        (self.0 & Self::SET_MASK) >> 6
    }

    /// Return the offset bits
    pub fn offset(&self) -> usize {
        self.0 & Self::OFFSET_MASK
    }

    /// Return the micro-tag input bits
    pub fn utag_input(&self) -> usize {
        (self.0 & Self::UTAG_MASK) >> 12
    }

    /// Return the micro-tag input bits
    pub fn hibits(&self) -> usize {
        (self.0 & Self::HI_MASK) >> 28
    }


    /// Compute and return the micro-tag for this address.
    pub fn utag(&self) -> usize { 
        ZEN2_L1D_UTAG_FN.evaluate(self.0)
    }

    /// Return the 64-bit virtual address as a [`usize`].
    pub fn value(&self) -> usize { 
        self.0
    }

    /// Create a new virtual address from the given offset, set index, 
    /// micro-tag input bits, and high bits. 
    pub fn new(offset: usize, set: usize, utag_input: usize, hi_bits: usize) -> Self {
        let offset = offset & 0x3f;
        let set = set & 0b0011_1111;
        let utag_input = utag_input & 0xffff;
        Self(offset | set << 6 | utag_input << 12 | hi_bits << 28)
    }

    // Generate the set of all 256 addresses whose micro-tags are colliding
    // with the micro-tag for this address.
    pub fn generate_collisions(&self) -> Vec<Self> {
        let mut res = Vec::new();
        let map = Self::compute_utag_map();
        let inputs = map.get(&self.utag()).unwrap();
        for input in inputs {
            res.push(VirtualAddress::new(0b000000, 0b000000, *input, 0));
        }
        res
    }

    // Generate a random address whose micro-tag is colliding with the 
    // micro-tag for this address. 
    pub fn random_collision(&self) -> Self { 
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let colls = self.generate_collisions();
        let x = rng.gen_range(0..256);
        colls[x]
    }

    /// Build a map of all possible micro-tags and their associated input bits.
    pub fn compute_utag_map() -> BTreeMap<usize, BTreeSet<usize>> {
        let mut map: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();

        for input in (0x0000..=0xffffusize) {
            let utag = ZEN2_L1D_UTAG_FN.evaluate(input << 12);
            if let Some(inputs) = map.get_mut(&utag) {
                inputs.insert(input);
            } else { 
                let mut s = BTreeSet::new();
                s.insert(input);
                map.insert(utag, s);
            }
        }
        map
    }
}




