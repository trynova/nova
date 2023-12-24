use crate::{
    ecmascript::{
        builtins::{ArgumentsList, Behaviour},
        execution::{Agent, JsResult},
        types::{Object, Value},
    },
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        BuiltinFunctionHeapData, Heap,
    },
};

pub fn initialize_string_heap(heap: &mut Heap) {
    heap.insert_builtin_object(
        BuiltinObjectIndexes::StringConstructorIndex,
        true,
        Some(Object::BuiltinFunction(
            BuiltinObjectIndexes::FunctionPrototypeIndex.into(),
        )),
        // TODO: Methods and properties
        Vec::with_capacity(0),
    );
    heap.builtin_functions
        [get_constructor_index(BuiltinObjectIndexes::StringConstructorIndex).into_index()] =
        Some(BuiltinFunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::StringConstructorIndex.into()),
            length: 1,
            initial_name: Value::Null,
            behaviour: Behaviour::Constructor(constructor_binding),
        });
    heap.insert_builtin_object(
        BuiltinObjectIndexes::StringPrototypeIndex,
        true,
        Some(Object::Object(
            BuiltinObjectIndexes::ObjectPrototypeIndex.into(),
        )),
        // TODO: Methods and properties
        Vec::with_capacity(0),
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
