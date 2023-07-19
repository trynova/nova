use crate::{
    heap::{
        heap_constants::{
            FUNCTION_PROTOTYPE_INDEX, OBJECT_PROTOTYPE_INDEX, STRING_CONSTRUCTOR_INDEX,
            STRING_PROTOTYPE_INDEX,
        },
        FunctionHeapData, Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

pub fn initialize_string_heap(heap: &mut Heap) {
    let string_constructor_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(FUNCTION_PROTOTYPE_INDEX),
        Vec::with_capacity(24),
    );
    debug_assert!(heap.objects.len() as u32 == STRING_CONSTRUCTOR_INDEX);
    heap.objects.push(Some(string_constructor_object));
    heap.functions.push(Some(FunctionHeapData {
        bits: HeapBits::new(),
        object_index: STRING_CONSTRUCTOR_INDEX,
        length: 1,
        uses_arguments: false,
        bound: None,
        visible: None,
        binding: string_constructor_binding,
    }));
    let string_prototype_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(OBJECT_PROTOTYPE_INDEX),
        Vec::with_capacity(7),
    );
    debug_assert!(heap.objects.len() as u32 == STRING_PROTOTYPE_INDEX);
    heap.objects.push(Some(string_prototype_object));
}

fn string_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::EmptyString
}
