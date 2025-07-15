
/// Interpreter for simple boolean functions. 
///
/// NOTE: support "nice-looking" (as in "beauty is in the eye of the 
/// beholder") descriptions of hash functions. 
///
pub enum BfOp { 
    /// Read the value of an input bit
    In(usize),

    /// N-ary logical AND 
    And(&'static [Self]),

    /// N-ary logical XOR
    Xor(&'static [Self]),
}
impl BfOp {
    pub fn evaluate(&self, input: usize) -> bool { 
        match self { 
            Self::In(n) => (input & (1 << n)) != 0,
            Self::And(ops) => { 
                ops.iter().map(|op| op.evaluate(input))
                    .reduce(|res, val| res & val).unwrap()
            },
            Self::Xor(ops) => {
                ops.iter().map(|op| op.evaluate(input))
                    .reduce(|res, val| res ^ val).unwrap()
            },
        }
    }
}

/// Describes a simple boolean function. 
///
/// Each N-th [`BfOp`] in the program computes the value of output bit 'N'. 
///
pub struct BfProg<const SZ: usize>(pub [BfOp; SZ]);
impl <const SZ: usize> BfProg<SZ> { 
    pub fn evaluate(&self, input: usize) -> usize { 
        let mask = (1 << SZ) - 1;
        let mut res = 0;
        for idx in 0..SZ { 
            res |= ((self.0[idx].evaluate(input) as usize) << idx);
        }
        res & mask
    }
}

