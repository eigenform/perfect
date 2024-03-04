
// NOTE: This really sucks. 

use crate::events::*;

pub enum FpRetSseAvxOpsMask {
    SseMovOps,
    SseMovOpsElim,
    OptPotential,
    Optimized,
    Unk(u8),
}
impl FpRetSseAvxOpsMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::SseMovOps => MaskDesc::new(0x01, "SseMovOps"),
            Self::SseMovOpsElim => MaskDesc::new(0x02, "SseMovOpsElim"),
            Self::OptPotential => MaskDesc::new(0x04, "OptPotential"),
            Self::Optimized => MaskDesc::new(0x08, "Optimized"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}

pub enum LsBadStatus2Mask {
    UnkWidthMismatch,
    Unk(u8),
}
impl LsBadStatus2Mask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::UnkWidthMismatch => MaskDesc::new(0x02, "UnkWidthMismatch"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}



pub enum LsDispatchMask {
    LdDispatch,
    StDispatch,
    LdStDispatch,
    Unk(u8),
}
impl LsDispatchMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::LdDispatch => MaskDesc::new(0x01, "LdDispatch"),
            Self::StDispatch => MaskDesc::new(0x02, "StDispatch"),
            Self::LdStDispatch => MaskDesc::new(0x04, "LdStDispatch"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}


pub enum BpRedirectMask {
    BpL2Redir,
    Unk(u8),
}
impl BpRedirectMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::BpL2Redir => MaskDesc::new(0x20,"BpL2Redir"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}


pub enum DeMsStallMask {
    Serialize,
    WaitForQuiet,
    WaitForSegId,
    WaitForStQ,
    WaitForQuietCurTID,
    WaitForQuietOthrTID,
    MutexStall,
    WaitForCount,
    Unk(u8),
}
impl DeMsStallMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::Serialize => MaskDesc::new(0x01, "Serialize"),
            Self::WaitForQuiet => MaskDesc::new(0x02, "WaitForQuiet"),
            Self::WaitForSegId => MaskDesc::new(0x04, "WaitForSegId"),
            Self::WaitForStQ => MaskDesc::new(0x08, "WaitForStQ"),
            Self::WaitForQuietCurTID => MaskDesc::new(0x10, "WaitForQuietCurTID"),
            Self::WaitForQuietOthrTID => MaskDesc::new(0x20, "WaitForQuietOthrTID"),
            Self::MutexStall => MaskDesc::new(0x40, "MutexStall"),
            Self::WaitForCount => MaskDesc::new(0x80, "WaitForCount"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}


pub enum DeDisOpsFromDecoderMask {
    FastPath,
    Microcode,
    Fp,
    Int,
    Unk(u8),
}
impl DeDisOpsFromDecoderMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::FastPath => MaskDesc::new(0x01, "FastPath"),
            Self::Microcode => MaskDesc::new(0x02, "Microcode"),
            Self::Fp => MaskDesc::new(0x04, "Fp"),
            Self::Int => MaskDesc::new(0x08, "Int"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}


pub enum DeDisDispatchTokenStalls1Mask {
    IntPhyRegFileRsrcStall,
    LoadQueueRsrcStall,
    StoreQueueRsrcStall,
    IntSchedulerMiscRsrcStall,
    TakenBrnchBufferRsrc,
    FpRegFileRsrcStall,
    FpSchRsrcStall,
    FpMiscRsrcStall,
    Unk(u8),
}
impl DeDisDispatchTokenStalls1Mask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::IntPhyRegFileRsrcStall => 
                MaskDesc::new(0x01, "IntPhyRegFileRsrcStall"),
            Self::LoadQueueRsrcStall => 
                MaskDesc::new(0x02, "LoadQueueRsrcStall"),
            Self::StoreQueueRsrcStall => 
                MaskDesc::new(0x04, "StoreQueueRsrcStall"),
            Self::IntSchedulerMiscRsrcStall => 
                MaskDesc::new(0x08, "IntSchedulerMiscRsrcStall"),
            Self::TakenBrnchBufferRsrc => 
                MaskDesc::new(0x10, "TakenBrnchBufferRsrc"),
            Self::FpRegFileRsrcStall => 
                MaskDesc::new(0x20, "FpRegFileRsrcStall"),
            Self::FpSchRsrcStall => 
                MaskDesc::new(0x40, "FpSchRsrcStall"),
            Self::FpMiscRsrcStall => 
                MaskDesc::new(0x80, "FpMiscRsrcStall"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}


