use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, ObjectHeapData, PropertyDescriptor,
    },
    value::{JsResult, Value},
};

use super::{
    object::{ObjectEntry, PropertyKey},
    ElementArrayKey, ElementsVector, Heap,
};

pub fn initialize_boolean_heap(heap: &mut Heap) {
    let entries = vec![ObjectEntry::new_constructor_prototype_entry(
        heap,
        BuiltinObjectIndexes::BooleanPrototypeIndex as u32,
    )];
    heap.objects[BuiltinObjectIndexes::BooleanConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
            ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::BooleanConstructorIndex) as usize] =
        Some(FunctionHeapData {
            object_index: BuiltinObjectIndexes::BooleanConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: boolean_constructor_binding,
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                BuiltinObjectIndexes::BooleanConstructorIndex,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false, boolean_todo),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false, boolean_todo),
    ];
    heap.objects[BuiltinObjectIndexes::BooleanPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
        ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
    ));
}

fn boolean_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::Boolean(false))
}

fn boolean_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!();
}
