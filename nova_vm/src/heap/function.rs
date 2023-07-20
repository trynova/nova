use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

use super::heap_trace::HeapTrace;

pub type JsBindingFunction = fn(heap: &mut Heap, this: Value, args: &[Value]) -> Value;

pub(crate) struct FunctionHeapData {
    pub(super) bits: HeapBits,
    pub(super) object_index: u32,
    pub(super) length: u8,
    pub(super) uses_arguments: bool,
    pub(super) bound: Option<Box<[Value]>>,
    pub(super) visible: Option<Vec<Value>>,
    pub(super) binding: JsBindingFunction,
}

impl HeapTrace for Option<FunctionHeapData> {
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

pub fn initialize_function_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::FunctionConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            // TODO: Methods and properties
            Vec::with_capacity(0),
        ));
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::FunctionConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: BuiltinObjectIndexes::FunctionConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: function_constructor_binding,
        });
    heap.objects[BuiltinObjectIndexes::FunctionPrototypeIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
            // TODO: Methods and properties
            Vec::with_capacity(0),
        ));
}

fn function_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::Function(0)
}
