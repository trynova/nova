use crate::heap::{HeapBits, ObjectHeapData, PropertyDescriptor};

pub fn create_string_prototype() -> ObjectHeapData {
    ObjectHeapData::new(
        HeapBits::new(),
        true,
        PropertyDescriptor::Data {
            value: crate::value::Value::Object(0),
            writable: false,
            enumerable: false,
            configurable: false,
        },
        vec![],
    )
}
