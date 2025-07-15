
use crate::ir::bf::*;

/// The function used to compute the BTB index on Zen 2. 
///
/// This is reproduced from the description in the RETBLEED paper[^1].
///
///
/// ``` 
///     47      39      31      23      15      07
///     v       v       v       v       v       v
/// 00  ...........+...........+........................
/// 01  ..........+...........+.........................
/// 02  .........+...........+..........................
/// 03  ........+...........+...........&....&..........
/// 04  .......+...........+...........&....&...........
/// 05  ......+...........+...........+.................
/// 06  .....+...........+...........+..................
/// 07  ....+...........+...........+...................
/// 08  ...+...........+...........+....................
/// 09  ..+...........+...........+.....................
/// 10  .+...........+...........+......................
/// 11  +...........+...........+.......................
/// ```
///
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


