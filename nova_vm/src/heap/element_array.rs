use super::{
    indexes::{ElementIndex, FunctionIndex},
    object::{ObjectEntry, PropertyDescriptor, PropertyKey},
};
use crate::value::Value;
use core::panic;
use std::{collections::HashMap, mem::MaybeUninit, num::NonZeroU16};

#[derive(Debug, Clone, Copy)]
pub(crate) enum ElementArrayKey {
    /// up to 16 elements
    E4,
    /// up to 64 elements
    E6,
    /// up to 256 elements
    E8,
    /// up to 1024 elements
    E10,
    /// up to 4096 elements
    E12,
    /// up to 65536 elements
    E16,
    /// up to 16777216 elements
    E24,
    /// up to 4294967296 elements
    E32,
}

impl From<usize> for ElementArrayKey {
    fn from(value: usize) -> Self {
        if value <= usize::pow(2, 4) {
            ElementArrayKey::E4
        } else if value <= usize::pow(2, 6) {
            ElementArrayKey::E6
        } else if value <= usize::pow(2, 8) {
            ElementArrayKey::E8
        } else if value <= usize::pow(2, 10) {
            ElementArrayKey::E10
        } else if value <= usize::pow(2, 12) {
            ElementArrayKey::E12
        } else if value <= usize::pow(2, 16) {
            ElementArrayKey::E16
        } else if value <= usize::pow(2, 24) {
            ElementArrayKey::E24
        } else if value <= usize::pow(2, 32) {
            ElementArrayKey::E32
        } else {
            panic!("Elements array length over 2 ** 32");
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ElementsVector {
    pub(crate) elements_index: ElementIndex,
    pub(crate) cap: ElementArrayKey,
    pub(crate) len: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum ElementDescriptor {
    /// ```js
    /// { value, writable: true, enumerable: true, configurable: true }
    /// ```
    WritableEnumerableConfigurableData,
    /// ```js
    /// { value, writable: true, enumerable: true, configurable: false }
    /// ```
    WritableEnumerableUnconfigurableData,
    /// ```js
    /// { value, writable: true, enumerable: false, configurable: true }
    /// ```
    WritableUnenumerableConfigurableData,
    /// ```js
    /// { value, writable: true, enumerable: false, configurable: false }
    /// ```
    WritableUnenumerableUnconfigurableData,
    /// ```js
    /// { value, writable: false, enumerable: true, configurable: true }
    /// ```
    ReadOnlyEnumerableConfigurableData,
    /// ```js
    /// { value, writable: false, enumerable: true, configurable: false }
    /// ```
    ReadOnlyEnumerableUnconfigurableData,
    /// ```js
    /// { value, writable: false, enumerable: false, configurable: true }
    /// ```
    ReadOnlyUnenumerableConfigurableData,
    /// ```js
    /// { value, writable: false, enumerable: false, configurable: false }
    /// ```
    ReadOnlyUnenumerableUnconfigurableData,
    // TODO: Is { enumerable, configurable } actually a real case or is that just in the spec?
    // If it is then a NoReadNoWrite*umerable*onfigurable set of descriptors is needed
    /// ```js
    /// { get, enumerable: true, configurable: true }
    /// ```
    ReadOnlyEnumerableConfigurableAccessor(u8, NonZeroU16),
    /// ```js
    /// { get, enumerable: true, configurable: false }
    /// ```
    ReadOnlyEnumerableUnconfigurableAccessor(u8, NonZeroU16),
    /// ```js
    /// { get, enumerable: false, configurable: true }
    /// ```
    ReadOnlyUnenumerableConfigurableAccessor(u8, NonZeroU16),
    /// ```js
    /// { get, enumerable: false, configurable: false }
    /// ```
    ReadOnlyUnenumerableUnconfigurableAccessor(u8, NonZeroU16),
    /// ```js
    /// { set, enumerable: true, configurable: true }
    /// ```
    WriteOnlyEnumerableConfigurableAccessor(u8, NonZeroU16),
    /// ```js
    /// { set, enumerable: true, configurable: false }
    /// ```
    WriteOnlyEnumerableUnconfigurableAccessor(u8, NonZeroU16),
    /// ```js
    /// { set, enumerable: false, configurable: true }
    /// ```
    WriteOnlyUnenumerableConfigurableAccessor(u8, NonZeroU16),
    /// ```js
    /// { set, enumerable: false, configurable: false }
    /// ```
    WriteOnlyUnenumerableUnconfigurableAccessor(u8, NonZeroU16),
    /// ```js
    /// { get, set, enumerable: true, configurable: true }
    /// ```
    ReadWriteEnumerableConfigurableAccessor(u8, u8, NonZeroU16, NonZeroU16),
    /// ```js
    /// { get, set, enumerable: true, configurable: false }
    /// ```
    ReadWriteEnumerableUnconfigurableAccessor(u8, u8, NonZeroU16, NonZeroU16),
    /// ```js
    /// { get, set, enumerable: false, configurable: true }
    /// ```
    ReadWriteUnenumerableConfigurableAccessor(u8, u8, NonZeroU16, NonZeroU16),
    /// ```js
    /// { get, set, enumerable: false, configurable: false }
    /// ```
    ReadWriteUnenumerableUnconfigurableAccessor(u8, u8, NonZeroU16, NonZeroU16),
}

impl ElementDescriptor {
    pub fn from_property_descriptor(
        desc: PropertyDescriptor,
    ) -> (Option<ElementDescriptor>, Option<Value>) {
        match desc {
            PropertyDescriptor::Data {
                value,
                writable,
                enumerable,
                configurable,
            } => match (writable, enumerable, configurable) {
                (true, true, true) => (None, Some(value)),
                (true, true, false) => (
                    Some(ElementDescriptor::WritableEnumerableUnconfigurableData),
                    Some(value),
                ),
                (true, false, true) => (
                    Some(ElementDescriptor::WritableUnenumerableConfigurableData),
                    Some(value),
                ),
                (true, false, false) => (
                    Some(ElementDescriptor::WritableUnenumerableUnconfigurableData),
                    Some(value),
                ),
                (false, true, true) => (
                    Some(ElementDescriptor::ReadOnlyEnumerableConfigurableData),
                    Some(value),
                ),
                (false, true, false) => (
                    Some(ElementDescriptor::ReadOnlyEnumerableUnconfigurableData),
                    Some(value),
                ),
                (false, false, true) => (
                    Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableData),
                    Some(value),
                ),
                (false, false, false) => (
                    Some(ElementDescriptor::ReadOnlyUnenumerableUnconfigurableData),
                    Some(value),
                ),
            },
            PropertyDescriptor::Blocked { .. } => unreachable!(),
            PropertyDescriptor::ReadOnly {
                get,
                enumerable,
                configurable,
            } => {
                let get = get.into_u32();
                let top = (get >> 16) as u8;
                let bottom = NonZeroU16::new(get as u16).unwrap();
                match (enumerable, configurable) {
                    (true, true) => (
                        Some(ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor(
                            top, bottom,
                        )),
                        None,
                    ),
                    (true, false) => (
                        Some(ElementDescriptor::ReadOnlyEnumerableUnconfigurableAccessor(
                            top, bottom,
                        )),
                        None,
                    ),
                    (false, true) => (
                        Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableAccessor(
                            top, bottom,
                        )),
                        None,
                    ),
                    (false, false) => (
                        Some(
                            ElementDescriptor::ReadOnlyUnenumerableUnconfigurableAccessor(
                                top, bottom,
                            ),
                        ),
                        None,
                    ),
                }
            }
            PropertyDescriptor::WriteOnly {
                set,
                enumerable,
                configurable,
            } => {
                let set = set.into_u32();
                let top = (set >> 16) as u8;
                let bottom = NonZeroU16::new(set as u16).unwrap();
                match (enumerable, configurable) {
                    (true, true) => (
                        Some(ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor(
                            top, bottom,
                        )),
                        None,
                    ),
                    (true, false) => (
                        Some(
                            ElementDescriptor::WriteOnlyEnumerableUnconfigurableAccessor(
                                top, bottom,
                            ),
                        ),
                        None,
                    ),
                    (false, true) => (
                        Some(
                            ElementDescriptor::WriteOnlyUnenumerableConfigurableAccessor(
                                top, bottom,
                            ),
                        ),
                        None,
                    ),
                    (false, false) => (
                        Some(
                            ElementDescriptor::WriteOnlyUnenumerableUnconfigurableAccessor(
                                top, bottom,
                            ),
                        ),
                        None,
                    ),
                }
            }
            PropertyDescriptor::ReadWrite {
                get,
                set,
                enumerable,
                configurable,
            } => {
                let get = get.into_u32();
                let get_top = (get >> 16) as u8;
                let get_bottom = NonZeroU16::new(get as u16).unwrap();
                let set = set.into_u32();
                let set_top = (set >> 16) as u8;
                let set_bottom = NonZeroU16::new(set as u16).unwrap();
                match (enumerable, configurable) {
                    (true, true) => (
                        Some(ElementDescriptor::ReadWriteEnumerableConfigurableAccessor(
                            get_top, set_top, get_bottom, set_bottom,
                        )),
                        None,
                    ),
                    (true, false) => (
                        Some(
                            ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor(
                                get_top, set_top, get_bottom, set_bottom,
                            ),
                        ),
                        None,
                    ),
                    (false, true) => (
                        Some(
                            ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor(
                                get_top, set_top, get_bottom, set_bottom,
                            ),
                        ),
                        None,
                    ),
                    (false, false) => (
                        Some(
                            ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor(
                                get_top, set_top, get_bottom, set_bottom,
                            ),
                        ),
                        None,
                    ),
                }
            }
        }
    }

    pub fn getter_index(&self) -> Option<FunctionIndex> {
        match self {
            ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor(get_top, get_bottom)
            | ElementDescriptor::ReadOnlyEnumerableUnconfigurableAccessor(get_top, get_bottom)
            | ElementDescriptor::ReadOnlyUnenumerableConfigurableAccessor(get_top, get_bottom)
            | ElementDescriptor::ReadOnlyUnenumerableUnconfigurableAccessor(get_top, get_bottom)
            | ElementDescriptor::ReadWriteEnumerableConfigurableAccessor(
                get_top,
                _,
                get_bottom,
                _,
            )
            | ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor(
                get_top,
                _,
                get_bottom,
                _,
            )
            | ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor(
                get_top,
                _,
                get_bottom,
                _,
            )
            | ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor(
                get_top,
                _,
                get_bottom,
                _,
            ) => Some(FunctionIndex::from_u32(
                (*get_top as u32) << 16 + get_bottom.get() as u32,
            )),
            _ => None,
        }
    }

    pub fn setter_index(&self) -> Option<FunctionIndex> {
        match self {
            ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor(set_top, set_bottom)
            | ElementDescriptor::WriteOnlyEnumerableUnconfigurableAccessor(set_top, set_bottom)
            | ElementDescriptor::WriteOnlyUnenumerableConfigurableAccessor(set_top, set_bottom)
            | ElementDescriptor::WriteOnlyUnenumerableUnconfigurableAccessor(set_top, set_bottom)
            | ElementDescriptor::ReadWriteEnumerableConfigurableAccessor(
                _,
                set_top,
                _,
                set_bottom,
            )
            | ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor(
                _,
                set_top,
                _,
                set_bottom,
            )
            | ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor(
                _,
                set_top,
                _,
                set_bottom,
            )
            | ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor(
                _,
                set_top,
                _,
                set_bottom,
            ) => Some(FunctionIndex::from_u32(
                (*set_top as u32) << 16 + set_bottom.get() as u32,
            )),
            _ => None,
        }
    }
}

/// Element arrays of up to 16 elements
#[derive(Debug)]
pub(crate) struct ElementArray2Pow4 {
    pub(crate) values: Vec<Option<[Option<Value>; usize::pow(2, 4)]>>,
    pub(crate) descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
}

impl Default for ElementArray2Pow4 {
    fn default() -> Self {
        Self {
            values: Default::default(),
            descriptors: Default::default(),
        }
    }
}

impl ElementArray2Pow4 {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            values: Vec::with_capacity(cap),
            descriptors: Default::default(),
        }
    }
}

