use crate::{
    heap::{HeapBits, ObjectHeapData, PropertyDescriptor},
    heap_constants::{STRING_CONSTRUCTOR_INDEX, STRING_PROTOTYPE_INDEX},
};

pub fn initiate_string_heap() {
    let string_constructor_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(STRING_PROTOTYPE_INDEX),
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
        binding: todo!(),
    }));
    let object_prototype_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::Data {
            value: Value::Null,
            writable: false,
            enumerable: false,
            configurable: false,
        },
        Vec::with_capacity(7),
    );
    debug_assert!(heap.objects.len() as u32 == STRING_PROTOTYPE_INDEX);
    heap.objects.push(Some(object_prototype_object));
}
