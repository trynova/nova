use crate::heap::{
    element_array::{ElementArrayKey, ElementsVector},
    indexes::{ElementIndex, ObjectIndex},
};

#[derive(Debug, Clone, Copy)]
pub struct SealableElementsVector {
    pub(crate) elements_index: ElementIndex,
    pub(crate) cap: ElementArrayKey,
    pub(crate) len: u32,
    /// Array length property can be set to unwritable
    pub(crate) len_writable: bool,
}

impl SealableElementsVector {
    pub(crate) fn len(&self) -> u32 {
        self.len
    }

    pub(crate) fn writable(&self) -> bool {
        self.len_writable
    }

    pub(crate) fn from_elements_vector(elements: ElementsVector) -> Self {
        Self {
            elements_index: elements.elements_index,
            cap: elements.cap,
            len: elements.len,
            len_writable: true,
        }
    }
}

impl From<SealableElementsVector> for ElementsVector {
    #[inline(always)]
    fn from(value: SealableElementsVector) -> Self {
        Self {
            elements_index: value.elements_index,
            cap: value.cap,
            len: value.len,
        }
    }
}

/// An Array is an exotic object that gives special treatment to array index
/// property keys (see 6.1.7). A property whose property name is an array index
/// is also called an element. Every Array has a non-configurable "**length**"
/// property whose value is always a non-negative integral Number whose
/// mathematical value is strictly less than 2**32.
#[derive(Debug, Clone, Copy)]
pub struct ArrayHeapData {
    pub object_index: Option<ObjectIndex>,
    // TODO: Use enum { ElementsVector, SmallVec<[Value; 3]> }
    // to get some inline benefit together with a 32 byte size
    // for ArrayHeapData to fit two in one cache line.
    pub elements: SealableElementsVector,
}