/// Element arrays of up to 64 elements
#[derive(Debug)]
pub(crate) struct ElementArray2Pow6 {
    pub(crate) values: Vec<Option<[Option<Value>; usize::pow(2, 6)]>>,
    pub(crate) descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
}

impl Default for ElementArray2Pow6 {
    fn default() -> Self {
        Self {
            values: Default::default(),
            descriptors: Default::default(),
        }
    }
}

impl ElementArray2Pow6 {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            values: Vec::with_capacity(cap),
            descriptors: Default::default(),
        }
    }
}

/// Element arrays of up to 256 elements
#[derive(Debug)]
pub(crate) struct ElementArray2Pow8 {
    pub(crate) values: Vec<Option<[Option<Value>; usize::pow(2, 8)]>>,
    pub(crate) descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
}

impl Default for ElementArray2Pow8 {
    fn default() -> Self {
        Self {
            values: Default::default(),
            descriptors: Default::default(),
        }
    }
}

impl ElementArray2Pow8 {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            values: Vec::with_capacity(cap),
            descriptors: Default::default(),
        }
    }
}

/// Element arrays of up to 1024 elements
#[derive(Debug)]
pub(crate) struct ElementArray2Pow10 {
    pub(crate) values: Vec<Option<[Option<Value>; usize::pow(2, 10)]>>,
    pub(crate) descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
}

