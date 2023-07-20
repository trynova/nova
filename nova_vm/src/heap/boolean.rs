use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

use super::Heap;

pub fn initialize_boolean_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::BooleanConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            // TODO: Methods and properties
            Vec::with_capacity(1),
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::BooleanConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: BuiltinObjectIndexes::BooleanConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: boolean_constructor_binding,
        });
    heap.objects[BuiltinObjectIndexes::BooleanPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        // TODO: Methods and properties
        Vec::with_capacity(7),
    ));
}

fn boolean_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::Boolean(false)
}
