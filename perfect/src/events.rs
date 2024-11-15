
use std::collections::*;

#[macro_use]
pub mod zen2;
pub mod tremont;

pub use zen2::*;
pub use tremont::*;

/// Implemented on some type representing an event for a particular platform.
pub trait AsEventDesc: Clone + Copy + PartialOrd + Ord + PartialEq + Eq {
    fn as_desc(&self) -> EventDesc;
    fn unk_desc(id: u16, mask: u8) -> Self;
}

/// Representation of an event mask. 
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

/// Representation of an event used for formatting output, and for passing 
/// the actual values to `perf` APIs. 
///
/// Users might want to implement their own types (ie. an enum) with events 
/// for a particular platform. The [AsEventDesc] trait represents an interface
/// between this crate and those platform-specific types.
///
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
    pub fn new_unk(id: u16, mask: MaskDesc) -> Self { 
        Self { 
            id,
            mask: mask.mask,
            name: format!("Event{:03x}:{:02x}", id, mask.mask),
        }
    }
    pub fn name(&self) -> &str { &self.name }
    pub fn fs_name(&self) -> String {
        self.name().replace(".", "_").to_lowercase()
    }

    pub fn id(&self) -> u16 { self.id }
    pub fn mask(&self) -> u8 { self.mask }
}


/// A set of [potentially platform-specific] events. 
#[derive(Clone)]
pub struct EventSet<E: AsEventDesc> {
    pub set: BTreeSet<E>,
}
impl <E: AsEventDesc> EventSet<E> {

    /// Create a new [empty] set of events.
    pub fn new() -> Self { 
        Self { 
            set: BTreeSet::new(),
        }
    }

    /// Create a new set from a slice of events.
    pub fn new_from_slice(events: &[E]) -> Self { 
        Self {
            set: BTreeSet::from_iter(events.iter().copied())
        }
    }

    /// Add an event to the set.
    pub fn add(&mut self, evt: E) {
        self.set.insert(evt);
    }

    /// Add each event in some slice to the set.
    pub fn add_list(&mut self, evt: &[E]) {
        for event in evt.iter() {
            self.set.insert(event.clone());
        }
    }

    /// Add all unit mask bits for a particular event to the set.
    pub fn add_unknown(&mut self, id: u16) {
        for mask in &[0x00,0x01,0x02,0x04,0x08,0x10,0x20,0x40,0x80] {
            self.set.insert(E::unk_desc(id, *mask));
        }
    }

    /// Clear this set.
    pub fn clear(&mut self) {
        self.set.clear();
    }

    /// Return an iterator over this set. 
    pub fn iter(&self) -> std::collections::btree_set::Iter<E> {
        self.set.iter()
    }
}


pub struct EventGroup<E: AsEventDesc + 'static> {
    pub set: &'static [E],
}
impl <E: AsEventDesc + 'static> EventGroup<E> {
    pub const fn new(set: &'static [E]) -> Self { 
        Self { set }
    }
}