impl Default for ElementArray2Pow10 {
    fn default() -> Self {
        Self {
            values: Default::default(),
            descriptors: Default::default(),
        }
    }
}

impl ElementArray2Pow10 {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            values: Vec::with_capacity(cap),
            descriptors: Default::default(),
        }
    }
}

/// Element arrays of up to 4096 elements
#[derive(Debug)]
pub(crate) struct ElementArray2Pow12 {
    pub(crate) values: Vec<Option<[Option<Value>; usize::pow(2, 12)]>>,
    pub(crate) descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
}

impl Default for ElementArray2Pow12 {
    fn default() -> Self {
        Self {
            values: Default::default(),
            descriptors: Default::default(),
        }
    }
}

impl ElementArray2Pow12 {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            values: Vec::with_capacity(cap),
            descriptors: Default::default(),
        }
    }
}

/// Element arrays of up to 65536 elements
#[derive(Debug)]
pub(crate) struct ElementArray2Pow16 {
    pub(crate) values: Vec<Option<[Option<Value>; usize::pow(2, 16)]>>,
    pub(crate) descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
}

impl Default for ElementArray2Pow16 {
    fn default() -> Self {
        Self {
            values: Default::default(),
            descriptors: Default::default(),
        }
    }
}

impl ElementArray2Pow16 {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            values: Vec::with_capacity(cap),
            descriptors: Default::default(),
        }
    }
}

