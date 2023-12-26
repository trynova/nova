use super::{
    heap_constants::WellKnownSymbolIndexes, indexes::BuiltinFunctionIndex, object::ObjectEntry,
};
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

pub fn initialize_function_heap(heap: &mut Heap) {
    let entries = vec![ObjectEntry::new_constructor_prototype_entry(
        heap,
        BuiltinObjectIndexes::FunctionPrototype.into(),
    )];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::FunctionConstructor,
        true,
        Some(Object::BuiltinFunction(
            BuiltinObjectIndexes::FunctionPrototype.into(),
        )),
        entries,
    );
    heap.builtin_functions
        [get_constructor_index(BuiltinObjectIndexes::FunctionConstructor).into_index()] =
        Some(BuiltinFunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::FunctionConstructor.into()),
            length: 1,
            initial_name: Value::Null,
            behaviour: Behaviour::Constructor(function_constructor_binding),
        });
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "apply", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "bind", 1, true),
        ObjectEntry::new_prototype_function_entry(heap, "call", 1, true),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
                BuiltinObjectIndexes::FunctionConstructor,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
        ObjectEntry::new_prototype_symbol_function_entry(
            heap,
            "hasInstance",
            WellKnownSymbolIndexes::HasInstance.into(),
            1,
            false,
        ),
    ];
    // NOTE: According to ECMAScript spec https://tc39.es/ecma262/#sec-properties-of-the-function-prototype-object
    // the %Function.prototype% object should itself be a function that always returns undefined. This is not
    // upheld here and we probably do not care. It's seemingly the only prototype that is a function.
    heap.insert_builtin_object(
        BuiltinObjectIndexes::FunctionPrototype,
        true,
        Some(Object::Object(BuiltinObjectIndexes::ObjectPrototype.into())),
        entries,
    );
}

fn function_constructor_binding(
    _agent: &mut Agent,
    _this: Value,
    _args: ArgumentsList,
    _target: Option<Object>,
) -> JsResult<Value> {
    Ok(Value::BuiltinFunction(BuiltinFunctionIndex::from_index(0)))
}
