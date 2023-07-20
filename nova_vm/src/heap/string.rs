use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

pub fn initialize_string_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::StringConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            // TODO: Methods and properties
            Vec::with_capacity(0),
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::StringConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: BuiltinObjectIndexes::StringConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: string_constructor_binding,
        });
    heap.objects[BuiltinObjectIndexes::StringPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        // TODO: Methods and properties
        Vec::with_capacity(0),
    ));
}

fn string_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::EmptyString
}
