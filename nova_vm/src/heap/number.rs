use super::{heap_trace::HeapTrace, Heap};
use crate::heap::HeapBits;

#[derive(Debug, Clone)]
pub struct NumberHeapData {
    pub(super) bits: HeapBits,
    pub(super) data: f64,
}

impl NumberHeapData {
    pub(super) fn new(data: f64) -> NumberHeapData {
        NumberHeapData {
            bits: HeapBits::new(),
            data,
        }
    }

    pub(crate) fn value(&self) -> f64 {
        self.data
    }
}

impl HeapTrace for Option<NumberHeapData> {
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
