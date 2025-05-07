// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashMap;
use small_string::SmallString;

use super::{
    CompactionLists, HeapMarkAndSweep, WorkQueues,
    indexes::{ElementIndex, PropertyKeyIndex},
    object_entry::{ObjectEntry, ObjectEntryPropertyDescriptor},
};
use crate::{
    SmallInteger,
    ecmascript::{
        execution::Agent,
        types::{Function, HeapString, PropertyDescriptor, PropertyKey, Symbol, Value},
    },
    engine::context::{Bindable, NoGcScope},
};
use core::{
    mem::MaybeUninit,
    ops::{Index, IndexMut},
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
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
    pub(crate) fn cap(self) -> u32 {
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
        } else {
            ElementArrayKey::E32
        }
    }
}

pub(crate) trait ElementsIndexable {
    fn elements_index(&self) -> ElementIndex<'static>;
    fn index(&self) -> usize;
    fn cap(&self) -> ElementArrayKey;
    fn len(&self) -> u32;
}

impl ElementsIndexable for ElementsVector<'_> {
    fn elements_index(&self) -> ElementIndex<'static> {
        self.elements_index.unbind()
    }

    fn index(&self) -> usize {
        self.elements_index.into_index()
    }

    fn cap(&self) -> ElementArrayKey {
        self.cap
    }

    fn len(&self) -> u32 {
        self.len
    }
}

impl ElementsIndexable for PropertyStorageVector<'_> {
    fn elements_index(&self) -> ElementIndex<'static> {
        self.values_index.unbind()
    }

    fn index(&self) -> usize {
        self.values_index.into_index()
    }

    fn cap(&self) -> ElementArrayKey {
        self.cap
    }

    fn len(&self) -> u32 {
        self.len
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ElementsVector<'a> {
    pub(crate) elements_index: ElementIndex<'a>,
    pub(crate) cap: ElementArrayKey,
    pub(crate) len: u32,
    /// Array length property can be set to unwritable
    pub(crate) len_writable: bool,
}

impl Default for ElementsVector<'static> {
    fn default() -> Self {
        Self {
            elements_index: ElementIndex::from_u32_index(0),
            cap: ElementArrayKey::Empty,
            len: 0,
            len_writable: true,
        }
    }
}

impl ElementsVector<'_> {
    pub(crate) fn cap(&self) -> u32 {
        self.cap.cap()
    }

    pub(crate) fn len(&self) -> u32 {
        self.len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn is_full(&self) -> bool {
        self.len == self.cap()
    }

    pub(crate) fn writable(&self) -> bool {
        self.len_writable
    }

    /// An elements vector is simple if it contains no accessor descriptors.
    pub(crate) fn is_simple(&self, arena: &impl AsRef<ElementArrays>) -> bool {
        let backing_store = arena.as_ref().get_descriptors_and_values(self);
        backing_store.0.is_none_or(|hashmap| {
            !hashmap
                .iter()
                .any(|desc| desc.1.has_getter() || desc.1.has_setter())
        })
    }

    /// An elements vector is trivial if it contains no descriptors.
    pub(crate) fn is_trivial(&self, arena: &impl AsRef<ElementArrays>) -> bool {
        let backing_store = arena.as_ref().get_descriptors_and_values(self);
        backing_store.0.is_none()
    }

    pub(crate) fn is_dense(&self, arena: &impl AsRef<ElementArrays>) -> bool {
        let (descriptors, elements) = arena.as_ref().get_descriptors_and_values(self);
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

    pub(crate) fn reserve(&mut self, elements: &mut ElementArrays, new_len: u32) {
        if new_len <= self.cap() {
            // Enough capacity present already
            return;
        }

        elements.reserve_values(self, new_len);
    }

    pub(crate) fn push(
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
        *next_over_end = value.map(Value::unbind);
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
                .entry(self.elements_index.unbind())
                .or_default()
                .insert(self.len, descriptor.unbind());
        }
        self.len += 1;
    }

    pub(crate) fn remove(&mut self, elements: &mut ElementArrays, index: usize) {
        let len = usize::try_from(self.len()).unwrap();
        assert!(index < len);

        match self.cap {
            ElementArrayKey::Empty => unreachable!(),
            ElementArrayKey::E4 => {
                elements.e2pow4.remove(self, index);
            }
            ElementArrayKey::E6 => {
                elements.e2pow6.remove(self, index);
            }
            ElementArrayKey::E8 => {
                elements.e2pow8.remove(self, index);
            }
            ElementArrayKey::E10 => {
                elements.e2pow10.remove(self, index);
            }
            ElementArrayKey::E12 => {
                elements.e2pow12.remove(self, index);
            }
            ElementArrayKey::E16 => {
                elements.e2pow16.remove(self, index);
            }
            ElementArrayKey::E24 => {
                elements.e2pow24.remove(self, index);
            }
            ElementArrayKey::E32 => {
                elements.e2pow32.remove(self, index);
            }
        };

        self.len -= 1;
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ElementsVector<'_> {
    type Of<'a> = ElementsVector<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for ElementsVector<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            elements_index,
            cap,
            len,
            len_writable: _,
        } = self;
        match cap {
            ElementArrayKey::Empty => {}
            ElementArrayKey::E4 => queues.e_2_4.push((*elements_index, *len)),
            ElementArrayKey::E6 => queues.e_2_6.push((*elements_index, *len)),
            ElementArrayKey::E8 => queues.e_2_8.push((*elements_index, *len)),
            ElementArrayKey::E10 => queues.e_2_10.push((*elements_index, *len)),
            ElementArrayKey::E12 => queues.e_2_12.push((*elements_index, *len)),
            ElementArrayKey::E16 => queues.e_2_16.push((*elements_index, *len)),
            ElementArrayKey::E24 => queues.e_2_24.push((*elements_index, *len)),
            ElementArrayKey::E32 => queues.e_2_32.push((*elements_index, *len)),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            elements_index,
            cap,
            len: _,
            len_writable: _,
        } = self;
        match cap {
            ElementArrayKey::Empty => {}
            ElementArrayKey::E4 => compactions.e_2_4.shift_index(elements_index),
            ElementArrayKey::E6 => compactions.e_2_6.shift_index(elements_index),
            ElementArrayKey::E8 => compactions.e_2_8.shift_index(elements_index),
            ElementArrayKey::E10 => compactions.e_2_10.shift_index(elements_index),
            ElementArrayKey::E12 => compactions.e_2_12.shift_index(elements_index),
            ElementArrayKey::E16 => compactions.e_2_16.shift_index(elements_index),
            ElementArrayKey::E24 => compactions.e_2_24.shift_index(elements_index),
            ElementArrayKey::E32 => compactions.e_2_32.shift_index(elements_index),
        };
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PropertyStorageVector<'a> {
    pub(crate) keys_index: PropertyKeyIndex<'a>,
    pub(crate) values_index: ElementIndex<'a>,
    pub(crate) cap: ElementArrayKey,
    pub(crate) len: u32,
    pub(crate) extensible: bool,
}

impl<'a> PropertyStorageVector<'a> {
    pub(crate) fn cap(&self) -> u32 {
        self.cap.cap()
    }

