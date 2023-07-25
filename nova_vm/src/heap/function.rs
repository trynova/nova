use super::{heap_trace::HeapTrace, Handle, HeapBits, ObjectHeapData};
use crate::{builtins::JsFunction, Heap};

#[derive(Debug, Clone)]
pub struct FunctionHeapData {
    pub(crate) bits: HeapBits,
    pub(crate) object: Handle<ObjectHeapData>,
    pub(crate) length: i64,
    pub(crate) binding: JsFunction,
    // TODO: Should we create a `BoundFunctionHeapData` for the exotic object?
    // pub(super) uses_arguments: bool,
    // pub(super) bound: Option<Box<[Value]>>,
    // pub(super) visible: Option<Vec<Value>>,
}

impl HeapTrace for Option<FunctionHeapData> {
    fn trace(&self, heap: &Heap) {
        assert!(self.is_some());
        heap.objects[self.as_ref().unwrap().object.id as usize].trace(heap);
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
