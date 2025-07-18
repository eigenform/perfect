//! State associated with the harness. 

use crate::asm::{ Gpr, VectorGpr, };

/// Harness stack layout.
#[repr(C, align(0x10000))]
pub struct HarnessStack { data: [u8; 0x8000], }
impl HarnessStack {
    pub fn new() -> Self { Self { data: [0; 0x8000] } }
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { self.data.as_ptr().offset(0x3000) }
    }
}

/// Saved general-purpose register state.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct GprState(pub [usize; 16]);
impl GprState {
    pub fn new() -> Self { Self([0; 16]) }
    pub fn clear(&mut self) { self.0 = [0; 16]; }
    pub fn read_gpr(&self, gpr: Gpr) -> usize { self.0[gpr as usize] }
    pub fn rax(&self) -> usize { self.0[0] }
    pub fn rcx(&self) -> usize { self.0[1] }
    pub fn rdx(&self) -> usize { self.0[2] }
    pub fn rbx(&self) -> usize { self.0[3] }
    pub fn rsp(&self) -> usize { self.0[4] }
    pub fn rbp(&self) -> usize { self.0[5] }
    pub fn rsi(&self) -> usize { self.0[6] }
    pub fn rdi(&self) -> usize { self.0[7] }
    pub fn r8(&self)  -> usize { self.0[8] }
    pub fn r9(&self)  -> usize { self.0[9] }
    pub fn r10(&self) -> usize { self.0[10] }
    pub fn r11(&self) -> usize { self.0[11] }
    pub fn r12(&self) -> usize { self.0[12] }
    pub fn r13(&self) -> usize { self.0[13] }
    pub fn r14(&self) -> usize { self.0[14] }
    pub fn r15(&self) -> usize { self.0[15] }
}
impl std::fmt::Debug for GprState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GprState")
         .field("rax", &self.0[0])
         .field("rcx", &self.0[1])
         .field("rdx", &self.0[2])
         .field("rbx", &self.0[3])
         .field("rsp", &self.0[4])
         .field("rbp", &self.0[5])
         .field("rsi", &self.0[6])
         .field("rdi", &self.0[7])
         .field("r8",  &self.0[8])
         .field("r9",  &self.0[9])
         .field("r10", &self.0[10])
         .field("r11", &self.0[11])
         .field("r12", &self.0[12])
         .field("r13", &self.0[13])
         .field("r14", &self.0[14])
         .field("r15", &self.0[15])
         .finish()
    }
}

/// Saved vector register state.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct VectorGprState(pub [[u64; 4]; 16]);
impl VectorGprState {
    pub fn new() -> Self { Self([[0; 4]; 16]) }
    pub fn clear(&mut self) { self.0 = [[0; 4]; 16] }
    pub fn read_vgpr(&self, vgpr: VectorGpr) -> [u64; 4] { self.0[vgpr as usize] }
    pub fn ymm0(&self)  -> [u64; 4] { self.0[0] }
    pub fn ymm1(&self)  -> [u64; 4] { self.0[1] }
    pub fn ymm2(&self)  -> [u64; 4] { self.0[2] }
    pub fn ymm3(&self)  -> [u64; 4] { self.0[3] }
    pub fn ymm4(&self)  -> [u64; 4] { self.0[4] }
    pub fn ymm5(&self)  -> [u64; 4] { self.0[5] }
    pub fn ymm6(&self)  -> [u64; 4] { self.0[6] }
    pub fn ymm7(&self)  -> [u64; 4] { self.0[7] }
    pub fn ymm8(&self)  -> [u64; 4] { self.0[8] }
    pub fn ymm9(&self)  -> [u64; 4] { self.0[9] }
    pub fn ymm10(&self) -> [u64; 4] { self.0[10] }
    pub fn ymm11(&self) -> [u64; 4] { self.0[11] }
    pub fn ymm12(&self) -> [u64; 4] { self.0[12] }
    pub fn ymm13(&self) -> [u64; 4] { self.0[13] }
    pub fn ymm14(&self) -> [u64; 4] { self.0[14] }
    pub fn ymm15(&self) -> [u64; 4] { self.0[15] }
}
impl std::fmt::Debug for VectorGprState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GprState")
         .field("ymm0", &self.0[0])
         .field("ymm1", &self.0[1])
         .field("ymm2", &self.0[2])
         .field("ymm3", &self.0[3])
         .field("ymm4", &self.0[4])
         .field("ymm5", &self.0[5])
         .field("ymm6", &self.0[6])
         .field("ymm7", &self.0[7])
         .field("ymm8",  &self.0[8])
         .field("ymm9",  &self.0[9])
         .field("ymm10", &self.0[10])
         .field("ymm11", &self.0[11])
         .field("ymm12", &self.0[12])
         .field("ymm13", &self.0[13])
         .field("ymm14", &self.0[14])
         .field("ymm15", &self.0[15])
         .finish()
    }
}