pub enum DeDisDispatchTokenStalls0Mask {
    ALSQ1RsrcStall,
    ALSQ2RsrcStall,
    ALSQ3_0_TokenStall,
    ALUTokenStall,
    AGSQTokenStall,
    RetireTokenStall,
    ScAguDispatchStall,
    Unk(u8),
}
impl DeDisDispatchTokenStalls0Mask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::ALSQ1RsrcStall => 
                MaskDesc::new(0x01, "ALSQ1RsrcStall"),
            Self::ALSQ2RsrcStall => 
                MaskDesc::new(0x02, "ALSQ2RsrcStall"),
            Self::ALSQ3_0_TokenStall => 
                MaskDesc::new(0x04, "ALSQ3_0_TokenStall"),
            Self::ALUTokenStall => 
                MaskDesc::new(0x08, "ALUTokenStall"),
            Self::AGSQTokenStall => 
                MaskDesc::new(0x10, "AGSQTokenStall"),
            Self::RetireTokenStall => 
                MaskDesc::new(0x20, "RetireTokenStall"),
            Self::ScAguDispatchStall => 
                MaskDesc::new(0x40, "ScAguDispatchStall"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}


pub enum DsTokStall3Mask {
    Cop1Disp,
    Cop2Disp,
    Cop3Disp,
    Cop4Disp,
    Cop5Disp,
    Unk(u8),
}
impl DsTokStall3Mask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::Cop1Disp => MaskDesc::new(0x02, "Cop1Disp"),
            Self::Cop2Disp => MaskDesc::new(0x04, "Cop2Disp"),
            Self::Cop3Disp => MaskDesc::new(0x08, "Cop3Disp"),
            Self::Cop4Disp => MaskDesc::new(0x10, "Cop4Disp"),
            Self::Cop5Disp => MaskDesc::new(0x20, "Cop5Disp"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}




pub enum Zen2Event { 
    FpRetSseAvxOps(FpRetSseAvxOpsMask),
    FpOpsRetiredByType(u8),
    FpSseAvxOpsRetired(u8),
    FpPackOpsRetired(u8),
    FpPackedIntOpType(u8),
    FpDispFaults(u8),

    LsBadStatus2(LsBadStatus2Mask),
    LsLocks(u8),
    LsRetClFlush(u8),
    LsRetCpuid(u8),
    LsDispatch(LsDispatchMask),
    LsDataPipe(u8),
    LsStMisalign(u8),
    LsSTLF(u8),
    LsStoreCommitCancel(u8),
    LsStoreCommitCancel2(u8),
    LsMabAlloc(u8),
    LsMisalLoads(u8),
    LsPrefInstrDisp(u8),
    LsWcbClosePremature(u8),
    LsNotHaltedCyc(u8),

    BpL1BTBCorrect(u8),
    BpL2BTBCorrect(u8),
    BpL0BTBHit(u8),
    BpDynIndPred(u8),
    IfDqBytesFetched(u8),
    BpDeReDirect(u8),
    BpRedirect(BpRedirectMask),

    DeMsStall(DeMsStallMask),
    DeDisUopQueueEmpty(u8),
    DeSrcOpDisp(u8),
    DeDisOpsFromDecoder(DeDisOpsFromDecoderMask),
    DeDisDispatchTokenStalls1(DeDisDispatchTokenStalls1Mask),
    DeDisDispatchTokenStalls0(DeDisDispatchTokenStalls0Mask),
    MemFileHit(u8),
    MemRenLdDsp(u8),
    MemRenLdElim(u8),
    DsTokStall3(DsTokStall3Mask),
    Dsp0Stall(u8),
    DsCopsAfterBrnInDspGrp(u8),

    ExRetInstr(u8),
    ExRetCops(u8),
    ExRetBrn(u8),
    ExRetBrnMisp(u8),
    ExRetBrnTaken(u8),
    ExRetBrnTakenMisp(u8),
    ExRetBrnFar(u8),
    ExRetNearRet(u8),
    ExRetNearRetMisp(u8),
    ExRetBrnIndMisp(u8),
    ExRetMmxFpInstr(u8),
    ExRetCond(u8),
    ExRetireEmpty(u8),

