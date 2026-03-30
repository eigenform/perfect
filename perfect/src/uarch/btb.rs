
use crate::ir::bf::*;
use std::collections::*;
use bitvec::prelude::*;
use itertools::*;

/// The function used to compute the BTB index on Zen 2. 
///
/// This is reproduced from the description in the RETBLEED paper[^1].
/// [^1]: [RETBLEED: Arbitrary Speculative Code Execution with Return Instructions](https://comsec.ethz.ch/wp-content/files/retbleed_sec22.pdf)
pub const ZEN2_BTB_INDEX_FN: BfProg<12> = {  
    use BfOp::*;
    BfProg([ 
        Xor(&[In(36), In(24)]),
        Xor(&[In(37), In(25)]),
        Xor(&[In(38), In(26)]),
        Xor(&[In(39), In(27), And(&[In(15), In(10)])]),
        Xor(&[In(40), In(28), And(&[In(16), In(11)])]),
        Xor(&[In(41), In(29), In(17)]),
        Xor(&[In(42), In(30), In(18)]),
        Xor(&[In(43), In(31), In(19)]),
        Xor(&[In(44), In(32), In(20)]),
        Xor(&[In(45), In(33), In(21)]),
        Xor(&[In(46), In(34), In(22)]),
        Xor(&[In(47), In(35), In(23)]),
    ])
};

/// Operations used to create BTB collisions
pub enum BTBCollideOp { 
    /// Flip two bits
    Flip(usize, usize),
    /// Set the value of three bits
    Set3((usize, usize, usize), (bool, bool, bool)),

    ///// Set the value of five bits
    //Set5((usize, usize, usize), (bool, bool, bool), (usize, usize), (bool, bool)),
}

/// Compute the allowed operations on the given address that do not affect 
/// the output of the gates associated with [`ZEN2_BTB_INDEX_FN`].
///
/// Each operation in the list does not change the BTB index, and always 
/// yields an address colliding with `vaddr`. Any combination of operations
/// in the list should yield an aliasing address.
/// 
/// 1. For the 2-input XOR gates, we can always flip both bits. 
///
/// 2. For the 3-input XOR gates, the parity (whether or not the number
///    of set bits is even or odd) must be the same. 
///
/// Evaluating the powerset of the resulting list would yield the set of *all* 
/// possible addresses whose BTB index is aliasing with `vaddr`. 
///
/// FIXME: We're just skipping the bits with AND gates here because the 
/// cases with only XOR gates are easiest. 
///
pub fn zen2_btb_valid_ops(vaddr: usize) -> Vec<BTBCollideOp> { 

    /// 3-ary XOR inputs yielding 0
    const ODD: [(bool, bool, bool); 4] = [
        (false, false, true),
        (false, true, false),
        (true, false, false),
        (true, true, true),
    ];
    /// 3-ary XOR inputs yielding 1
    const EVEN: [(bool, bool, bool); 4] = [
        (false, false, false),
        (false, true, true),
        (true, false, true),
        (true, true, false),
    ];


    /// Bit indexes for 2-input XOR gates
    const XOR2: [(usize, usize); 3] = [ 
        (24, 36), // bit 0
        (25, 37), // bit 1
        (26, 38), // bit 2
    ];

    /// Bit indexes for 3-input XOR gates (where the first bit is the result
    /// of a 2-input AND gate)
    const ANDXOR: [((usize, usize), (usize, usize)); 2] = [
        ( (10, 15), (27, 39) ),
        ( (11, 16), (28, 30) ),
    ];

    /// Bit indexes for 3-input XOR gates
    const XOR3: [(usize, usize, usize); 7] = [ 
        (17, 29, 41), // bit 5
        (18, 30, 42), // bit 6
        (19, 31, 43), // bit 7
        (20, 32, 44), // bit 8
        (21, 33, 45), // bit 9
        (22, 34, 46), // bit 10
        (23, 35, 47), // bit 11
    ];

    let index = ZEN2_BTB_INDEX_FN.evaluate(vaddr);
    let mut ops = Vec::new();

    for (x, y) in XOR2 { 
        ops.push(BTBCollideOp::Flip(x, y));
    }

    let bits = vaddr.view_bits::<Lsb0>();

    //for ((x0, x1), (y, z)) in ANDXOR { 
    //    let mut cnt = 0;
    //    let x = bits[x0] & bits[x1];

    //    if x { cnt += 1; }
    //    if bits[y] { cnt += 1; }
    //    if bits[z] { cnt += 1; }

    //    if cnt & 1 == 1 { 
    //        if x { 
    //        } else { 
    //        }
    //        ops.push(BTBCollideOp::Set3((x, y, z), ODD[0]));
    //        ops.push(BTBCollideOp::Set3((x, y, z), ODD[1]));
    //        ops.push(BTBCollideOp::Set3((x, y, z), ODD[2]));
    //        ops.push(BTBCollideOp::Set3((x, y, z), ODD[3]));
    //    } else { 
    //        ops.push(BTBCollideOp::Set3((x, y, z), EVEN[0]));
    //        ops.push(BTBCollideOp::Set3((x, y, z), EVEN[1]));
    //        ops.push(BTBCollideOp::Set3((x, y, z), EVEN[2]));
    //        ops.push(BTBCollideOp::Set3((x, y, z), EVEN[3]));
    //    }
    //}

    for (x,y,z) in XOR3 { 
        let mut cnt = 0;
        if bits[x] { cnt += 1 };
        if bits[y] { cnt += 1 };
        if bits[z] { cnt += 1 };
        if cnt & 1 == 1 { 
            ops.push(BTBCollideOp::Set3((x, y, z), ODD[0]));
            ops.push(BTBCollideOp::Set3((x, y, z), ODD[1]));
            ops.push(BTBCollideOp::Set3((x, y, z), ODD[2]));
            ops.push(BTBCollideOp::Set3((x, y, z), ODD[3]));
        } else { 
            ops.push(BTBCollideOp::Set3((x, y, z), EVEN[0]));
            ops.push(BTBCollideOp::Set3((x, y, z), EVEN[1]));
            ops.push(BTBCollideOp::Set3((x, y, z), EVEN[2]));
            ops.push(BTBCollideOp::Set3((x, y, z), EVEN[3]));
        }
    }

    ops
}


