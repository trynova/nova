use super::Heap;
use crate::{
    heap::{
        heap_constants::{
            FUNCTION_PROTOTYPE_INDEX, NUMBER_CONSTRUCTOR_INDEX, NUMBER_PROTOTYPE_INDEX,
            OBJECT_PROTOTYPE_INDEX,
        },
        FunctionHeapData, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

pub fn initialize_number_heap(heap: &mut Heap) {
    let number_constructor_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(FUNCTION_PROTOTYPE_INDEX),
        Vec::with_capacity(24),
    );
    debug_assert!(heap.objects.len() as u32 == NUMBER_CONSTRUCTOR_INDEX);
    heap.objects.push(Some(number_constructor_object));
    heap.functions.push(Some(FunctionHeapData {
        bits: HeapBits::new(),
        object_index: NUMBER_CONSTRUCTOR_INDEX,
        length: 1,
        uses_arguments: false,
        bound: None,
        visible: None,
        binding: number_constructor_binding,
    }));
    let number_prototype_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(OBJECT_PROTOTYPE_INDEX),
        Vec::with_capacity(7),
    );
    debug_assert!(heap.objects.len() as u32 == NUMBER_PROTOTYPE_INDEX);
    heap.objects.push(Some(number_prototype_object));
}

fn number_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::SmiU(0)
}
