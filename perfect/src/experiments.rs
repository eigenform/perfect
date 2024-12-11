
pub mod template;
pub mod branch;
pub mod pmcdisc;
pub mod decoder;

use crate::asm::*;
use crate::harness::*;
use dynasmrt::*;

pub trait Experiment<I> {
    fn emit(input: I) -> X64Assembler;
    fn run(harness: &mut PerfectHarness);
}

/// Generic container for an "emitter". 
#[derive(Copy, Clone)]
pub struct EmitterDesc<I> {
    pub desc: &'static str,
    pub func: fn(&mut X64Assembler, I),
}
impl <I> EmitterDesc<I> {
    pub const fn new(
        desc: &'static str, 
        func: fn(&mut X64Assembler, I)
    ) -> Self 
    {
        Self { desc, func }
    }

    pub fn desc(&self) -> &'static str { 
        self.desc 
    }
}

/// A "static" list of emitter cases (determined before runtime).
pub struct StaticEmitterCases<I: 'static>(&'static [EmitterDesc<I>]);
impl <I: 'static> StaticEmitterCases<I> {
    pub const fn new(cases: &'static [EmitterDesc<I>]) -> Self {
        Self(cases)
    }
    pub fn iter(&self) -> impl Iterator<Item=&EmitterDesc<I>> {
        self.0.iter()
    }
}

/// A "dynamic" list of emitter cases (extensible during runtime).
pub struct DynamicEmitterCases<I> {
    pub cases: Vec<EmitterDesc<I>>,
}
impl <I> DynamicEmitterCases<I> {
    pub fn new() -> Self { 
        Self { cases: Vec::new() }
    }
    pub fn add_case(&mut self, desc: EmitterDesc<I>) {
        self.cases.push(desc);
    }
    pub fn iter(&self) -> impl Iterator<Item=&EmitterDesc<I>> {
        self.cases.iter()
    }
}


