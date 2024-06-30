use super::{
    indexes::ElementIndex,
    object_entry::{ObjectEntry, ObjectEntryPropertyDescriptor},
    CompactionLists, HeapMarkAndSweep, WorkQueues,
};
use crate::ecmascript::{
    builtins::SealableElementsVector,
    execution::Agent,
    types::{Function, PropertyDescriptor, PropertyKey, Value},
};
use core::panic;
use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

#[derive(Default, Debug, Clone, Copy)]
pub enum ElementArrayKey {
    #[default]
    Empty,
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

impl ElementArrayKey {
    pub fn cap(self) -> u32 {
        match self {
            ElementArrayKey::Empty => 0,
            ElementArrayKey::E4 => 2u32.pow(4),
            ElementArrayKey::E6 => 2u32.pow(6),
            ElementArrayKey::E8 => 2u32.pow(8),
            ElementArrayKey::E10 => 2u32.pow(10),
            ElementArrayKey::E12 => 2u32.pow(12),
            ElementArrayKey::E16 => 2u32.pow(16),
            ElementArrayKey::E24 => 2u32.pow(24),
            ElementArrayKey::E32 => u32::MAX,
        }
    }
}

impl From<u32> for ElementArrayKey {
    fn from(value: u32) -> Self {
        if value == 0 {
            ElementArrayKey::Empty
        } else if value <= u32::pow(2, 4) {
            ElementArrayKey::E4
        } else if value <= u32::pow(2, 6) {
            ElementArrayKey::E6
        } else if value <= u32::pow(2, 8) {
            ElementArrayKey::E8
        } else if value <= u32::pow(2, 10) {
            ElementArrayKey::E10
        } else if value <= u32::pow(2, 12) {
            ElementArrayKey::E12
        } else if value <= u32::pow(2, 16) {
            ElementArrayKey::E16
        } else if value <= u32::pow(2, 24) {
            ElementArrayKey::E24
        } else {
            ElementArrayKey::E32
        }
    }
}

impl From<usize> for ElementArrayKey {
    fn from(value: usize) -> Self {
        if value == 0 {
            ElementArrayKey::Empty
        } else if value <= usize::pow(2, 4) {
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
        } else if value < usize::pow(2, 32) - 1 {
            ElementArrayKey::E32
        } else {
            panic!("Elements array length over 2 ** 32 - 1");
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ElementsVector {
    pub(crate) elements_index: ElementIndex,
    pub(crate) cap: ElementArrayKey,
    pub(crate) len: u32,
}

impl ElementsVector {
    pub fn cap(&self) -> u32 {
        self.cap.cap()
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == self.cap()
    }

    /// An elements vector is simple if it contains no accessor descriptors.
    pub(crate) fn is_simple(&self, agent: &Agent) -> bool {
        let backing_store = agent.heap.elements.get_full(*self);
        backing_store.0.map_or(true, |hashmap| {
            !hashmap
                .iter()
                .any(|desc| desc.1.has_getter() || desc.1.has_setter())
        })
    }

    /// An elements vector is trivial if it contains no descriptors.
    pub(crate) fn is_trivial(&self, agent: &Agent) -> bool {
        let backing_store = agent.heap.elements.get_full(*self);
        backing_store.0.is_none()
    }

    pub(crate) fn is_dense(&self, agent: &Agent) -> bool {
        let (descriptors, elements) = agent.heap.elements.get_full(*self);
        if let Some(descriptors) = descriptors {
            for (index, ele) in elements.iter().enumerate() {
                let index = index as u32;
                if ele.is_none() {
                    let ele_descriptor = descriptors.get(&index);
                    let Some(ele_descriptor) = ele_descriptor else {
                        // No value, no descriptor: That's a hole.
                        return false;
                    };
                    if !ele_descriptor.has_getter() {
                        // No value, no getter: That's effectively a hole.
                        return false;
                    }
                }
            }
        } else {
            for ele in elements {
                if ele.is_none() {
                    // No descriptors, no value: That's a hole.
                    return false;
                }
            }
        }
        true
    }

    pub fn reserve(&mut self, elements: &mut ElementArrays, new_len: u32) {
        if new_len <= self.cap() {
            // Enough capacity present already
            return;
        }

        let new_key = ElementArrayKey::from(new_len);
        let (values, descriptors) = {
            let elements_index = self.elements_index;
            let usize_index = elements_index.into_index();
            let len = self.len() as usize;
            match self.cap {
                ElementArrayKey::Empty => (vec![], None),
                ElementArrayKey::E4 => {
                    let descriptors = elements.e2pow4.descriptors.get(&elements_index).cloned();
                    let elements = elements
                        .e2pow4
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()[0..len]
                        .to_vec();
                    (elements, descriptors)
                }
                ElementArrayKey::E6 => {
                    let descriptors = elements.e2pow6.descriptors.get(&elements_index).cloned();
                    let elements = elements
                        .e2pow6
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()[0..len]
                        .to_vec();
                    (elements, descriptors)
                }
                ElementArrayKey::E8 => {
                    let descriptors = elements.e2pow8.descriptors.get(&elements_index).cloned();
                    let elements = elements
                        .e2pow8
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()[0..len]
                        .to_vec();
                    (elements, descriptors)
                }
                ElementArrayKey::E10 => {
                    let descriptors = elements.e2pow10.descriptors.get(&elements_index).cloned();
                    let elements = elements
                        .e2pow10
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()[0..len]
                        .to_vec();
                    (elements, descriptors)
                }
                ElementArrayKey::E12 => {
                    let descriptors = elements.e2pow12.descriptors.get(&elements_index).cloned();
                    let elements = elements
                        .e2pow12
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()[0..len]
                        .to_vec();
                    (elements, descriptors)
                }
                ElementArrayKey::E16 => {
                    let descriptors = elements.e2pow16.descriptors.get(&elements_index).cloned();
                    let elements = elements
                        .e2pow16
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()[0..len]
                        .to_vec();
                    (elements, descriptors)
                }
                ElementArrayKey::E24 => {
                    let descriptors = elements.e2pow24.descriptors.get(&elements_index).cloned();
                    let elements = elements
                        .e2pow24
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()[0..len]
                        .to_vec();
                    (elements, descriptors)
                }
                ElementArrayKey::E32 => unreachable!(),
            }
        };
        let new_index = elements.push_with_key(new_key, &values, descriptors);
        self.cap = new_key;
        self.elements_index = new_index;
    }

    pub fn push(
        &mut self,
        elements: &mut ElementArrays,
        value: Option<Value>,
        descriptor: Option<ElementDescriptor>,
    ) {
        if self.is_full() {
            self.reserve(elements, self.len() + 1);
        }
        let next_over_end = match self.cap {
            ElementArrayKey::Empty => unreachable!(),
            ElementArrayKey::E4 => {
                &mut elements.e2pow4.values[self.elements_index][self.len as usize]
            }
            ElementArrayKey::E6 => {
                &mut elements.e2pow6.values[self.elements_index][self.len as usize]
            }
            ElementArrayKey::E8 => {
                &mut elements.e2pow8.values[self.elements_index][self.len as usize]
            }
            ElementArrayKey::E10 => {
                &mut elements.e2pow10.values[self.elements_index][self.len as usize]
            }
            ElementArrayKey::E12 => {
                &mut elements.e2pow12.values[self.elements_index][self.len as usize]
            }
            ElementArrayKey::E16 => {
                &mut elements.e2pow16.values[self.elements_index][self.len as usize]
            }
            ElementArrayKey::E24 => {
                &mut elements.e2pow24.values[self.elements_index][self.len as usize]
            }
            ElementArrayKey::E32 => {
                &mut elements.e2pow32.values[self.elements_index][self.len as usize]
            }
        };
        *next_over_end = value;
        if let Some(descriptor) = descriptor {
            let descriptors_map = match self.cap {
                ElementArrayKey::Empty => unreachable!(),
                ElementArrayKey::E4 => &mut elements.e2pow4.descriptors,
                ElementArrayKey::E6 => &mut elements.e2pow6.descriptors,
                ElementArrayKey::E8 => &mut elements.e2pow8.descriptors,
                ElementArrayKey::E10 => &mut elements.e2pow10.descriptors,
                ElementArrayKey::E12 => &mut elements.e2pow12.descriptors,
                ElementArrayKey::E16 => &mut elements.e2pow16.descriptors,
                ElementArrayKey::E24 => &mut elements.e2pow24.descriptors,
                ElementArrayKey::E32 => &mut elements.e2pow32.descriptors,
            };
            descriptors_map
                .entry(self.elements_index)
                .or_default()
                .insert(self.len, descriptor);
        }
        self.len += 1;
    }

    pub fn remove(&mut self, elements: &mut ElementArrays, index: usize) {
        let len = usize::try_from(self.len()).unwrap();
        assert!(index < len);

        let (values, descriptors) = match self.cap {
            ElementArrayKey::Empty => unreachable!(),
            ElementArrayKey::E4 => (
                &mut elements.e2pow4.values[self.elements_index][..],
                elements.e2pow4.descriptors.get_mut(&self.elements_index),
            ),
            ElementArrayKey::E6 => (
                &mut elements.e2pow6.values[self.elements_index][..],
                elements.e2pow6.descriptors.get_mut(&self.elements_index),
            ),
            ElementArrayKey::E8 => (
                &mut elements.e2pow8.values[self.elements_index][..],
                elements.e2pow8.descriptors.get_mut(&self.elements_index),
            ),
            ElementArrayKey::E10 => (
                &mut elements.e2pow10.values[self.elements_index][..],
                elements.e2pow10.descriptors.get_mut(&self.elements_index),
            ),
            ElementArrayKey::E12 => (
                &mut elements.e2pow12.values[self.elements_index][..],
                elements.e2pow12.descriptors.get_mut(&self.elements_index),
            ),
            ElementArrayKey::E16 => (
                &mut elements.e2pow16.values[self.elements_index][..],
                elements.e2pow16.descriptors.get_mut(&self.elements_index),
            ),
            ElementArrayKey::E24 => (
                &mut elements.e2pow24.values[self.elements_index][..],
                elements.e2pow24.descriptors.get_mut(&self.elements_index),
            ),
            ElementArrayKey::E32 => (
                &mut elements.e2pow32.values[self.elements_index][..],
                elements.e2pow32.descriptors.get_mut(&self.elements_index),
            ),
        };

        values.copy_within((index + 1)..len, index);
        values[len - 1] = None;
        self.len -= 1;

        if let Some(descriptor_map) = descriptors {
            let mut new_map = HashMap::new();
            for (k, v) in descriptor_map.drain() {
                match usize::try_from(k).unwrap().cmp(&index) {
                    std::cmp::Ordering::Less => {
                        new_map.insert(k, v);
                    }
                    std::cmp::Ordering::Equal => {}
                    std::cmp::Ordering::Greater => {
                        new_map.insert(k - 1, v);
                    }
                }
            }
            *descriptor_map = new_map;
        }
    }
}

impl HeapMarkAndSweep for ElementsVector {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self.cap {
            ElementArrayKey::Empty => {}
            ElementArrayKey::E4 => queues.e_2_4.push((self.elements_index, self.len)),
            ElementArrayKey::E6 => queues.e_2_6.push((self.elements_index, self.len)),
            ElementArrayKey::E8 => queues.e_2_8.push((self.elements_index, self.len)),
            ElementArrayKey::E10 => queues.e_2_10.push((self.elements_index, self.len)),
            ElementArrayKey::E12 => queues.e_2_12.push((self.elements_index, self.len)),
            ElementArrayKey::E16 => queues.e_2_16.push((self.elements_index, self.len)),
            ElementArrayKey::E24 => queues.e_2_24.push((self.elements_index, self.len)),
            ElementArrayKey::E32 => queues.e_2_32.push((self.elements_index, self.len)),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.elements_index.into_u32();
        let shift = match self.cap {
            ElementArrayKey::Empty => {
                return;
            }
            ElementArrayKey::E4 => compactions.e_2_4.get_shift_for_index(self_index),
            ElementArrayKey::E6 => compactions.e_2_6.get_shift_for_index(self_index),
            ElementArrayKey::E8 => compactions.e_2_8.get_shift_for_index(self_index),
            ElementArrayKey::E10 => compactions.e_2_10.get_shift_for_index(self_index),
            ElementArrayKey::E12 => compactions.e_2_12.get_shift_for_index(self_index),
            ElementArrayKey::E16 => compactions.e_2_16.get_shift_for_index(self_index),
            ElementArrayKey::E24 => compactions.e_2_24.get_shift_for_index(self_index),
            ElementArrayKey::E32 => compactions.e_2_32.get_shift_for_index(self_index),
        };
        self.elements_index = ElementIndex::from_u32(self_index - shift);
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
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
    ReadOnlyEnumerableConfigurableAccessor { get: Function },
    /// ```js
    /// { get, enumerable: true, configurable: false }
    /// ```
    ReadOnlyEnumerableUnconfigurableAccessor { get: Function },
    /// ```js
    /// { get, enumerable: false, configurable: true }
    /// ```
    ReadOnlyUnenumerableConfigurableAccessor { get: Function },
    /// ```js
    /// { get, enumerable: false, configurable: false }
    /// ```
    ReadOnlyUnenumerableUnconfigurableAccessor { get: Function },
    /// ```js
    /// { set, enumerable: true, configurable: true }
    /// ```
    WriteOnlyEnumerableConfigurableAccessor { set: Function },
    /// ```js
    /// { set, enumerable: true, configurable: false }
    /// ```
    WriteOnlyEnumerableUnconfigurableAccessor { set: Function },
    /// ```js
    /// { set, enumerable: false, configurable: true }
    /// ```
    WriteOnlyUnenumerableConfigurableAccessor { set: Function },
    /// ```js
    /// { set, enumerable: false, configurable: false }
    /// ```
    WriteOnlyUnenumerableUnconfigurableAccessor { set: Function },
    /// ```js
    /// { get, set, enumerable: true, configurable: true }
    /// ```
    ReadWriteEnumerableConfigurableAccessor { get: Function, set: Function },
    /// ```js
    /// { get, set, enumerable: true, configurable: false }
    /// ```
    ReadWriteEnumerableUnconfigurableAccessor { get: Function, set: Function },
    /// ```js
    /// { get, set, enumerable: false, configurable: true }
    /// ```
    ReadWriteUnenumerableConfigurableAccessor { get: Function, set: Function },
    /// ```js
    /// { get, set, enumerable: false, configurable: false }
    /// ```
    ReadWriteUnenumerableUnconfigurableAccessor { get: Function, set: Function },
}

impl ElementDescriptor {
    pub(crate) fn has_getter(&self) -> bool {
        matches!(
            self,
            ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor { .. }
                | ElementDescriptor::ReadOnlyEnumerableUnconfigurableAccessor { .. }
                | ElementDescriptor::ReadOnlyUnenumerableConfigurableAccessor { .. }
                | ElementDescriptor::ReadOnlyUnenumerableUnconfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor { .. }
        )
    }

    pub(crate) fn has_setter(&self) -> bool {
        matches!(
            self,
            ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor { .. }
                | ElementDescriptor::WriteOnlyEnumerableUnconfigurableAccessor { .. }
                | ElementDescriptor::WriteOnlyUnenumerableConfigurableAccessor { .. }
                | ElementDescriptor::WriteOnlyUnenumerableUnconfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor { .. }
        )
    }

    pub(crate) const fn new_with_wec(w: bool, e: bool, c: bool) -> Option<Self> {
        match (w, e, c) {
            (true, true, true) => None,
            (true, true, false) => Some(Self::WritableEnumerableUnconfigurableData),
            (true, false, true) => Some(Self::WritableUnenumerableConfigurableData),
            (true, false, false) => Some(Self::WritableUnenumerableUnconfigurableData),
            (false, true, true) => Some(Self::ReadOnlyEnumerableConfigurableData),
            (false, true, false) => Some(Self::ReadOnlyEnumerableUnconfigurableData),
            (false, false, true) => Some(Self::ReadOnlyUnenumerableConfigurableData),
            (false, false, false) => Some(Self::ReadOnlyUnenumerableUnconfigurableData),
        }
    }

    pub(crate) const fn new_with_get_ec(get: Function, e: bool, c: bool) -> Self {
        match (e, c) {
            (true, true) => Self::ReadOnlyEnumerableConfigurableAccessor { get },
            (true, false) => Self::ReadOnlyEnumerableUnconfigurableAccessor { get },
            (false, true) => Self::ReadOnlyUnenumerableConfigurableAccessor { get },
            (false, false) => Self::ReadOnlyUnenumerableUnconfigurableAccessor { get },
        }
    }

    pub(crate) const fn new_with_set_ec(set: Function, e: bool, c: bool) -> Self {
        match (e, c) {
            (true, true) => Self::WriteOnlyEnumerableConfigurableAccessor { set },
            (true, false) => Self::WriteOnlyEnumerableUnconfigurableAccessor { set },
            (false, true) => Self::WriteOnlyUnenumerableConfigurableAccessor { set },
            (false, false) => Self::WriteOnlyUnenumerableUnconfigurableAccessor { set },
        }
    }

    pub(crate) const fn new_with_get_set_ec(
        get: Function,
        set: Function,
        e: bool,
        c: bool,
    ) -> Self {
        match (e, c) {
            (true, true) => Self::ReadWriteEnumerableConfigurableAccessor { get, set },
            (true, false) => Self::ReadWriteEnumerableUnconfigurableAccessor { get, set },
            (false, true) => Self::ReadWriteUnenumerableConfigurableAccessor { get, set },
            (false, false) => Self::ReadWriteUnenumerableUnconfigurableAccessor { get, set },
        }
    }

    pub(crate) fn from_property_descriptor(
        desc: &ObjectEntryPropertyDescriptor,
    ) -> (Option<ElementDescriptor>, Option<Value>) {
        match desc {
            ObjectEntryPropertyDescriptor::Data {
                value,
                writable,
                enumerable,
                configurable,
            } => match (writable, enumerable, configurable) {
                (true, true, true) => (None, Some(*value)),
                (true, true, false) => (
                    Some(ElementDescriptor::WritableEnumerableUnconfigurableData),
                    Some(*value),
                ),
                (true, false, true) => (
                    Some(ElementDescriptor::WritableUnenumerableConfigurableData),
                    Some(*value),
                ),
                (true, false, false) => (
                    Some(ElementDescriptor::WritableUnenumerableUnconfigurableData),
                    Some(*value),
                ),
                (false, true, true) => (
                    Some(ElementDescriptor::ReadOnlyEnumerableConfigurableData),
                    Some(*value),
                ),
                (false, true, false) => (
                    Some(ElementDescriptor::ReadOnlyEnumerableUnconfigurableData),
                    Some(*value),
                ),
                (false, false, true) => (
                    Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableData),
                    Some(*value),
                ),
                (false, false, false) => (
                    Some(ElementDescriptor::ReadOnlyUnenumerableUnconfigurableData),
                    Some(*value),
                ),
            },
            ObjectEntryPropertyDescriptor::Blocked { .. } => unreachable!(),
            ObjectEntryPropertyDescriptor::ReadOnly {
                get,
                enumerable,
                configurable,
            } => match (enumerable, configurable) {
                (true, true) => (
                    Some(ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor { get: *get }),
                    None,
                ),
                (true, false) => (
                    Some(ElementDescriptor::ReadOnlyEnumerableUnconfigurableAccessor { get: *get }),
                    None,
                ),
                (false, true) => (
                    Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableAccessor { get: *get }),
                    None,
                ),
                (false, false) => (
                    Some(
                        ElementDescriptor::ReadOnlyUnenumerableUnconfigurableAccessor { get: *get },
                    ),
                    None,
                ),
            },
            ObjectEntryPropertyDescriptor::WriteOnly {
                set,
                enumerable,
                configurable,
            } => match (enumerable, configurable) {
                (true, true) => (
                    Some(ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor { set: *set }),
                    None,
                ),
                (true, false) => (
                    Some(
                        ElementDescriptor::WriteOnlyEnumerableUnconfigurableAccessor { set: *set },
                    ),
                    None,
                ),
                (false, true) => (
                    Some(
                        ElementDescriptor::WriteOnlyUnenumerableConfigurableAccessor { set: *set },
                    ),
                    None,
                ),
                (false, false) => (
                    Some(
                        ElementDescriptor::WriteOnlyUnenumerableUnconfigurableAccessor {
                            set: *set,
                        },
                    ),
                    None,
                ),
            },
            ObjectEntryPropertyDescriptor::ReadWrite {
                get,
                set,
                enumerable,
                configurable,
            } => match (enumerable, configurable) {
                (true, true) => (
                    Some(ElementDescriptor::ReadWriteEnumerableConfigurableAccessor {
                        get: *get,
                        set: *set,
                    }),
                    None,
                ),
                (true, false) => (
                    Some(
                        ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor {
                            get: *get,
                            set: *set,
                        },
                    ),
                    None,
                ),
                (false, true) => (
                    Some(
                        ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor {
                            get: *get,
                            set: *set,
                        },
                    ),
                    None,
                ),
                (false, false) => (
                    Some(
                        ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor {
                            get: *get,
                            set: *set,
                        },
                    ),
                    None,
                ),
            },
        }
    }

    pub fn to_property_descriptor(
        descriptor: Option<Self>,
        value: Option<Value>,
    ) -> PropertyDescriptor {
        let descriptor =
            descriptor.unwrap_or(ElementDescriptor::WritableEnumerableConfigurableData);
        match descriptor {
            ElementDescriptor::WritableEnumerableConfigurableData => PropertyDescriptor {
                value,
                configurable: Some(true),
                enumerable: Some(true),
                get: None,
                set: None,
                writable: Some(true),
            },
            ElementDescriptor::WritableEnumerableUnconfigurableData => PropertyDescriptor {
                value,
                configurable: Some(false),
                enumerable: Some(true),
                get: None,
                set: None,
                writable: Some(true),
            },
            ElementDescriptor::WritableUnenumerableConfigurableData => PropertyDescriptor {
                value,
                configurable: Some(true),
                enumerable: Some(false),
                get: None,
                set: None,
                writable: Some(true),
            },
            ElementDescriptor::WritableUnenumerableUnconfigurableData => PropertyDescriptor {
                value,
                configurable: Some(false),
                enumerable: Some(false),
                get: None,
                set: None,
                writable: Some(true),
            },
            ElementDescriptor::ReadOnlyEnumerableConfigurableData => PropertyDescriptor {
                value,
                configurable: Some(true),
                enumerable: Some(true),
                get: None,
                set: None,
                writable: Some(false),
            },
            ElementDescriptor::ReadOnlyEnumerableUnconfigurableData => PropertyDescriptor {
                value,
                configurable: Some(false),
                enumerable: Some(true),
                get: None,
                set: None,
                writable: Some(false),
            },
            ElementDescriptor::ReadOnlyUnenumerableConfigurableData => PropertyDescriptor {
                value,
                configurable: Some(true),
                enumerable: Some(false),
                get: None,
                set: None,
                writable: Some(false),
            },
            ElementDescriptor::ReadOnlyUnenumerableUnconfigurableData => PropertyDescriptor {
                value,
                configurable: Some(false),
                enumerable: Some(false),
                get: None,
                set: None,
                writable: Some(false),
            },
            ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor { get } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(true),
                    enumerable: Some(true),
                    get: Some(get),
                    set: None,
                    writable: None,
                }
            }
            ElementDescriptor::ReadOnlyEnumerableUnconfigurableAccessor { get } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(false),
                    enumerable: Some(true),
                    get: Some(get),
                    set: None,
                    writable: None,
                }
            }
            ElementDescriptor::ReadOnlyUnenumerableConfigurableAccessor { get } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(true),
                    enumerable: Some(false),
                    get: Some(get),
                    set: None,
                    writable: None,
                }
            }
            ElementDescriptor::ReadOnlyUnenumerableUnconfigurableAccessor { get } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(false),
                    enumerable: Some(false),
                    get: Some(get),
                    set: None,
                    writable: None,
                }
            }
            ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor { set } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(true),
                    enumerable: Some(true),
                    get: None,
                    set: Some(set),
                    writable: None,
                }
            }
            ElementDescriptor::WriteOnlyEnumerableUnconfigurableAccessor { set } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(false),
                    enumerable: Some(true),
                    get: None,
                    set: Some(set),
                    writable: None,
                }
            }
            ElementDescriptor::WriteOnlyUnenumerableConfigurableAccessor { set } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(true),
                    enumerable: Some(false),
                    get: None,
                    set: Some(set),
                    writable: None,
                }
            }
            ElementDescriptor::WriteOnlyUnenumerableUnconfigurableAccessor { set } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(false),
                    enumerable: Some(false),
                    get: None,
                    set: Some(set),
                    writable: None,
                }
            }
            ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { get, set } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(true),
                    enumerable: Some(true),
                    get: Some(get),
                    set: Some(set),
                    writable: None,
                }
            }
            ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor { get, set } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(false),
                    enumerable: Some(true),
                    get: Some(get),
                    set: Some(set),
                    writable: None,
                }
            }
            ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor { get, set } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(true),
                    enumerable: Some(false),
                    get: Some(get),
                    set: Some(set),
                    writable: None,
                }
            }
            ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor { get, set } => {
                PropertyDescriptor {
                    value: None,
                    configurable: Some(false),
                    enumerable: Some(false),
                    get: Some(get),
                    set: Some(set),
                    writable: None,
                }
            }
        }
    }

    pub fn getter_index(&self) -> Option<Function> {
        match self {
            ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyEnumerableUnconfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyUnenumerableConfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyUnenumerableUnconfigurableAccessor { get }
            | ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { get, .. }
            | ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor { get, .. }
            | ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor { get, .. }
            | ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor { get, .. } => {
                Some(*get)
            }
            _ => None,
        }
    }

    pub fn setter_index(&self) -> Option<Function> {
        match self {
            ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyEnumerableUnconfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyUnenumerableConfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyUnenumerableUnconfigurableAccessor { set }
            | ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { set, .. }
            | ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor { set, .. }
            | ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor { set, .. }
            | ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor { set, .. } => {
                Some(*set)
            }
            _ => None,
        }
    }
}

