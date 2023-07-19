use crate::{
    heap::{
        heap_constants::{
            BOOLEAN_CONSTRUCTOR_INDEX, BOOLEAN_PROTOTYPE_INDEX, OBJECT_PROTOTYPE_INDEX,
        },
        FunctionHeapData, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

use super::Heap;

pub fn initialize_boolean_heap(heap: &mut Heap) {
    let boolean_constructor_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(BOOLEAN_PROTOTYPE_INDEX),
        Vec::with_capacity(1),
    );
    debug_assert!(heap.objects.len() as u32 == BOOLEAN_CONSTRUCTOR_INDEX);
    heap.objects.push(Some(boolean_constructor_object));
    heap.functions.push(Some(FunctionHeapData {
        bits: HeapBits::new(),
        object_index: BOOLEAN_CONSTRUCTOR_INDEX,
        length: 1,
        uses_arguments: false,
        bound: None,
        visible: None,
        binding: boolean_constructor_binding,
    }));
    let boolean_prototype_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(OBJECT_PROTOTYPE_INDEX),
        Vec::with_capacity(7),
    );
    debug_assert!(heap.objects.len() as u32 == BOOLEAN_PROTOTYPE_INDEX);
    heap.objects.push(Some(boolean_prototype_object));
}

fn boolean_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::Boolean(false)
}
