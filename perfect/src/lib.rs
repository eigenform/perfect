
#![allow(unused_mut)]
#![allow(unused_parens)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(dead_code)]


pub mod asm;
pub mod ir;
pub mod harness;
pub mod experiments;
pub mod stats; 
pub mod util;
pub mod events;
pub mod uarch; 

pub use rand::Rng;
pub use rand::rngs::ThreadRng;
pub use rand::prelude::*;
pub use itertools::*;

pub use dynasmrt::{
    dynasm, 
    DynasmApi, 
    DynasmLabelApi, 
    DynamicLabel,
    components::StaticLabel,
    Assembler, 
    AssemblyOffset, 
    ExecutableBuffer, 
    Executor,
    x64::X64Relocation,
};

pub use crate::asm::*;
pub use crate::harness::*;
pub use crate::util::*;
pub use crate::experiments::{ 
    Experiment,
    EmitterDesc,
    DynamicEmitterCases,
    StaticEmitterCases,
};
pub use crate::experiments::template::*;