/// Element arrays of up to 16 elements
#[derive(Debug, Default)]
pub struct ElementArray2Pow4 {
    pub values: Vec<Option<[Option<Value>; usize::pow(2, 4)]>>,
    pub descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
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
#[derive(Debug, Default)]
pub struct ElementArray2Pow6 {
    pub values: Vec<Option<[Option<Value>; usize::pow(2, 6)]>>,
    pub descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
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
#[derive(Debug, Default)]
pub struct ElementArray2Pow8 {
    pub values: Vec<Option<[Option<Value>; usize::pow(2, 8)]>>,
    pub descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
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
#[derive(Debug, Default)]
pub struct ElementArray2Pow10 {
    pub values: Vec<Option<[Option<Value>; usize::pow(2, 10)]>>,
    pub descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
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
#[derive(Debug, Default)]
pub struct ElementArray2Pow12 {
    pub values: Vec<Option<[Option<Value>; usize::pow(2, 12)]>>,
    pub descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
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
#[derive(Debug, Default)]
pub struct ElementArray2Pow16 {
    pub values: Vec<Option<[Option<Value>; usize::pow(2, 16)]>>,
    pub descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
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
#[derive(Debug, Default)]
pub struct ElementArray2Pow24 {
    pub values: Vec<Option<[Option<Value>; usize::pow(2, 24)]>>,
    pub descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
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
#[derive(Debug, Default)]
pub struct ElementArray2Pow32 {
    pub values: Vec<Option<[Option<Value>; usize::pow(2, 32)]>>,
    pub descriptors: HashMap<ElementIndex, HashMap<u32, ElementDescriptor>>,
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
pub struct ElementArrays {
    /// up to 16 elements
    pub e2pow4: ElementArray2Pow4,
    /// up to 64 elements
    pub e2pow6: ElementArray2Pow6,
    /// up to 256 elements
    pub e2pow8: ElementArray2Pow8,
    /// up to 1024 elements
    pub e2pow10: ElementArray2Pow10,
    /// up to 4096 elements
    pub e2pow12: ElementArray2Pow12,
    /// up to 65536 elements
    pub e2pow16: ElementArray2Pow16,
    /// up to 16777216 elements
    pub e2pow24: ElementArray2Pow24,
    /// up to 4294967296 elements
    pub e2pow32: ElementArray2Pow32,
}

impl Index<ElementsVector> for ElementArrays {
    type Output = [Option<Value>];

