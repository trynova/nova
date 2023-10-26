use crate::heap::{element_array::ElementsVector, indexes::ObjectIndex};

/// An Array is an exotic object that gives special treatment to array index property keys (see 6.1.7).
/// A property whose property name is an array index is also called an element. Every Array has a
/// non-configurable "**length**" property whose value is always a non-negative integral Number whose
/// mathematical value is strictly less than 2**32.
#[derive(Debug, Clone, Copy)]
pub struct ArrayHeapData {
    pub object_index: Option<ObjectIndex>,
    // TODO: Use enum { ElementsVector, SmallVec<[Value; 3]> }
    // to get some inline benefit together with a 32 byte size
    // for ArrayHeapData to fit two in one cache line.
    pub elements: ElementsVector,
}
