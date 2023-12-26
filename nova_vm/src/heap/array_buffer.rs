use super::{heap_constants::WellKnownSymbolIndexes, object::ObjectEntry};
use crate::{
    ecmascript::{
        builtins::{ArgumentsList, Behaviour},
        execution::{Agent, JsResult},
        types::{BuiltinFunctionHeapData, Object, PropertyKey, Value},
    },
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, PropertyDescriptor,
    },
};

pub fn initialize_array_buffer_heap(heap: &mut Heap) {
    let species_function_name = Value::from_str(heap, "get [Symbol.species]");
    let byte_length_key = Value::from_str(heap, "get byteLength");
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "isView", 1, false),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::ArrayBufferPrototype.into(),
        ),
        ObjectEntry::new(
            PropertyKey::Symbol(WellKnownSymbolIndexes::Species.into()),
            PropertyDescriptor::ReadOnly {
                get: heap.create_function(species_function_name, 0, false),
                enumerable: false,
                configurable: true,
            },
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::ArrayBufferConstructor,
        true,
        Some(Object::BuiltinFunction(
            BuiltinObjectIndexes::FunctionPrototype.into(),
        )),
        entries,
    );
    heap.builtin_functions
        [get_constructor_index(BuiltinObjectIndexes::ArrayBufferConstructor).into_index()] =
        Some(BuiltinFunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::ArrayBufferConstructor.into()),
            length: 1,
            initial_name: Value::Null,
            behaviour: Behaviour::Constructor(constructor_binding),
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "byteLength"),
            PropertyDescriptor::ReadOnly {
                get: heap.create_function(byte_length_key, 0, false),
                enumerable: false,
                configurable: true,
            },
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
                BuiltinObjectIndexes::ArrayBufferConstructor,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "slice", 2, false),
        ObjectEntry::new(
            PropertyKey::Symbol(WellKnownSymbolIndexes::ToStringTag.into()),
            PropertyDescriptor::roxh(Value::from_str(heap, "ArrayBuffer")),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::ArrayBufferPrototype,
        true,
        Some(Object::Object(BuiltinObjectIndexes::ObjectPrototype.into())),
        entries,
    );
}

fn constructor_binding(
    _agent: &mut Agent,
    _this: Value,
    _args: ArgumentsList,
    _target: Option<Object>,
) -> JsResult<Value> {
    todo!()
}
