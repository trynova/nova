use super::{heap_trace::HeapTrace, Handle, ObjectHeapData};
use crate::{
    heap::{Heap, HeapBits},
    types::{Object, Value},
};

#[derive(Debug, Clone)]
pub struct ArrayHeapData {
    pub(super) bits: HeapBits,
    pub(super) object: Option<Handle<ObjectHeapData>>,
    // TODO: Use SmallVec<[Value; 4]>
    pub(super) elements: Vec<Option<Value>>,
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
        if let Some(object) = self.as_ref().unwrap().object {
            heap.objects[object.id.get() as usize].trace(heap);
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
