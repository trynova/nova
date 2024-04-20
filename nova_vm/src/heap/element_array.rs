use super::{
    indexes::ElementIndex,
    object_entry::{ObjectEntry, ObjectEntryPropertyDescriptor},
};
use crate::ecmascript::types::{Function, PropertyKey, Value};
use core::panic;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum ElementArrayKey {
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
        } else if value < usize::pow(2, 32) {
            ElementArrayKey::E32
        } else {
            panic!("Elements array length over 2 ** 32 - 1");
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ElementsVector {
    pub(crate) elements_index: ElementIndex,
    pub(crate) cap: ElementArrayKey,
    pub(crate) len: u32,
}

impl ElementsVector {
    pub fn cap(&self) -> u32 {
        match self.cap {
            ElementArrayKey::Empty => 0,
            ElementArrayKey::E4 => 2u32.pow(4),
            ElementArrayKey::E6 => 2u32.pow(6),
            ElementArrayKey::E8 => 2u32.pow(8),
            ElementArrayKey::E10 => 2u32.pow(10),
            ElementArrayKey::E12 => 2u32.pow(12),
            ElementArrayKey::E16 => 2u32.pow(16),
            ElementArrayKey::E24 => 2u32.pow(24),
            ElementArrayKey::E32 => 2u32.pow(32),
        }
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

    fn grow_inner(&mut self, elements: &mut ElementArrays) {
        let next_key = match self.cap {
            ElementArrayKey::Empty => ElementArrayKey::E4,
            ElementArrayKey::E4 => ElementArrayKey::E6,
            ElementArrayKey::E6 => ElementArrayKey::E8,
            ElementArrayKey::E8 => ElementArrayKey::E10,
            ElementArrayKey::E10 => ElementArrayKey::E12,
            ElementArrayKey::E12 => ElementArrayKey::E16,
            ElementArrayKey::E16 => ElementArrayKey::E24,
            ElementArrayKey::E24 => ElementArrayKey::E32,
            ElementArrayKey::E32 => panic!("Attempted to grow a full array"),
        };
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
        let next_index = elements.push_with_key(next_key, &values, descriptors);
        self.cap = next_key;
        self.elements_index = next_index;
    }

    pub fn push(
        &mut self,
        elements: &mut ElementArrays,
        value: Option<Value>,
        descriptor: Option<ElementDescriptor>,
    ) {
        if self.is_full() {
            self.grow_inner(elements);
        }
        let next_over_end = match self.cap {
            ElementArrayKey::Empty => unreachable!(),
            ElementArrayKey::E4 => elements
                .e2pow4
                .values
                .get_mut(self.elements_index.into_index())
                .expect("Invalid ElementsVector: No item at index")
                .as_mut()
                .expect("Invalid ElementsVector: Found None at index")
                .get_mut(self.len as usize)
                .expect("Invalid ElementsVector: Length points beyond vector bounds"),
            ElementArrayKey::E6 => elements
                .e2pow6
                .values
                .get_mut(self.elements_index.into_index())
                .expect("Invalid ElementsVector: No item at index")
                .as_mut()
                .expect("Invalid ElementsVector: Found None at index")
                .get_mut(self.len as usize)
                .expect("Invalid ElementsVector: Length points beyond vector bounds"),
            ElementArrayKey::E8 => elements
                .e2pow8
                .values
                .get_mut(self.elements_index.into_index())
                .expect("Invalid ElementsVector: No item at index")
                .as_mut()
                .expect("Invalid ElementsVector: Found None at index")
                .get_mut(self.len as usize)
                .expect("Invalid ElementsVector: Length points beyond vector bounds"),
            ElementArrayKey::E10 => elements
                .e2pow10
                .values
                .get_mut(self.elements_index.into_index())
                .expect("Invalid ElementsVector: No item at index")
                .as_mut()
                .expect("Invalid ElementsVector: Found None at index")
                .get_mut(self.len as usize)
                .expect("Invalid ElementsVector: Length points beyond vector bounds"),
            ElementArrayKey::E12 => elements
                .e2pow12
                .values
                .get_mut(self.elements_index.into_index())
                .expect("Invalid ElementsVector: No item at index")
                .as_mut()
                .expect("Invalid ElementsVector: Found None at index")
                .get_mut(self.len as usize)
                .expect("Invalid ElementsVector: Length points beyond vector bounds"),
            ElementArrayKey::E16 => elements
                .e2pow16
                .values
                .get_mut(self.elements_index.into_index())
                .expect("Invalid ElementsVector: No item at index")
                .as_mut()
                .expect("Invalid ElementsVector: Found None at index")
                .get_mut(self.len as usize)
                .expect("Invalid ElementsVector: Length points beyond vector bounds"),
            ElementArrayKey::E24 => elements
                .e2pow24
                .values
                .get_mut(self.elements_index.into_index())
                .expect("Invalid ElementsVector: No item at index")
                .as_mut()
                .expect("Invalid ElementsVector: Found None at index")
                .get_mut(self.len as usize)
                .expect("Invalid ElementsVector: Length points beyond vector bounds"),
            ElementArrayKey::E32 => elements
                .e2pow32
                .values
                .get_mut(self.elements_index.into_index())
                .expect("Invalid ElementsVector: No item at index")
                .as_mut()
                .expect("Invalid ElementsVector: Found None at index")
                .get_mut(self.len as usize)
                .expect("Invalid ElementsVector: Length points beyond vector bounds"),
        };
        *next_over_end = value;
        if let Some(_descriptor) = descriptor {
            todo!("Descriptors");
        }
        self.len += 1;
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
        desc: ObjectEntryPropertyDescriptor,
    ) -> (Option<ElementDescriptor>, Option<Value>) {
        match desc {
            ObjectEntryPropertyDescriptor::Data {
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
            ObjectEntryPropertyDescriptor::Blocked { .. } => unreachable!(),
            ObjectEntryPropertyDescriptor::ReadOnly {
                get,
                enumerable,
                configurable,
            } => match (enumerable, configurable) {
                (true, true) => (
                    Some(ElementDescriptor::ReadOnlyEnumerableConfigurableAccessor { get }),
                    None,
                ),
                (true, false) => (
                    Some(ElementDescriptor::ReadOnlyEnumerableUnconfigurableAccessor { get }),
                    None,
                ),
                (false, true) => (
                    Some(ElementDescriptor::ReadOnlyUnenumerableConfigurableAccessor { get }),
                    None,
                ),
                (false, false) => (
                    Some(ElementDescriptor::ReadOnlyUnenumerableUnconfigurableAccessor { get }),
                    None,
                ),
            },
            ObjectEntryPropertyDescriptor::WriteOnly {
                set,
                enumerable,
                configurable,
            } => match (enumerable, configurable) {
                (true, true) => (
                    Some(ElementDescriptor::WriteOnlyEnumerableConfigurableAccessor { set }),
                    None,
                ),
                (true, false) => (
                    Some(ElementDescriptor::WriteOnlyEnumerableUnconfigurableAccessor { set }),
                    None,
                ),
                (false, true) => (
                    Some(ElementDescriptor::WriteOnlyUnenumerableConfigurableAccessor { set }),
                    None,
                ),
                (false, false) => (
                    Some(ElementDescriptor::WriteOnlyUnenumerableUnconfigurableAccessor { set }),
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
                    Some(ElementDescriptor::ReadWriteEnumerableConfigurableAccessor { get, set }),
                    None,
                ),
                (true, false) => (
                    Some(ElementDescriptor::ReadWriteEnumerableUnconfigurableAccessor { get, set }),
                    None,
                ),
                (false, true) => (
                    Some(ElementDescriptor::ReadWriteUnenumerableConfigurableAccessor { get, set }),
                    None,
                ),
                (false, false) => (
                    Some(
                        ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor { get, set },
                    ),
                    None,
                ),
            },
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

    pub fn get(&self, vector: ElementsVector) -> &[Option<Value>] {
        match vector.cap {
            ElementArrayKey::Empty => &[],
            ElementArrayKey::E4 => &self
                .e2pow4
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .as_ref()
                .unwrap()
                .as_slice()[0..vector.len as usize],
            ElementArrayKey::E6 => &self
                .e2pow6
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .as_ref()
                .unwrap()
                .as_slice()[0..vector.len as usize],
            ElementArrayKey::E8 => &self
                .e2pow8
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .as_ref()
                .unwrap()
                .as_slice()[0..vector.len as usize],
            ElementArrayKey::E10 => &self
                .e2pow10
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .as_ref()
                .unwrap()
                .as_slice()[0..vector.len as usize],
            ElementArrayKey::E12 => &self
                .e2pow12
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .as_ref()
                .unwrap()
                .as_slice()[0..vector.len as usize],
            ElementArrayKey::E16 => &self
                .e2pow16
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .as_ref()
                .unwrap()
                .as_slice()[0..vector.len as usize],
            ElementArrayKey::E24 => &self
                .e2pow24
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .as_ref()
                .unwrap()
                .as_slice()[0..vector.len as usize],
            ElementArrayKey::E32 => &self
                .e2pow32
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .as_ref()
                .unwrap()
                .as_slice()[0..vector.len as usize],
        }
    }

    pub fn get_mut(&mut self, vector: ElementsVector) -> &mut [Option<Value>] {
        match vector.cap {
            ElementArrayKey::Empty => &mut [],
            ElementArrayKey::E4 => &mut self
                .e2pow4
                .values
                .get_mut(vector.elements_index.into_index())
                .unwrap()
                .as_mut()
                .unwrap()
                .as_mut_slice()[0..vector.len as usize],
            ElementArrayKey::E6 => &mut self
                .e2pow6
                .values
                .get_mut(vector.elements_index.into_index())
                .unwrap()
                .as_mut()
                .unwrap()
                .as_mut_slice()[0..vector.len as usize],
            ElementArrayKey::E8 => &mut self
                .e2pow8
                .values
                .get_mut(vector.elements_index.into_index())
                .unwrap()
                .as_mut()
                .unwrap()
                .as_mut_slice()[0..vector.len as usize],
            ElementArrayKey::E10 => &mut self
                .e2pow10
                .values
                .get_mut(vector.elements_index.into_index())
                .unwrap()
                .as_mut()
                .unwrap()
                .as_mut_slice()[0..vector.len as usize],
            ElementArrayKey::E12 => &mut self
                .e2pow12
                .values
                .get_mut(vector.elements_index.into_index())
                .unwrap()
                .as_mut()
                .unwrap()
                .as_mut_slice()[0..vector.len as usize],
            ElementArrayKey::E16 => &mut self
                .e2pow16
                .values
                .get_mut(vector.elements_index.into_index())
                .unwrap()
                .as_mut()
                .unwrap()
                .as_mut_slice()[0..vector.len as usize],
            ElementArrayKey::E24 => &mut self
                .e2pow24
                .values
                .get_mut(vector.elements_index.into_index())
                .unwrap()
                .as_mut()
                .unwrap()
                .as_mut_slice()[0..vector.len as usize],
            ElementArrayKey::E32 => &mut self
                .e2pow32
                .values
                .get_mut(vector.elements_index.into_index())
                .unwrap()
                .as_mut()
                .unwrap()
                .as_mut_slice()[0..vector.len as usize],
        }
    }

    pub fn has(&self, vector: ElementsVector, element: Value) -> bool {
        match vector.cap {
            ElementArrayKey::Empty => false,
            ElementArrayKey::E4 => self
                .e2pow4
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .unwrap()
                .as_slice()[0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E6 => self
                .e2pow6
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .unwrap()
                .as_slice()[0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E8 => self
                .e2pow8
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .unwrap()
                .as_slice()[0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E10 => self
                .e2pow10
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .unwrap()
                .as_slice()[0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E12 => self
                .e2pow12
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .unwrap()
                .as_slice()[0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E16 => self
                .e2pow16
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .unwrap()
                .as_slice()[0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E24 => self
                .e2pow24
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .unwrap()
                .as_slice()[0..vector.len as usize]
                .contains(&Some(element)),
            ElementArrayKey::E32 => self
                .e2pow32
                .values
                .get(vector.elements_index.into_index())
                .unwrap()
                .unwrap()
                .as_slice()[0..vector.len as usize]
                .contains(&Some(element)),
        }
    }
}
