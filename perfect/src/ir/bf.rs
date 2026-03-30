
/// Interpreter for simple boolean functions. 
///
/// NOTE: support "nice-looking" (as in "beauty is in the eye of the 
/// beholder") descriptions of hash functions. 
///
#[derive(Debug)]
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
        let res = match self { 
            Self::In(n) => (input & (1 << n)) != 0,
            Self::And(ops) => { 
                ops.iter().map(|op| op.evaluate(input))
                    .reduce(|res, val| res & val).unwrap()
            },
            Self::Xor(ops) => {
                ops.iter().map(|op| op.evaluate(input))
                    .reduce(|res, val| res ^ val).unwrap()
            },
        };
        //println!("{:?} := {}", self, res);
        res
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

#[cfg(test)]
mod test { 
    use super::*;
    #[test]
    fn sanity() { 
        use BfOp::*;
        let p: BfProg<1> = BfProg([
            Xor(&[In(0), In(1)]),
        ]);
        let cases = [
            (0b0000_0000, 0),
            (0b0000_0011, 0),
            (0b0000_0001, 1),
            (0b0000_0010, 1),
        ];
        for (val, res) in cases { 
            let r = p.evaluate(val);
            assert!(r == res, "val={:08b} res={} expected={}", val, r, res);
        }

        let p: BfProg<1> = BfProg([
            Xor(&[In(0), In(1), In(2)]),
        ]);
        let cases = [
            (0b0000_0000, 0),
            (0b0000_0001, 1),
            (0b0000_0010, 1),
            (0b0000_0011, 0),
            (0b0000_0100, 1),
            (0b0000_0101, 0),
            (0b0000_0110, 0),
            (0b0000_0111, 1),
        ];
        for (val, res) in cases { 
            let r = p.evaluate(val);
            assert!(r == res, "val={:08b} res={} expected={}", val, r, res);
        }

    }


}



