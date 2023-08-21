use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, ObjectHeapData, PropertyDescriptor,
    },
    value::{JsResult, Value},
};

use super::{
    function::FunctionHeapData,
    object::{ObjectEntry, PropertyKey}, ElementArrayKey, ElementsVector,
};

#[derive(Debug)]
pub(crate) struct ErrorHeapData {
    pub(super) object_index: u32,
    // TODO: stack? name?
}

pub fn initialize_error_heap(heap: &mut Heap) {
    let entries = vec![ObjectEntry::new_constructor_prototype_entry(
        heap,
        BuiltinObjectIndexes::ErrorPrototypeIndex as u32,
    )];
    heap.objects[BuiltinObjectIndexes::ErrorConstructorIndex as usize] = Some(ObjectHeapData::new(
        true,
        Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
        ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
        ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
    ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::ErrorConstructorIndex) as usize] =
        Some(FunctionHeapData {
            object_index: BuiltinObjectIndexes::ErrorConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: error_constructor_binding,
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
            PropertyDescriptor::rwx(Value::EmptyString),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "name"),
            PropertyDescriptor::rwx(Value::new_string(heap, "Error")),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false, error_todo),
    ];
    heap.objects[BuiltinObjectIndexes::ErrorPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
        ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
    ));
}

fn error_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::Function(0))
}

fn error_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!()
}
