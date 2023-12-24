use crate::{
    ecmascript::{
        execution::JsResult,
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
            // uses_arguments: false,
            // bound: None,
            // visible: None,
            initial_name: Value::Null,
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

fn string_constructor_binding(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    Ok(Value::Null)
}
