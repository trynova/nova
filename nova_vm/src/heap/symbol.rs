use super::{Handle, StringHeapData};
use crate::heap::{heap_trace::HeapTrace, Heap, HeapBits};

#[derive(Debug, Clone)]
pub struct SymbolHeapData {
    pub(super) bits: HeapBits,
    pub(super) descriptor: Option<Handle<StringHeapData>>,
}

impl SymbolHeapData {
    pub fn dummy() -> Self {
        Self {
            bits: HeapBits::new(),
            descriptor: None,
        }
    }
}

impl HeapTrace for Option<SymbolHeapData> {
    fn trace(&self, heap: &Heap) {
        assert!(self.is_some());
        if let Some(handle) = self.as_ref().unwrap().descriptor {
            heap.strings[handle.id.get() as usize].trace(heap);
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
