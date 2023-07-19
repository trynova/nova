use crate::{
    heap::{
        heap_constants::{
            FUNCTION_CONSTRUCTOR_INDEX, OBJECT_CONSTRUCTOR_INDEX, OBJECT_PROTOTYPE_INDEX,
        },
        FunctionHeapData, Heap, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::Value,
};

pub fn initialize_object_heap(heap: &mut Heap) {
    let object_constructor_object = ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(FUNCTION_CONSTRUCTOR_INDEX),
        Vec::with_capacity(24),
    );
    debug_assert!(heap.objects.len() as u32 == OBJECT_CONSTRUCTOR_INDEX);
    heap.objects.push(Some(object_constructor_object));
    heap.functions.push(Some(FunctionHeapData {
        bits: HeapBits::new(),
        object_index: OBJECT_CONSTRUCTOR_INDEX,
        length: 1,
        uses_arguments: false,
        bound: None,
        visible: None,
        binding: object_constructor_binding,
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
    debug_assert!(heap.objects.len() as u32 == OBJECT_PROTOTYPE_INDEX);
    heap.objects.push(Some(object_prototype_object));
}

fn object_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> Value {
    Value::Object(0)
}

pub fn populate_object_heap(heap: &mut Heap) {}

// ObjectHeapData::new(
//     true,
//     PropertyDescriptor::Data {
//         value: crate::value::Value::Null,
//         writable: false,
//         enumerable: false,
//         configurable: false,
//     },
//     vec![
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("constructor")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("assign")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("create")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("defineProperties")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("defineProperty")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("defineProperties")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("entries")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("defineProperties")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("freeze")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("defineProperties")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("fromEntries")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("getOwnPropertyDescriptor")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("getOwnPropertyDescriptors")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("getOwnPropertyNames")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("getOwnPropertySymbols")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("getPrototypeOf")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("is")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("isExtensible")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("isFrozen")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("setPrototypeOf")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("isExtensible")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("preventExtensions")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("getOwnProperty")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("hasOwn")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("hasProperty")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//         ObjectEntry::new(
//             PropertyKey::String(heap.alloc_string("ownPropertyKeys")),
//             PropertyDescriptor::Data {
//                 value: Value::Function(0),
//                 writable: true,
//                 enumerable: false,
//                 configurable: true,
//             },
//         ),
//     ],
// )
