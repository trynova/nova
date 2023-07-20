use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

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
