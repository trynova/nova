use std::vec;

use super::{
    object::{ObjectEntry, PropertyKey},
    ElementArrayKey, ElementsVector, Heap,
};
use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, ObjectHeapData, PropertyDescriptor,
    },
    value::{JsResult, Value},
};

#[derive(Debug)]
pub(crate) struct NumberHeapData {
    pub(super) data: f64,
}

impl NumberHeapData {
    pub(super) fn new(data: f64) -> NumberHeapData {
        NumberHeapData { data }
    }

    pub(crate) fn value(&self) -> f64 {
        self.data
    }
}

pub fn initialize_number_heap(heap: &mut Heap) {
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "EPSILON"),
            PropertyDescriptor::roh(Value::from_f64(heap, f64::EPSILON)),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "isFinite", 1, false, number_todo),
        ObjectEntry::new_prototype_function_entry(heap, "isInteger", 1, false, number_todo),
        ObjectEntry::new_prototype_function_entry(heap, "isNan", 1, false, number_todo),
        ObjectEntry::new_prototype_function_entry(heap, "isSafeInteger", 1, false, number_todo),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "MAX_SAFE_INTEGER"),
            PropertyDescriptor::roh(Value::from_f64(heap, 9007199254740991.0)),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "MAX_VALUE"),
            PropertyDescriptor::roh(Value::from_f64(heap, f64::MAX)),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "MIN_SAFE_INTEGER"),
            PropertyDescriptor::roh(Value::from_f64(heap, -9007199254740991.0)),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "MIN_VALUE"),
            PropertyDescriptor::roh(Value::from_f64(heap, f64::MIN)),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "NaN"),
            PropertyDescriptor::roh(Value::NaN),
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "NEGATIVE_INFINITY"),
            PropertyDescriptor::roh(Value::NegativeInfinity),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "parseFloat", 1, false, number_todo),
        ObjectEntry::new_prototype_function_entry(heap, "parseInt", 2, false, number_todo),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "POSITIVE_INFINITY"),
            PropertyDescriptor::roh(Value::Infinity),
        ),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::NumberPrototypeIndex as u32,
        ),
    ];
    heap.objects[BuiltinObjectIndexes::NumberConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            Value::Function(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
            ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::NumberConstructorIndex) as usize] =
        Some(FunctionHeapData {
            object_index: BuiltinObjectIndexes::NumberConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: number_constructor_binding,
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                BuiltinObjectIndexes::NumberConstructorIndex,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "toExponential", 1, false, number_todo),
        ObjectEntry::new_prototype_function_entry(heap, "toExponential", 1, false, number_todo),
        ObjectEntry::new_prototype_function_entry(heap, "toLocaleString", 0, false, number_todo),
        ObjectEntry::new_prototype_function_entry(heap, "toPrecision", 1, false, number_todo),
        ObjectEntry::new_prototype_function_entry(heap, "toString", 0, false, number_todo),
        ObjectEntry::new_prototype_function_entry(heap, "valueOf", 0, false, number_todo),
    ];
    heap.objects[BuiltinObjectIndexes::NumberPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
        ElementsVector::new(0, ElementArrayKey::from_usize(entries.len()), entries.len()),
    ));
}

fn number_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::SmiU(0))
}

fn number_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!();
}
