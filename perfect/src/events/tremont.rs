
use crate::events::*;

#[derive(PartialOrd, Ord, PartialEq, Eq)]
pub enum TopdownBeBoundMask {
    All,
    AllocRestrictions,
    MemScheduler,
    NonMemScheduler,
    Register,
    ReorderBuffer,
    Serialization,
    Unk(u8),
}
impl TopdownBeBoundMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::All => MaskDesc::new(0x00, "All"),
            Self::AllocRestrictions => MaskDesc::new(0x01, "AllocRestrictions"),
            Self::MemScheduler => MaskDesc::new(0x02, "MemScheduler"),
            Self::NonMemScheduler => MaskDesc::new(0x08, "NonMemScheduler"),
            Self::Register => MaskDesc::new(0x20, "Register"),
            Self::ReorderBuffer => MaskDesc::new(0x40, "ReorderBuffer"),
            Self::Serialization => MaskDesc::new(0x10, "Serialization"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq)]
pub enum BrMispMask {
    IndCall,
    Jcc,
    NonReturnInd,
    Return,
    TakenJcc,
    Unk(u8),
}
impl BrMispMask {
    pub fn desc(&self) -> MaskDesc { 
        match self { 
            Self::IndCall => MaskDesc::new(0xfb, "IndCall"),
            Self::Jcc => MaskDesc::new(0x7e, "Jcc"),
            Self::NonReturnInd => MaskDesc::new(0xeb, "NonReturnInd"),
            Self::Return => MaskDesc::new(0xf7, "Return"),
            Self::TakenJcc => MaskDesc::new(0xfe, "TakenJcc"),
            Self::Unk(x) => MaskDesc::new(*x, "Unk"),
        }
    }
}



#[derive(PartialOrd, Ord, PartialEq, Eq)]
pub enum TremontEvent { 
    TopdownBeBound(TopdownBeBoundMask),
    BrMisp(BrMispMask),

    DecodeRestriction(u8),

    // MS_DECODED.MS_ENTRY - 0xe7,0x01 ?
    // NO_ALLOC_CYCLES - 0xca ?

    Unk(u16, u8, &'static str)
}
impl AsEventDesc for TremontEvent {
    fn as_desc(&self) -> EventDesc { 
        match self { 
            Self::TopdownBeBound(m) => {
                let mask = m.desc();
                EventDesc::new(0x074, "TOPDOWN_BE_BOUND", mask)
            },
            Self::BrMisp(m) => {
                let mask = m.desc();
                EventDesc::new(0x0c5, "BR_MISP_RETIRED", mask)
            },
            Self::DecodeRestriction(x) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new(0xe9, "DECODE_RESTRICTION", mask)
            },

            Self::Unk(v, x, name) => {
                let mask = MaskDesc::new_unk(*x);
                EventDesc::new_unk(*v, mask)
            },
        }
    }
}
