use super::heap_trace::HeapTrace;
use crate::heap::{Heap, HeapBits};
use num_bigint_dig::BigInt;

#[derive(Debug, Clone)]
pub struct BigIntHeapData {
    pub(super) bits: HeapBits,
    pub(super) data: BigInt,
}

impl HeapTrace for Option<BigIntHeapData> {
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
