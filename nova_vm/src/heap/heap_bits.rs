use std::sync::atomic::AtomicBool;

use crate::value::Value;

use super::Heap;

pub(crate) struct HeapBits {
    pub e_2_4: Box<[AtomicBool]>,
    pub e_2_6: Box<[AtomicBool]>,
    pub e_2_8: Box<[AtomicBool]>,
    pub e_2_10: Box<[AtomicBool]>,
    pub e_2_12: Box<[AtomicBool]>,
    pub e_2_16: Box<[AtomicBool]>,
    pub e_2_24: Box<[AtomicBool]>,
    pub e_2_32: Box<[AtomicBool]>,
    pub arrays: Box<[AtomicBool]>,
    pub bigints: Box<[AtomicBool]>,
    pub errors: Box<[AtomicBool]>,
    pub functions: Box<[AtomicBool]>,
    pub dates: Box<[AtomicBool]>,
    pub numbers: Box<[AtomicBool]>,
    pub objects: Box<[AtomicBool]>,
    pub regexps: Box<[AtomicBool]>,
    pub strings: Box<[AtomicBool]>,
    pub symbols: Box<[AtomicBool]>,
}

pub(crate) struct WorkQueues {
    pub e_2_4: Vec<u32>,
    pub e_2_6: Vec<u32>,
    pub e_2_8: Vec<u32>,
    pub e_2_10: Vec<u32>,
    pub e_2_12: Vec<u32>,
    pub e_2_16: Vec<u32>,
    pub e_2_24: Vec<u32>,
    pub e_2_32: Vec<u32>,
    pub arrays: Vec<u32>,
    pub bigints: Vec<u32>,
    pub errors: Vec<u32>,
    pub functions: Vec<u32>,
    pub dates: Vec<u32>,
    pub numbers: Vec<u32>,
    pub objects: Vec<u32>,
    pub regexps: Vec<u32>,
    pub strings: Vec<u32>,
    pub symbols: Vec<u32>,
}

impl HeapBits {
    pub(crate) fn new(heap: &Heap) -> Self {
        Self {
            e_2_4: Vec::with_capacity(heap.elements.e_2_4.len()).into_boxed_slice(),
            e_2_6: Vec::with_capacity(heap.elements.e_2_6.len()).into_boxed_slice(),
            e_2_8: Vec::with_capacity(heap.elements.e_2_8.len()).into_boxed_slice(),
            e_2_10: Vec::with_capacity(heap.elements.e_2_10.len()).into_boxed_slice(),
            e_2_12: Vec::with_capacity(heap.elements.e_2_12.len()).into_boxed_slice(),
            e_2_16: Vec::with_capacity(heap.elements.e_2_16.len()).into_boxed_slice(),
            e_2_24: Vec::with_capacity(heap.elements.e_2_24.len()).into_boxed_slice(),
            e_2_32: Vec::with_capacity(heap.elements.e_2_32.len()).into_boxed_slice(),
            arrays: Vec::with_capacity(heap.arrays.len()).into_boxed_slice(),
            bigints: Vec::with_capacity(heap.bigints.len()).into_boxed_slice(),
            errors: Vec::with_capacity(heap.errors.len()).into_boxed_slice(),
            functions: Vec::with_capacity(heap.functions.len()).into_boxed_slice(),
            dates: Vec::with_capacity(heap.dates.len()).into_boxed_slice(),
            numbers: Vec::with_capacity(heap.numbers.len()).into_boxed_slice(),
            objects: Vec::with_capacity(heap.objects.len()).into_boxed_slice(),
            regexps: Vec::with_capacity(heap.regexps.len()).into_boxed_slice(),
            strings: Vec::with_capacity(heap.strings.len()).into_boxed_slice(),
            symbols: Vec::with_capacity(heap.symbols.len()).into_boxed_slice(),
        }
    }
}

impl WorkQueues {
    pub(crate) fn new(heap: &Heap) -> Self {
        Self {
            e_2_4: Vec::with_capacity(heap.elements.e_2_4.len() / 4),
            e_2_6: Vec::with_capacity(heap.elements.e_2_6.len() / 4),
            e_2_8: Vec::with_capacity(heap.elements.e_2_8.len() / 4),
            e_2_10: Vec::with_capacity(heap.elements.e_2_10.len() / 4),
            e_2_12: Vec::with_capacity(heap.elements.e_2_12.len() / 4),
            e_2_16: Vec::with_capacity(heap.elements.e_2_16.len() / 4),
            e_2_24: Vec::with_capacity(heap.elements.e_2_24.len() / 4),
            e_2_32: Vec::with_capacity(heap.elements.e_2_32.len() / 4),
            arrays: Vec::with_capacity(heap.arrays.len() / 4),
            bigints: Vec::with_capacity(heap.bigints.len() / 4),
            errors: Vec::with_capacity(heap.errors.len() / 4),
            functions: Vec::with_capacity(heap.functions.len() / 4),
            dates: Vec::with_capacity(heap.dates.len() / 4),
            numbers: Vec::with_capacity(heap.numbers.len() / 4),
            objects: Vec::with_capacity(heap.objects.len() / 4),
            regexps: Vec::with_capacity(heap.regexps.len() / 4),
            strings: Vec::with_capacity(heap.strings.len() / 4),
            symbols: Vec::with_capacity(heap.symbols.len() / 4),
        }
    }

    pub(crate) fn push_value(&mut self, value: Value) {
        match value {
            Value::Array(idx) => self.arrays.push(idx),
            Value::BigIntObject(_) => todo!(),
            Value::BooleanObject(idx) => todo!(),
            Value::Boolean(_) => {}
            Value::Date(idx) => self.dates.push(idx),
            Value::EmptyString => {}
            Value::Error(idx) => self.errors.push(idx),
            Value::Function(idx) => todo!(),
            Value::HeapBigInt(idx) => self.bigints.push(idx),
            Value::HeapNumber(idx) => self.numbers.push(idx),
            Value::HeapString(idx) => self.strings.push(idx),
            Value::Infinity => {}
            Value::NaN => {}
            Value::NegativeInfinity => {}
            Value::NegativeZero => {}
            Value::Null => {}
            Value::NumberObject(_) => todo!(),
            Value::Object(idx) => self.objects.push(idx),
            Value::RegExp(idx) => self.regexps.push(idx),
            Value::StackString(_) => {}
            Value::SmallBigInt(_) => {}
            Value::SmallBigIntU(_) => {}
            Value::Smi(_) => {}
            Value::SmiU(_) => {}
            Value::StringObject(_) => todo!(),
            Value::Symbol(idx) => self.symbols.push(idx),
            Value::SymbolObject(_) => todo!(),
            Value::Undefined => {}
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.e_2_4.is_empty()
            && self.e_2_6.is_empty()
            && self.e_2_8.is_empty()
            && self.e_2_10.is_empty()
            && self.e_2_12.is_empty()
            && self.e_2_16.is_empty()
            && self.e_2_24.is_empty()
            && self.e_2_32.is_empty()
            && self.arrays.is_empty()
            && self.bigints.is_empty()
            && self.errors.is_empty()
            && self.functions.is_empty()
            && self.dates.is_empty()
            && self.numbers.is_empty()
            && self.objects.is_empty()
            && self.regexps.is_empty()
            && self.strings.is_empty()
            && self.symbols.is_empty()
    }
}
