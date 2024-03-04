
#[macro_use]
pub mod zen2;
pub use zen2::*;

use std::collections::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unit {
    None,
    Cycle,
}

pub struct MaskDesc {
    mask: u8,
    name: &'static str,
}
impl MaskDesc {
    pub const fn new(mask: u8, name: &'static str) -> Self {
        Self { mask, name }
    }
    pub const fn new_unk(mask: u8) -> Self { 
        Self { mask, name: "Unk" }
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq)]
pub struct EventDesc { 
    id: u16,
    mask: u8,
    name: String
}
impl EventDesc { 
    pub fn new(id: u16, name: &str, mask: MaskDesc) -> Self { 
        Self { 
            id, 
            mask: mask.mask,
            name: format!("{}.{}", name, mask.name),
        }
    }
    pub fn name(&self) -> &str { &self.name }
    pub fn id(&self) -> u16 { self.id }
    pub fn mask(&self) -> u8 { self.mask }
}

pub struct EventSet {
    pub set: BTreeSet<EventDesc>,
}
impl EventSet {
    pub fn new() -> Self { 
        Self { 
            set: BTreeSet::new(),
        }
    }

    pub fn add(&mut self, evt: Zen2Event) {
        self.set.insert(evt.event());
    }

    pub fn add_manual(&mut self, id: u16, mask: u8, name: &'static str) {
        self.set.insert(EventDesc { id, mask, name: name.to_string() });
    }

    pub fn clear(&mut self) {
        self.set.clear();
    }

    pub fn iter(&self) -> std::collections::btree_set::Iter<EventDesc> {
        self.set.iter()
    }


}



