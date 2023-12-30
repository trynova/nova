use super::{indexes::ObjectIndex, object::ObjectEntry};
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

#[derive(Debug, Clone, Copy)]
pub struct ErrorHeapData {
    pub(super) object_index: ObjectIndex,
    // TODO: stack? name?
}

pub fn initialize_error_heap(heap: &mut Heap) {
    let entries = vec![ObjectEntry::new_constructor_prototype_entry(
        heap,
        BuiltinObjectIndexes::ErrorPrototype.into(),
    )];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::ErrorConstructor,
        true,
        Some(Object::BuiltinFunction(
            BuiltinObjectIndexes::FunctionPrototype.into(),
        )),
        entries,
    );
    heap.builtin_functions
        [get_constructor_index(BuiltinObjectIndexes::ErrorConstructor).into_index()] =
        Some(BuiltinFunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::ErrorConstructor.into()),
            length: 1,
            initial_name: Value::Null,
            behaviour: Behaviour::Constructor(constructor_binding),
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
                BuiltinObjectIndexes::ErrorConstructor,
            ))),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "name"),
            PropertyDescriptor::rwx(Value::try_from("").unwrap()),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "name"),
            PropertyDescriptor::rwx(Value::from_str(heap, "Error")),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::ErrorPrototype,
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