    pub(crate) fn len(&self) -> u32 {
        self.len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn is_full(&self) -> bool {
        self.len == self.cap()
    }

    pub(crate) fn reserve(&mut self, elements: &mut impl AsMut<ElementArrays>, new_len: u32) {
        if new_len <= self.cap() {
            // Enough capacity present already
            return;
        }

        elements.as_mut().reserve_keys_and_values(self, new_len);
    }

    pub(crate) fn keys<'b>(
        &self,
        elements: &'b impl AsRef<ElementArrays>,
    ) -> &'b [PropertyKey<'a>] {
        elements.as_ref().get_keys(self)
    }

    pub(crate) fn keys_mut<'b>(
        &self,
        elements: &'b mut impl AsMut<ElementArrays>,
    ) -> &'b mut [Option<PropertyKey<'static>>] {
        elements.as_mut().get_keys_mut(self)
    }

    pub(crate) fn values<'b>(
        &self,
        elements: &'b impl AsRef<ElementArrays>,
    ) -> &'b [Option<Value<'a>>] {
        elements.as_ref().get_values(self)
    }

    pub(crate) fn values_mut<'b>(
        &self,
        elements: &'b mut impl AsMut<ElementArrays>,
    ) -> &'b mut [Option<Value<'static>>] {
        elements.as_mut().get_values_mut(self)
    }

    pub(crate) fn push(
        &mut self,
        elements: &mut impl AsMut<ElementArrays>,
        key: PropertyKey,
        value: Option<Value>,
        descriptor: Option<ElementDescriptor>,
    ) {
        let elements = elements.as_mut();
        if self.is_full() {
            elements.reserve_keys_and_values(self, self.len + 1);
        }
        let (next_key_over_end, next_value_over_end, descriptors_map) = match self.cap {
            ElementArrayKey::Empty => unreachable!(),
            ElementArrayKey::E4 => (
                &mut elements.k2pow4.keys[self.keys_index.into_index()][self.len as usize],
                &mut elements.e2pow4.values[self.values_index][self.len as usize],
                &mut elements.e2pow4.descriptors,
            ),
            ElementArrayKey::E6 => (
                &mut elements.k2pow6.keys[self.keys_index.into_index()][self.len as usize],
                &mut elements.e2pow6.values[self.values_index][self.len as usize],
                &mut elements.e2pow6.descriptors,
            ),
            ElementArrayKey::E8 => (
                &mut elements.k2pow8.keys[self.keys_index.into_index()][self.len as usize],
                &mut elements.e2pow8.values[self.values_index][self.len as usize],
                &mut elements.e2pow8.descriptors,
            ),
            ElementArrayKey::E10 => (
                &mut elements.k2pow10.keys[self.keys_index.into_index()][self.len as usize],
                &mut elements.e2pow10.values[self.values_index][self.len as usize],
                &mut elements.e2pow10.descriptors,
            ),
            ElementArrayKey::E12 => (
                &mut elements.k2pow12.keys[self.keys_index.into_index()][self.len as usize],
                &mut elements.e2pow12.values[self.values_index][self.len as usize],
                &mut elements.e2pow12.descriptors,
            ),
            ElementArrayKey::E16 => (
                &mut elements.k2pow16.keys[self.keys_index.into_index()][self.len as usize],
                &mut elements.e2pow16.values[self.values_index][self.len as usize],
                &mut elements.e2pow16.descriptors,
            ),
            ElementArrayKey::E24 => (
                &mut elements.k2pow24.keys[self.keys_index.into_index()][self.len as usize],
                &mut elements.e2pow24.values[self.values_index][self.len as usize],
                &mut elements.e2pow24.descriptors,
            ),
            ElementArrayKey::E32 => (
                &mut elements.k2pow32.keys[self.keys_index.into_index()][self.len as usize],
                &mut elements.e2pow32.values[self.values_index][self.len as usize],
                &mut elements.e2pow32.descriptors,
            ),
        };
        *next_key_over_end = Some(key.unbind());
        *next_value_over_end = value.map(Value::unbind);
        if let Some(descriptor) = descriptor {
            descriptors_map
                .entry(self.values_index.unbind())
                .or_default()
                .insert(self.len, descriptor.unbind());
        }
        self.len += 1;
    }

    pub(crate) fn remove(&mut self, elements: &mut ElementArrays, index: usize) {
        let len = usize::try_from(self.len()).unwrap();
        assert!(index < len);

        match self.cap {
            ElementArrayKey::Empty => unreachable!(),
            ElementArrayKey::E4 => {
                elements.k2pow4.remove(self, index);
                elements.e2pow4.remove(self, index);
            }
            ElementArrayKey::E6 => {
                elements.k2pow6.remove(self, index);
                elements.e2pow6.remove(self, index);
            }
            ElementArrayKey::E8 => {
                elements.k2pow8.remove(self, index);
                elements.e2pow8.remove(self, index);
            }
            ElementArrayKey::E10 => {
                elements.k2pow10.remove(self, index);
                elements.e2pow10.remove(self, index);
            }
            ElementArrayKey::E12 => {
                elements.k2pow12.remove(self, index);
                elements.e2pow12.remove(self, index);
            }
            ElementArrayKey::E16 => {
                elements.k2pow16.remove(self, index);
                elements.e2pow16.remove(self, index);
            }
            ElementArrayKey::E24 => {
                elements.k2pow24.remove(self, index);
                elements.e2pow24.remove(self, index);
            }
            ElementArrayKey::E32 => {
                elements.k2pow32.remove(self, index);
                elements.e2pow32.remove(self, index);
            }
        };

        self.len -= 1;
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for PropertyStorageVector<'_> {
    type Of<'a> = PropertyStorageVector<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ElementDescriptor<'a> {
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
    ReadOnlyEnumerableConfigurableAccessor { get: Function<'a> },
    /// ```js
    /// { get, enumerable: true, configurable: false }
    /// ```
    ReadOnlyEnumerableUnconfigurableAccessor { get: Function<'a> },
    /// ```js
    /// { get, enumerable: false, configurable: true }
    /// ```
    ReadOnlyUnenumerableConfigurableAccessor { get: Function<'a> },
    /// ```js
    /// { get, enumerable: false, configurable: false }
    /// ```
    ReadOnlyUnenumerableUnconfigurableAccessor { get: Function<'a> },
    /// ```js
    /// { set, enumerable: true, configurable: true }
    /// ```
    WriteOnlyEnumerableConfigurableAccessor { set: Function<'a> },
    /// ```js
    /// { set, enumerable: true, configurable: false }
    /// ```
    WriteOnlyEnumerableUnconfigurableAccessor { set: Function<'a> },
    /// ```js
    /// { set, enumerable: false, configurable: true }
    /// ```
    WriteOnlyUnenumerableConfigurableAccessor { set: Function<'a> },
    /// ```js
    /// { set, enumerable: false, configurable: false }
    /// ```
    WriteOnlyUnenumerableUnconfigurableAccessor { set: Function<'a> },
    /// ```js
    /// { get, set, enumerable: true, configurable: true }
    /// ```
    ReadWriteEnumerableConfigurableAccessor {
        get: Function<'a>,
        set: Function<'a>,
    },
    /// ```js
    /// { get, set, enumerable: true, configurable: false }
    /// ```
    ReadWriteEnumerableUnconfigurableAccessor {
        get: Function<'a>,
        set: Function<'a>,
    },
    /// ```js
    /// { get, set, enumerable: false, configurable: true }
    /// ```
    ReadWriteUnenumerableConfigurableAccessor {
        get: Function<'a>,
        set: Function<'a>,
    },
    /// ```js
    /// { get, set, enumerable: false, configurable: false }
    /// ```
    ReadWriteUnenumerableUnconfigurableAccessor {
        get: Function<'a>,
        set: Function<'a>,
    },
}

impl<'a> ElementDescriptor<'a> {
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

    pub(crate) const fn new_with_get_ec(get: Function<'a>, e: bool, c: bool) -> Self {
        match (e, c) {
            (true, true) => Self::ReadOnlyEnumerableConfigurableAccessor { get },
            (true, false) => Self::ReadOnlyEnumerableUnconfigurableAccessor { get },
            (false, true) => Self::ReadOnlyUnenumerableConfigurableAccessor { get },
            (false, false) => Self::ReadOnlyUnenumerableUnconfigurableAccessor { get },
        }
    }

    pub(crate) const fn new_with_set_ec(set: Function<'a>, e: bool, c: bool) -> Self {
        match (e, c) {
            (true, true) => Self::WriteOnlyEnumerableConfigurableAccessor { set },
            (true, false) => Self::WriteOnlyEnumerableUnconfigurableAccessor { set },
            (false, true) => Self::WriteOnlyUnenumerableConfigurableAccessor { set },
            (false, false) => Self::WriteOnlyUnenumerableUnconfigurableAccessor { set },
        }
    }

    pub(crate) const fn new_with_get_set_ec(
        get: Function<'a>,
        set: Function<'a>,
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

    pub(crate) fn from_object_entry_property_descriptor(
        desc: &ObjectEntryPropertyDescriptor<'a>,
    ) -> (Option<Self>, Option<Value<'a>>) {
        match desc {
            ObjectEntryPropertyDescriptor::Data {
                value,
                writable,
                enumerable,
                configurable,
            } => match (writable, enumerable, configurable) {
                (true, true, true) => (None, Some(*value)),
                (true, true, false) => (
                    Some(Self::WritableEnumerableUnconfigurableData),
                    Some(*value),
                ),
                (true, false, true) => (
                    Some(Self::WritableUnenumerableConfigurableData),
                    Some(*value),
                ),
                (true, false, false) => (
                    Some(Self::WritableUnenumerableUnconfigurableData),
                    Some(*value),
                ),
                (false, true, true) => {
                    (Some(Self::ReadOnlyEnumerableConfigurableData), Some(*value))
                }
                (false, true, false) => (
                    Some(Self::ReadOnlyEnumerableUnconfigurableData),
                    Some(*value),
                ),
                (false, false, true) => (
                    Some(Self::ReadOnlyUnenumerableConfigurableData),
                    Some(*value),
                ),
                (false, false, false) => (
                    Some(Self::ReadOnlyUnenumerableUnconfigurableData),
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
                    Some(Self::ReadOnlyEnumerableConfigurableAccessor { get: *get }),
                    None,
                ),
                (true, false) => (
                    Some(Self::ReadOnlyEnumerableUnconfigurableAccessor { get: *get }),
                    None,
                ),
                (false, true) => (
                    Some(Self::ReadOnlyUnenumerableConfigurableAccessor { get: *get }),
                    None,
                ),
                (false, false) => (
                    Some(Self::ReadOnlyUnenumerableUnconfigurableAccessor { get: *get }),
                    None,
                ),
            },
            ObjectEntryPropertyDescriptor::WriteOnly {
                set,
                enumerable,
                configurable,
            } => match (enumerable, configurable) {
                (true, true) => (
                    Some(Self::WriteOnlyEnumerableConfigurableAccessor { set: *set }),
                    None,
                ),
                (true, false) => (
                    Some(Self::WriteOnlyEnumerableUnconfigurableAccessor { set: *set }),
                    None,
                ),
                (false, true) => (
                    Some(Self::WriteOnlyUnenumerableConfigurableAccessor { set: *set }),
                    None,
                ),
                (false, false) => (
                    Some(Self::WriteOnlyUnenumerableUnconfigurableAccessor { set: *set }),
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
                    Some(Self::ReadWriteEnumerableConfigurableAccessor {
                        get: *get,
                        set: *set,
                    }),
                    None,
                ),
                (true, false) => (
                    Some(Self::ReadWriteEnumerableUnconfigurableAccessor {
                        get: *get,
                        set: *set,
                    }),
                    None,
                ),
                (false, true) => (
                    Some(Self::ReadWriteUnenumerableConfigurableAccessor {
                        get: *get,
                        set: *set,
                    }),
                    None,
                ),
                (false, false) => (
                    Some(Self::ReadWriteUnenumerableUnconfigurableAccessor {
                        get: *get,
                        set: *set,
                    }),
                    None,
                ),
            },
        }
    }

    pub(crate) fn from_property_descriptor(descriptor: PropertyDescriptor<'a>) -> Option<Self> {
        let configurable = descriptor.configurable.unwrap_or(false);
        let enumerable = descriptor.enumerable.unwrap_or(false);
        let writable = descriptor.writable.unwrap_or(false);
        if configurable
            && enumerable
            && descriptor.get.is_none()
            && descriptor.set.is_none()
            && writable
        {
            // Default data descriptor, return None.
            return None;
        }
        Some(match (descriptor.get, descriptor.set) {
            (None, None) => match (writable, enumerable, configurable) {
                (true, true, true) => unreachable!(),
                (true, true, false) => Self::WritableEnumerableUnconfigurableData,
                (true, false, true) => Self::WritableUnenumerableConfigurableData,
                (true, false, false) => Self::WritableUnenumerableUnconfigurableData,
                (false, true, true) => Self::ReadOnlyEnumerableConfigurableData,
                (false, true, false) => Self::ReadOnlyEnumerableUnconfigurableData,
                (false, false, true) => Self::ReadOnlyUnenumerableConfigurableData,
                (false, false, false) => Self::ReadOnlyUnenumerableUnconfigurableData,
            },
            (None, Some(set)) => match (enumerable, configurable) {
                (true, true) => Self::WriteOnlyEnumerableConfigurableAccessor { set },
                (true, false) => Self::WriteOnlyEnumerableUnconfigurableAccessor { set },
                (false, true) => Self::WriteOnlyUnenumerableConfigurableAccessor { set },
                (false, false) => Self::WriteOnlyUnenumerableUnconfigurableAccessor { set },
            },
            (Some(get), None) => match (enumerable, configurable) {
                (true, true) => Self::ReadOnlyEnumerableConfigurableAccessor { get },
                (true, false) => Self::ReadOnlyEnumerableUnconfigurableAccessor { get },
                (false, true) => Self::ReadOnlyUnenumerableConfigurableAccessor { get },
                (false, false) => Self::ReadOnlyUnenumerableUnconfigurableAccessor { get },
            },
            (Some(get), Some(set)) => match (enumerable, configurable) {
                (true, true) => Self::ReadWriteEnumerableConfigurableAccessor { get, set },
                (true, false) => Self::ReadWriteEnumerableUnconfigurableAccessor { get, set },
                (false, true) => Self::ReadWriteUnenumerableConfigurableAccessor { get, set },
                (false, false) => Self::ReadWriteUnenumerableUnconfigurableAccessor { get, set },
            },
        })
    }

    pub(crate) fn from_data_descriptor(descriptor: PropertyDescriptor<'a>) -> Option<Self> {
        debug_assert!(descriptor.is_data_descriptor());
        let configurable = descriptor.configurable.unwrap_or(false);
        let enumerable = descriptor.enumerable.unwrap_or(false);
        let writable = descriptor.writable.unwrap_or(false);
        if configurable && enumerable && writable {
            // Default data descriptor, return None.
            return None;
        }
        Some(match (writable, enumerable, configurable) {
            (true, true, true) => unreachable!(),
            (true, true, false) => Self::WritableEnumerableUnconfigurableData,
            (true, false, true) => Self::WritableUnenumerableConfigurableData,
            (true, false, false) => Self::WritableUnenumerableUnconfigurableData,
            (false, true, true) => Self::ReadOnlyEnumerableConfigurableData,
            (false, true, false) => Self::ReadOnlyEnumerableUnconfigurableData,
            (false, false, true) => Self::ReadOnlyUnenumerableConfigurableData,
            (false, false, false) => Self::ReadOnlyUnenumerableUnconfigurableData,
        })
    }

    pub(crate) fn from_accessor_descriptor(descriptor: PropertyDescriptor<'a>) -> Self {
        let enumerable = descriptor.enumerable.unwrap_or(false);
        let configurable = descriptor.configurable.unwrap_or(false);
        match (descriptor.get, descriptor.set) {
            (None, Some(set)) => match (enumerable, configurable) {
                (true, true) => Self::WriteOnlyEnumerableConfigurableAccessor { set },
                (true, false) => Self::WriteOnlyEnumerableUnconfigurableAccessor { set },
                (false, true) => Self::WriteOnlyUnenumerableConfigurableAccessor { set },
                (false, false) => Self::WriteOnlyUnenumerableUnconfigurableAccessor { set },
            },
            (Some(get), None) => match (enumerable, configurable) {
                (true, true) => Self::ReadOnlyEnumerableConfigurableAccessor { get },
                (true, false) => Self::ReadOnlyEnumerableUnconfigurableAccessor { get },
                (false, true) => Self::ReadOnlyUnenumerableConfigurableAccessor { get },
                (false, false) => Self::ReadOnlyUnenumerableUnconfigurableAccessor { get },
            },
            (Some(get), Some(set)) => match (enumerable, configurable) {
                (true, true) => Self::ReadWriteEnumerableConfigurableAccessor { get, set },
                (true, false) => Self::ReadWriteEnumerableUnconfigurableAccessor { get, set },
                (false, true) => Self::ReadWriteUnenumerableConfigurableAccessor { get, set },
                (false, false) => Self::ReadWriteUnenumerableUnconfigurableAccessor { get, set },
            },
            _ => unreachable!(),
        }
    }

    pub(crate) fn from_accessor_descriptor_fields(
        get: Option<Function<'a>>,
        set: Option<Function<'a>>,
        enumerable: bool,
        configurable: bool,
    ) -> Self {
        match (get, set) {
            (None, Some(set)) => match (enumerable, configurable) {
                (true, true) => Self::WriteOnlyEnumerableConfigurableAccessor { set },
                (true, false) => Self::WriteOnlyEnumerableUnconfigurableAccessor { set },
                (false, true) => Self::WriteOnlyUnenumerableConfigurableAccessor { set },
                (false, false) => Self::WriteOnlyUnenumerableUnconfigurableAccessor { set },
            },
            (Some(get), None) => match (enumerable, configurable) {
                (true, true) => Self::ReadOnlyEnumerableConfigurableAccessor { get },
                (true, false) => Self::ReadOnlyEnumerableUnconfigurableAccessor { get },
                (false, true) => Self::ReadOnlyUnenumerableConfigurableAccessor { get },
                (false, false) => Self::ReadOnlyUnenumerableUnconfigurableAccessor { get },
            },
            (Some(get), Some(set)) => match (enumerable, configurable) {
                (true, true) => Self::ReadWriteEnumerableConfigurableAccessor { get, set },
                (true, false) => Self::ReadWriteEnumerableUnconfigurableAccessor { get, set },
                (false, true) => Self::ReadWriteUnenumerableConfigurableAccessor { get, set },
                (false, false) => Self::ReadWriteUnenumerableUnconfigurableAccessor { get, set },
            },
            _ => unreachable!(),
        }
    }

    pub(crate) fn to_property_descriptor(
        descriptor: Option<Self>,
        value: Option<Value>,
    ) -> PropertyDescriptor<'a> {
        let descriptor =
            descriptor.unwrap_or(ElementDescriptor::WritableEnumerableConfigurableData);
        let value = value.map(Value::unbind);
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

    pub(crate) fn getter_function<'gc>(&self, gc: NoGcScope<'gc, '_>) -> Option<Function<'gc>> {
        match self {
            ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyEnumerableUnconfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyUnenumerableConfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyUnenumerableUnconfigurableAccessor { get }
            | ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { get, .. }
            | ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor { get, .. }
            | ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor { get, .. }
            | ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor { get, .. } => {
                Some(get.bind(gc))
            }
            _ => None,
        }
    }

    pub(crate) fn setter_function<'gc>(&self, gc: NoGcScope<'gc, '_>) -> Option<Function<'gc>> {
        match self {
            ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyEnumerableUnconfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyUnenumerableConfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyUnenumerableUnconfigurableAccessor { set }
            | ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { set, .. }
            | ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor { set, .. }
            | ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor { set, .. }
            | ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor { set, .. } => {
                Some(set.bind(gc))
            }
            _ => None,
        }
    }

    pub(crate) fn is_writable(&self) -> Option<bool> {
        match self {
            ElementDescriptor::WritableEnumerableConfigurableData
            | ElementDescriptor::WritableEnumerableUnconfigurableData
            | ElementDescriptor::WritableUnenumerableConfigurableData
            | ElementDescriptor::WritableUnenumerableUnconfigurableData => Some(true),
            ElementDescriptor::ReadOnlyEnumerableConfigurableData
            | ElementDescriptor::ReadOnlyEnumerableUnconfigurableData
            | ElementDescriptor::ReadOnlyUnenumerableConfigurableData
            | ElementDescriptor::ReadOnlyUnenumerableUnconfigurableData => Some(false),
            _ => None,
        }
    }

    pub(crate) fn is_enumerable(&self) -> bool {
        matches!(
            self,
            ElementDescriptor::WritableEnumerableConfigurableData
                | ElementDescriptor::WritableEnumerableUnconfigurableData
                | ElementDescriptor::ReadOnlyEnumerableConfigurableData
                | ElementDescriptor::ReadOnlyEnumerableUnconfigurableData
                | ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor { .. }
                | ElementDescriptor::ReadOnlyEnumerableUnconfigurableAccessor { .. }
                | ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor { .. }
                | ElementDescriptor::WriteOnlyEnumerableUnconfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor { .. }
        )
    }

    pub(crate) fn is_configurable(&self) -> bool {
        matches!(
            self,
            ElementDescriptor::WritableEnumerableConfigurableData
                | ElementDescriptor::WritableUnenumerableConfigurableData
                | ElementDescriptor::ReadOnlyEnumerableConfigurableData
                | ElementDescriptor::ReadOnlyUnenumerableConfigurableData
                | ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor { .. }
                | ElementDescriptor::ReadOnlyUnenumerableConfigurableAccessor { .. }
                | ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor { .. }
                | ElementDescriptor::WriteOnlyUnenumerableConfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { .. }
                | ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor { .. },
        )
    }

    pub(crate) fn is_accessor_descriptor(&self) -> bool {
        !matches!(
            self,
            ElementDescriptor::WritableEnumerableConfigurableData
                | ElementDescriptor::WritableEnumerableUnconfigurableData
                | ElementDescriptor::WritableUnenumerableConfigurableData
                | ElementDescriptor::WritableUnenumerableUnconfigurableData
                | ElementDescriptor::ReadOnlyEnumerableConfigurableData
                | ElementDescriptor::ReadOnlyEnumerableUnconfigurableData
                | ElementDescriptor::ReadOnlyUnenumerableConfigurableData
                | ElementDescriptor::ReadOnlyUnenumerableUnconfigurableData
        )
    }

    pub(crate) fn is_data_descriptor(&self) -> bool {
        !self.is_accessor_descriptor()
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ElementDescriptor<'_> {
    type Of<'a> = ElementDescriptor<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

#[derive(Debug, Default)]
pub struct ElementArray<const N: usize> {
    pub values: Vec<Option<[Option<Value<'static>>; N]>>,
    pub descriptors: AHashMap<ElementIndex<'static>, AHashMap<u32, ElementDescriptor<'static>>>,
}

impl<const N: usize> ElementArray<N> {
    pub(crate) fn with_capacity(cap: usize) -> Self {
        const { assert!(N.is_power_of_two()) }
        Self {
            values: Vec::with_capacity(cap),
            descriptors: Default::default(),
        }
    }

    fn get_values(&self, vector: &impl ElementsIndexable) -> &[Option<Value<'static>>] {
        &self.values[vector.elements_index()].as_slice()[0..vector.len() as usize]
    }

    fn get_values_mut(&mut self, vector: &impl ElementsIndexable) -> &mut [Option<Value<'static>>] {
        &mut self.values[vector.elements_index()].as_mut_slice()[0..vector.len() as usize]
    }

    fn get_descriptors_and_values(
        &self,
        vector: &impl ElementsIndexable,
    ) -> (
        Option<&AHashMap<u32, ElementDescriptor<'static>>>,
        &[Option<Value<'static>>],
    ) {
        (
            self.descriptors.get(&vector.elements_index()),
            &self
                .values
                .get(vector.index())
                .unwrap()
                .as_ref()
                .unwrap()
                .as_slice()[0..vector.len() as usize],
        )
    }

    fn get_descriptors_and_values_mut(
        &mut self,
        vector: &impl ElementsIndexable,
    ) -> (
        Option<&mut AHashMap<u32, ElementDescriptor<'static>>>,
        &mut [Option<Value<'static>>],
    ) {
        (
            self.descriptors.get_mut(&vector.elements_index()),
            &mut self
                .values
                .get_mut(vector.index())
                .unwrap()
                .as_mut()
                .unwrap()
                .as_mut_slice()[0..vector.len() as usize],
        )
    }

    fn push(
        &mut self,
        source: &[Option<Value>],
        descriptors: Option<AHashMap<u32, ElementDescriptor<'static>>>,
    ) -> ElementIndex<'static> {
        let length = source.len();
        self.values.reserve(1);
        let remaining = self.values.spare_capacity_mut();
        assert!(length <= N);
        let last = remaining.get_mut(0).unwrap();
        debug_assert_eq!(
            core::mem::size_of::<Option<[Option<Value>; N]>>(),
            core::mem::size_of::<[Option<Value>; N]>()
        );
        // SAFETY: We can move MaybeUninit from outside of the array into individual items in it.
        // Moving inside the Option<[_]> is less well defined; the size is asserted to be the same
        // but it could theoretically be that we end up copying a bit that says "the array is None".
        // Experimentally however, this works and we do not copy None but Some([_]).
        let last = unsafe {
            core::mem::transmute::<
                &mut MaybeUninit<Option<[Option<Value>; N]>>,
                &mut [MaybeUninit<Option<Value>>; N],
            >(last)
        };
        // SAFETY: Interpreting any T as MaybeUninit<T> is always safe.
        let len_slice = unsafe {
            core::mem::transmute::<&[Option<Value>], &[MaybeUninit<Option<Value>>]>(source)
        };
        last[..length].copy_from_slice(len_slice);
        last[length..].fill(MaybeUninit::new(None));
        // SAFETY: We have fully initialized the next item.
        unsafe {
            self.values.set_len(self.values.len() + 1);
        }
        // Check that above our moving inside of the `Option<[_]>` to copy data into the inner
        // array is indeed Some(_) afterwards.
        assert!(self.values.last().unwrap().is_some());
        let index = ElementIndex::last_element_index(&self.values);
        if let Some(descriptors) = descriptors {
            self.descriptors.insert(index, descriptors);
        }
        index
    }

    fn remove(&mut self, vector: &impl ElementsIndexable, index: usize) {
        let len = vector.len() as usize;
        let elements_index = vector.elements_index();
        let values = &mut self.values[elements_index][..];
        values.copy_within((index + 1)..len, index);
        values[len - 1] = None;
        let descriptors = self.descriptors.get_mut(&elements_index);
        if let Some(descriptor_map) = descriptors {
            let index = index as u32;
            descriptor_map.remove(&index);
            if descriptor_map.is_empty() {
                self.descriptors.remove(&elements_index);
                return;
            }
            let mut keys_to_move = descriptor_map
                .keys()
                .filter(|k| *k > &index)
                .map(|k| *k)
                .collect::<Vec<u32>>();
            // Note: keys must be sorted before moving them in the hash map as
            // otherwise it's possible to overwrite a not-yet-moved key.
            keys_to_move.sort();
            for k in keys_to_move {
                let v = descriptor_map.remove(&k).unwrap();
                descriptor_map.insert(k - 1, v);
            }
        }
    }
}

/// Element arrays of up to 16 elements
pub type ElementArray2Pow4 = ElementArray<16>;
/// Element arrays of up to 64 elements
pub type ElementArray2Pow6 = ElementArray<64>;
/// Element arrays of up to 256 elements
pub type ElementArray2Pow8 = ElementArray<256>;
/// Element arrays of up to 1024 elements
pub type ElementArray2Pow10 = ElementArray<1024>;
/// Element arrays of up to 4096 elements
pub type ElementArray2Pow12 = ElementArray<4096>;
/// Element arrays of up to 65536 elements
pub type ElementArray2Pow16 = ElementArray<65536>;
/// Element arrays of up to 16777216 elements
pub type ElementArray2Pow24 = ElementArray<16777216>;
/// Element arrays of up to 4294967296 elements
pub type ElementArray2Pow32 = ElementArray<4294967296>;

/// Element arrays of up to 16 elements
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct PropertyKeyArray<const N: usize> {
    pub keys: Vec<[Option<PropertyKey<'static>>; N]>,
}

impl<const N: usize> PropertyKeyArray<N> {
    pub(crate) fn with_capacity(cap: usize) -> Self {
        const { assert!(N.is_power_of_two()) }
        Self {
            keys: Vec::with_capacity(cap),
        }
    }

    fn has(&self, props: &PropertyStorageVector, key: PropertyKey) -> bool {
        let keys = self.get(props);
        keys.contains(&key)
    }

    fn get<'a>(&self, props: &PropertyStorageVector<'a>) -> &[PropertyKey<'a>] {
        let keys = &self.keys[props.keys_index.into_index()].as_slice()[0..props.len as usize];
        debug_assert!(keys.iter().all(|k| k.is_some()));
        // SAFETY: We're indexing into an initialized part of the slice where
        // only Some keys are present, and PropertyKey uses enum niches so Some
        // PropertyKey is equal to just a PropertyKey.
        unsafe { std::mem::transmute::<&[Option<PropertyKey<'a>>], &[PropertyKey<'a>]>(keys) }
    }

    fn get_mut(&mut self, props: &PropertyStorageVector) -> &mut [Option<PropertyKey<'static>>] {
        self.keys[props.keys_index.into_index()].as_mut_slice()
    }

    fn push(&mut self, source: &[PropertyKey]) -> PropertyKeyIndex<'static> {
        let length = source.len();
        self.keys.reserve(1);
        let remaining = self.keys.spare_capacity_mut();
        assert!(length <= N);
        let last = remaining.get_mut(0).unwrap();
        // SAFETY: We can move MaybeUninit from outside of the array into individual items in it.
        let last = unsafe {
            core::mem::transmute::<
                &mut MaybeUninit<[Option<PropertyKey>; N]>,
                &mut [MaybeUninit<Option<PropertyKey>>; N],
            >(last)
        };
        // SAFETY: Interpreting any T as MaybeUninit<T> is always safe
        // and we checked above in const that PropertyKey can be
        // reinterpreted as Option<PropertyKey>.
        let len_slice = unsafe {
            core::mem::transmute::<&[PropertyKey], &[MaybeUninit<Option<PropertyKey>>]>(source)
        };
        last[..length].copy_from_slice(len_slice);
        last[length..].fill(MaybeUninit::new(None));
        // SAFETY: We have fully initialized the next item.
        unsafe {
            self.keys.set_len(self.keys.len() + 1);
        }
        PropertyKeyIndex::last_property_key_index(&self.keys)
    }

    fn remove(&mut self, props: &PropertyStorageVector, index: usize) {
        let len = props.len as usize;
        let keys = &mut self.keys[props.keys_index.into_index()][..];
        keys.copy_within((index + 1)..len, index);
        keys[len - 1] = None;
    }
}

/// Property key arrays of up to 16 elements
pub type PropertyKeyArray2Pow4 = PropertyKeyArray<16>;
/// Property key arrays of up to 64 elements
pub type PropertyKeyArray2Pow6 = PropertyKeyArray<64>;
/// Property key arrays of up to 256 elements
pub type PropertyKeyArray2Pow8 = PropertyKeyArray<256>;
/// Property key arrays of up to 1024 elements
pub type PropertyKeyArray2Pow10 = PropertyKeyArray<1024>;
/// Property key arrays of up to 4096 elements
pub type PropertyKeyArray2Pow12 = PropertyKeyArray<4096>;
/// Property key arrays of up to 65536 elements
pub type PropertyKeyArray2Pow16 = PropertyKeyArray<65536>;
/// Property key arrays of up to 16777216 elements
pub type PropertyKeyArray2Pow24 = PropertyKeyArray<16777216>;
/// Property key arrays of up to 4294967296 elements
pub type PropertyKeyArray2Pow32 = PropertyKeyArray<4294967296>;

#[derive(Debug)]
pub struct ElementArrays {
    /// up to 16 elements
    pub k2pow4: PropertyKeyArray2Pow4,
    pub e2pow4: ElementArray2Pow4,
    /// up to 64 elements
    pub k2pow6: PropertyKeyArray2Pow6,
    pub e2pow6: ElementArray2Pow6,
    /// up to 256 elements
    pub k2pow8: PropertyKeyArray2Pow8,
    pub e2pow8: ElementArray2Pow8,
    /// up to 1024 elements
    pub k2pow10: PropertyKeyArray2Pow10,
    pub e2pow10: ElementArray2Pow10,
    /// up to 4096 elements
    pub k2pow12: PropertyKeyArray2Pow12,
    pub e2pow12: ElementArray2Pow12,
    /// up to 65536 elements
    pub k2pow16: PropertyKeyArray2Pow16,
    pub e2pow16: ElementArray2Pow16,
    /// up to 16777216 elements
    pub k2pow24: PropertyKeyArray2Pow24,
    pub e2pow24: ElementArray2Pow24,
    /// up to 4294967296 elements
    pub k2pow32: PropertyKeyArray2Pow32,
    pub e2pow32: ElementArray2Pow32,
}

impl Index<&ElementsVector<'_>> for ElementArrays {
    type Output = [Option<Value<'static>>];

    fn index(&self, index: &ElementsVector) -> &Self::Output {
        self.get_values(index)
    }
}

impl IndexMut<&ElementsVector<'_>> for ElementArrays {
    fn index_mut(&mut self, index: &ElementsVector) -> &mut Self::Output {
        self.get_values_mut(index)
    }
}

impl Index<&ElementsVector<'_>> for Agent {
    type Output = [Option<Value<'static>>];

    fn index(&self, index: &ElementsVector) -> &Self::Output {
        &self.heap.elements[index]
    }
}

impl IndexMut<&ElementsVector<'_>> for Agent {
    fn index_mut(&mut self, index: &ElementsVector) -> &mut Self::Output {
        &mut self.heap.elements[index]
    }
}

impl ElementArrays {
    fn push_values(
        &mut self,
        key: ElementArrayKey,
        source: &[Option<Value>],
        descriptors: Option<AHashMap<u32, ElementDescriptor<'static>>>,
    ) -> ElementIndex<'static> {
        debug_assert_eq!(
            core::mem::size_of::<Option<[Option<Value>; 1]>>(),
            core::mem::size_of::<[Option<Value>; 1]>()
        );
        match key {
            ElementArrayKey::Empty => {
                assert!(source.is_empty() && descriptors.is_none());
                ElementIndex::from_u32_index(0)
            }
            ElementArrayKey::E4 => self.e2pow4.push(source, descriptors),
            ElementArrayKey::E6 => self.e2pow6.push(source, descriptors),
            ElementArrayKey::E8 => self.e2pow8.push(source, descriptors),
            ElementArrayKey::E10 => self.e2pow10.push(source, descriptors),
            ElementArrayKey::E12 => self.e2pow12.push(source, descriptors),
            ElementArrayKey::E16 => self.e2pow16.push(source, descriptors),
            ElementArrayKey::E24 => self.e2pow24.push(source, descriptors),
            ElementArrayKey::E32 => self.e2pow32.push(source, descriptors),
        }
    }

    fn push_keys(
        &mut self,
        key: ElementArrayKey,
        source: &[PropertyKey],
    ) -> PropertyKeyIndex<'static> {
        const {
            // Check at compile time that PropertyKey can be transmuted into
            // Option<PropertyKey> without issues and will show up as Some.
            assert!(
                core::mem::size_of::<Option<PropertyKey>>() == core::mem::size_of::<PropertyKey>()
            );
            let property_key_array: [PropertyKey; 4] = [
                PropertyKey::Integer(SmallInteger::zero()),
                PropertyKey::SmallString(SmallString::EMPTY),
                PropertyKey::String(HeapString::_def()),
                PropertyKey::Symbol(Symbol::_def()),
            ];
            // SAFETY: Sizes match, this should be nothing more than a
            // reinterpretation.
            let property_key_option_array = unsafe {
                std::mem::transmute::<[PropertyKey; 4], [Option<PropertyKey>; 4]>(
                    property_key_array,
                )
            };
            let mut i = 0;
            loop {
                let key = property_key_array[i];
                let option_key = property_key_option_array[i];
                assert!(option_key.is_some());
                let option_key = option_key.unwrap();
                match (option_key, key) {
                    (PropertyKey::Integer(a), PropertyKey::Integer(b)) => {
                        assert!(a.into_i64() == b.into_i64())
                    }
                    (PropertyKey::SmallString(a), PropertyKey::SmallString(b)) => {
                        let mut i: u8 = 0;
                        let a = a.data();
                        let b = b.data();
                        loop {
                            assert!(a[i as usize] == b[i as usize]);
                            i += 1;
                            if i == 7 {
                                break;
                            }
                        }
                    }
                    (PropertyKey::String(a), PropertyKey::String(b)) => {
                        assert!(a.get_index() == b.get_index())
                    }
                    (PropertyKey::Symbol(a), PropertyKey::Symbol(b)) => {
                        assert!(a.get_index() == b.get_index())
                    }
                    _ => unreachable!(),
                };
                i += 1;
                if i > 3 {
                    break;
                }
            }
        }
        match key {
            ElementArrayKey::Empty => {
                assert!(source.is_empty());
                PropertyKeyIndex::from_u32_index(0)
            }
            ElementArrayKey::E4 => self.k2pow4.push(source),
            ElementArrayKey::E6 => self.k2pow6.push(source),
            ElementArrayKey::E8 => self.k2pow8.push(source),
            ElementArrayKey::E10 => self.k2pow10.push(source),
            ElementArrayKey::E12 => self.k2pow12.push(source),
            ElementArrayKey::E16 => self.k2pow16.push(source),
            ElementArrayKey::E24 => self.k2pow24.push(source),
            ElementArrayKey::E32 => self.k2pow32.push(source),
        }
    }

    fn reserve_keys_and_values(&mut self, props: &mut PropertyStorageVector, new_len: u32) {
        if new_len <= props.cap() {
            // Already big enough, no need to grow
            return;
        }
        let new_key = ElementArrayKey::from(new_len);
        assert_ne!(new_key, props.cap);
        let ElementArrays {
            e2pow4,
            e2pow6,
            e2pow8,
            e2pow10,
            e2pow12,
            e2pow16,
            e2pow24,
            e2pow32,
            k2pow4,
            k2pow6,
            k2pow8,
            k2pow10,
            k2pow12,
            k2pow16,
            k2pow24,
            k2pow32,
        } = self;
        let (new_keys_index, new_values_index) = match new_key {
            ElementArrayKey::Empty => {
                // 0 <= elements_vector.cap() for all possible values.
                unreachable!();
            }
            ElementArrayKey::E4 => {
                let (keys, (descriptors, source)): (
                    &[PropertyKey],
                    (
                        Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                        &[Option<Value<'static>>],
                    ),
                ) = match props.cap {
                    ElementArrayKey::Empty => (&[], (None, &[])),
                    // Note: Only Empty is smaller than E4.
                    ElementArrayKey::E4
                    | ElementArrayKey::E6
                    | ElementArrayKey::E8
                    | ElementArrayKey::E10
                    | ElementArrayKey::E12
                    | ElementArrayKey::E16
                    | ElementArrayKey::E24
                    | ElementArrayKey::E32 => unreachable!(),
                };
                (k2pow4.push(keys), e2pow4.push(source, descriptors.cloned()))
            }
            ElementArrayKey::E6 => {
                let (keys, (descriptors, source)): (
                    &[PropertyKey],
                    (
                        Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                        &[Option<Value<'static>>],
                    ),
                ) = match props.cap {
                    ElementArrayKey::Empty => (&[], (None, &[])),
                    ElementArrayKey::E4 => {
                        (k2pow4.get(props), e2pow4.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E6
                    | ElementArrayKey::E8
                    | ElementArrayKey::E10
                    | ElementArrayKey::E12
                    | ElementArrayKey::E16
                    | ElementArrayKey::E24
                    | ElementArrayKey::E32 => unreachable!(),
                };
                (k2pow6.push(keys), e2pow6.push(source, descriptors.cloned()))
            }
            ElementArrayKey::E8 => {
                let (keys, (descriptors, source)): (
                    &[PropertyKey],
                    (
                        Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                        &[Option<Value<'static>>],
                    ),
                ) = match props.cap {
                    ElementArrayKey::Empty => (&[], (None, &[])),
                    ElementArrayKey::E4 => {
                        (k2pow4.get(props), e2pow4.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E6 => {
                        (k2pow6.get(props), e2pow6.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E8
                    | ElementArrayKey::E10
                    | ElementArrayKey::E12
                    | ElementArrayKey::E16
                    | ElementArrayKey::E24
                    | ElementArrayKey::E32 => unreachable!(),
                };
                (k2pow8.push(keys), e2pow8.push(source, descriptors.cloned()))
            }
            ElementArrayKey::E10 => {
                let (keys, (descriptors, source)): (
                    &[PropertyKey],
                    (
                        Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                        &[Option<Value<'static>>],
                    ),
                ) = match props.cap {
                    ElementArrayKey::Empty => (&[], (None, &[])),
                    ElementArrayKey::E4 => {
                        (k2pow4.get(props), e2pow4.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E6 => {
                        (k2pow6.get(props), e2pow6.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E8 => {
                        (k2pow8.get(props), e2pow8.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E10
                    | ElementArrayKey::E12
                    | ElementArrayKey::E16
                    | ElementArrayKey::E24
                    | ElementArrayKey::E32 => unreachable!(),
                };
                (
                    k2pow10.push(keys),
                    e2pow10.push(source, descriptors.cloned()),
                )
            }
            ElementArrayKey::E12 => {
                let (keys, (descriptors, source)): (
                    &[PropertyKey],
                    (
                        Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                        &[Option<Value<'static>>],
                    ),
                ) = match props.cap {
                    ElementArrayKey::Empty => (&[], (None, &[])),
                    ElementArrayKey::E4 => {
                        (k2pow4.get(props), e2pow4.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E6 => {
                        (k2pow6.get(props), e2pow6.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E8 => {
                        (k2pow8.get(props), e2pow8.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E10 => (
                        k2pow10.get(props),
                        e2pow10.get_descriptors_and_values(props),
                    ),
                    ElementArrayKey::E12
                    | ElementArrayKey::E16
                    | ElementArrayKey::E24
                    | ElementArrayKey::E32 => unreachable!(),
                };
                (
                    k2pow12.push(keys),
                    e2pow12.push(source, descriptors.cloned()),
                )
            }
            ElementArrayKey::E16 => {
                let (keys, (descriptors, source)): (
                    &[PropertyKey],
                    (
                        Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                        &[Option<Value<'static>>],
                    ),
                ) = match props.cap {
                    ElementArrayKey::Empty => (&[], (None, &[])),
                    ElementArrayKey::E4 => {
                        (k2pow4.get(props), e2pow4.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E6 => {
                        (k2pow6.get(props), e2pow6.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E8 => {
                        (k2pow8.get(props), e2pow8.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E10 => (
                        k2pow10.get(props),
                        e2pow10.get_descriptors_and_values(props),
                    ),
                    ElementArrayKey::E12 => (
                        k2pow12.get(props),
                        e2pow12.get_descriptors_and_values(props),
                    ),
                    ElementArrayKey::E16 | ElementArrayKey::E24 | ElementArrayKey::E32 => {
                        unreachable!()
                    }
                };
                (
                    k2pow16.push(keys),
                    e2pow16.push(source, descriptors.cloned()),
                )
            }
            ElementArrayKey::E24 => {
                let (keys, (descriptors, source)): (
                    &[PropertyKey],
                    (
                        Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                        &[Option<Value<'static>>],
                    ),
                ) = match props.cap {
                    ElementArrayKey::Empty => (&[], (None, &[])),
                    ElementArrayKey::E4 => {
                        (k2pow4.get(props), e2pow4.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E6 => {
                        (k2pow6.get(props), e2pow6.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E8 => {
                        (k2pow8.get(props), e2pow8.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E10 => (
                        k2pow10.get(props),
                        e2pow10.get_descriptors_and_values(props),
                    ),
                    ElementArrayKey::E12 => (
                        k2pow12.get(props),
                        e2pow12.get_descriptors_and_values(props),
                    ),
                    ElementArrayKey::E16 => (
                        k2pow16.get(props),
                        e2pow16.get_descriptors_and_values(props),
                    ),
                    ElementArrayKey::E24 | ElementArrayKey::E32 => unreachable!(),
                };
                (
                    k2pow24.push(keys),
                    e2pow24.push(source, descriptors.cloned()),
                )
            }
            ElementArrayKey::E32 => {
                let (keys, (descriptors, source)): (
                    &[PropertyKey],
                    (
                        Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                        &[Option<Value<'static>>],
                    ),
                ) = match props.cap {
                    ElementArrayKey::Empty => (&[], (None, &[])),
                    ElementArrayKey::E4 => {
                        (k2pow4.get(props), e2pow4.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E6 => {
                        (k2pow6.get(props), e2pow6.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E8 => {
                        (k2pow8.get(props), e2pow8.get_descriptors_and_values(props))
                    }
                    ElementArrayKey::E10 => (
                        k2pow10.get(props),
                        e2pow10.get_descriptors_and_values(props),
                    ),
                    ElementArrayKey::E12 => (
                        k2pow12.get(props),
                        e2pow12.get_descriptors_and_values(props),
                    ),
                    ElementArrayKey::E16 => (
                        k2pow16.get(props),
                        e2pow16.get_descriptors_and_values(props),
                    ),
                    ElementArrayKey::E24 => (
                        k2pow24.get(props),
                        e2pow24.get_descriptors_and_values(props),
                    ),
                    ElementArrayKey::E32 => unreachable!(),
                };
                (
                    k2pow32.push(keys),
                    e2pow32.push(source, descriptors.cloned()),
                )
            }
        };
        props.cap = new_key;
        props.keys_index = new_keys_index;
        props.values_index = new_values_index;
    }

    fn reserve_values(&mut self, elements_vector: &mut ElementsVector, new_len: u32) {
        if new_len <= elements_vector.cap() {
            // Already big enough, no need to grow
            return;
        }
        let new_key = ElementArrayKey::from(new_len);
        assert_ne!(new_key, elements_vector.cap);
        let ElementArrays {
            e2pow4,
            e2pow6,
            e2pow8,
            e2pow10,
            e2pow12,
            e2pow16,
            e2pow24,
            e2pow32,
            ..
        } = self;
        let new_index = match new_key {
            ElementArrayKey::Empty => {
                // 0 <= elements_vector.cap() for all possible values.
                unreachable!();
            }
            ElementArrayKey::E4 => {
                let (descriptors, source): (
                    Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                    &[Option<Value<'static>>],
                ) = match elements_vector.cap {
                    ElementArrayKey::Empty => (None, &[]),
                    ElementArrayKey::E4 => {
                        unreachable!()
                    }
                    ElementArrayKey::E6 => e2pow6.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E8 => e2pow8.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E10 => e2pow10.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E12 => e2pow12.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E16 => e2pow16.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E24 => e2pow24.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E32 => e2pow32.get_descriptors_and_values(elements_vector),
                };
                e2pow4.push(source, descriptors.cloned())
            }
            ElementArrayKey::E6 => {
                let (descriptors, source): (
                    Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                    &[Option<Value<'static>>],
                ) = match elements_vector.cap {
                    ElementArrayKey::Empty => (None, &[]),
                    ElementArrayKey::E4 => e2pow4.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E6 => {
                        unreachable!()
                    }
                    ElementArrayKey::E8 => e2pow8.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E10 => e2pow10.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E12 => e2pow12.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E16 => e2pow16.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E24 => e2pow24.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E32 => e2pow32.get_descriptors_and_values(elements_vector),
                };
                e2pow6.push(source, descriptors.cloned())
            }
            ElementArrayKey::E8 => {
                let (descriptors, source): (
                    Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                    &[Option<Value<'static>>],
                ) = match elements_vector.cap {
                    ElementArrayKey::Empty => (None, &[]),
                    ElementArrayKey::E4 => e2pow4.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E6 => e2pow6.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E8 => {
                        unreachable!()
                    }
                    ElementArrayKey::E10 => e2pow10.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E12 => e2pow12.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E16 => e2pow16.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E24 => e2pow24.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E32 => e2pow32.get_descriptors_and_values(elements_vector),
                };
                e2pow8.push(source, descriptors.cloned())
            }
            ElementArrayKey::E10 => {
                let (descriptors, source): (
                    Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                    &[Option<Value<'static>>],
                ) = match elements_vector.cap {
                    ElementArrayKey::Empty => (None, &[]),
                    ElementArrayKey::E4 => e2pow4.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E6 => e2pow6.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E8 => e2pow8.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E10 => {
                        unreachable!()
                    }
                    ElementArrayKey::E12 => e2pow12.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E16 => e2pow16.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E24 => e2pow24.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E32 => e2pow32.get_descriptors_and_values(elements_vector),
                };
                e2pow10.push(source, descriptors.cloned())
            }
            ElementArrayKey::E12 => {
                let (descriptors, source): (
                    Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                    &[Option<Value<'static>>],
                ) = match elements_vector.cap {
                    ElementArrayKey::Empty => (None, &[]),
                    ElementArrayKey::E4 => e2pow4.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E6 => e2pow6.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E8 => e2pow8.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E10 => e2pow10.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E12 => {
                        unreachable!()
                    }
                    ElementArrayKey::E16 => e2pow16.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E24 => e2pow24.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E32 => e2pow32.get_descriptors_and_values(elements_vector),
                };
                e2pow12.push(source, descriptors.cloned())
            }
            ElementArrayKey::E16 => {
                let (descriptors, source): (
                    Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                    &[Option<Value<'static>>],
                ) = match elements_vector.cap {
                    ElementArrayKey::Empty => (None, &[]),
                    ElementArrayKey::E4 => e2pow4.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E6 => e2pow6.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E8 => e2pow8.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E10 => e2pow10.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E12 => e2pow12.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E16 => {
                        unreachable!()
                    }
                    ElementArrayKey::E24 => e2pow24.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E32 => e2pow32.get_descriptors_and_values(elements_vector),
                };
                e2pow16.push(source, descriptors.cloned())
            }
            ElementArrayKey::E24 => {
                let (descriptors, source): (
                    Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                    &[Option<Value<'static>>],
                ) = match elements_vector.cap {
                    ElementArrayKey::E4 => e2pow4.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E6 => e2pow6.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E8 => e2pow8.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E10 => e2pow10.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E12 => e2pow12.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E16 => e2pow16.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E24 => {
                        unreachable!()
                    }
                    ElementArrayKey::E32 => e2pow32.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::Empty => (None, &[]),
                };
                e2pow24.push(source, descriptors.cloned())
            }
            ElementArrayKey::E32 => {
                let (descriptors, source): (
                    Option<&AHashMap<u32, ElementDescriptor<'static>>>,
                    &[Option<Value<'static>>],
                ) = match elements_vector.cap {
                    ElementArrayKey::Empty => (None, &[]),
                    ElementArrayKey::E4 => e2pow4.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E6 => e2pow6.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E8 => e2pow8.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E10 => e2pow10.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E12 => e2pow12.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E16 => e2pow16.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E24 => e2pow24.get_descriptors_and_values(elements_vector),
                    ElementArrayKey::E32 => {
                        unreachable!()
                    }
                };
                e2pow32.push(source, descriptors.cloned())
            }
        };
        elements_vector.cap = new_key;
        elements_vector.elements_index = new_index;
    }

    pub(crate) fn allocate_elements_with_capacity(
        &mut self,
        capacity: usize,
    ) -> ElementsVector<'static> {
        let cap = ElementArrayKey::from(capacity);
        ElementsVector {
            elements_index: self.push_values(cap, &[], None),
            cap,
            len: 0,
            len_writable: true,
        }
    }

    fn allocate_object_property_storage(
        &mut self,
        length: usize,
        keys: &[PropertyKey],
        values: &[Option<Value>],
        descriptors: Option<AHashMap<u32, ElementDescriptor<'static>>>,
    ) -> PropertyStorageVector<'static> {
        let cap = ElementArrayKey::from(length);
        let len = length as u32;
        let keys_index = self.push_keys(cap, keys);
        let values_index = self.push_values(cap, values, descriptors);
        PropertyStorageVector {
            keys_index,
            values_index,
            cap,
            len,
            extensible: true,
        }
    }

    pub(crate) fn allocate_object_property_storage_from_entries_vec<'a>(
        &mut self,
        mut entries: Vec<(
            PropertyKey<'a>,
            Option<ElementDescriptor>,
            Option<Value<'a>>,
        )>,
    ) -> PropertyStorageVector<'a> {
        let length = entries.len();
        let mut keys: Vec<PropertyKey> = Vec::with_capacity(length);
        let mut values: Vec<Option<Value>> = Vec::with_capacity(length);
        let mut descriptors: Option<AHashMap<u32, ElementDescriptor<'static>>> = None;
        entries.drain(..).enumerate().for_each(|(index, entry)| {
            let (key, maybe_descriptor, maybe_value) = entry;
            keys.push(key);
            values.push(maybe_value);
            if let Some(descriptor) = maybe_descriptor {
                if descriptors.is_none() {
                    descriptors = Some(Default::default());
                }
                descriptors
                    .as_mut()
                    .unwrap()
                    .insert(index as u32, descriptor.unbind());
            }
        });
        self.allocate_object_property_storage(length, &keys, &values, descriptors)
    }

    pub(crate) fn allocate_object_property_storage_from_entries_slice<'a>(
        &mut self,
        entries: &[ObjectEntry<'a>],
    ) -> PropertyStorageVector<'a> {
        let length = entries.len();
        let mut keys: Vec<PropertyKey> = Vec::with_capacity(length);
        let mut values: Vec<Option<Value>> = Vec::with_capacity(length);
        let mut descriptors: Option<AHashMap<u32, ElementDescriptor<'static>>> = None;
        for (index, entry) in entries.iter().enumerate() {
            let ObjectEntry { key, value } = entry;
            let (maybe_descriptor, maybe_value) =
                ElementDescriptor::from_object_entry_property_descriptor(value);
            keys.push(*key);
            values.push(maybe_value);
            if let Some(descriptor) = maybe_descriptor {
                if descriptors.is_none() {
                    descriptors = Some(Default::default());
                }
                descriptors
                    .as_mut()
                    .unwrap()
                    .insert(index as u32, descriptor.unbind());
            }
        }
        self.allocate_object_property_storage(length, &keys, &values, descriptors)
    }

    pub(crate) fn get_keys<'a>(&self, props: &PropertyStorageVector<'a>) -> &[PropertyKey<'a>] {
        match props.cap {
            ElementArrayKey::Empty => &[],
            ElementArrayKey::E4 => self.k2pow4.get(props),
            ElementArrayKey::E6 => self.k2pow6.get(props),
            ElementArrayKey::E8 => self.k2pow8.get(props),
            ElementArrayKey::E10 => self.k2pow10.get(props),
            ElementArrayKey::E12 => self.k2pow12.get(props),
            ElementArrayKey::E16 => self.k2pow16.get(props),
            ElementArrayKey::E24 => self.k2pow24.get(props),
            ElementArrayKey::E32 => self.k2pow32.get(props),
        }
    }

    pub(crate) fn get_keys_mut(
        &mut self,
        props: &PropertyStorageVector,
    ) -> &mut [Option<PropertyKey<'static>>] {
        match props.cap {
            ElementArrayKey::Empty => &mut [],
            ElementArrayKey::E4 => self.k2pow4.get_mut(props),
            ElementArrayKey::E6 => self.k2pow6.get_mut(props),
            ElementArrayKey::E8 => self.k2pow8.get_mut(props),
            ElementArrayKey::E10 => self.k2pow10.get_mut(props),
            ElementArrayKey::E12 => self.k2pow12.get_mut(props),
            ElementArrayKey::E16 => self.k2pow16.get_mut(props),
            ElementArrayKey::E24 => self.k2pow24.get_mut(props),
            ElementArrayKey::E32 => self.k2pow32.get_mut(props),
        }
    }

    pub(crate) fn get_values<'a>(&self, vector: &impl ElementsIndexable) -> &[Option<Value<'a>>] {
        match vector.cap() {
            ElementArrayKey::Empty => &[],
            ElementArrayKey::E4 => self.e2pow4.get_values(vector),
            ElementArrayKey::E6 => self.e2pow6.get_values(vector),
            ElementArrayKey::E8 => self.e2pow8.get_values(vector),
            ElementArrayKey::E10 => self.e2pow10.get_values(vector),
            ElementArrayKey::E12 => self.e2pow12.get_values(vector),
            ElementArrayKey::E16 => self.e2pow16.get_values(vector),
            ElementArrayKey::E24 => self.e2pow24.get_values(vector),
            ElementArrayKey::E32 => self.e2pow32.get_values(vector),
        }
    }

    pub(crate) fn get_values_mut(
        &mut self,
        vector: &impl ElementsIndexable,
    ) -> &mut [Option<Value<'static>>] {
        match vector.cap() {
            ElementArrayKey::Empty => &mut [],
            ElementArrayKey::E4 => self.e2pow4.get_values_mut(vector),
            ElementArrayKey::E6 => self.e2pow6.get_values_mut(vector),
            ElementArrayKey::E8 => self.e2pow8.get_values_mut(vector),
            ElementArrayKey::E10 => self.e2pow10.get_values_mut(vector),
            ElementArrayKey::E12 => self.e2pow12.get_values_mut(vector),
            ElementArrayKey::E16 => self.e2pow16.get_values_mut(vector),
            ElementArrayKey::E24 => self.e2pow24.get_values_mut(vector),
            ElementArrayKey::E32 => self.e2pow32.get_values_mut(vector),
        }
    }

    pub(crate) fn get_descriptors_and_values(
        &self,
        vector: &impl ElementsIndexable,
    ) -> (
        Option<&AHashMap<u32, ElementDescriptor<'static>>>,
        &[Option<Value<'static>>],
    ) {
        match vector.cap() {
            ElementArrayKey::Empty => (None, &[]),
            ElementArrayKey::E4 => self.e2pow4.get_descriptors_and_values(vector),
            ElementArrayKey::E6 => self.e2pow6.get_descriptors_and_values(vector),
            ElementArrayKey::E8 => self.e2pow8.get_descriptors_and_values(vector),
            ElementArrayKey::E10 => self.e2pow10.get_descriptors_and_values(vector),
            ElementArrayKey::E12 => self.e2pow12.get_descriptors_and_values(vector),
            ElementArrayKey::E16 => self.e2pow16.get_descriptors_and_values(vector),
            ElementArrayKey::E24 => self.e2pow24.get_descriptors_and_values(vector),
            ElementArrayKey::E32 => self.e2pow32.get_descriptors_and_values(vector),
        }
    }

    pub(crate) fn get_descriptors_and_values_mut(
        &mut self,
        vector: &impl ElementsIndexable,
    ) -> (
        Option<&mut AHashMap<u32, ElementDescriptor<'static>>>,
        &mut [Option<Value<'static>>],
    ) {
        match vector.cap() {
            ElementArrayKey::Empty => (None, &mut []),
            ElementArrayKey::E4 => self.e2pow4.get_descriptors_and_values_mut(vector),
            ElementArrayKey::E6 => self.e2pow6.get_descriptors_and_values_mut(vector),
            ElementArrayKey::E8 => self.e2pow8.get_descriptors_and_values_mut(vector),
            ElementArrayKey::E10 => self.e2pow10.get_descriptors_and_values_mut(vector),
            ElementArrayKey::E12 => self.e2pow12.get_descriptors_and_values_mut(vector),
            ElementArrayKey::E16 => self.e2pow16.get_descriptors_and_values_mut(vector),
            ElementArrayKey::E24 => self.e2pow24.get_descriptors_and_values_mut(vector),
            ElementArrayKey::E32 => self.e2pow32.get_descriptors_and_values_mut(vector),
        }
    }

    pub(crate) fn get_descriptor(
        &self,
        vector: &impl ElementsIndexable,
        index: usize,
    ) -> Option<ElementDescriptor> {
        let Ok(index) = u32::try_from(index) else {
            return None;
        };
        let descriptors = match vector.cap() {
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
            .get(&vector.elements_index())?
            .get(&index)
            .copied()
    }

    pub(crate) fn set_descriptor(
        &mut self,
        vector: &impl ElementsIndexable,
        index: usize,
        descriptor: Option<ElementDescriptor>,
    ) {
        let index: u32 = index.try_into().unwrap();
        assert!(index < vector.len());
        let descriptors = match vector.cap() {
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
        if let Some(inner_map) = descriptors.get_mut(&vector.elements_index()) {
            if let Some(descriptor) = descriptor {
                inner_map.insert(index, descriptor.unbind());
            } else {
                inner_map.remove(&index);
            }
        } else if let Some(descriptor) = descriptor {
            let mut inner_map = AHashMap::default();
            inner_map.insert(index, descriptor.unbind());
            descriptors.insert(vector.elements_index(), inner_map);
        }
    }

    pub(crate) fn has(&self, props: &PropertyStorageVector, key: PropertyKey) -> bool {
        match props.cap {
            ElementArrayKey::Empty => false,
            ElementArrayKey::E4 => self.k2pow4.has(props, key),
            ElementArrayKey::E6 => self.k2pow6.has(props, key),
            ElementArrayKey::E8 => self.k2pow8.has(props, key),
            ElementArrayKey::E10 => self.k2pow10.has(props, key),
            ElementArrayKey::E12 => self.k2pow12.has(props, key),
            ElementArrayKey::E16 => self.k2pow16.has(props, key),
            ElementArrayKey::E24 => self.k2pow24.has(props, key),
            ElementArrayKey::E32 => self.k2pow32.has(props, key),
        }
    }

    /// This method creates a "shallow clone" of the elements of a trivial/dense array.
    /// It does not do anything with descriptors and assumes there is a previous validation in place.
    pub(crate) fn shallow_clone<'a>(
        &mut self,
        elements_vector: ElementsVector<'a>,
    ) -> ElementsVector<'a> {
        let index = elements_vector.elements_index.into_index();
        let ElementArrays {
            e2pow4,
            e2pow6,
            e2pow8,
            e2pow10,
            e2pow12,
            e2pow16,
            e2pow24,
            e2pow32,
            ..
        } = self;
        let new_index = match elements_vector.cap {
            ElementArrayKey::Empty => ElementIndex::from_u32_index(0),
            ElementArrayKey::E4 => {
                let elements = e2pow4;
                elements.values.extend_from_within(index..index + 1);
                ElementIndex::last_element_index(&elements.values)
            }
            ElementArrayKey::E6 => {
                let elements = e2pow6;
                elements.values.extend_from_within(index..index + 1);
                ElementIndex::last_element_index(&elements.values)
            }
            ElementArrayKey::E8 => {
                let elements = e2pow8;
                elements.values.extend_from_within(index..index + 1);
                ElementIndex::last_element_index(&elements.values)
            }
            ElementArrayKey::E10 => {
                let elements = e2pow10;
                elements.values.extend_from_within(index..index + 1);
                ElementIndex::last_element_index(&elements.values)
            }
            ElementArrayKey::E12 => {
                let elements = e2pow12;
                elements.values.extend_from_within(index..index + 1);
                ElementIndex::last_element_index(&elements.values)
            }
            ElementArrayKey::E16 => {
                let elements = e2pow16;
                elements.values.extend_from_within(index..index + 1);
                ElementIndex::last_element_index(&elements.values)
            }
            ElementArrayKey::E24 => {
                let elements = e2pow24;
                elements.values.extend_from_within(index..index + 1);
                ElementIndex::last_element_index(&elements.values)
            }
            ElementArrayKey::E32 => {
                let elements = e2pow32;
                elements.values.extend_from_within(index..index + 1);
                ElementIndex::last_element_index(&elements.values)
            }
        };

        ElementsVector {
            cap: elements_vector.cap,
            elements_index: new_index,
            len: elements_vector.len(),
            len_writable: true,
        }
    }
}

impl HeapMarkAndSweep for ElementDescriptor<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            ElementDescriptor::WritableEnumerableConfigurableData
            | ElementDescriptor::WritableEnumerableUnconfigurableData
            | ElementDescriptor::WritableUnenumerableConfigurableData
            | ElementDescriptor::WritableUnenumerableUnconfigurableData
            | ElementDescriptor::ReadOnlyEnumerableConfigurableData
            | ElementDescriptor::ReadOnlyEnumerableUnconfigurableData
            | ElementDescriptor::ReadOnlyUnenumerableConfigurableData
            | ElementDescriptor::ReadOnlyUnenumerableUnconfigurableData => {}
            ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyEnumerableUnconfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyUnenumerableConfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyUnenumerableUnconfigurableAccessor { get } => {
                get.mark_values(queues)
            }
            ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyEnumerableUnconfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyUnenumerableConfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyUnenumerableUnconfigurableAccessor { set } => {
                set.mark_values(queues)
            }
            ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { get, set }
            | ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor { get, set }
            | ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor { get, set }
            | ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor { get, set } => {
                get.mark_values(queues);
                set.mark_values(queues);
            }
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            ElementDescriptor::WritableEnumerableConfigurableData
            | ElementDescriptor::WritableEnumerableUnconfigurableData
            | ElementDescriptor::WritableUnenumerableConfigurableData
            | ElementDescriptor::WritableUnenumerableUnconfigurableData
            | ElementDescriptor::ReadOnlyEnumerableConfigurableData
            | ElementDescriptor::ReadOnlyEnumerableUnconfigurableData
            | ElementDescriptor::ReadOnlyUnenumerableConfigurableData
            | ElementDescriptor::ReadOnlyUnenumerableUnconfigurableData => {}
            ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyEnumerableUnconfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyUnenumerableConfigurableAccessor { get }
            | ElementDescriptor::ReadOnlyUnenumerableUnconfigurableAccessor { get } => {
                get.sweep_values(compactions)
            }
            ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyEnumerableUnconfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyUnenumerableConfigurableAccessor { set }
            | ElementDescriptor::WriteOnlyUnenumerableUnconfigurableAccessor { set } => {
                set.sweep_values(compactions)
            }
            ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { get, set }
            | ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor { get, set }
            | ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor { get, set }
            | ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor { get, set } => {
                get.sweep_values(compactions);
                set.sweep_values(compactions);
            }
        }
    }
}

impl AsRef<ElementArrays> for Agent {
    fn as_ref(&self) -> &ElementArrays {
        &self.heap.elements
    }
}

impl AsMut<ElementArrays> for Agent {
    fn as_mut(&mut self) -> &mut ElementArrays {
        &mut self.heap.elements
    }
}

impl AsRef<ElementArrays> for ElementArrays {
    fn as_ref(&self) -> &ElementArrays {
        self
    }
}

impl AsMut<ElementArrays> for ElementArrays {
    fn as_mut(&mut self) -> &mut ElementArrays {
        self
    }
}

impl HeapMarkAndSweep for PropertyStorageVector<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            keys_index,
            values_index,
            cap,
            len,
            extensible: _,
        } = self;

        match cap {
            ElementArrayKey::Empty => {}
            ElementArrayKey::E4 => {
                queues.k_2_4.push((*keys_index, *len));
                queues.e_2_4.push((*values_index, *len));
            }
            ElementArrayKey::E6 => {
                queues.k_2_6.push((*keys_index, *len));
                queues.e_2_6.push((*values_index, *len));
            }
            ElementArrayKey::E8 => {
                queues.k_2_8.push((*keys_index, *len));
                queues.e_2_8.push((*values_index, *len));
            }
            ElementArrayKey::E10 => {
                queues.k_2_10.push((*keys_index, *len));
                queues.e_2_10.push((*values_index, *len));
            }
            ElementArrayKey::E12 => {
                queues.k_2_12.push((*keys_index, *len));
                queues.e_2_12.push((*values_index, *len));
            }
            ElementArrayKey::E16 => {
                queues.k_2_16.push((*keys_index, *len));
                queues.e_2_16.push((*values_index, *len));
            }
            ElementArrayKey::E24 => {
                queues.k_2_24.push((*keys_index, *len));
                queues.e_2_24.push((*values_index, *len));
            }
            ElementArrayKey::E32 => {
                queues.k_2_32.push((*keys_index, *len));
                queues.e_2_32.push((*values_index, *len));
            }
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            keys_index,
            values_index,
            cap,
            len: _,
            extensible: _,
        } = self;

        match cap {
            ElementArrayKey::Empty => {}
            ElementArrayKey::E4 => {
                compactions.k_2_4.shift_index(keys_index);
                compactions.e_2_4.shift_index(values_index);
            }
            ElementArrayKey::E6 => {
                compactions.k_2_6.shift_index(keys_index);
                compactions.e_2_6.shift_index(values_index);
            }
            ElementArrayKey::E8 => {
                compactions.k_2_8.shift_index(keys_index);
                compactions.e_2_8.shift_index(values_index);
            }
            ElementArrayKey::E10 => {
                compactions.k_2_10.shift_index(keys_index);
                compactions.e_2_10.shift_index(values_index);
            }
            ElementArrayKey::E12 => {
                compactions.k_2_12.shift_index(keys_index);
                compactions.e_2_12.shift_index(values_index);
            }
            ElementArrayKey::E16 => {
                compactions.k_2_16.shift_index(keys_index);
                compactions.e_2_16.shift_index(values_index);
            }
            ElementArrayKey::E24 => {
                compactions.k_2_24.shift_index(keys_index);
                compactions.e_2_24.shift_index(values_index);
            }
            ElementArrayKey::E32 => {
                compactions.k_2_32.shift_index(keys_index);
                compactions.e_2_32.shift_index(values_index);
            }
        }
    }
}
