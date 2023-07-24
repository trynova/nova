use super::heap_trace::HeapTrace;
use crate::{
    heap::{Heap, HeapBits},
    types::Value,
};

#[derive(Debug, Clone)]
pub struct ArrayHeapData {
    pub(super) bits: HeapBits,
    pub(super) object_index: u32,
    // TODO: Use SmallVec<[Value; 4]>
    pub(super) elements: Vec<Option<Value>>,
}

impl HeapTrace for Option<ArrayHeapData> {
    fn trace(&self, heap: &Heap) {
        assert!(self.is_some());
        heap.objects[self.as_ref().unwrap().object_index as usize].trace(heap);
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
