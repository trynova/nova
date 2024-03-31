use super::indexes::ObjectIndex;
use crate::{
    ecmascript::{
        builtins::ArgumentsList,
        execution::{Agent, JsResult},
        types::{Object, Value},
    },
    heap::Heap,
};

#[derive(Debug, Clone, Copy)]
pub struct ErrorHeapData {
    pub(super) object_index: ObjectIndex,
    // TODO: stack? name?
}

pub fn initialize_error_heap(_heap: &mut Heap) {
    // let entries = vec![ObjectEntry::new_constructor_prototype_entry(
    //     heap,
    //     IntrinsicObjectIndexes::ErrorPrototype.into(),
    // )];
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::ErrorConstructor,
    //     true,
    //     Some(Object::BuiltinFunction(
    //         IntrinsicObjectIndexes::FunctionPrototype.into(),
    //     )),
    //     entries,
    // );
    // heap.builtin_functions
    //     [get_constructor_index(IntrinsicObjectIndexes::ErrorConstructor).into_index()] =
    //     Some(BuiltinFunctionHeapData {
    //         object_index: Some(IntrinsicObjectIndexes::ErrorConstructor.into()),
    //         length: 1,
    //         initial_name: None,
    //         behaviour: Behaviour::Constructor(constructor_binding),
    //         realm: RealmIdentifier::from_index(0),
    //     });
    // let entries = vec![
    //     ObjectEntry::new(
    //         PropertyKey::from_str(heap, "constructor"),
    //         ObjectEntryPropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
    //             IntrinsicObjectIndexes::ErrorConstructor,
    //         ))),
    //     ),
    //     ObjectEntry::new(
    //         NAME_KEY.into(),
    //         ObjectEntryPropertyDescriptor::rwx(EMPTY_STRING.into()),
    //     ),
    //     ObjectEntry::new(
    //         NAME_KEY.into(),
    //         ObjectEntryPropertyDescriptor::rwx(ERROR_CLASS_NAME.into()),
    //     ),
    //     ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
    // ];
    // heap.insert_builtin_object(
    //     IntrinsicObjectIndexes::ErrorPrototype,
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