    ExRetUcodeInst(u8),
    ExRetUcodeOps(u8),
    ExRetMsprdBrnchInstrDirMsmtch(u8),
    Bp1RetBrUncondMisp(u8),
    ExMovElim(u8),
}
impl Zen2Event { 
    pub fn event(&self) -> EventDesc { 
        match self { 
            Self::FpRetSseAvxOps(m) => {
                let mask = m.desc();
                EventDesc::new(0x003, "FpRetSseAvxOps", mask)
            },
            Self::FpOpsRetiredByType(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x00a, "FpOpsRetiredByType", mask)
            },
            Self::FpSseAvxOpsRetired(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x00b, "FpSseAvxOpsRetired", mask)
            },
            Self::FpPackOpsRetired(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x00c, "FpPackOpsRetired", mask)
            },
            Self::FpPackedIntOpType(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x00d, "FpPackedIntOpType", mask)
            },
            Self::FpDispFaults(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x00e, "FpDispFaults", mask)
            },

            Self::LsBadStatus2(m) => {
                let mask = m.desc();
                EventDesc::new(0x024, "LsBadStatus2", mask)
            },
            Self::LsLocks(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x025, "LsLocks", mask)
            },
            Self::LsRetClFlush(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x026, "LsRetClFlush", mask)
            },
            Self::LsRetCpuid(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x027, "LsRetCpuid", mask)
            },
            Self::LsDispatch(m) => {
                let mask = m.desc();
                EventDesc::new(0x029, "LsDispatch", mask)
            },
            Self::LsDataPipe(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x02f, "LsDataPipe", mask)
            },
            Self::LsStMisalign(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x032, "LsStMisalign", mask)
            },
            Self::LsSTLF(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x035, "LsSTLF", mask)
            },
            Self::LsStoreCommitCancel(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x036, "LsStoreCommitCancel", mask)
            },
            Self::LsStoreCommitCancel2(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x037, "LsStoreCommitCancel2", mask)
            },
            Self::LsMabAlloc(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x041, "LsMabAlloc", mask)
            },
            Self::LsMisalLoads(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x047, "LsMisalLoads", mask)
            },
            Self::LsPrefInstrDisp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x04b, "LsPrefInstrDisp", mask)
            },
            Self::LsWcbClosePremature(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x050, "LsWcbClosePremature", mask)
            },
            Self::LsNotHaltedCyc(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x076, "LsNotHaltedCyc", mask)
            },

            Self::BpL1BTBCorrect(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x08a, "BpL1BTBCorrect", mask)
            },
            Self::BpL2BTBCorrect(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x08b, "BpL2BTBCorrect", mask)
            },
            Self::BpL0BTBHit(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x08d, "BpL0BTBHit", mask)
            },
            Self::BpDynIndPred(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x08e, "BpDynIndPred", mask)
            },
            Self::IfDqBytesFetched(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x08f, "IfDqBytesFetched", mask)
            },
            Self::BpDeReDirect(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x091, "BpDeReDirect", mask)
            },
            Self::BpRedirect(m) => {
                let mask = m.desc();
                EventDesc::new(0x09f, "BpRedirect", mask)
            },

            Self::DeMsStall(m) => {
                let mask = m.desc();
                EventDesc::new(0x0a8, "DeMsStall", mask)
            },
            Self::DeDisUopQueueEmpty(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0a9, "DeDisUopQueueEmpty", mask)
            },
            Self::DeSrcOpDisp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0aa, "DeSrcOpDisp", mask)
            },
            Self::DeDisOpsFromDecoder(m) => {
                let mask = m.desc();
                EventDesc::new(0x0ab, "DeDisOpsFromDecoder", mask)
            },
            Self::DeDisDispatchTokenStalls1(m) => {
                let mask = m.desc();
                EventDesc::new(0x0ae, "DeDisDispatchTokenStalls1", mask)
            },
            Self::DeDisDispatchTokenStalls0(m) => {
                let mask = m.desc();
                EventDesc::new(0x0af, "DeDisDispatchTokenStalls0", mask)
            },
            Self::MemFileHit(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0b1, "MemFileHit", mask)
            },
            Self::MemRenLdDsp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0b2, "MemRenLdDsp", mask)
            },
            Self::MemRenLdElim(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0b3, "MemRenLdElim", mask)
            },
            Self::DsTokStall3(m) => {
                let mask = m.desc();
                EventDesc::new(0x0b4, "DsTokStall3", mask)
            },
            Self::Dsp0Stall(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0b5, "Dsp0Stall", mask)
            },
            Self::DsCopsAfterBrnInDspGrp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0b6, "DsCopsAfterBrnInDspGrp", mask)
            },
            Self::ExRetInstr(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0c0, "ExRetInstr", mask)
            },
            Self::ExRetCops(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0c1, "ExRetCops", mask)
            },
            Self::ExRetBrn(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0c2, "ExRetBrn", mask)
            },
            Self::ExRetBrnMisp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0c3, "ExRetBrnMisp", mask)
            },
            Self::ExRetBrnTaken(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0c4, "ExRetBrnTaken", mask)
            },
            Self::ExRetBrnTakenMisp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0c5, "ExRetBrnTakenMisp", mask)
            },
            Self::ExRetBrnFar(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0c6, "ExRetBrnFar", mask)
            },
            Self::ExRetNearRet(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0c8, "ExRetNearRet", mask)
            },
            Self::ExRetNearRetMisp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0c9, "ExRetNearRetMisp", mask)
            },
            Self::ExRetBrnIndMisp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0ca, "ExRetBrnIndMisp", mask)
            },
            Self::ExRetMmxFpInstr(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0cb, "ExRetMmxFpInstr", mask)
            },
            Self::ExRetCond(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0d1, "ExRetCond", mask)
            },
            Self::ExRetireEmpty(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0d9, "ExRetireEmpty", mask)
            },

            Self::ExRetUcodeInst(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1c0, "ExRetUcodeInst", mask)
            },
            Self::ExRetUcodeOps(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1c1, "ExRetUcodeOps", mask)
            },
            Self::ExRetMsprdBrnchInstrDirMsmtch(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1c7, "ExRetMsprdBrnchInstrDirMsmtch", mask)
            },
            Self::Bp1RetBrUncondMisp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1c8, "Bp1RetBrUncondMisp", mask)
            },
            Self::ExMovElim(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1db, "ExMovElim", mask)
            },

        }
    }
}


