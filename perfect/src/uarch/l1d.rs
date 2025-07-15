
use crate::ir::bf::*;

/// The function used to compute the L1D micro-tag (utag) used to implement
/// the L1D way predictor. 
///
/// ```
///     27  23  19
///     v   v   v
/// 12  +........
/// 13  .+.......
/// 14  ..+......
/// 15  .......+.
/// 16  ......+..
/// 17  .....+...
/// 18  ....+....
/// 19  ...+.....
/// ```
///
/// This is reproduced from the description in the "Take A Way" paper[^1]. 
///
/// [^1]: [Take A Way: Exploring the Security Implications of AMD's Cache Way Predictors](https://dl.acm.org/doi/10.1145/3320269.3384746)
///
pub const ZEN2_L1D_UTAG_FN: BfProg<8> = {
    use BfOp::*;
    BfProg([ 
        Xor(&[In(12), In(27)]),
        Xor(&[In(13), In(26)]),
        Xor(&[In(14), In(25)]),

        Xor(&[In(15), In(20)]),
        Xor(&[In(16), In(21)]),
        Xor(&[In(17), In(22)]),
        Xor(&[In(18), In(23)]),
        Xor(&[In(19), In(24)]),
    ])
};


