
pub mod group; 

pub use group::*;

use crate::asm::*;
use crate::experiments::*;
use clap::ValueEnum;

/// Describes an emitter for some `pmcdisc` test.
pub struct TestEmitter { 
    /// Decription of this emitter
    pub desc: Option<&'static str>,

    /// Function implementing the emitter
    pub func: fn(&mut X64Assembler),

    pub single: bool,
}
impl TestEmitter { 
    pub const fn new(desc: &'static str, func: fn(&mut X64Assembler)) -> Self { 
        Self { desc: Some(desc), func, single: false }
    }
    pub const fn new_anon(func: fn(&mut X64Assembler)) -> Self { 
        Self { desc: None, func, single: true }
    }
}

/// A group of one or more [`TestEmitter`].
pub struct TestGroup { 
    pub name: &'static str,

    /// A prologue common to all emitters in this group, executed before 
    /// the start of the measurement. 
    pub prologue: Option<fn(&mut X64Assembler)>,

    /// An epilogue common to all emitters in this group, executed after 
    /// the end of the measurement. 
    pub epilogue: Option<fn(&mut X64Assembler)>,

    /// A common block of code emitted *after* the start of the measurement,
    /// for all emitters in this group. 
    pub common_measured: Option<fn(&mut X64Assembler)>,

    /// Common block of code used to measure the "floor" for this test
    pub floor: Option<fn(&mut X64Assembler)>,

    //pub emitters: &'static [fn(&mut X64Assembler)],
    pub emitters: &'static [TestEmitter],
}



/// Implemented by an *identifier* for some test
pub trait TestToken: Clone + Copy + std::fmt::Debug + ValueEnum { 
    /// Return the [`TestGroup`] associated with this token. 
    fn group(&self) -> TestGroup;
}


