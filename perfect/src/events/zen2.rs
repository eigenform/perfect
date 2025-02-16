
// NOTE: This strategy for defining events kind of sucks. 

use crate::events::*;

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
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

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum LsL1DtlbMissMask {
    TlbReload4KL2Hit,
    TlbReload32KL2Hit,
    TlbReload2ML2Hit,
    TlbReload1GL2Hit,
    TlbReload4KL2Miss,
    TlbReload32KL2Miss,
    TlbReload2ML2Miss,
    TlbReload1GL2Miss,
    Unk(u8),
}
impl LsL1DtlbMissMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::TlbReload4KL2Hit => MaskDesc::new(0x01,  "TlbReload4KL2Hit"),
            Self::TlbReload32KL2Hit => MaskDesc::new(0x02, "TlbReload32KL2Hit"),
            Self::TlbReload2ML2Hit => MaskDesc::new(0x04,  "TlbReload2ML2Hit"),
            Self::TlbReload1GL2Hit => MaskDesc::new(0x08,  "TlbReload1GL2Hit"),
            Self::TlbReload4KL2Miss => MaskDesc::new(0x10, "TlbReload4KL2Miss"),
            Self::TlbReload32KL2Miss => MaskDesc::new(0x20,"TlbReload32KL2Miss"),
            Self::TlbReload2ML2Miss => MaskDesc::new(0x40, "TlbReload2ML2Miss"),
            Self::TlbReload1GL2Miss => MaskDesc::new(0x80, "TlbReload1GL2Miss"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}


#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
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

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum LsDataPipeMask {
    PureLd,
    LdOpSt,
    PureSt,
    HwPfStreamPick,
    HwPfStrideRegPick,
    Unk(u8),
}
impl LsDataPipeMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::PureLd => MaskDesc::new(0x01, "PureLd"),
            Self::LdOpSt => MaskDesc::new(0x02, "LdOpSt"),
            Self::PureSt => MaskDesc::new(0x04, "PureSt"),
            Self::HwPfStreamPick => MaskDesc::new(0x08, "HwPfStreamPick"),
            Self::HwPfStrideRegPick => MaskDesc::new(0x30, "HwPfStrideRegPick"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}


#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum LsMabAllocMask {
    Loads,
    Stores,
    DcPrefetcher,
    Unk(u8),
}
impl LsMabAllocMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::Loads => MaskDesc::new(0x01, "Loads"),
            Self::Stores => MaskDesc::new(0x02, "Stores"),
            Self::DcPrefetcher => MaskDesc::new(0x08, "DcPrefetcher"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}




