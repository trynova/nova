use super::{heap_trace::HeapTrace, Handle, HeapBits, ObjectHeapData};
use crate::{
    builtins::{todo_builtin, Behaviour},
    types::{Object, Value},
    Heap,
};

#[derive(Debug, Clone)]
pub struct FunctionHeapData {
    pub(crate) bits: HeapBits,
    pub(crate) object: Option<Object>,
    pub(crate) initial_name: Value,
    pub(crate) length: i64,
    pub(crate) behaviour: Behaviour,
    // TODO: Should we create a `BoundFunctionHeapData` for an exotic object
    //       that allows setting fields and other deoptimizations?
    // pub(super) uses_arguments: bool,
    // pub(super) bound: Option<Box<[Value]>>,
    // pub(super) visible: Option<Vec<Value>>,
}

impl FunctionHeapData {
    pub fn dummy() -> Self {
        Self {
            bits: HeapBits::new(),
            object: None,
            initial_name: Value::Null,
            length: 0,
            behaviour: Behaviour::Regular(todo_builtin),
        }
    }
}

impl HeapTrace for Option<FunctionHeapData> {
    fn trace(&self, heap: &Heap) {
        assert!(self.is_some());

        if let Some(object) = self.as_ref().unwrap().object {
            object.into_value().trace(heap);
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
