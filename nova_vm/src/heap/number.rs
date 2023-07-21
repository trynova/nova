use std::vec;

use super::{
    heap_trace::HeapTrace,
    object::{ObjectEntry, PropertyKey},
    Heap,
};
use crate::{
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        FunctionHeapData, HeapBits, ObjectHeapData, PropertyDescriptor,
    },
    value::{JsResult, Value},
};

pub(crate) struct NumberHeapData {
    pub(super) bits: HeapBits,
    pub(super) data: f64,
}

impl NumberHeapData {
    pub(super) fn new(data: f64) -> NumberHeapData {
        NumberHeapData {
            bits: HeapBits::new(),
            data,
        }
    }

    pub(crate) fn value(&self) -> f64 {
        self.data
    }
}

impl HeapTrace for Option<NumberHeapData> {
    fn trace(&self, _heap: &Heap) {}

    fn root(&self, _heap: &Heap) {
        assert!(self.is_some());
        self.as_ref().unwrap().bits.root();
    }

    fn unroot(&self, _heap: &Heap) {
        assert!(self.is_some());
        self.as_ref().unwrap().bits.unroot();
    }

    fn finalize(&mut self, _heap: &Heap) {
        self.take();
    }
}

pub fn initialize_number_heap(heap: &mut Heap) {
    heap.objects[BuiltinObjectIndexes::NumberConstructorIndex as usize] =
        Some(ObjectHeapData::new(
            true,
            PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::FunctionPrototypeIndex as u32),
            vec![
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "EPSILON"),
                    PropertyDescriptor::roh(Value::from_f64(heap, f64::EPSILON)),
                ),
                ObjectEntry::new_prototype_function(heap, "isFinite", 1, false, number_todo),
                ObjectEntry::new_prototype_function(heap, "isInteger", 1, false, number_todo),
                ObjectEntry::new_prototype_function(heap, "isNan", 1, false, number_todo),
                ObjectEntry::new_prototype_function(heap, "isSafeInteger", 1, false, number_todo),
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
                ObjectEntry::new_prototype_function(heap, "parseFloat", 1, false, number_todo),
                ObjectEntry::new_prototype_function(heap, "parseInt", 2, false, number_todo),
                ObjectEntry::new(
                    PropertyKey::from_str(heap, "POSITIVE_INFINITY"),
                    PropertyDescriptor::roh(Value::Infinity),
                ),
                ObjectEntry::new_prototype(heap, BuiltinObjectIndexes::NumberPrototypeIndex as u32),
            ],
        ));
    heap.functions[get_constructor_index(BuiltinObjectIndexes::NumberConstructorIndex) as usize] =
        Some(FunctionHeapData {
            bits: HeapBits::new(),
            object_index: BuiltinObjectIndexes::NumberConstructorIndex as u32,
            length: 1,
            uses_arguments: false,
            bound: None,
            visible: None,
            binding: number_constructor_binding,
        });
    heap.objects[BuiltinObjectIndexes::NumberPrototypeIndex as usize] = Some(ObjectHeapData::new(
        true,
        PropertyDescriptor::prototype_slot(BuiltinObjectIndexes::ObjectPrototypeIndex as u32),
        vec![
            ObjectEntry::new(
                PropertyKey::from_str(heap, "constructor"),
                PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                    BuiltinObjectIndexes::NumberConstructorIndex,
                ))),
            ),
            ObjectEntry::new_prototype_function(heap, "toExponential", 1, false, number_todo),
            ObjectEntry::new_prototype_function(heap, "toExponential", 1, false, number_todo),
            ObjectEntry::new_prototype_function(heap, "toLocaleString", 0, false, number_todo),
            ObjectEntry::new_prototype_function(heap, "toPrecision", 1, false, number_todo),
            ObjectEntry::new_prototype_function(heap, "toString", 0, false, number_todo),
            ObjectEntry::new_prototype_function(heap, "valueOf", 0, false, number_todo),
        ],
    ));
}

fn number_constructor_binding(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    Ok(Value::SmiU(0))
}

fn number_todo(heap: &mut Heap, _this: Value, args: &[Value]) -> JsResult<Value> {
    todo!();
}
