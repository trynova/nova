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
        if let Some(object) = self.as_ref().unwrap().object {
            match object.into_value() {
                Value::Object(object) => {
                    let object = heap.get(object);

                    if let Some(value) = object.prototype.value {}
                }
                Value::ArrayObject(array) => {
                    let array = heap.get(array);

                    if let Some(object) = array.object {
						object.internal_methods(agent).
                    }
                }
                _ => unreachable!(),
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
