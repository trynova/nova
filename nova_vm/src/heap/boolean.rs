use super::Heap;
use crate::ecmascript::{
    builtins::ArgumentsList,
    execution::{Agent, JsResult},
    types::{Object, Value},
};

pub fn initialize_boolean_heap(_heap: &mut Heap) {
    // let entries = vec![ObjectEntry::new_constructor_prototype_entry(
    //     heap,
    //     IntrinsicObjectIndexes::BooleanPrototype.into(),
    // )];
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::BooleanConstructor,
    //     true,
    //     Some(Object::BuiltinFunction(
    //         IntrinsicObjectIndexes::FunctionPrototype.into(),
    //     )),
    //     entries,
    // );
    // heap.builtin_functions
    //     [get_constructor_index(IntrinsicObjectIndexes::BooleanConstructor).into_index()] =
    //     Some(BuiltinFunctionHeapData {
    //         object_index: Some(IntrinsicObjectIndexes::BooleanConstructor.into()),
    //         length: 1,
    //         initial_name: None,
    //         behaviour: Behaviour::Constructor(constructor_binding),
    //         realm: RealmIdentifier::from_index(0),
    //     });
    // let entries = vec![
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "constructor"),
    //         ObjectEntryPropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
    //             IntrinsicObjectIndexes::BooleanConstructor,
    //         ))),
    //     ),
    //     ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
    //     ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false),
    // ];
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::BooleanPrototype,
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