//decl_event_masks!(FpRetSseAvxOps, 0x003, "Retired FP ops",
//    SseMovOps, 0x01, "Move operations"
//    SseMovOpsElim, 0x02, "Move operations eliminated"
//    OptPotential, 0x04, ""
//    Optimized, 0x08, ""
//);
//decl_event!(FpOpsRetiredByType, 0x00a, "");
//decl_event!(FpSseAvxOpsRetired, 0x00b, "");
//decl_event!(FpPackOpsRetired, 0x00c, "");
//decl_event!(FpPackedIntOpType, 0x00d, "");
//decl_event!(FpDispFaults, 0x00e, "");
//
//decl_event_masks!(LsBadStatus2, 0x024, "",
//    UnkWidthMismatch, 0x02, ""
//);
//decl_event!(LsLocks, 0x025, "");
//decl_event!(LsRetClFlush, 0x026, "");
//decl_event!(LsRetCpuid, 0x027, "");
//decl_event_masks!(LsDispatch, 0x029, "",
//    LdDispatch, 0x01, "Loads"
//    StDispatch, 0x02, "Stores"
//    LdStDispatch, 0x04, "Load/stores"
//);
//decl_event!(LsDataPipe, 0x02f, "");
//decl_event!(LsStMisalign, 0x032, "");
//decl_event!(LsSTLF, 0x035, "");
//decl_event!(LsStoreCommitCancel, 0x036, "");
//decl_event!(LsStoreCommitCancel2, 0x037, "");
//decl_event!(LsMabAlloc, 0x041, "");
//decl_event!(LsMisalLoads, 0x047, "");
//decl_event!(LsPrefInstrDisp, 0x04b, "");
//decl_event!(LsWcbClosePremature, 0x050, "");
//decl_event!(LsNotHaltedCyc, 0x076, "Cycles not in halt");
//
//decl_event!(BpL1BTBCorrect, 0x08a, "");
//decl_event!(BpL2BTBCorrect, 0x08b, "");
//decl_event!(BpL0BTBHit, 0x08d, "");
//decl_event!(BpDynIndPred, 0x08e, "");
//decl_event!(IfDqBytesFetched, 0x08f, "");
//decl_event!(BpDeReDirect, 0x091, "");
//decl_event_masks!(BpRedirect, 0x09f, "",
//    BpL2Redir, 0x20, ""
//);
//
//decl_event_masks!(DeMsStall, 0x0a8, "",
//    Serialize, 0x01, ""
//    WaitForQuiet, 0x02, ""
//    WaitForSegId, 0x04, ""
//    WaitForStQ, 0x08, ""
//    WaitForQuietCurTID, 0x10, ""
//    WaitForQuietOthrTID, 0x20, ""
//    MutexStall, 0x40, ""
//    WaitForCount, 0x80, ""
//);
//decl_event!(DeDisUopQueueEmpty, 0x0a9, "");
//decl_event!(DeSrcOpDisp, 0x0aa, "");
//decl_event_masks!(DeDisOpsFromDecoder, 0x0ab, "",
//    FastPath, 0x01, ""
//    Microcode, 0x02, ""
//    Fp, 0x04, ""
//    Int, 0x08, ""
//);
//decl_event_masks!(DeDisDispatchTokenStalls1, 0x0ae, "",
//    IntPhyRegFileRsrcStall, 0x01, ""
//    LoadQueueRsrcStall, 0x02, ""
//    StoreQueueRsrcStall, 0x04, ""
//    IntSchedulerMiscRsrcStall, 0x08, ""
//    TakenBrnchBufferRsrc, 0x10, ""
//    FpRegFileRsrcStall, 0x20, ""
//    FpSchRsrcStall, 0x40, ""
//    FpMiscRsrcStall, 0x80, ""
//);
//decl_event_masks!(DeDisDispatchTokenStalls0, 0x0af, "",
//    ALSQ1RsrcStall, 0x01, ""
//    ALSQ2RsrcStall, 0x02, ""
//    ALSQ3_0_TokenStall, 0x04, ""
//    ALUTokenStall, 0x08, ""
//    AGSQTokenStall, 0x10, ""
//    RetireTokenStall, 0x20, ""
//    ScAguDispatchStall, 0x40, ""
//);
//decl_event!(MemFileHit, 0x0b1, "");
//decl_event!(MemRenLdDsp, 0x0b2, "");
//decl_event!(MemRenLdElim, 0x0b3, "");
//
//decl_event_masks!(DsTokStall3, 0x0b4, "",
//    Cop1Disp, 0x02, ""
//    Cop2Disp, 0x04, ""
//    Cop3Disp, 0x08, ""
//    Cop4Disp, 0x10, ""
//    Cop5Disp, 0x20, ""
//);
//decl_event!(Dsp0Stall, 0x0b5, "");
//decl_event!(DsCopsAfterBrnInDspGrp, 0x0b6, "");
//
//decl_event!(ExRetInstr, 0x0c0, "");
//decl_event!(ExRetCops, 0x0c1, "");
//decl_event!(ExRetBrn, 0x0c2, "");
//decl_event!(ExRetBrnMisp, 0x0c3, "");
//decl_event!(ExRetBrnTaken, 0x0c4, "");
//decl_event!(ExRetBrnTakenMisp, 0x0c5, "");
//decl_event!(ExRetBrnFar, 0x0c6, "");
//decl_event!(ExRetNearRet, 0x0c8, "");
//decl_event!(ExRetNearRetMisp, 0x0c9, "");
//decl_event!(ExRetBrnIndMisp, 0x0ca, "");
//decl_event!(ExRetMmxFpInstr, 0x0cb, "");
//decl_event!(ExRetCond, 0x0d1, "");
//decl_event!(ExRetireEmpty, 0x0d9, "");
//
//decl_event!(ExRetUcodeInst, 0x1c0, "");
//decl_event!(ExRetUcodeOps, 0x1c1, "");
//decl_event!(ExRetMsprdBrnchInstrDirMsmtch, 0x1c7, "");
//decl_event!(Bp1RetBrUncondMisp, 0x1c8, "");
//decl_event!(ExMovElim, 0x1db, "");


