use super::Heap;
use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

pub fn initialize_number_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::NumberConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            Vec::with_capacity(24),
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::NumberConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: BuiltinObjectIndexes::NumberConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: number_constructor_binding,
        });
    heap.objects[BuiltinObjectIndexes::NumberPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        Vec::with_capacity(7),
    ));
}

fn number_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::SmiU(0)
}
