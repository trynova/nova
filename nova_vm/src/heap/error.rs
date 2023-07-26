use crate::{
    execution::JsResult,
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, PropertyDescriptor,
    },
    types::Value,
};

use super::{
    function::FunctionHeapData,
    indexes::{FunctionIndex, ObjectIndex},
    object::{ObjectEntry, PropertyKey},
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ErrorHeapData {
    pub(super) object_index: ObjectIndex,
    // TODO: stack? name?
}

pub fn initialize_error_heap(heap: &mut Heap) {
    let entries = vec![ObjectEntry::new_constructor_prototype_entry(
        heap,
        BuiltinObjectIndexes::ErrorPrototypeIndex.into(),
    )];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::ErrorConstructorIndex,
        true,
        Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex.into()),
        entries,
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::ErrorConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: BuiltinObjectIndexes::ErrorConstructorIndex.into(),
            length: 1,
            // uses_arguments: false,
            // bound: None,
            // visible: None,
            initial_name: Value::Null,
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                BuiltinObjectIndexes::ErrorConstructorIndex,
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
        BuiltinObjectIndexes::ErrorPrototypeIndex,
        true,
        Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex.into()),
        entries,
    );
}

fn error_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::Function(FunctionIndex::from_index(0)))
}

fn error_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!()
}
