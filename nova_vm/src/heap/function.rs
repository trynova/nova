use super::indexes::BuiltinFunctionIndex;
use crate::{
    ecmascript::{
        builtins::ArgumentsList,
        execution::{Agent, JsResult},
        types::{Object, Value},
    },
    heap::Heap,
};

pub fn initialize_function_heap(_heap: &mut Heap) {
    // let entries = vec![ObjectEntry::new_constructor_prototype_entry(
    //     heap,
    //     IntrinsicObjectIndexes::FunctionPrototype.into(),
    // )];
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::FunctionConstructor,
    //     true,
    //     Some(Object::BuiltinFunction(
    //         IntrinsicObjectIndexes::FunctionPrototype.into(),
    //     )),
    //     entries,
    // );
    // heap.builtin_functions
    //     [get_constructor_index(IntrinsicObjectIndexes::FunctionConstructor).into_index()] =
    //     Some(BuiltinFunctionHeapData {
    //         object_index: Some(IntrinsicObjectIndexes::FunctionConstructor.into()),
    //         length: 1,
    //         initial_name: None,
    //         behaviour: Behaviour::Constructor(function_constructor_binding),
    //         realm: RealmIdentifier::from_index(0),
    //     });
    // let entries = vec![
    //     ObjectEntry::new_prototype_function_entry(heap, "apply", 2, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "bind", 1, true),
    //     ObjectEntry::new_prototype_function_entry(heap, "call", 1, true),
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "constructor"),
    //         ObjectEntryPropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
    //             IntrinsicObjectIndexes::FunctionConstructor,
    //         ))),
    //     ),
    //     ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
    //     ObjectEntry::new_prototype_symbol_function_entry(
    //         heap,
    //         "hasInstance",
    //         WellKnownSymbolIndexes::HasInstance.into(),
    //         1,
    //         false,
    //     ),
    // ];
    // // NOTE: According to ECMAScript spec https://tc39.es/ecma262/#sec-properties-of-the-function-prototype-object
    // // the %Function.prototype% object should itself be a function that always returns undefined. This is not
    // // upheld here and we probably do not care. It's seemingly the only prototype that is a function.
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::FunctionPrototype,
    //     true,
    //     Some(Object::Object(
    //         IntrinsicObjectIndexes::ObjectPrototype.into(),
    //     )),
    //     entries,
    // );
}

fn function_constructor_binding(
    _agent: &mut Agent,
    _this: Value,
    _args: ArgumentsList,
    _target: Option<Object>,
) -> JsResult<Value> {
    Ok(Value::BuiltinFunction(BuiltinFunctionIndex::from_index(0)))
}
