use super::indexes::ObjectIndex;
use crate::{
    execution::JsResult,
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap, ObjectEntry, PropertyDescriptor, PropertyKey,
    },
    types::Value,
};
use num_bigint_dig::BigInt;

#[derive(Debug, Clone)]
pub(crate) struct BigIntHeapData {
    pub(super) data: BigInt,
}

impl BigIntHeapData {
    pub(crate) fn try_into_f64(&self) -> Option<f64> {
        None
    }
}

pub fn initialize_bigint_heap(heap: &mut Heap) {
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "asIntN", 2, false, bigint_as_int_n),
        ObjectEntry::new_prototype_function_entry(heap, "asUintN", 2, false, bigint_as_uint_n),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::BigintPrototypeIndex.into(),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::BigintConstructorIndex,
        true,
        Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex.into()),
        entries,
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::BigintConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: ObjectIndex::last(&heap.objects),
            length: 1,
            // uses_arguments: false,
            // bound: None,
            // visible: None,
            binding: bigint_constructor,
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                BuiltinObjectIndexes::BigintConstructorIndex,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(
            heap,
            "toLocaleString",
            0,
            false,
            bigint_prototype_to_locale_string,
        ),
        ObjectEntry::new_prototype_function_entry(
            heap,
            "toString",
            0,
            false,
            bigint_prototype_to_string,
        ),
        ObjectEntry::new_prototype_function_entry(
            heap,
            "valueOf",
            0,
            false,
            bigint_prototype_value_of,
        ),
        // @@ToStringTag
        // ObjectEntry { key: PropertyKey::Symbol(), PropertyDescriptor }
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::BigintPrototypeIndex,
        true,
        Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex.into()),
        entries,
    );
}

fn bigint_constructor(heap: &mut Heap, this: Value, args: &[Value]) -> JsResult<Value> {
    // if !this.is_undefined() {
    //     // TODO: Throw TypeError
    //     return Err(Value::Error(ErrorIndex::from_index(0)));
    // } else {
    //      Ok(Value::SmallBigInt(3))
    // }
    Ok(Value::Null)
}

fn bigint_as_int_n(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    // Ok(Value::SmallBigInt(3))
    Ok(Value::Null)
}

fn bigint_as_uint_n(heap: &mut Heap, this: Value, args: &[Value]) -> JsResult<Value> {
    // Ok(Value::SmallBigIntU(3))
    Ok(Value::Null)
}

fn bigint_prototype_to_locale_string(
    heap: &mut Heap,
    this: Value,
    args: &[Value],
) -> JsResult<Value> {
    Ok(Value::from_str(heap, "BigInt(3n)"))
}

fn bigint_prototype_to_string(heap: &mut Heap, this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::from_str(heap, "BigInt(3n)"))
}

fn bigint_prototype_value_of(heap: &mut Heap, this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::from_str(heap, "BigInt(3n)"))
}
