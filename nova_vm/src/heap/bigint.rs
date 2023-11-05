use super::indexes::ObjectIndex;
use crate::{
    ecmascript::{
        execution::JsResult,
        types::{Object, PropertyKey, Value},
    },
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap, ObjectEntry, PropertyDescriptor,
    },
};

pub fn initialize_bigint_heap(heap: &mut Heap) {
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "asIntN", 2, false),
        ObjectEntry::new_prototype_function_entry(heap, "asUintN", 2, false),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::BigintPrototypeIndex.into(),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::BigintConstructorIndex,
        true,
        Some(Object::Function(
            BuiltinObjectIndexes::FunctionPrototypeIndex.into(),
        )),
        entries,
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::BigintConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: Some(ObjectIndex::last(&heap.objects)),
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
                BuiltinObjectIndexes::BigintConstructorIndex,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false),
        // @@ToStringTag
        // ObjectEntry { key: PropertyKey::Symbol(), PropertyDescriptor }
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::BigintPrototypeIndex,
        true,
        Some(Object::Object(
            BuiltinObjectIndexes::ObjectPrototypeIndex.into(),
        )),
        entries,
    );
}

fn bigint_constructor(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    // if !this.is_undefined() {
    //     // TODO: Throw TypeError
    //     return Err(Value::Error(ErrorIndex::from_index(0)));
    // } else {
    //      Ok(Value::BigIntI56(3))
    // }
    Ok(Value::Null)
}

fn bigint_as_int_n(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    // Ok(Value::BigIntI56(3))
    Ok(Value::Null)
}

fn bigint_as_uint_n(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    // Ok(Value::SmallBigIntU(3))
    Ok(Value::Null)
}

fn bigint_prototype_to_locale_string(
    heap: &mut Heap,
    _this: Value,
    _args: &[Value],
) -> JsResult<Value> {
    Ok(Value::from_str(heap, "BigInt(3n)"))
}

fn bigint_prototype_to_string(heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    Ok(Value::from_str(heap, "BigInt(3n)"))
}

fn bigint_prototype_value_of(heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    Ok(Value::from_str(heap, "BigInt(3n)"))
}
