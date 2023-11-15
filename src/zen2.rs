
use std::collections::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PMCDesc {
    pub id: u16,
    pub umask: u8, 
    pub name: &'static str,
    pub unit: Unit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unit {
    None,
    Cycle,
}

#[derive(Clone, Copy, Debug)]
pub struct PMCEvent {
    pub id: u16,
    pub name: &'static str,
    pub unit: Unit,
    pub mask: &'static [PMCMask],
    pub has_umask: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct PMCMask {
    pub umask: u8,
    pub name: &'static str,
}

pub struct EventSet {
    pub set: BTreeSet<(u16, u8)>,
    pub db: BTreeMap<u16, PMCEvent>,
}
impl EventSet {
    pub fn new() -> Self { 
        let mut db = BTreeMap::new();
        for event in ZEN2_EVENTS {
            db.insert(event.id, *event);
        }
        Self { 
            set: BTreeSet::new(),
            db,
        }
    }
    pub fn add_event_nomask(&mut self, event: u16) {
        self.set.insert((event, 0x00));
    }
    pub fn add_event_mask(&mut self, event: u16, mask: u8) {
        self.set.insert((event, mask));
    }
    pub fn add_event_bits(&mut self, event: u16) {
        for mask in [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80] {
            self.set.insert((event, mask));
        }
    }
    pub fn clear(&mut self) {
        self.set.clear();
    }

    pub fn iter(&self) -> std::collections::btree_set::Iter<(u16, u8)> {
        self.set.iter()
    }

}


pub struct Zen2EventMap {
    by_event: BTreeMap<u16, PMCEvent>,
}
impl Zen2EventMap {
    pub fn new() -> Self {
        let mut by_event = BTreeMap::new();
        for event in ZEN2_EVENTS {
            by_event.insert(event.id, *event);
        }
        Self {
            by_event,
        }
    }
    pub fn lookup(&self, event: u16) -> Option<&PMCEvent> {
        self.by_event.get(&event)
    }

}

pub const ZEN2_EVENTS: &[PMCEvent] = &[

    // ------------------------------------------------------------------
    // Floating-point

    PMCEvent { id: 0x003, name: "FpRetSseAvxOps", unit: Unit::None, 
        has_umask: true,
        mask: &[
            PMCMask { umask: 0x01, name: "SseMovOps" },
            PMCMask { umask: 0x02, name: "SseMovOpsElim" },
            PMCMask { umask: 0x04, name: "OptPotential" },
            PMCMask { umask: 0x08, name: "Optimized" },
        ],
    },
    PMCEvent { id: 0x00a, name: "FpOpsRetiredByType", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x00b, name: "SseAvxOpsRetired", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x00c, name: "FpPackOpsRetired", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x00d, name: "PackedIntOpType", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x00e, name: "FpDispFaults", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },


    // ------------------------------------------------------------------
    // Load/store

    PMCEvent { id: 0x024, name: "LsBadStatus2", unit: Unit::None, 
        has_umask: true,
        mask: &[
            PMCMask { umask: 0x02, name: "UnkWidthMismatch?" },
        ],
    },
    PMCEvent { id: 0x025, name: "LsLocks", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x026, name: "LsRetClFlush", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x027, name: "LsRetCpuid", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x029, name: "LsDispatch", unit: Unit::None, 
        has_umask: true,
        mask: &[
            PMCMask { umask: 0x01, name: "LdDispatch" },
            PMCMask { umask: 0x02, name: "StDispatch" },
            PMCMask { umask: 0x04, name: "LdStDispatch" },
        ],
    },

    PMCEvent { id: 0x02f, name: "LsDataPipe", unit: Unit::None, 
        has_umask: true,
        mask: &[
            PMCMask { umask: 0x01, name: "PureLd" },
            PMCMask { umask: 0x02, name: "LdOpSt" },
            PMCMask { umask: 0x04, name: "PureSt" },
        ],
    },



    PMCEvent { id: 0x032, name: "LsStMisalign", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x035, name: "LsSTLF", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x036, name: "LsStoreCommitCancel", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x037, name: "LsStoreCommitCancel2", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },

    PMCEvent { id: 0x041, name: "LsMabAlloc", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x047, name: "LsMisalLoads", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x04b, name: "LsPrefInstrDisp", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },

    PMCEvent { id: 0x50, name: "WcbClosePremature", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },

    PMCEvent { id: 0x76, name: "LsNotHaltedCyc", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },

    // ------------------------------------------------------------------
    // Instruction fetch / Branch prediction

    PMCEvent { id: 0x087, name: "unk_087", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },

    PMCEvent { id: 0x08a, name: "BpL1BTBCorrect", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },
    PMCEvent { id: 0x08b, name: "BpL2BTBCorrect", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },
    PMCEvent { id: 0x08d, name: "BpL0BTBHit?", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },

    PMCEvent { id: 0x08e, name: "BpDynIndPred", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },


    PMCEvent { id: 0x08f, name: "IfDqBytesFetched?", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },

    PMCEvent { id: 0x091, name: "BpDeReDirect", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },


    PMCEvent { id: 0x09f, name: "BpRedirect?", unit: Unit::None, 
        has_umask: true,
        mask: &[
            PMCMask { umask: 0x20, name: "BpL2Redir?" },
        ],
    },

    // ------------------------------------------------------------------
    // Decode / dispatch / scheduling?

    PMCEvent { id: 0x0a8, name: "DeMsStall", unit: Unit::None, 
        has_umask: true,
        mask: &[
            PMCMask { umask: 0x01, name: "Serialize" },
            PMCMask { umask: 0x02, name: "WaitForQuiet" },
            PMCMask { umask: 0x04, name: "WaitForSegId" },
            PMCMask { umask: 0x08, name: "WaitForStQ" },
            PMCMask { umask: 0x10, name: "WaitForQuietCurTID" },
            PMCMask { umask: 0x20, name: "WaitForQuietOthrTID" },
            PMCMask { umask: 0x40, name: "MutexStall" },
            PMCMask { umask: 0x80, name: "WaitForCount" },
        ],
    },
    PMCEvent { id: 0x0a9, name: "DeDisUopQueueEmpty", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },
    PMCEvent { id: 0x0aa, name: "DeSrcOpDisp", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },
    PMCEvent { id: 0x0ab, name: "DeDisOpsFromDecoder", unit: Unit::None, 
        has_umask: true,
        mask: &[
            PMCMask { umask: 0x01, name: "FastPath" },
            PMCMask { umask: 0x02, name: "Microcode" },
            PMCMask { umask: 0x04, name: "Fp" },
            PMCMask { umask: 0x08, name: "Int" },
        ],
    },

    PMCEvent { id: 0x0ac, name: "unk_0ac", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },
    PMCEvent { id: 0x0ad, name: "unk_0ad", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },



    PMCEvent { id: 0x0ae, name: "DeDisDispatchTokenStalls1", unit: Unit::None, 
        has_umask: true,
        mask: &[
            PMCMask { umask: 0x01, name: "IntPhyRegFileRsrcStall" },
            PMCMask { umask: 0x02, name: "LoadQueueRsrcStall" },
            PMCMask { umask: 0x04, name: "StoreQueueRsrcStall" },
            PMCMask { umask: 0x08, name: "IntSchedulerMiscRsrcStall" },
            PMCMask { umask: 0x10, name: "TakenBrnchBufferRsrc" },
            PMCMask { umask: 0x20, name: "FpRegFileRsrcStall" },
            PMCMask { umask: 0x40, name: "FpSchRsrcStall" },
            PMCMask { umask: 0x80, name: "FpMiscRsrcStall" },
        ],
    },
    PMCEvent { id: 0x0af, name: "DeDisDispatchTokenStalls0", unit: Unit::None, 
        has_umask: true,
        mask: &[
            PMCMask { umask: 0x01, name: "ALSQ1RsrcStall" },
            PMCMask { umask: 0x02, name: "ALSQ2RsrcStall" },
            PMCMask { umask: 0x04, name: "ALSQ3_0_TokenStall" },
            PMCMask { umask: 0x08, name: "ALUTokenStall" },
            PMCMask { umask: 0x10, name: "AGSQTokenStall" },
            PMCMask { umask: 0x20, name: "RetireTokenStall" },
            PMCMask { umask: 0x40, name: "ScAguDispatchStall" },
        ],
    },

    PMCEvent { id: 0x0b1, name: "MemFileHit", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },

    PMCEvent { id: 0x0b2, name: "MemRenLdDsp", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },

    PMCEvent { id: 0x0b3, name: "RmeRenLdElim", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },


    PMCEvent { id: 0x0b4, name: "DsTokStall3", unit: Unit::None, 
        has_umask: true,
        mask: &[
            PMCMask { umask: 0x01, name: "Unk01" },
            PMCMask { umask: 0x02, name: "Cop1Disp" },
            PMCMask { umask: 0x04, name: "Cop2Disp" },
            PMCMask { umask: 0x08, name: "Cop3Disp" },
            PMCMask { umask: 0x10, name: "Cop4Disp" },
            PMCMask { umask: 0x20, name: "Cop5Disp" },
            PMCMask { umask: 0x40, name: "Unk40" },
            PMCMask { umask: 0x80, name: "Unk80" },
        ],
    },

    PMCEvent { id: 0x0b5, name: "Dsp0Stall", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },


    PMCEvent { id: 0x0b6, name: "DsCopsAfterBrnInDspGrp", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },

    PMCEvent { id: 0x0be, name: "unk_0be", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },



    // ------------------------------------------------------------------
    // Execution

    PMCEvent { id: 0x0c0, name: "ExRetInstr", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },

    PMCEvent { id: 0x0c1, name: "ExRetCops", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },
    PMCEvent { id: 0x0c2, name: "ExRetBrn", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },
    PMCEvent { id: 0x0c3, name: "ExRetBrnMisp", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },
    PMCEvent { id: 0x0c4, name: "ExRetBrnTaken", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },
    PMCEvent { id: 0x0c5, name: "ExRetBrnTakenMisp", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },
    PMCEvent { id: 0x0c6, name: "ExRetBrnFar", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },
    PMCEvent { id: 0x0c8, name: "ExRetNearRet", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },

    PMCEvent { id: 0x0c9, name: "ExRetNearRetMisp", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },

    PMCEvent { id: 0x0ca, name: "ExRetBrnIndMisp", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },

    PMCEvent { id: 0x0cb, name: "ExRetMmxFpInstr", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },

    PMCEvent { id: 0x0d0, name: "unk_0d0", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },



    PMCEvent { id: 0x0d1, name: "ExRetCond", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },


    PMCEvent { id: 0x0d5, name: "unk_0d5", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },

    PMCEvent { id: 0x0d9, name: "ExRetireEmpty?", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },


    // ------------------------------------------------------------------
    // Miscellaneous

    PMCEvent { id: 0x181, name: "BpL2TgeDirOvr?", unit: Unit::None, 
        has_umask: true,
        mask: &[
        ],
    },

    // Retired microcoded instructions?
    PMCEvent { id: 0x1c0, name: "ExRetUcodeInst?", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },

    // Retired uops generated by microcoded instructions?
    PMCEvent { id: 0x1c1, name: "ExRetUcodeInstOps?", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },

    PMCEvent { id: 0x1c2, name: "unk_1c2", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },


    PMCEvent { id: 0x1c3, name: "unk_1c3", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },


    PMCEvent { id: 0x1d6, name: "unk_1d6", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },

    // Counts eliminated moves in the integer pipeline?
    PMCEvent { id: 0x1db, name: "ExMovElim?", unit: Unit::None, 
        has_umask: false,
        mask: &[
        ],
    },


];


