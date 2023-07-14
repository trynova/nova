use crate::heap::{HeapBits, ObjectHeapData, PropertyDescriptor};

pub fn create_number_prototype() -> ObjectHeapData {
    ObjectHeapData::new(
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
