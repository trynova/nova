use super::{object::ObjectEntry, Heap};
use crate::{
    ecmascript::{
        builtins::{ArgumentsList, Behaviour},
        execution::{Agent, JsResult},
        types::{Object, PropertyKey, Value},
    },
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        BuiltinFunctionHeapData, PropertyDescriptor,
    },
};

pub fn initialize_boolean_heap(heap: &mut Heap) {
    let entries = vec![ObjectEntry::new_constructor_prototype_entry(
        heap,
        BuiltinObjectIndexes::BooleanPrototypeIndex.into(),
    )];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::BooleanConstructorIndex,
        true,
        Some(Object::BuiltinFunction(
            BuiltinObjectIndexes::FunctionPrototypeIndex.into(),
        )),
        entries,
    );
    heap.builtin_functions
        [get_constructor_index(BuiltinObjectIndexes::BooleanConstructorIndex).into_index()] =
        Some(BuiltinFunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::BooleanConstructorIndex.into()),
            length: 1,
            initial_name: Value::Null,
            behaviour: Behaviour::Constructor(constructor_binding),
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::BuiltinFunction(get_constructor_index(
                BuiltinObjectIndexes::BooleanConstructorIndex,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::BooleanPrototypeIndex,
        true,
        Some(Object::Object(
            BuiltinObjectIndexes::ObjectPrototypeIndex.into(),
        )),
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
