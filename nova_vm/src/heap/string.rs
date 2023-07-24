use crate::heap::{heap_trace::HeapTrace, Heap, HeapBits};
use wtf8::Wtf8Buf;

#[derive(Debug, Clone)]
pub struct StringHeapData {
    pub(crate) bits: HeapBits,
    pub(crate) data: Wtf8Buf,
}

impl StringHeapData {
    pub fn from_str(str: &str) -> Self {
        StringHeapData {
            bits: HeapBits::new(),
            data: Wtf8Buf::from_str(str),
        }
    }
}

impl HeapTrace for Option<StringHeapData> {
    fn trace(&self, _heap: &Heap) {}

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
