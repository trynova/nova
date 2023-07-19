use crate::{
    heap::{
        heap_constants::{
            FUNCTION_PROTOTYPE_INDEX, OBJECT_PROTOTYPE_INDEX, SYMBOL_CONSTRUCTOR_INDEX,
            SYMBOL_PROTOTYPE_INDEX,
        },
        FunctionHeapData, Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

pub fn initialize_symbol_heap(heap: &mut Heap) {
    let symbol_constructor_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(FUNCTION_PROTOTYPE_INDEX),
        Vec::with_capacity(24),
    );
    debug_assert!(heap.objects.len() as u32 == SYMBOL_CONSTRUCTOR_INDEX);
    heap.objects.push(Some(symbol_constructor_object));
    heap.functions.push(Some(FunctionHeapData {
        bits: HeapBits::new(),
        object_index: SYMBOL_CONSTRUCTOR_INDEX,
        length: 1,
        uses_arguments: false,
        bound: None,
        visible: None,
        binding: symbol_constructor_binding,
    }));
    let symbol_prototype_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(OBJECT_PROTOTYPE_INDEX),
        Vec::with_capacity(7),
    );
    debug_assert!(heap.objects.len() as u32 == SYMBOL_PROTOTYPE_INDEX);
    heap.objects.push(Some(symbol_prototype_object));
}

fn symbol_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::Symbol(0)
}
