use super::{
    indexes::{
        ArrayBufferIndex, ArrayIndex, BigIntIndex, DateIndex, ElementIndex, ErrorIndex,
        FunctionIndex, NumberIndex, ObjectIndex, RegExpIndex, StringIndex, SymbolIndex,
    },
    Heap,
};
use crate::ecmascript::types::Value;
use std::sync::atomic::AtomicBool;

pub struct HeapBits {
    pub e_2_4: Box<[AtomicBool]>,
    pub e_2_6: Box<[AtomicBool]>,
    pub e_2_8: Box<[AtomicBool]>,
    pub e_2_10: Box<[AtomicBool]>,
    pub e_2_12: Box<[AtomicBool]>,
    pub e_2_16: Box<[AtomicBool]>,
    pub e_2_24: Box<[AtomicBool]>,
    pub e_2_32: Box<[AtomicBool]>,
    pub arrays: Box<[AtomicBool]>,
    pub array_buffers: Box<[AtomicBool]>,
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

pub struct WorkQueues {
    pub e_2_4: Vec<ElementIndex>,
    pub e_2_6: Vec<ElementIndex>,
    pub e_2_8: Vec<ElementIndex>,
    pub e_2_10: Vec<ElementIndex>,
    pub e_2_12: Vec<ElementIndex>,
    pub e_2_16: Vec<ElementIndex>,
    pub e_2_24: Vec<ElementIndex>,
    pub e_2_32: Vec<ElementIndex>,
    pub arrays: Vec<ArrayIndex>,
    pub array_buffers: Vec<ArrayBufferIndex>,
    pub bigints: Vec<BigIntIndex>,
    pub errors: Vec<ErrorIndex>,
    pub functions: Vec<FunctionIndex>,
    pub dates: Vec<DateIndex>,
    pub numbers: Vec<NumberIndex>,
    pub objects: Vec<ObjectIndex>,
    pub regexps: Vec<RegExpIndex>,
    pub strings: Vec<StringIndex>,
    pub symbols: Vec<SymbolIndex>,
}

impl HeapBits {
    pub fn new(heap: &Heap) -> Self {
        Self {
            e_2_4: Vec::with_capacity(heap.elements.e2pow4.values.len()).into_boxed_slice(),
            e_2_6: Vec::with_capacity(heap.elements.e2pow6.values.len()).into_boxed_slice(),
            e_2_8: Vec::with_capacity(heap.elements.e2pow8.values.len()).into_boxed_slice(),
            e_2_10: Vec::with_capacity(heap.elements.e2pow10.values.len()).into_boxed_slice(),
            e_2_12: Vec::with_capacity(heap.elements.e2pow12.values.len()).into_boxed_slice(),
            e_2_16: Vec::with_capacity(heap.elements.e2pow16.values.len()).into_boxed_slice(),
            e_2_24: Vec::with_capacity(heap.elements.e2pow24.values.len()).into_boxed_slice(),
            e_2_32: Vec::with_capacity(heap.elements.e2pow32.values.len()).into_boxed_slice(),
            arrays: Vec::with_capacity(heap.arrays.len()).into_boxed_slice(),
            array_buffers: Vec::with_capacity(heap.array_buffers.len()).into_boxed_slice(),
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
    pub fn new(heap: &Heap) -> Self {
        Self {
            e_2_4: Vec::with_capacity(heap.elements.e2pow4.values.len() / 4),
            e_2_6: Vec::with_capacity(heap.elements.e2pow6.values.len() / 4),
            e_2_8: Vec::with_capacity(heap.elements.e2pow8.values.len() / 4),
            e_2_10: Vec::with_capacity(heap.elements.e2pow10.values.len() / 4),
            e_2_12: Vec::with_capacity(heap.elements.e2pow12.values.len() / 4),
            e_2_16: Vec::with_capacity(heap.elements.e2pow16.values.len() / 4),
            e_2_24: Vec::with_capacity(heap.elements.e2pow24.values.len() / 4),
            e_2_32: Vec::with_capacity(heap.elements.e2pow32.values.len() / 4),
            arrays: Vec::with_capacity(heap.arrays.len() / 4),
            array_buffers: Vec::with_capacity(heap.array_buffers.len() / 4),
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

    pub fn push_value(&mut self, value: Value) {
        match value {
            Value::Array(idx) => self.arrays.push(idx),
            Value::ArrayBuffer(idx) => self.array_buffers.push(idx),
            // Value::BigIntObject(_) => todo!(),
            // Value::BooleanObject(idx) => todo!(),
            Value::Boolean(_) => {}
            Value::Date(idx) => self.dates.push(idx),
            Value::Error(idx) => self.errors.push(idx),
            Value::Function(_idx) => todo!(),
            Value::BigInt(idx) => self.bigints.push(idx),
            Value::Number(idx) => self.numbers.push(idx),
            Value::String(idx) => self.strings.push(idx),
            Value::Null => {}
            // Value::NumberObject(_) => todo!(),
            Value::Object(idx) => self.objects.push(idx),
            Value::RegExp(idx) => self.regexps.push(idx),
            Value::SmallString(_) => {}
            Value::SmallBigInt(_) => {}
            // Value::StringObject(_) => todo!(),
            Value::Symbol(idx) => self.symbols.push(idx),
            // Value::SymbolObject(_) => todo!(),
            Value::Undefined => {}
            Value::Integer(_) => {}
            Value::Float(_) => {}
        }
    }

    pub fn is_empty(&self) -> bool {
        self.e_2_4.is_empty()
            && self.e_2_6.is_empty()
            && self.e_2_8.is_empty()
            && self.e_2_10.is_empty()
            && self.e_2_12.is_empty()
            && self.e_2_16.is_empty()
            && self.e_2_24.is_empty()
            && self.e_2_32.is_empty()
            && self.arrays.is_empty()
            && self.array_buffers.is_empty()
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