#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
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

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum BpL1TlbMissL2TlbMissMask {
    If4k,
    If2m,
    If1g,
    Unk(u8),
}
impl BpL1TlbMissL2TlbMissMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            // These are from Family 0x1A ..
            Self::If4k => MaskDesc::new(0x01,"IF4K"),
            Self::If2m => MaskDesc::new(0x02,"IF2M"),
            Self::If1g => MaskDesc::new(0x04,"IF1G"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum IcFetchStallCycMask {
    BackPressure,
    DqEmpty,
    Any,
    Unk(u8),
}
impl IcFetchStallCycMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            // These are from Family 0x1A ..
            Self::BackPressure => MaskDesc::new(0x01,"BackPressure"),
            Self::DqEmpty => MaskDesc::new(0x02,"DqEmpty"),

            Self::Any => MaskDesc::new(0x04,"Any"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum BpRedirectMask {
    BpL2Redir,
    Unk(u8),
}
impl BpRedirectMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            // These are from Family 0x1A ..
            Self::BpL2Redir => MaskDesc::new(0x01,"Resync"),
            Self::BpL2Redir => MaskDesc::new(0x02,"ExRedir"),

            Self::BpL2Redir => MaskDesc::new(0x20,"BpL2Redir"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum BpL1TlbFetchHitMask {
    /// 4K page
    If4k,
    /// 2M page
    If2m,
    /// 1G page
    If1g,
    Unk(u8),
}
impl BpL1TlbFetchHitMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            // These are from Family 0x1A ..
            Self::If4k => MaskDesc::new(0x01,"IF4K"),
            Self::If2m => MaskDesc::new(0x02,"IF2M"),
            Self::If1g => MaskDesc::new(0x04,"IF1G"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}



#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
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


#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
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


#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
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


#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
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


#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum DsTokStall3Mask {
    /// Cycles where no ops were dispatched? 
    Zero,
    /// Cycles where one op was dispatched
    Cop1Disp,
    /// Cycles where two ops were dispatched
    Cop2Disp,
    /// Cycles where three ops were dispatched
    Cop3Disp,
    /// Cycles where four ops were dispatched
    Cop4Disp,
    /// Cycles where five ops were dispatched
    Cop5Disp,
    /// Cycles where six ops were dispatched
    Cop6Disp,
    /// Cycles where at least one op was dispatched
    NonZero,
    Unk(u8),
}
impl DsTokStall3Mask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::Zero     => MaskDesc::new(0x01, "Zero"),
            Self::Cop1Disp => MaskDesc::new(0x02, "Cop1Disp"),
            Self::Cop2Disp => MaskDesc::new(0x04, "Cop2Disp"),
            Self::Cop3Disp => MaskDesc::new(0x08, "Cop3Disp"),
            Self::Cop4Disp => MaskDesc::new(0x10, "Cop4Disp"),
            Self::Cop5Disp => MaskDesc::new(0x20, "Cop5Disp"),
            Self::Cop6Disp => MaskDesc::new(0x40, "Cop6Disp"),
            Self::NonZero  => MaskDesc::new(0x7e, "NonZero"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum StkEngFxOpMask {
    // Micro-op queue? 
    UopQ,
    // Dispatch? 
    Dsp,
    Unk(u8),
}
impl StkEngFxOpMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::UopQ => MaskDesc::new(0x01, "UopQ"),
            Self::Dsp => MaskDesc::new(0x02, "Dsp"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}


/// Zen 2 events. 
///
/// This list is cobbled together from all publically-available documentation 
/// for various Zen families (PPRs for 17h and 19h parts) and *lots* of 
/// experiments. 
///
/// Instead of statically defining all of these, the full event is built out 
/// of this enum during runtime. This is largely just a hack to get a nice 
/// auto-complete-able enum of events in my editor with 'rust-analyzer'. 
///
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Zen2Event { 

    // 0x00 - FpuPipeAssignment

    // 0x01 - FpSchedEmpty
    
    // 0x02 - FpRetx87FpOps

    // 0x03 - FpRetSseAvxOps
    // (from old Family 17h PPRs)
    FpRetSseAvxOps(FpRetSseAvxOpsMask),

    // 0x04 - FpNumMovElimScalOp, move elimination [fp] ops 
    // (from old Family 17h PPRs)
    //FpNumMovElimScalOp(u8),

    // 0x05 - FpRetiredSerOps, retired [fp] serializing ops
    // (from old Family 17h PPRs)
    //FpRetiredSerOps(u8),

    // 0x06:01 - seemingly valid, highly variable and spurious

    // 0x07 - FpModsToFpCntrlWrd ?
  
    // 0x08 - FpOpsRetiredByWidth?

    // 0x09 - FpThrottlePipeClks?

    // 0x0a - FpOpsRetiredByType ?
    FpOpsRetiredByType(u8),

    // 0x0b
    FpSseAvxOpsRetired(u8),

    // 0x0c
    FpPackOpsRetired(u8),

    // 0x0d
    FpPackedIntOpType(u8),

    // 0x0e
    FpDispFaults(u8),

    // 0x0f ? 
    
    // NOTE: No observations for 0x10 - 0x1f

    // 0x20:02,04 is valid


    // 0x21

    // 0x22

    // 0x23 is valid
    //  - :02, rdrand,rdseed
    //  - :10, xsave
    //  - :80, rdrand, rdseed

    // 0x24
    LsBadStatus2(LsBadStatus2Mask),
    
    // 0x25
    LsLocks(u8),

    // 0x26
    LsRetClFlush(u8),

    // 0x27
    LsRetCpuid(u8),

    // 0x28?

    // 0x29
    LsDispatch(LsDispatchMask),

    // 0x2a?

    // 0x2b?

    // 0x2c - valid

    // 0x2d - [speculative] rdtsc reads
    LsRdTsc(u8),

    // 0x2e?

    // 0x2f - valid
    LsDataPipe(LsDataPipeMask),

    // 0x30?
    // 0x31?

    // 0x32
    LsStMisalign(u8),

    // 0x33
    // 0x34

    // 0x35
    LsSTLF(u8),

    // 0x36
    LsStoreCommitCancel(u8),

    // 0x37
    LsStoreCommitCancel2(u8),

    // 0x38?

    // 0x39:01 counts for 8-bit stores?
    // 0x39:02 counts for 8-bit stores? 

    // 0x3a?

    // 0x3b?

    // 0x3c?

    // 0x3d?

    // 0x3e?

    // 0x3f?

    // 0x40 - valid
    LsDcAccesses(u8),

    // 0x41
    LsMabAlloc(LsMabAllocMask),

    // 0x42?

    // 0x43 - LsRefillsFromSys? 

    // 0x44

    // 0x45 - LsL1DTlbMiss?
    // 0x45:04 - prefetching kernel
    LsL1DTlbMiss(LsL1DtlbMissMask),

    // 0x46 - LsTablewalker?  (from 17h, Model 01h PPR)
    // 0x46:01 - 'PerfmonTablewalkAllocDside0'
    // 0x46:02 - 'PerfmonTablewalkAllocDside1'
    // 0x46:04 - 'PerfmonTablewalkAllocIside0'
    // 0x46:08 - 'PerfmonTablewalkAllocIside1'

    // 0x47
    LsMisalLoads(u8),

    // 0x48

    // 0x4b
    LsPrefInstrDisp(u8),

    // 0x4d:01 - counts for xsave (???)
    // 0x4d:02 - counts for xsave (???)
    // 0x4d:00 - counts for xsave (???)

    // 0x4e - counts for xsave (???)

    // 0x4f - counts for xsave (???), no mask?

    // 0x50? 
    LsWcbClosePremature(u8),


    // 0x51:06 - DemHitHwPfMabStrideReg? 

    // 0x52 - LsInefSwPref?

    // 0x53

    // 0x54

    // 0x55

    // 0x56 - counts for rdrand/rdseed, no mask?

    // 0x57:06 - HwPfMabAllocStrideReg? 

    // 0x58 - NumMabMatchHwPref?

    // 0x59 - LSSwPfDcFills

    // 0x5a - LsHwPfDcFills

    // 0x5b - LsTwDcFills

    // 0x5c

    // 0x5d

    // 0x5e

    // 0x5f - LsAllocMabCount

    // 0x60 - L2RequestG1

    // 0x61 - L2RequestG2

    // 0x62 - L2Latency?

    // 0x63 - L2WbcReq

    // 0x64 - L2CacheReqStat?

    // 0x65

    // 0x66:c0 - L24IcVal1DcVal1 
    // 0x66:30 - L24IcVal1DcVal0
    // 0x66:0c - L24IcVal0DcVal1
    // 0x66:03 - L24IcVal0DcVal0

    // 0x67

    // 0x68

    // 0x69

    // 0x6a:01 - counts for rdrand/rdseed?
    // 0x6a:04 - counts for clflush and clzero?

    // 0x6b

    // 0x6c:02 - L26L2ClksUnderDvmSyncQuiesce (???)

    // 0x6d - L2FillPending

    // 0x6e

    // 0x6f:0c - L211PromotedLsHwPfDemCnt? 

    // 0x70 - L2PfHitL2

    // 0x71 - L2PfHitL3

    // 0x72 - L2PfMissL3

    // 0x73

    // 0x74 - counts for rdrand/rdseed

    // 0x75

    // 0x76 
    LsNotHaltedCyc(u8),

    // 0x77:01 - LsNotHaltedP0Cyc.P0FreqCyc (?)

    // 0x78:01 - LsTlbFlush.all?

    // 0x79 - valid? (L2 prefetch accuracy?)

    // 0x7a - valid?

    // 0x7b

    // 0x7c - valid? (l2 related?)

    // 0x7d

    // 0x7e

    // 0x7f

    // 0x80 - Number of 32B windows passed to decoder?
    IcFw32(u8),

    // 0x81 - Number of L1I tag misses
    IcFw32Miss(u8),

    // 0x82
    IcCacheFillL2(u8),

    // 0x83
    IcCacheFillSys(u8),

    // 0x84 - L1 ITLB miss into L2 ITLB hit ?
    BpL1TlbMissL2TlbHit(u8),

    // 0x85 - L1 ITLB miss into L2 ITLB miss ?
    BpL1TlbMissL2TlbMiss(BpL1TlbMissL2TlbMissMask),

    // 0x86 - BpSnpReSync? "Pipeline Restart Due to Instruction Stream Probe" 

    // 0x87
    IcFetchStallCyc(IcFetchStallCycMask),

    // 0x88 - valid, no mask?
    //  - conditional branch loops
    //  - ret
    //  - mfence
    //  - cpuid
    //  - xrstor
    //

    // 0x89

    // 0x8a
    BpL1BTBCorrect(u8),

    // 0x8b
    BpL2BTBCorrect(u8),

    // 0x8c - IcCacheInval?

    // 0x8d - valid?
    // NOTE: from 19h, I'm not convinced this is accurately labeled?
    BpL0BTBHit(u8),

    // 0x8e
    BpDynIndPred(u8),

    // 0x8f
    // NOTE: From 19h - This doesn't seem accurate..
    IfDqBytesFetched(u8),




    // 0x90: counts for ret only? no mask? (this is ClksBpStalled in 19h?)
    // - ret?

    // 0x91 - Redirect from decode
    BpDeReDirect(u8),

    // 0x92: ?

    // 0x93: ?

    /// 0x94 - L1 ITLB fetch hit
    BpL1TlbFetchHit(BpL1TlbFetchHitMask),
 
    // 0x95: ?

    // 0x96: ?

    // 0x97: valid?

    // 0x98: ?

    // 0x99: ?

    // 0x9a: ?

    // 0x9b: valid?, no mask?
    //  - 1 for conditional branches, jmp, ind jmp, ind call
    //  - 3 for ret?
    //  - 23 for verr,verw,mfence,cpuid,xrstor?

    // 0x9c: valid?, no mask?
    //  - 1 for branches, jmp,call,ret
    //  - 16 for lsl,lar,verr,verw,mfence,cpuid,xrstor?

    // 0x9d: valid? same as 9c?, no mask?

    // 0x9e: valid? same as 9c and 9d?, no mask?

    // 0x9f - Redirects? no mask?
    //  - 1 for ret, mfence, cpuid, xrstor, conditional branch loop?
    // NOTE: this name is from 19h? maybe not accurate? 
    BpRedirect(BpRedirectMask),

    // 0xa0:01 counts during jcc loop

    // 0xa1? - oc_set_way_ent_acc in 19h
    // 0x0a:01 counts during jcc, loop

    // 0xa2? - no mask. oc_builds in 19h?

    // 0xa3 - no mask. highly variable, probably some kind of latency?

    // 0xa4 - no mask. 
    //  - indirect call
    //  - ret
    //  - mfence
    //  - cpuid
    //  - xrstor

    // 0xa5:01 - counts for most ops
    // 0xa5:04 - ret,mfence,cpuid,xrstor?
    // 0xa5:10 - call,ret,lsl,lar,verr,verw,mfence,cpuid,xrstor?

    // 0xa6:02 - cpuid, f2xm1?
    // 0xa6:04 - lsl,lar,verr,verw,mfence,cpuid,xrstor?
    //

    // 0xa7:01 is valid (counts during most-if-not-all ops?)
    // 0xa7:08 is valid (counts during ucoded instrs, some branches? misp?)

    // 0xa8
    DeMsStall(DeMsStallMask),

    // 0xa9
    DeDisUopQueueEmpty(u8),

    // 0xaa 
    DeSrcOpDisp(u8),

    // 0xab
    DeDisOpsFromDecoder(DeDisOpsFromDecoderMask),

    // NOTE: 0xac and 0xad kind of behave the same? Adds up to LsNotHaltedCyc?
    // I would not be suprised if these are for the schedulers, or reflect
    // the ALU pipelines
    // 0xac is valid, no mask

    // 0xad:01 - loops, ret
    // 0xad:02 - most ops
    // 0xad:04 - most ops
    // 0xad:08 - most ops
    // 0xad:10 - most ops
    // 0xad:20 - most ops
    // 0xad:40 - most ops

    // 0xae
    DeDisDispatchTokenStalls1(DeDisDispatchTokenStalls1Mask),

    // 0xaf
    DeDisDispatchTokenStalls0(DeDisDispatchTokenStalls0Mask),


    // 0xb0?

    // 0xb1
    MemFileHit(u8),

    // 0xb2
    MemRenLdDsp(u8),

    // 0xb3
    MemRenLdElim(u8),

    // 0xb4
    // seems valid; all masks vaguely add up to LsNotHaltedCyc?
    DsTokStall3(DsTokStall3Mask),

    // 0xb5:01 seems valid (inconsistent floor?)
    // - rep lodsq [rsi]
    // - xsave [0x100]
    // - rdseed
    // - mfence
    // - lsl/lar/verr/verw
    // - cld
    // - pdep/pext
    // - ret
    Dsp0Stall(u8),

    // 0xb6:01,02,04 seem valid? 
    // - rep lodsq [rsi]
    // - xgetbv
    // - rdpmc
    // - rdtsc/p
    // - cpuid
    // - rdseed/rdrand
    // - mfence
    // - lsl/lar/verr/verw
    // - call
    // - jmp
    // - branches
    DsCopsAfterBrnInDspGrp(u8),

    // 0xb7 ? 
    DsLoopModeInstrs(u8),

    // 0xb8:01,02 - Counts when rsp is used in an integer op? 
    StkEngFxOp(StkEngFxOpMask),
    
    // 0xb9?

    // 0xba?

    // 0xbb?

    // 0xbc seems valid? counts for fp ops and vzeroupper? no mask?

    // 0xbd seems valid? counts only for vzeroall? no mask?

    // 0xbe:00 - counts for call, ret, push, pop, no mask?
    // Also counts when rsp is used in integer ops, or in addressing? 
    StkEngRspDltUs(u8),

    //0xbe - RipRelAgenUsesDisp? 

    //0xbf

    // 0xc0
    ExRetInstr(u8),

    // 0xc1
    ExRetCops(u8),

    // 0xc2
    ExRetBrn(u8),

    // 0xc3
    ExRetBrnMisp(u8),

    // 0xc4
    ExRetBrnTaken(u8),

    // 0xc5
    ExRetBrnTakenMisp(u8),

    // 0xc6
    ExRetBrnFar(u8),

    // 0xc7
    ExRetBrnResync(u8),

    // 0xc8
    ExRetNearRet(u8),

    // 0xc9
    ExRetNearRetMisp(u8),

    // 0xca
    ExRetBrnIndMisp(u8),

    // 0xcb
    ExRetMmxFpInstr(u8),

    // 0xcc
    //ExRetIndBrchInstr(u8)

    // 0xcd

    // 0xce

    // 0xcf:01 - "fp_micro_faults"?
    // 0xcf:02 - "fp_micro_traps"?
    // 0xcf:10 - "fp_bypass_faults"?
    // 0xcf:0c - "fp_ext2_int_int2_ext_faults"?

    // 0xd0 is valid but inconsistent; could be memory related?, no mask
    //  - xsave [0x100]
    //  - rdseed, rdrand

    // 0xd1, from 19h, old 17h - unverified?
    ExRetCond(u8),

    // 0xd2, unverified, from 17h
    ExRetCondMisp(u8),

    // 0xd3
    //   - div, idiv
    ExDivBusy(u8),

    // 0xd4
    //   - div, idiv
    ExDivCount(u8),

    // 0xd5 (seems related to 0x1d6?)
    // - no mask
    // - counts either speculative ops, or cycles? 
    // Must only be relevant for integer/mem ops? 
    // NOTE: This might actually be *scheduled* ops
    //
    //  Counts for: 
    //      - zero idioms, add immediate
    //      - register-to-register moves from a nonzero register? 
    //      - scheduled ops?
    //      - moves from fp to int, int to fp
    //      - avx ops that reference memory? 
    //      - comiss? 
    //
    //  Doesn't count for:
    //      - register-to-register moves from a zeroed register?
    //      - direct unconditional branches

    // 0xd6 (doesn't seem to count anything...? ex_no_retire in 19h?)

    // 0xd7 ?

    // 0xd8 ?

    // 0xd9, from 19h - unverified?
    ExRetireEmpty(u8),

    // 0xda ?

    // 0xdb ?

    // 0xdc ?

    // 0xdd ?

    // 0xde ?

    // 0xdf ?

    // 0x1c0?
    
    // 0x1c1
    ExRetUcodeInst(u8),

    // 0x1c2
    ExRetUcodeOps(u8),

    // 0x1c3, valid (from 19h?), unverified
    UopReqInterruptCheck(u8),

    // 0x1c4?

    // 0x1c5? - only counts for ret, no masks

    // 0x1c6?

    // 0x1c7, unverified?
    ExRetMsprdBrnchInstrDirMsmtch(u8),

    // 0x1c8, unverified?
    // - ret
    Bp1RetBrUncondMisp(u8),

    // 0x1c9 - unconditional branch related? no masks
    // - ret, call, jmp

    // 0x1ca

    // 0x1cb - "SmExMul1RegOutput"
    //   - imul eax/ax
    //   - mul al/ah
    SmExMul1RegOutput(u8),

    // 0x1cc - "SmExMul2RegOutput"
    //   - mulx eax,eax,eax
    //   - mul eax/ax
    SmExMul2RegOutput(u8),

    // 0x1cd - "LgExMul1RegOutput"
    //   - crc32, 
    //   - imul rax,rax
    LgExMul1RegOutput(u8),

    // 0x1ce - "LgExMul2RegOutput"
    //   - rdtsc,rdtscp (presumably for TSC scaling?)
    //   - mul rax/eax
    //   - mulx rax,rax,rax
    //   - mul rax
    LgExMul2RegOutput(u8),


    // 0x1cf

    // 0x1d0 - ExRetFusBrnchInst? 

    // 0x1d1 ?

    // 0x1d2 ?

    // 0x1d3 ?

    // 0x1d4 ?

    // 0x1d5 ?

    // 0x1d6 - valid, unclear? 

    // 0x1d7

    // 0x1d8

    // 0x1d9

    // 0x1da

    // 0x1db- Think this is *retired* eliminated moves
    ExMovElim(u8),

    // 0x1dc - valid, seemingly inconsistent?, nomask
    // - counts during lock prefix ops?
    // - rdseed/rdrand


    /// Bring-your-own event and mask. 
    Unk(u16, u8),
}
impl AsEventDesc for Zen2Event { 
    fn unk_desc(id: u16, mask: u8) -> Self { 
        Self::Unk(id, mask)
    }
    fn as_desc(&self) -> EventDesc { 
        match self { 
            Self::Unk(v, x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new_unk(*v, mask)
            },

            // ---------------------------------
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


            // ---------------------------------
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

            Self::LsRdTsc(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x02d, "LsRdTsc", mask)
            },

            Self::LsDataPipe(m) => {
                let mask = m.desc();
                EventDesc::new(0x02f, "LsDataPipe", mask)
            },

            // ---------------------------------
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

            // ---------------------------------
            Self::LsDcAccesses(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x040, "LsDcAccesses", mask)
            },
            Self::LsMabAlloc(m) => {
                let mask = m.desc();
                EventDesc::new(0x041, "LsMabAlloc", mask)
            },
            Self::LsL1DTlbMiss(m) => {
                let mask = m.desc();
                EventDesc::new(0x045, "LsL1DTlbMiss", mask)
            },
            Self::LsMisalLoads(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x047, "LsMisalLoads", mask)
            },
            Self::LsPrefInstrDisp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x04b, "LsPrefInstrDisp", mask)
            },

            // ---------------------------------
            Self::LsWcbClosePremature(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x050, "LsWcbClosePremature", mask)
            },

            // ---------------------------------
            Self::LsNotHaltedCyc(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x076, "LsNotHaltedCyc", mask)
            },


            // ---------------------------------
            Self::IcFw32(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x080, "IcFw32", mask)
            },
            Self::IcFw32Miss(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x081, "IcFw32Miss", mask)
            },

            Self::IcCacheFillL2(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x082, "IcCacheFillL2", mask)
            },
            Self::IcCacheFillSys(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x083, "IcCacheFillSys", mask)
            },


            Self::BpL1TlbMissL2TlbHit(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x084, "BpL1TlbMissL2TlbHit", mask)
            },
            Self::BpL1TlbMissL2TlbMiss(m) => {
                let mask = m.desc();
                EventDesc::new(0x085, "BpL1TlbMissL2TlbMiss", mask)
            },
            Self::IcFetchStallCyc(m) => {
                let mask = m.desc();
                EventDesc::new(0x087, "IcFetchStallCyc", mask)
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

            // ---------------------------------
            Self::BpDeReDirect(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x091, "BpDeReDirect", mask)
            },

            Self::BpL1TlbFetchHit(m) => {
                let mask = m.desc();
                EventDesc::new(0x094, "BpL1TlbFetchHit", mask)
            }

            Self::BpRedirect(m) => {
                let mask = m.desc();
                EventDesc::new(0x09f, "BpRedirect", mask)
            },

            // ---------------------------------
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


            // ---------------------------------
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

            Self::DsLoopModeInstrs(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0b7, "DsLoopModeInstrs", mask)
            },

            Self::StkEngFxOp(m) => {
                let mask = m.desc();
                EventDesc::new(0x0b8, "StkEngFxOp", mask)
            },

            Self::StkEngRspDltUs(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0be, "StkEngRspDltUs", mask)
            },

            // ---------------------------------
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
            Self::ExRetBrnResync(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0c7, "ExRetBrnResync", mask)
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

            // ---------------------------------
            Self::ExRetCond(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0d1, "ExRetCond", mask)
            },
            Self::ExRetCondMisp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0d2, "ExRetCondMisp", mask)
            },


            Self::ExDivBusy(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0xd3, "ExDivBusy", mask)
            },
            Self::ExDivCount(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0xd4, "ExDivCount", mask)
            },



            Self::ExRetireEmpty(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x0d9, "ExRetireEmpty", mask)
            },


            // ---------------------------------
            Self::ExRetUcodeInst(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1c1, "ExRetUcodeInst", mask)
            },
            Self::ExRetUcodeOps(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1c2, "ExRetUcodeOps", mask)
            },
            Self::UopReqInterruptCheck(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1c3, "UopReqInterruptCheck?", mask)
            },


            Self::ExRetMsprdBrnchInstrDirMsmtch(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1c7, "ExRetMsprdBrnchInstrDirMsmtch", mask)
            },
            Self::Bp1RetBrUncondMisp(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1c8, "Bp1RetBrUncondMisp", mask)
            },

            Self::SmExMul1RegOutput(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1cb, "SmExMul1RegOutput", mask)
            },
            Self::SmExMul2RegOutput(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1cc, "SmExMul2RegOutput", mask)
            },
            Self::LgExMul1RegOutput(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1cd, "LgExMul1RegOutput", mask)
            },
            Self::LgExMul2RegOutput(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1ce, "LgExMul2RegOutput", mask)
            },

            Self::ExMovElim(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0x1db, "ExMovElim", mask)
            },

        }
    }
}


