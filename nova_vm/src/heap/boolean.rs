use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, PropertyDescriptor,
    },
    value::{JsResult, Value},
};

use super::{
    object::{ObjectEntry, PropertyKey},
    Heap,
};

pub fn initialize_boolean_heap(heap: &mut Heap) {
    let entries = vec![ObjectEntry::new_constructor_prototype_entry(
        heap,
        BuiltinObjectIndexes::BooleanPrototypeIndex as u32,
    )];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::BooleanConstructorIndex,
        true,
        Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
        entries,
    );
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
    heap.insert_builtin_object(
        BuiltinObjectIndexes::BooleanPrototypeIndex,
        true,
        Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        entries,
    );
}

fn boolean_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::Boolean(false))
}

fn boolean_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!();
}