    fn index(&self, index: ElementsVector) -> &Self::Output {
        self.get(index)
    }
}

impl IndexMut<ElementsVector> for ElementArrays {
    fn index_mut(&mut self, index: ElementsVector) -> &mut Self::Output {
        self.get_mut(index)
    }
}

impl Index<ElementsVector> for Agent {
    type Output = [Option<Value>];

    fn index(&self, index: ElementsVector) -> &Self::Output {
        &self.heap.elements[index]
    }
}

impl IndexMut<ElementsVector> for Agent {
    fn index_mut(&mut self, index: ElementsVector) -> &mut Self::Output {
        &mut self.heap.elements[index]
    }
}

impl Index<SealableElementsVector> for ElementArrays {
    type Output = [Option<Value>];

    fn index(&self, index: SealableElementsVector) -> &Self::Output {
        self.get(index.into())
    }
}

impl IndexMut<SealableElementsVector> for ElementArrays {
    fn index_mut(&mut self, index: SealableElementsVector) -> &mut Self::Output {
        self.get_mut(index.into())
    }
}

impl Index<SealableElementsVector> for Agent {
    type Output = [Option<Value>];

    fn index(&self, index: SealableElementsVector) -> &Self::Output {
        &self.heap.elements[index]
    }
}

impl IndexMut<SealableElementsVector> for Agent {
    fn index_mut(&mut self, index: SealableElementsVector) -> &mut Self::Output {
        &mut self.heap.elements[index]
    }
}

impl ElementArrays {
    fn push_with_key(
        &mut self,
        key: ElementArrayKey,
        vector: &[Option<Value>],
        descriptors: Option<HashMap<u32, ElementDescriptor>>,
    ) -> ElementIndex {
        debug_assert_eq!(
            std::mem::size_of::<Option<[Option<Value>; 1]>>(),
            std::mem::size_of::<[Option<Value>; 1]>()
        );
        match key {
            ElementArrayKey::Empty => ElementIndex::from_u32_index(0),
            ElementArrayKey::E4 => {
                let elements = &mut self.e2pow4;
                const N: usize = usize::pow(2, 4);
                elements.values.reserve(1);
                let remaining = elements.values.spare_capacity_mut();
                let length = vector.len();
                assert!(length <= N);
                let last = remaining.get_mut(0).unwrap();
                // SAFETY: The last elements array of the spare capacity is valid for writes up to N and
                // length is smaller or equal to that. The vector is valid for reads up to length.
                // Both are property aligned and do not alias.
                unsafe {
                    debug_assert_eq!(
                        std::mem::size_of::<Option<[Option<Value>; N]>>(),
                        std::mem::size_of::<[Option<Value>; N]>()
                    );
                    let element_ptr: *mut Option<Value> = std::mem::transmute(last.as_mut_ptr());
                    std::ptr::copy_nonoverlapping(vector.as_ptr(), element_ptr, length);
                    for index in length..N {
                        element_ptr.add(index).write(None);
                    }
                    elements.values.set_len(elements.values.len() + 1);
                }
                let index = ElementIndex::last_element_index(&elements.values);
                if let Some(descriptors) = descriptors {
                    elements.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E6 => {
                let elements = &mut self.e2pow6;
                const N: usize = usize::pow(2, 6);
                elements.values.reserve(1);
                let remaining = elements.values.spare_capacity_mut();
                let length = vector.len();
                assert!(length <= N);
                let last = remaining.get_mut(0).unwrap();
                // SAFETY: The last elements array of the spare capacity is valid for writes up to N and
                // length is smaller or equal to that. The vector is valid for reads up to length.
                // Both are property aligned and do not alias.
                unsafe {
                    debug_assert_eq!(
                        std::mem::size_of::<Option<[Option<Value>; N]>>(),
                        std::mem::size_of::<[Option<Value>; N]>()
                    );
                    let element_ptr: *mut Option<Value> = std::mem::transmute(last.as_mut_ptr());
                    std::ptr::copy_nonoverlapping(vector.as_ptr(), element_ptr, length);
                    for index in length..N {
                        element_ptr.add(index).write(None);
                    }
                    elements.values.set_len(elements.values.len() + 1);
                }
                let index = ElementIndex::last_element_index(&elements.values);
                if let Some(descriptors) = descriptors {
                    elements.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E8 => {
                let elements = &mut self.e2pow8;
                const N: usize = usize::pow(2, 8);
                elements.values.reserve(1);
                let remaining = elements.values.spare_capacity_mut();
                let length = vector.len();
                assert!(length <= N);
                let last = remaining.get_mut(0).unwrap();
                // SAFETY: The last elements array of the spare capacity is valid for writes up to N and
                // length is smaller or equal to that. The vector is valid for reads up to length.
                // Both are property aligned and do not alias.
                unsafe {
                    debug_assert_eq!(
                        std::mem::size_of::<Option<[Option<Value>; N]>>(),
                        std::mem::size_of::<[Option<Value>; N]>()
                    );
                    let element_ptr: *mut Option<Value> = std::mem::transmute(last.as_mut_ptr());
                    std::ptr::copy_nonoverlapping(vector.as_ptr(), element_ptr, length);
                    for index in length..N {
                        element_ptr.add(index).write(None);
                    }
                    elements.values.set_len(elements.values.len() + 1);
                }
                let index = ElementIndex::last_element_index(&elements.values);
                if let Some(descriptors) = descriptors {
                    elements.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E10 => {
                let elements = &mut self.e2pow10;
                const N: usize = usize::pow(2, 10);
                elements.values.reserve(1);
                let remaining = elements.values.spare_capacity_mut();
                let length = vector.len();
                assert!(length <= N);
                let last = remaining.get_mut(0).unwrap();
                // SAFETY: The last elements array of the spare capacity is valid for writes up to N and
                // length is smaller or equal to that. The vector is valid for reads up to length.
                // Both are property aligned and do not alias.
                unsafe {
                    debug_assert_eq!(
                        std::mem::size_of::<Option<[Option<Value>; N]>>(),
                        std::mem::size_of::<[Option<Value>; N]>()
                    );
                    let element_ptr: *mut Option<Value> = std::mem::transmute(last.as_mut_ptr());
                    std::ptr::copy_nonoverlapping(vector.as_ptr(), element_ptr, length);
                    for index in length..N {
                        element_ptr.add(index).write(None);
                    }
                    elements.values.set_len(elements.values.len() + 1);
                }
                let index = ElementIndex::last_element_index(&elements.values);
                if let Some(descriptors) = descriptors {
                    elements.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E12 => {
                let elements = &mut self.e2pow12;
                const N: usize = usize::pow(2, 12);
                elements.values.reserve(1);
                let remaining = elements.values.spare_capacity_mut();
                let length = vector.len();
                assert!(length <= N);
                let last = remaining.get_mut(0).unwrap();
                // SAFETY: The last elements array of the spare capacity is valid for writes up to N and
                // length is smaller or equal to that. The vector is valid for reads up to length.
                // Both are property aligned and do not alias.
                unsafe {
                    debug_assert_eq!(
                        std::mem::size_of::<Option<[Option<Value>; N]>>(),
                        std::mem::size_of::<[Option<Value>; N]>()
                    );
                    let element_ptr: *mut Option<Value> = std::mem::transmute(last.as_mut_ptr());
                    std::ptr::copy_nonoverlapping(vector.as_ptr(), element_ptr, length);
                    for index in length..N {
                        element_ptr.add(index).write(None);
                    }
                    elements.values.set_len(elements.values.len() + 1);
                }
                let index = ElementIndex::last_element_index(&elements.values);
                if let Some(descriptors) = descriptors {
                    elements.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E16 => {
                let elements = &mut self.e2pow16;
                const N: usize = usize::pow(2, 16);
                elements.values.reserve(1);
                let remaining = elements.values.spare_capacity_mut();
                let length = vector.len();
                assert!(length <= N);
                let last = remaining.get_mut(0).unwrap();
                // SAFETY: The last elements array of the spare capacity is valid for writes up to N and
                // length is smaller or equal to that. The vector is valid for reads up to length.
                // Both are property aligned and do not alias.
                unsafe {
                    debug_assert_eq!(
                        std::mem::size_of::<Option<[Option<Value>; N]>>(),
                        std::mem::size_of::<[Option<Value>; N]>()
                    );
                    let element_ptr: *mut Option<Value> = std::mem::transmute(last.as_mut_ptr());
                    std::ptr::copy_nonoverlapping(vector.as_ptr(), element_ptr, length);
                    for index in length..N {
                        element_ptr.add(index).write(None);
                    }
                    elements.values.set_len(elements.values.len() + 1);
                }
                let index = ElementIndex::last_element_index(&elements.values);
                if let Some(descriptors) = descriptors {
                    elements.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E24 => {
                let elements = &mut self.e2pow24;
                const N: usize = usize::pow(2, 24);
                elements.values.reserve(1);
                let remaining = elements.values.spare_capacity_mut();
                let length = vector.len();
                assert!(length <= N);
                let last = remaining.get_mut(0).unwrap();
                // SAFETY: The last elements array of the spare capacity is valid for writes up to N and
                // length is smaller or equal to that. The vector is valid for reads up to length.
                // Both are property aligned and do not alias.
                unsafe {
                    debug_assert_eq!(
                        std::mem::size_of::<Option<[Option<Value>; N]>>(),
                        std::mem::size_of::<[Option<Value>; N]>()
                    );
                    let element_ptr: *mut Option<Value> = std::mem::transmute(last.as_mut_ptr());
                    std::ptr::copy_nonoverlapping(vector.as_ptr(), element_ptr, length);
                    for index in length..N {
                        element_ptr.add(index).write(None);
                    }
                    elements.values.set_len(elements.values.len() + 1);
                }
                let index = ElementIndex::last_element_index(&elements.values);
                if let Some(descriptors) = descriptors {
                    elements.descriptors.insert(index, descriptors);
                }
                index
            }
            ElementArrayKey::E32 => {
                let elements = &mut self.e2pow32;
                const N: usize = usize::pow(2, 32);
                elements.values.reserve(1);
                let remaining = elements.values.spare_capacity_mut();
                let length = vector.len();
                assert!(length <= N);
                let last = remaining.get_mut(0).unwrap();
                // SAFETY: The last elements array of the spare capacity is valid for writes up to N and
                // length is smaller or equal to that. The vector is valid for reads up to length.
                // Both are property aligned and do not alias.
                unsafe {
                    debug_assert_eq!(
                        std::mem::size_of::<Option<[Option<Value>; N]>>(),
                        std::mem::size_of::<[Option<Value>; N]>()
                    );
                    let element_ptr: *mut Option<Value> = std::mem::transmute(last.as_mut_ptr());
                    std::ptr::copy_nonoverlapping(vector.as_ptr(), element_ptr, length);
                    for index in length..N {
                        element_ptr.add(index).write(None);
                    }
                    elements.values.set_len(elements.values.len() + 1);
                }
                let index = ElementIndex::last_element_index(&elements.values);
                if let Some(descriptors) = descriptors {
                    elements.descriptors.insert(index, descriptors);
                }
                index
            }
        }
    }

    pub fn allocate_elements_with_capacity(&mut self, capacity: usize) -> ElementsVector {
        let cap = ElementArrayKey::from(capacity);
        ElementsVector {
            elements_index: self.push_with_key(cap, &[], None),
            cap,
            len: 0,
        }
    }

    pub(crate) fn create_with_stuff(
        &mut self,
        mut entries: Vec<(PropertyKey, Option<ElementDescriptor>, Option<Value>)>,
    ) -> (ElementsVector, ElementsVector) {
        let length = entries.len();
        let mut keys: Vec<Option<Value>> = Vec::with_capacity(length);
        let mut values: Vec<Option<Value>> = Vec::with_capacity(length);
        let mut descriptors: Option<HashMap<u32, ElementDescriptor>> = None;
        entries.drain(..).enumerate().for_each(|(index, entry)| {
            let (key, maybe_descriptor, maybe_value) = entry;
            let key = match key {
                PropertyKey::Integer(data) => Value::Integer(data),
                PropertyKey::SmallString(data) => Value::SmallString(data),
                PropertyKey::String(data) => Value::String(data),
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
        let key_elements_index = self.push_with_key(cap, &keys, None);
        let value_elements_index = self.push_with_key(cap, &values, descriptors);
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

    pub(crate) fn create_object_entries(
        &mut self,
        entries: &[ObjectEntry],
    ) -> (ElementsVector, ElementsVector) {
        let length = entries.len();
        let mut keys: Vec<Option<Value>> = Vec::with_capacity(length);
        let mut values: Vec<Option<Value>> = Vec::with_capacity(length);
        let mut descriptors: Option<HashMap<u32, ElementDescriptor>> = None;
        entries.iter().enumerate().for_each(|(index, entry)| {
            let ObjectEntry { key, value } = entry;
            let (maybe_descriptor, maybe_value) =
                ElementDescriptor::from_property_descriptor(value);
            let key = match key {
                PropertyKey::Integer(data) => Value::Integer(*data),
                PropertyKey::SmallString(data) => Value::SmallString(*data),
                PropertyKey::String(data) => Value::String(*data),
                PropertyKey::Symbol(data) => Value::Symbol(*data),
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
        let key_elements_index = self.push_with_key(cap, &keys, None);
        let value_elements_index = self.push_with_key(cap, &values, descriptors);
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

    pub fn get(&self, vector: ElementsVector) -> &[Option<Value>] {
        match vector.cap {
            ElementArrayKey::Empty => &[],
            ElementArrayKey::E4 => {
                &self.e2pow4.values[vector.elements_index].as_slice()[0..vector.len as usize]
            }
            ElementArrayKey::E6 => {
                &self.e2pow6.values[vector.elements_index].as_slice()[0..vector.len as usize]
            }
            ElementArrayKey::E8 => {
                &self.e2pow8.values[vector.elements_index].as_slice()[0..vector.len as usize]
            }
            ElementArrayKey::E10 => {
                &self.e2pow10.values[vector.elements_index].as_slice()[0..vector.len as usize]
            }
            ElementArrayKey::E12 => {
                &self.e2pow12.values[vector.elements_index].as_slice()[0..vector.len as usize]
            }
            ElementArrayKey::E16 => {
                &self.e2pow16.values[vector.elements_index].as_slice()[0..vector.len as usize]
            }
            ElementArrayKey::E24 => {
                &self.e2pow24.values[vector.elements_index].as_slice()[0..vector.len as usize]
            }
            ElementArrayKey::E32 => {
                &self.e2pow32.values[vector.elements_index].as_slice()[0..vector.len as usize]
            }
        }
    }

    pub fn get_mut(&mut self, vector: ElementsVector) -> &mut [Option<Value>] {
        match vector.cap {
            ElementArrayKey::Empty => &mut [],
            ElementArrayKey::E4 => &mut self.e2pow4.values[vector.elements_index].as_mut_slice()
                [0..vector.len as usize],
            ElementArrayKey::E6 => &mut self.e2pow6.values[vector.elements_index].as_mut_slice()
                [0..vector.len as usize],
            ElementArrayKey::E8 => &mut self.e2pow8.values[vector.elements_index].as_mut_slice()
                [0..vector.len as usize],
            ElementArrayKey::E10 => &mut self.e2pow10.values[vector.elements_index].as_mut_slice()
                [0..vector.len as usize],
            ElementArrayKey::E12 => &mut self.e2pow12.values[vector.elements_index].as_mut_slice()
                [0..vector.len as usize],
            ElementArrayKey::E16 => &mut self.e2pow16.values[vector.elements_index].as_mut_slice()
                [0..vector.len as usize],
            ElementArrayKey::E24 => &mut self.e2pow24.values[vector.elements_index].as_mut_slice()
                [0..vector.len as usize],
            ElementArrayKey::E32 => &mut self.e2pow32.values[vector.elements_index].as_mut_slice()
                [0..vector.len as usize],
        }
    }

    pub fn get_full(
        &self,
        vector: ElementsVector,
    ) -> (Option<&HashMap<u32, ElementDescriptor>>, &[Option<Value>]) {
        let usize_index = vector.elements_index.into_index();
        match vector.cap {
            ElementArrayKey::Empty => (None, &[]),
            ElementArrayKey::E4 => {
                let epow = &self.e2pow4;
                (
                    epow.descriptors.get(&vector.elements_index),
                    &epow
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[0..vector.len as usize],
                )
            }
            ElementArrayKey::E6 => {
                let epow = &self.e2pow6;
                (
                    epow.descriptors.get(&vector.elements_index),
                    &epow
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[0..vector.len as usize],
                )
            }
            ElementArrayKey::E8 => {
                let epow = &self.e2pow8;
                (
                    epow.descriptors.get(&vector.elements_index),
                    &epow
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[0..vector.len as usize],
                )
            }
            ElementArrayKey::E10 => {
                let epow = &self.e2pow10;
                (
                    epow.descriptors.get(&vector.elements_index),
                    &epow
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[0..vector.len as usize],
                )
            }
            ElementArrayKey::E12 => {
                let epow = &self.e2pow12;
                (
                    epow.descriptors.get(&vector.elements_index),
                    &epow
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[0..vector.len as usize],
                )
            }
            ElementArrayKey::E16 => {
                let epow = &self.e2pow16;
                (
                    epow.descriptors.get(&vector.elements_index),
                    &epow
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[0..vector.len as usize],
                )
            }
            ElementArrayKey::E24 => {
                let epow = &self.e2pow24;
                (
                    epow.descriptors.get(&vector.elements_index),
                    &epow
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[0..vector.len as usize],
                )
            }
            ElementArrayKey::E32 => {
                let epow = &self.e2pow32;
                (
                    epow.descriptors.get(&vector.elements_index),
                    &epow
                        .values
                        .get(usize_index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[0..vector.len as usize],
                )
            }
        }
    }

    pub fn get_descriptor(
        &self,
        vector: ElementsVector,
        index: usize,
    ) -> Option<ElementDescriptor> {
        let Ok(index) = u32::try_from(index) else {
            return None;
        };
        let descriptors = match vector.cap {
            ElementArrayKey::Empty => return None,
            ElementArrayKey::E4 => &self.e2pow4.descriptors,
            ElementArrayKey::E6 => &self.e2pow6.descriptors,
            ElementArrayKey::E8 => &self.e2pow8.descriptors,
            ElementArrayKey::E10 => &self.e2pow10.descriptors,
            ElementArrayKey::E12 => &self.e2pow12.descriptors,
            ElementArrayKey::E16 => &self.e2pow16.descriptors,
            ElementArrayKey::E24 => &self.e2pow24.descriptors,
            ElementArrayKey::E32 => &self.e2pow32.descriptors,
        };
        descriptors
            .get(&vector.elements_index)?
            .get(&index)
            .copied()
    }

    pub fn set_descriptor(
        &mut self,
        vector: ElementsVector,
        index: usize,
        descriptor: Option<ElementDescriptor>,
    ) {
        let index: u32 = index.try_into().unwrap();
        assert!(index < vector.len);
        let descriptors = match vector.cap {
            ElementArrayKey::Empty => unreachable!(),
            ElementArrayKey::E4 => &mut self.e2pow4.descriptors,
            ElementArrayKey::E6 => &mut self.e2pow6.descriptors,
            ElementArrayKey::E8 => &mut self.e2pow8.descriptors,
            ElementArrayKey::E10 => &mut self.e2pow10.descriptors,
            ElementArrayKey::E12 => &mut self.e2pow12.descriptors,
            ElementArrayKey::E16 => &mut self.e2pow16.descriptors,
            ElementArrayKey::E24 => &mut self.e2pow24.descriptors,
            ElementArrayKey::E32 => &mut self.e2pow32.descriptors,
        };
        if let Some(inner_map) = descriptors.get_mut(&vector.elements_index) {
            if let Some(descriptor) = descriptor {
                inner_map.insert(index, descriptor);
            } else {
                inner_map.remove(&index);
            }
        } else if let Some(descriptor) = descriptor {
            let mut inner_map = HashMap::new();
            inner_map.insert(index, descriptor);
            descriptors.insert(vector.elements_index, inner_map);
        }
    }

    pub fn has(&self, vector: ElementsVector, element: Value) -> bool {
        match vector.cap {
            ElementArrayKey::Empty => false,
            ElementArrayKey::E4 => self.e2pow4.values[vector.elements_index].as_slice()
                [0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E6 => self.e2pow6.values[vector.elements_index].as_slice()
                [0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E8 => self.e2pow8.values[vector.elements_index].as_slice()
                [0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E10 => self.e2pow10.values[vector.elements_index].as_slice()
                [0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E12 => self.e2pow12.values[vector.elements_index].as_slice()
                [0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E16 => self.e2pow16.values[vector.elements_index].as_slice()
                [0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E24 => self.e2pow24.values[vector.elements_index].as_slice()
                [0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E32 => self.e2pow32.values[vector.elements_index].as_slice()
                [0..vector.len as usize]
                .contains(&Some(element)),
        }
    }
}
