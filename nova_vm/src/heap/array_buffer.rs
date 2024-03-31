use crate::{
    ecmascript::{
        builtins::ArgumentsList,
        execution::{Agent, JsResult},
        types::{Object, Value},
    },
    heap::Heap,
};

pub fn initialize_array_buffer_heap(_heap: &mut Heap) {
    // let species_function_name = Value::from_str(heap, "get [Symbol.species]");
    // let byte_length_key = Value::from_str(heap, "get byteLength");
    // let entries = vec![
    //     ObjectEntry::new_prototype_function_entry(heap, "isView", 1, false),
    //     ObjectEntry::new_constructor_prototype_entry(
    //         heap,
    //         IntrinsicObjectIndexes::ArrayBufferPrototype.into(),
    //     ),
    //     ObjectEntry::new(
    //         PropertyKey::Symbol(WellKnownSymbolIndexes::Species.into()),
    //         ObjectEntryPropertyDescriptor::ReadOnly {
    //             get: Function::BuiltinFunction(heap.create_function(
    //                 species_function_name,
    //                 0,
    //                 false,
    //             )),
    //             enumerable: false,
    //             configurable: true,
    //         },
    //     ),
    // ];
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::ArrayBufferConstructor,
    //     true,
    //     Some(Object::BuiltinFunction(
    //         IntrinsicObjectIndexes::FunctionPrototype.into(),
    //     )),
    //     entries,
    // );
    // heap.builtin_functions
    //     [get_constructor_index(IntrinsicObjectIndexes::ArrayBufferConstructor).into_index()] =
    //     Some(BuiltinFunctionHeapData {
    //         object_index: Some(IntrinsicObjectIndexes::ArrayBufferConstructor.into()),
    //         length: 1,
    //         initial_name: None,
    //         behaviour: Behaviour::Constructor(constructor_binding),
    //         realm: RealmIdentifier::from_index(0),
    //     });
    // let entries = vec![
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "byteLength"),
    //         ObjectEntryPropertyDescriptor::ReadOnly {
    //             get: Function::BuiltinFunction(heap.create_function(byte_length_key, 0, false)),
    //             enumerable: false,
    //             configurable: true,
    //         },
    //     ),
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "constructor"),
    //         ObjectEntryPropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
    //             IntrinsicObjectIndexes::ArrayBufferConstructor,
    //         ))),
    //     ),
    //     ObjectEntry::new_prototype_function_entry(heap, "slice", 2, false),
    //     ObjectEntry::new(
    //         PropertyKey::Symbol(WellKnownSymbolIndexes::ToStringTag.into()),
    //         ObjectEntryPropertyDescriptor::roxh(Value::from_str(heap, "ArrayBuffer")),
    //     ),
    // ];
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::ArrayBufferPrototype,
    //     true,
    //     Some(Object::Object(
    //         IntrinsicObjectIndexes::ObjectPrototype.into(),
    //     )),
    //     entries,
    // );
}

fn constructor_binding(
    _agent: &mut Agent,
    _this: Value,
    _args: ArgumentsList,
    _target: Option<Object>,
) -> JsResult<Value> {
    todo!()
}