/// Compute other addresses aliasing with this BTB index. 
pub fn zen2_btb_collisions(vaddr: usize, combinations: usize) 
    -> BTreeSet<usize> 
{
    let mut collisions = BTreeSet::new();

    let index = ZEN2_BTB_INDEX_FN.evaluate(vaddr);

    let ops = zen2_btb_valid_ops(vaddr);

    // NOTE: Unfortunately, doing this for the powerset would take a 
    // very very long time. We can probably get away with looking at 
    // combinations of only a couple possible permutations. 
    for oplist in ops.iter().combinations(combinations) {
        let mut res: usize = vaddr;
        let mut bits = res.view_bits_mut::<Lsb0>();
        for op in oplist { 
            match op { 
                BTBCollideOp::Flip(x, y) => { 
                    bits.set(*x, !bits[*x]);
                    bits.set(*y, !bits[*y]);
                },
                BTBCollideOp::Set3((x,y,z), (a,b,c)) => {
                    bits.set(*x, *a);
                    bits.set(*y, *b);
                    bits.set(*z, *c);
                },
            }
        }
        let new_index = ZEN2_BTB_INDEX_FN.evaluate(res);
        assert!(new_index == index, 
            "{:016x} ({:04x}) != {:016x} ({:04x})", 
            vaddr, index, res, new_index);
        collisions.insert(res);
    }

    collisions
}



///// A hypothetical BTB addressing scheme. 
/////
///// NOTE: The Family 17h SOG mentions the following: 
/////
///// - There are 8 L0 entries (that's 3-bit)
///// - There are 256 L1 entries (that's 8-bit)
///// - There are 4096 L2 entries (that's 12-bit)
///// - An entry can hold two branches in the same 64-byte cacheline
///// - An entry can hold two branches if the first branch is conditional
///// - Branches whose *target* crosses a 19-bit boundary cannot share a BTB
/////   entry with other branches
/////
//
//pub struct BTBConfig {
//    pub offset_mask: usize,
//    pub index_mask: usize,
//    pub tag_mask: usize,
//}
//
//pub struct BTBAddress(pub usize);
//impl BTBAddress {
//    // NOTE: Just sketching *something* out...
//    const OFFSET_MASK: usize = 0x0000_0000_0000_003f;
//    const INDEX_MASK: usize  = 0x0000_0000_0003_ffc0;
//    const TAG_MASK: usize    = 0xffff_ffff_fffc_0000;
//    pub fn offset_bits(&self) -> usize {
//        self.0 & Self::OFFSET_MASK
//    }
//    pub fn index_bits(&self) -> usize {
//        (self.0 & Self::INDEX_MASK) >> 6
//    }
//    pub fn tag_bits(&self) -> usize {
//        (self.0 & Self::TAG_MASK) >> 19
//    }
//
//    pub fn from_usize(x: usize) -> Self {
//        Self(x)
//    }
//
//    const OFFSET_MASK2: usize = 0x0000_0000_0000_003f;
//    const INDEX_MASK2: usize  = 0x0000_0000_0000_0fff;
//    const TAG_MASK2: usize    = 0x0000_3fff_ffff_ffff;
//    pub fn new(offset: usize, index: usize, tag: usize) -> Self {
//        let offset = (offset & Self::OFFSET_MASK2);
//        let index = (index & Self::INDEX_MASK2) << 6;
//        let tag = (tag & Self::TAG_MASK2) << 19;
//        Self(tag | index | offset)
//    }
//}
//impl std::fmt::Display for BTBAddress {
//    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
//        write!(f, "{:016x}:{:010x}:{:04x}:{:02x}", 
//            self.0, self.tag_bits(), self.index_bits(), self.offset_bits()
//        )
//    }
//}


