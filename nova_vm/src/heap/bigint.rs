use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, Heap, ObjectEntry, ObjectHeapData, PropertyDescriptor, PropertyKey,
    },
    value::{JsResult, Value},
};

use super::{ElementArrayKey, ElementsVector};

#[derive(Debug)]
pub(crate) struct BigIntHeapData {
    pub(super) sign: bool,
    pub(super) len: u32,
    pub(super) parts: Box<[u64]>,
}

impl BigIntHeapData {
    pub(crate) fn len(&self) -> u32 {
        self.len
    }

    pub(crate) fn try_into_f64(&self) -> Option<f64> {
        if self.len == 1 {
            Some(self.parts[0] as f64)
        } else {
            None
        }
    }
}

pub fn initialize_bigint_heap(heap: &mut Heap) {
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "asIntN", 2, false, bigint_as_int_n),
        ObjectEntry::new_prototype_function_entry(heap, "asUintN", 2, false, bigint_as_uint_n),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::BigintPrototypeIndex as u32,
        ),
    ];
    heap.objects[BuiltinObjectIndexes::BigintConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
            ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::BigintConstructorIndex) as usize] =
        Some(FunctionHeapData {
            object_index: heap.objects.len() as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
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
    heap.objects[BuiltinObjectIndexes::BigintPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
        ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
    ));
}

fn bigint_constructor(heap: &mut Heap, this: Value, args: &[Value]) -> JsResult<Value> {
    if !this.is_undefined() {
        // TODO: Throw TypeError
        return Err(Value::Error(0));
    } else {
        return Ok(Value::SmallBigInt(3));
    }
}

fn bigint_as_int_n(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::SmallBigInt(3))
}

fn bigint_as_uint_n(heap: &mut Heap, this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::SmallBigIntU(3))
}

fn bigint_prototype_to_locale_string(
    heap: &mut Heap,
    this: Value,
    args: &[Value],
) -> JsResult<Value> {
    Ok(Value::new_string(heap, "BigInt(3n)"))
}

fn bigint_prototype_to_string(heap: &mut Heap, this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::new_string(heap, "BigInt(3n)"))
}

fn bigint_prototype_value_of(heap: &mut Heap, this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::new_string(heap, "BigInt(3n)"))
}
