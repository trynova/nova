use crate::{
    heap::{Heap, HeapBits, ObjectEntry, ObjectHeapData, PropertyDescriptor, PropertyKey},
    value::Value,
};

pub fn create_object_prototype(heap: &mut Heap) -> ObjectHeapData {
    ObjectHeapData::new(
        HeapBits::new(),
        true,
        PropertyDescriptor::Data {
            value: crate::value::Value::Null,
            writable: false,
            enumerable: false,
            configurable: false,
        },
        vec![
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("constructor")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("assign")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("create")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("defineProperties")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("defineProperty")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("defineProperties")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("entries")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("defineProperties")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("freeze")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("defineProperties")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("fromEntries")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("getOwnPropertyDescriptor")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("getOwnPropertyDescriptors")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("getOwnPropertyNames")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("getOwnPropertySymbols")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("getPrototypeOf")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("is")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("isExtensible")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("isFrozen")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("setPrototypeOf")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("isExtensible")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("preventExtensions")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("getOwnProperty")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("hasOwn")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("hasProperty")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
            ObjectEntry::new(
                PropertyKey::String(heap.alloc_string("ownPropertyKeys")),
                PropertyDescriptor::Data {
                    value: Value::Function(0),
                    writable: true,
                    enumerable: false,
                    configurable: true,
                },
            ),
        ],
    )
}
