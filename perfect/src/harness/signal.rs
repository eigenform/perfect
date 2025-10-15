//! Module for catching signals.

use std::ffi::{c_int, c_void};
use std::sync::{Arc,Mutex};
use std::sync::atomic::AtomicBool;
use nix::{
    libc::{siginfo_t, ucontext_t},
    sys::signal::{
        sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal,
        sigprocmask, SigmaskHow, 
    },
};

use crate::harness::config::*;

thread_local! { 
    pub static GREGS: Arc<Mutex<Gregs>> = Arc::new(Mutex::new(Gregs::new()));
    pub static HANDLED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

/// Type mirroring the layout of 'gregset_t' in libc. 
/// A signal handler stores saved general-purpose registers in this format.
#[derive(Debug)]
#[repr(C)]
pub struct Gregs { 
    data: [u64; 23]
}
impl Gregs { 
    pub fn new() -> Self { 
        Self { data: [0; 23] }
    }

    pub fn r8(&self) -> u64 { self.data[0] }
    pub fn r9(&self) -> u64 { self.data[1] }
    pub fn r10(&self) -> u64 { self.data[2] }
    pub fn r11(&self) -> u64 { self.data[3] }
    pub fn r12(&self) -> u64 { self.data[4] }
    pub fn r13(&self) -> u64 { self.data[5] }
    pub fn r14(&self) -> u64 { self.data[6] }
    pub fn r15(&self) -> u64 { self.data[7] }

    pub fn rdi(&self) -> u64 { self.data[8] }
    pub fn rsi(&self) -> u64 { self.data[9] }
    pub fn rbp(&self) -> u64 { self.data[10] }
    pub fn rbx(&self) -> u64 { self.data[11] }
    pub fn rdx(&self) -> u64 { self.data[12] }
    pub fn rax(&self) -> u64 { self.data[13] }
    pub fn rcx(&self) -> u64 { self.data[14] }
    pub fn rsp(&self) -> u64 { self.data[15] }
    pub fn rip(&self) -> u64 { self.data[16] }
    pub fn efl(&self) -> u64 { self.data[17] }

    pub fn csgsfs(&self)  -> u64 { self.data[18] }
    pub fn err(&self)     -> u64 { self.data[19] }
    pub fn trapno(&self)  -> u64 { self.data[20] }
    pub fn oldmask(&self) -> u64 { self.data[21] }
    pub fn cr2(&self)     -> u64 { self.data[22] }
}


/// Handler for SIGSEGV.
///
/// FIXME: I'm not sure this is actually finished and working correctly
extern "C" 
fn sigsegv_handler(_sig: c_int, _si: *mut siginfo_t, ctx: *mut c_void) {

    //let uctx = ctx.cast::<ucontext_t>();
    //let gregs: &[u64] = unsafe { 
    //    std::slice::from_raw_parts(
    //        (*uctx).uc_mcontext.gregs.as_ptr() as *const u64,
    //        23
    //    )
    //};
    //let rip: u64 = gregs[16] + 9;

    // Unblock SIGSEGV so we can handle another signal later
    //let mut sigset = SigSet::empty();
    //sigset.add(Signal::SIGSEGV);
    //sigprocmask(SigmaskHow::SIG_UNBLOCK, Some(&sigset), None).unwrap();

    // FIXME: Automatically re-register the handler? 
    //register_sigsegv_handler();

    // Recover from this segfault by restoring the saved stack pointer 
    // and volatile registers. Instead of doing this *here*, we jump into the 
    // handler emitted by the harness. 
    unsafe { 
        core::arch::asm!(r#"
            mov r15, {foo}
            jmp r15
        "#,
        foo = const HarnessConfig::DEFAULT_HANDLER_ADDR,
        );
    }

}

/// Register the SIGSEGV handler. 
///
/// After calling, subsequent segfaults are handled with [`sigsegv_handler`].
pub fn register_sigsegv_handler() {
    let mut sigset = SigSet::empty();
    sigset.add(Signal::SIGSEGV);
    sigprocmask(SigmaskHow::SIG_UNBLOCK, Some(&sigset), None).unwrap();


    let handler = SigHandler::SigAction(sigsegv_handler);
    let flags = SaFlags::SA_SIGINFO | SaFlags::SA_RESETHAND;
    let action = SigAction::new(handler, flags, SigSet::empty());
    unsafe { 
        let _ = sigaction(Signal::SIGSEGV, &action);
    }
}


