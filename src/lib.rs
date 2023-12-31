
#![allow(unused_mut)]
#![allow(unused_parens)]
#![allow(unused_variables)]
#![allow(dead_code)]

pub mod zen2;
pub mod asm;
pub mod harness;
pub mod util;

pub use crate::asm::*;
pub use crate::harness::*;
pub use crate::util::*;
pub use rand::Rng;
pub use rand::rngs::ThreadRng;
pub use dynasmrt::{
    dynasm, 
    DynasmApi, 
    DynasmLabelApi, 
    DynamicLabel,
    Assembler, 
    AssemblyOffset, 
    ExecutableBuffer, 
    Executor,
    x64::X64Relocation
};