/// Element arrays of up to 16777216 elements
#[derive(Debug)]
pub(crate) struct ElementArray2Pow24 {
    pub(crate) values: Vec<Option<[Option<Value>; usize::pow(2, 24)]>>,
    pub(crate) descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
}

impl Default for ElementArray2Pow24 {
    fn default() -> Self {
        Self {
            values: Default::default(),
            descriptors: Default::default(),
        }
    }
}

impl ElementArray2Pow24 {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            values: Vec::with_capacity(cap),
            descriptors: Default::default(),
        }
    }
}

/// Element arrays of up to 4294967296 elements
#[derive(Debug)]
pub(crate) struct ElementArray2Pow32 {
    pub(crate) values: Vec<Option<[Option<Value>; usize::pow(2, 32)]>>,
    pub(crate) descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
}

impl Default for ElementArray2Pow32 {
    fn default() -> Self {
        Self {
            values: Default::default(),
            descriptors: Default::default(),
        }
    }
}

impl ElementArray2Pow32 {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            values: Vec::with_capacity(cap),
            descriptors: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ElementArrays {
    /// up to 16 elements
    pub(crate) e2pow4: ElementArray2Pow4,
    /// up to 64 elements
    pub(crate) e2pow6: ElementArray2Pow6,
    /// up to 256 elements
    pub(crate) e2pow8: ElementArray2Pow8,
    /// up to 1024 elements
    pub(crate) e2pow10: ElementArray2Pow10,
    /// up to 4096 elements
    pub(crate) e2pow12: ElementArray2Pow12,
    /// up to 65536 elements
    pub(crate) e2pow16: ElementArray2Pow16,
    /// up to 16777216 elements
    pub(crate) e2pow24: ElementArray2Pow24,
    /// up to 4294967296 elements
    pub(crate) e2pow32: ElementArray2Pow32,
}

impl ElementArrays {
    fn push_with_key(
        &mut self,
        key: ElementArrayKey,
        vector: Vec<Option<Value>>,
        desciptors: Option<HashMap<u32, ElementDescriptor>>,
    ) -> ElementIndex {
        match key {
            ElementArrayKey::E4 => {
                self.e2pow4.values.reserve(1);
                // SAFETY: We reserved an extra slot successfully.
                unsafe {
                    self.e2pow4.values.set_len(self.e2pow4.values.len() + 1);
                    let last = std::mem::transmute::<
                        _,
                        &mut MaybeUninit<[Option<Value>; usize::pow(2, 4)]>,
                    >(self.e2pow4.values.last_mut().unwrap());
                    let length = vector.len();
                    let last_slice = last.assume_init_mut().as_mut_slice();
                    last_slice[0..length].copy_from_slice(&vector.as_slice());
                    last_slice[vector.len()..].fill(None)
                };
                let index = ElementIndex::last(&self.e2pow4.values);
                if let Some(descriptors) = desciptors {
                    self.e2pow4.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E6 => {
                self.e2pow6.values.reserve(1);
                // SAFETY: We reserved an extra slot successfully.
                unsafe {
                    self.e2pow6.values.set_len(self.e2pow6.values.len() + 1);
                    let last = std::mem::transmute::<
                        _,
                        &mut MaybeUninit<[Option<Value>; usize::pow(2, 6)]>,
                    >(self.e2pow6.values.last_mut().unwrap());
                    let length = vector.len();
                    let last_slice = last.assume_init_mut().as_mut_slice();
                    last_slice[0..length].copy_from_slice(vector.as_slice());
                    last_slice[vector.len()..].fill(None)
                };
                let index = ElementIndex::last(&self.e2pow6.values);
                if let Some(descriptors) = desciptors {
                    self.e2pow6.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E8 => {
                self.e2pow8.values.reserve(1);
                // SAFETY: We reserved an extra slot successfully.
                unsafe {
                    self.e2pow8.values.set_len(self.e2pow8.values.len() + 1);
                    let last = std::mem::transmute::<
                        _,
                        &mut MaybeUninit<[Option<Value>; usize::pow(2, 8)]>,
                    >(self.e2pow8.values.last_mut().unwrap());
                    let length = vector.len();
                    let last_slice = last.assume_init_mut().as_mut_slice();
                    last_slice[0..length].copy_from_slice(vector.as_slice());
                    last_slice[vector.len()..].fill(None)
                };
                let index = ElementIndex::last(&self.e2pow8.values);
                if let Some(descriptors) = desciptors {
                    self.e2pow8.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E10 => {
                self.e2pow10.values.reserve(1);
                // SAFETY: We reserved an extra slot successfully.
                unsafe {
                    self.e2pow10.values.set_len(self.e2pow10.values.len() + 1);
                    let last = std::mem::transmute::<
                        _,
                        &mut MaybeUninit<[Option<Value>; usize::pow(2, 10)]>,
                    >(self.e2pow10.values.last_mut().unwrap());
                    let length = vector.len();
                    let last_slice = last.assume_init_mut().as_mut_slice();
                    last_slice[0..length].copy_from_slice(vector.as_slice());
                    last_slice[vector.len()..].fill(None)
                };
                let index = ElementIndex::last(&self.e2pow10.values);
                if let Some(descriptors) = desciptors {
                    self.e2pow10.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E12 => {
                self.e2pow12.values.reserve(1);
                // SAFETY: We reserved an extra slot successfully.
                unsafe {
                    self.e2pow12.values.set_len(self.e2pow12.values.len() + 1);
                    let last = std::mem::transmute::<
                        _,
                        &mut MaybeUninit<[Option<Value>; usize::pow(2, 12)]>,
                    >(self.e2pow12.values.last_mut().unwrap());
                    let length = vector.len();
                    let last_slice = last.assume_init_mut().as_mut_slice();
                    last_slice[0..length].copy_from_slice(vector.as_slice());
                    last_slice[vector.len()..].fill(None)
                };
                let index = ElementIndex::last(&self.e2pow12.values);
                if let Some(descriptors) = desciptors {
                    self.e2pow12.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E16 => {
                self.e2pow16.values.reserve(1);
                // SAFETY: We reserved an extra slot successfully.
                unsafe {
                    self.e2pow16.values.set_len(self.e2pow16.values.len() + 1);
                    let last = std::mem::transmute::<
                        _,
                        &mut MaybeUninit<[Option<Value>; usize::pow(2, 12)]>,
                    >(self.e2pow16.values.last_mut().unwrap());
                    let length = vector.len();
                    let last_slice = last.assume_init_mut().as_mut_slice();
                    last_slice[0..length].copy_from_slice(vector.as_slice());
                    last_slice[vector.len()..].fill(None)
                };
                let index = ElementIndex::last(&self.e2pow16.values);
                if let Some(descriptors) = desciptors {
                    self.e2pow16.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E24 => {
                self.e2pow24.values.reserve(1);
                // SAFETY: We reserved an extra slot successfully.
                unsafe {
                    self.e2pow24.values.set_len(self.e2pow24.values.len() + 1);
                    let last = std::mem::transmute::<
                        _,
                        &mut MaybeUninit<[Option<Value>; usize::pow(2, 12)]>,
                    >(self.e2pow24.values.last_mut().unwrap());
                    let length = vector.len();
                    let last_slice = last.assume_init_mut().as_mut_slice();
                    last_slice[0..length].copy_from_slice(vector.as_slice());
                    last_slice[vector.len()..].fill(None)
                };
                let index = ElementIndex::last(&self.e2pow24.values);
                if let Some(descriptors) = desciptors {
                    self.e2pow24.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E32 => {
                self.e2pow32.values.reserve(1);
                // SAFETY: We reserved an extra slot successfully.
                unsafe {
                    self.e2pow32.values.set_len(self.e2pow32.values.len() + 1);
                    let last = std::mem::transmute::<
                        _,
                        &mut MaybeUninit<[Option<Value>; usize::pow(2, 12)]>,
                    >(self.e2pow32.values.last_mut().unwrap());
                    let length = vector.len();
                    let last_slice = last.assume_init_mut().as_mut_slice();
                    last_slice[0..length].copy_from_slice(vector.as_slice());
                    last_slice[vector.len()..].fill(None)
                };
                let index = ElementIndex::last(&self.e2pow32.values);
                if let Some(descriptors) = desciptors {
                    self.e2pow32.descriptors.insert(index, descriptors);
                }
                index
            }
        }
    }

    pub(crate) fn create_object_entries(
        &mut self,
        mut entries: Vec<ObjectEntry>,
    ) -> (ElementsVector, ElementsVector) {
        let length = entries.len();
        let mut keys: Vec<Option<Value>> = Vec::with_capacity(length);
        let mut values: Vec<Option<Value>> = Vec::with_capacity(length);
        let mut descriptors: Option<HashMap<u32, ElementDescriptor>> = None;
        entries.drain(..).enumerate().for_each(|(index, entry)| {
            let ObjectEntry { key, value } = entry;
            let (maybe_descriptor, maybe_value) =
                ElementDescriptor::from_property_descriptor(value);
            let key = match key {
                PropertyKey::SmallAsciiString(data) => Value::StackString(data),
                PropertyKey::Smi(data) => Value::Smi(data),
                PropertyKey::String(data) => Value::HeapString(data),
                PropertyKey::Symbol(data) => Value::Symbol(data),
            };
            keys.push(Some(key));
            values.push(maybe_value);
            if let Some(descriptor) = maybe_descriptor {
                if descriptors.is_none() {
                    descriptors = Some(Default::default());
                }
                descriptors
                    .as_mut()
                    .unwrap()
                    .insert(index as u32, descriptor);
            }
        });
        let cap = ElementArrayKey::from(length);
        let len = length as u32;
        let key_elements_index = self.push_with_key(cap, keys, None);
        let value_elements_index = self.push_with_key(cap, values, descriptors);
        (
            ElementsVector {
                elements_index: key_elements_index,
                cap,
                len,
            },
            ElementsVector {
                elements_index: value_elements_index,
                cap,
                len,
            },
        )
    }
}
