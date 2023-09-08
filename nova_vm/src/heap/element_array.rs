use super::object::{ObjectEntry, PropertyDescriptor, PropertyKey};
use crate::value::Value;
use core::panic;
use std::{collections::HashMap, num::NonZeroU16};

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

#[derive(Debug)]
pub(crate) struct ElementsVector {
    pub(crate) elements_index: u32,
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
                let [a, b, top, overflow_top] = get.to_le_bytes();
                assert!(overflow_top == 0, "Too many functions");
                let bottom = u16::from_le_bytes([a, b]);
                let bottom = NonZeroU16::new(bottom).unwrap();
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
                let [a, b, top, overflow_top] = set.to_le_bytes();
                assert!(overflow_top == 0, "Too many functions");
                let bottom = u16::from_le_bytes([a, b]);
                let bottom = NonZeroU16::new(bottom).unwrap();
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
                let [a, b, get_top, overflow_top] = get.to_le_bytes();
                assert!(overflow_top == 0, "Too many functions");
                let get_bottom = u16::from_le_bytes([a, b]);
                let get_bottom = NonZeroU16::new(get_bottom).unwrap();
                let [a, b, set_top, overflow_top] = set.to_le_bytes();
                assert!(overflow_top == 0, "Too many functions");
                let set_bottom = u16::from_le_bytes([a, b]);
                let set_bottom = NonZeroU16::new(set_bottom).unwrap();
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
}

/// Element arrays of up to 16 elements
#[derive(Debug)]
pub(crate) struct ElementArray2Pow4 {
    pub(crate) values: Vec<Option<[Option<Value>; usize::pow(2, 4)]>>,
    pub(crate) descriptors: HashMap<u32, HashMap<u32, ElementDescriptor>>,
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
    pub(crate) descriptors: HashMap<u32, HashMap<u32, ElementDescriptor>>,
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
    pub(crate) descriptors: HashMap<u32, HashMap<u32, ElementDescriptor>>,
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
    pub(crate) descriptors: HashMap<u32, HashMap<u32, ElementDescriptor>>,
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
    pub(crate) descriptors: HashMap<u32, HashMap<u32, ElementDescriptor>>,
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
    pub(crate) descriptors: HashMap<u32, HashMap<u32, ElementDescriptor>>,
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
    pub(crate) descriptors: HashMap<u32, HashMap<u32, ElementDescriptor>>,
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
    pub(crate) descriptors: HashMap<u32, HashMap<u32, ElementDescriptor>>,
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
    ) -> u32 {
        match key {
            ElementArrayKey::E4 => {
                let array: [Option<Value>; usize::pow(2, 4)] = [None; usize::pow(2, 4)];
                array.as_slice().clone_from(&vector.as_slice());
                self.e2pow4.values.push(Some(array));
                let length = self.e2pow4.values.len() as u32;
                if let Some(descriptors) = desciptors {
                    self.e2pow4.descriptors.insert(length, descriptors);
                }
                length
            }
            ElementArrayKey::E6 => {
                let array: [Option<Value>; usize::pow(2, 6)] = [None; usize::pow(2, 6)];
                array.as_slice().clone_from(&vector.as_slice());
                self.e2pow6.values.push(Some(array));
                let length = self.e2pow6.values.len() as u32;
                if let Some(descriptors) = desciptors {
                    self.e2pow6.descriptors.insert(length, descriptors);
                }
                length
            }
            ElementArrayKey::E8 => {
                let array: [Option<Value>; usize::pow(2, 8)] = [None; usize::pow(2, 8)];
                array.as_slice().clone_from(&vector.as_slice());
                self.e2pow8.values.push(Some(array));
                let length = self.e2pow8.values.len() as u32;
                if let Some(descriptors) = desciptors {
                    self.e2pow8.descriptors.insert(length, descriptors);
                }
                length
            }
            ElementArrayKey::E10 => {
                let array: [Option<Value>; usize::pow(2, 10)] = [None; usize::pow(2, 10)];
                array.as_slice().clone_from(&vector.as_slice());
                self.e2pow10.values.push(Some(array));
                let length = self.e2pow10.values.len() as u32;
                if let Some(descriptors) = desciptors {
                    self.e2pow10.descriptors.insert(length, descriptors);
                }
                length
            }
            ElementArrayKey::E12 => {
                let array: [Option<Value>; usize::pow(2, 12)] = [None; usize::pow(2, 12)];
                array.as_slice().clone_from(&vector.as_slice());
                self.e2pow12.values.push(Some(array));
                let length = self.e2pow12.values.len() as u32;
                if let Some(descriptors) = desciptors {
                    self.e2pow12.descriptors.insert(length, descriptors);
                }
                length
            }
            ElementArrayKey::E16 => {
                let array: [Option<Value>; usize::pow(2, 16)] = [None; usize::pow(2, 16)];
                array.as_slice().clone_from(&vector.as_slice());
                self.e2pow16.values.push(Some(array));
                let length = self.e2pow16.values.len() as u32;
                if let Some(descriptors) = desciptors {
                    self.e2pow16.descriptors.insert(length, descriptors);
                }
                length
            }
            ElementArrayKey::E24 => {
                let array: [Option<Value>; usize::pow(2, 24)] = [None; usize::pow(2, 24)];
                array.as_slice().clone_from(&vector.as_slice());
                self.e2pow24.values.push(Some(array));
                let length = self.e2pow24.values.len() as u32;
                if let Some(descriptors) = desciptors {
                    self.e2pow24.descriptors.insert(length, descriptors);
                }
                length
            }
            ElementArrayKey::E32 => {
                let array: [Option<Value>; usize::pow(2, 32)] = [None; usize::pow(2, 32)];
                array.as_slice().clone_from(&vector.as_slice());
                self.e2pow32.values.push(Some(array));
                let length = self.e2pow32.values.len() as u32;
                if let Some(descriptors) = desciptors {
                    self.e2pow32.descriptors.insert(length, descriptors);
                }
                length
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
                    descriptors = Default::default();
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
