use super::{heap_trace::HeapTrace, Handle, ObjectHeapData};
use crate::{
    heap::{GetHeapData, Heap, HeapBits},
    types::{Object, Value},
};

#[derive(Debug, Clone)]
pub struct ArrayHeapData {
    pub(crate) bits: HeapBits,
    pub(crate) object: Option<Object>,
    // TODO: Use SmallVec<[Value; 4]>
    pub(crate) elements: Vec<Option<Value>>,
}

impl ArrayHeapData {
    pub fn dummy() -> Self {
        Self {
            bits: HeapBits::new(),
            object: None,
            elements: Vec::new(),
        }
    }
}

impl HeapTrace for Option<ArrayHeapData> {
    fn trace(&self, heap: &Heap) {
        assert!(self.is_some());

        let data = self.as_ref().unwrap();

        if let Some(object) = data.object {
            object.into_value().trace(heap);
        }

        for value in data.elements.iter() {
            if let Some(value) = value {
                value.trace(heap);
            }
        }
    }
    fn root(&self, _heap: &Heap) {
        assert!(self.is_some());
        self.as_ref().unwrap().bits.root();
    }

    fn unroot(&self, _heap: &Heap) {
        assert!(self.is_some());
        self.as_ref().unwrap().bits.unroot();
    }

    fn finalize(&mut self, _heap: &Heap) {
        self.take();
    }
}
