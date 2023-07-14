use crate::heap::{Heap, HeapBits, ObjectHeapData, PropertyDescriptor};
use crate::Value;

pub fn create_function_prototype(heap: &mut Heap) -> ObjectHeapData {
    ObjectHeapData::new(
        true,
        PropertyDescriptor::Data {
            value: Value::Object(0),
            writable: false,
            enumerable: false,
            configurable: false,
        },
        vec![],
    )
}
