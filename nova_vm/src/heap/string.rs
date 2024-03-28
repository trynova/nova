use crate::{
    ecmascript::{
        builtins::ArgumentsList,
        execution::{Agent, JsResult},
        types::{Object, Value},
    },
    heap::Heap,
};

pub fn initialize_string_heap(_heap: &mut Heap) {
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::StringConstructor,
    //     true,
    //     Some(Object::BuiltinFunction(
    //         IntrinsicObjectIndexes::FunctionPrototype.into(),
    //     )),
    //     // TODO: Methods and properties
    //     Vec::with_capacity(0),
    // );
    // heap.builtin_functions
    //     [get_constructor_index(IntrinsicObjectIndexes::StringConstructor).into_index()] =
    //     Some(BuiltinFunctionHeapData {
    //         object_index: Some(IntrinsicObjectIndexes::StringConstructor.into()),
    //         length: 1,
    //         initial_name: None,
    //         behaviour: Behaviour::Constructor(constructor_binding),
    //         realm: RealmIdentifier::from_index(0),
    //     });
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::StringPrototype,
    //     true,
    //     Some(Object::Object(
    //         IntrinsicObjectIndexes::ObjectPrototype.into(),
    //     )),
    //     // TODO: Methods and properties
    //     Vec::with_capacity(0),
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
