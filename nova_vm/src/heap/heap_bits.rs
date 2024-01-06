use std::borrow::{Borrow, BorrowMut};

use super::{
    date::DateHeapData,
    element_array::{ElementArrayKey, ElementsVector},
    error::ErrorHeapData,
    indexes::{
        ArrayBufferIndex, ArrayIndex, BigIntIndex, BoundFunctionIndex, BuiltinFunctionIndex,
        DateIndex, ECMAScriptFunctionIndex, ElementIndex, ErrorIndex, NumberIndex, ObjectIndex,
        RegExpIndex, StringIndex, SymbolIndex,
    },
    regexp::RegExpHeapData,
    ArrayHeapData, Heap, NumberHeapData, ObjectHeapData, StringHeapData, SymbolHeapData,
};
use crate::ecmascript::{
    builtins::ArrayBufferHeapData,
    execution::{
        DeclarativeEnvironment, DeclarativeEnvironmentIndex, EnvironmentIndex, FunctionEnvironment,
        FunctionEnvironmentIndex, GlobalEnvironment, GlobalEnvironmentIndex, Intrinsics,
        ObjectEnvironment, ObjectEnvironmentIndex, PrivateEnvironment, PrivateEnvironmentIndex,
        Realm, RealmIdentifier,
    },
    scripts_and_modules::{
        module::{Module, ModuleIdentifier},
        script::{Script, ScriptIdentifier},
        ScriptOrModule,
    },
    types::{
        BigIntHeapData, BoundFunctionHeapData, BuiltinFunctionHeapData, ECMAScriptFunctionHeapData,
        Function, Number, Object, String, Value,
    },
};

#[derive(Debug)]
pub struct HeapBits {
    pub modules: Box<[bool]>,
    pub scripts: Box<[bool]>,
    pub realms: Box<[bool]>,
    pub declarative_environments: Box<[bool]>,
    pub function_environments: Box<[bool]>,
    pub global_environments: Box<[bool]>,
    pub object_environments: Box<[bool]>,
    pub e_2_4: Box<[(bool, u8)]>,
    pub e_2_6: Box<[(bool, u8)]>,
    pub e_2_8: Box<[(bool, u8)]>,
    pub e_2_10: Box<[(bool, u16)]>,
    pub e_2_12: Box<[(bool, u16)]>,
    pub e_2_16: Box<[(bool, u16)]>,
    pub e_2_24: Box<[(bool, u32)]>,
    pub e_2_32: Box<[(bool, u32)]>,
    pub arrays: Box<[bool]>,
    pub array_buffers: Box<[bool]>,
    pub bigints: Box<[bool]>,
    pub bound_functions: Box<[bool]>,
    pub builtin_functions: Box<[bool]>,
    pub ecmascript_functions: Box<[bool]>,
    pub dates: Box<[bool]>,
    pub errors: Box<[bool]>,
    pub numbers: Box<[bool]>,
    pub objects: Box<[bool]>,
    pub regexps: Box<[bool]>,
    pub strings: Box<[bool]>,
    pub symbols: Box<[bool]>,
}

#[derive(Debug)]
pub struct WorkQueues {
    pub modules: Vec<ModuleIdentifier>,
    pub scripts: Vec<ScriptIdentifier>,
    pub realms: Vec<RealmIdentifier>,
    pub declarative_environments: Vec<DeclarativeEnvironmentIndex>,
    pub function_environments: Vec<FunctionEnvironmentIndex>,
    pub global_environments: Vec<GlobalEnvironmentIndex>,
    pub object_environments: Vec<ObjectEnvironmentIndex>,
    pub e_2_4: Vec<(ElementIndex, u32)>,
    pub e_2_6: Vec<(ElementIndex, u32)>,
    pub e_2_8: Vec<(ElementIndex, u32)>,
    pub e_2_10: Vec<(ElementIndex, u32)>,
    pub e_2_12: Vec<(ElementIndex, u32)>,
    pub e_2_16: Vec<(ElementIndex, u32)>,
    pub e_2_24: Vec<(ElementIndex, u32)>,
    pub e_2_32: Vec<(ElementIndex, u32)>,
    pub arrays: Vec<ArrayIndex>,
    pub array_buffers: Vec<ArrayBufferIndex>,
    pub bigints: Vec<BigIntIndex>,
    pub errors: Vec<ErrorIndex>,
    pub bound_functions: Vec<BoundFunctionIndex>,
    pub builtin_functions: Vec<BuiltinFunctionIndex>,
    pub ecmascript_functions: Vec<ECMAScriptFunctionIndex>,
    pub dates: Vec<DateIndex>,
    pub numbers: Vec<NumberIndex>,
    pub objects: Vec<ObjectIndex>,
    pub regexps: Vec<RegExpIndex>,
    pub strings: Vec<StringIndex>,
    pub symbols: Vec<SymbolIndex>,
}

impl HeapBits {
    pub fn new(heap: &Heap) -> Self {
        let modules = vec![false; heap.modules.len()];
        let scripts = vec![false; heap.scripts.len()];
        let realms = vec![false; heap.realms.len()];
        let declarative_environments = vec![false; heap.environments.declarative.len()];
        let function_environments = vec![false; heap.environments.function.len()];
        let global_environments = vec![false; heap.environments.global.len()];
        let object_environments = vec![false; heap.environments.object.len()];
        let e_2_4 = vec![(false, 0u8); heap.elements.e2pow4.values.len()];
        let e_2_6 = vec![(false, 0u8); heap.elements.e2pow6.values.len()];
        let e_2_8 = vec![(false, 0u8); heap.elements.e2pow8.values.len()];
        let e_2_10 = vec![(false, 0u16); heap.elements.e2pow10.values.len()];
        let e_2_12 = vec![(false, 0u16); heap.elements.e2pow12.values.len()];
        let e_2_16 = vec![(false, 0u16); heap.elements.e2pow16.values.len()];
        let e_2_24 = vec![(false, 0u32); heap.elements.e2pow24.values.len()];
        let e_2_32 = vec![(false, 0u32); heap.elements.e2pow32.values.len()];
        let arrays = vec![false; heap.arrays.len()];
        let array_buffers = vec![false; heap.array_buffers.len()];
        let bigints = vec![false; heap.bigints.len()];
        let errors = vec![false; heap.errors.len()];
        let bound_functions = vec![false; heap.bound_functions.len()];
        let builtin_functions = vec![false; heap.builtin_functions.len()];
        let ecmascript_functions = vec![false; heap.ecmascript_functions.len()];
        let dates = vec![false; heap.dates.len()];
        let numbers = vec![false; heap.numbers.len()];
        let objects = vec![false; heap.objects.len()];
        let regexps = vec![false; heap.regexps.len()];
        let strings = vec![false; heap.strings.len()];
        let symbols = vec![false; heap.symbols.len()];
        Self {
            modules: modules.into_boxed_slice(),
            scripts: scripts.into_boxed_slice(),
            realms: realms.into_boxed_slice(),
            declarative_environments: declarative_environments.into_boxed_slice(),
            function_environments: function_environments.into_boxed_slice(),
            global_environments: global_environments.into_boxed_slice(),
            object_environments: object_environments.into_boxed_slice(),
            e_2_4: e_2_4.into_boxed_slice(),
            e_2_6: e_2_6.into_boxed_slice(),
            e_2_8: e_2_8.into_boxed_slice(),
            e_2_10: e_2_10.into_boxed_slice(),
            e_2_12: e_2_12.into_boxed_slice(),
            e_2_16: e_2_16.into_boxed_slice(),
            e_2_24: e_2_24.into_boxed_slice(),
            e_2_32: e_2_32.into_boxed_slice(),
            errors: errors.into_boxed_slice(),
            arrays: arrays.into_boxed_slice(),
            array_buffers: array_buffers.into_boxed_slice(),
            bigints: bigints.into_boxed_slice(),
            bound_functions: bound_functions.into_boxed_slice(),
            builtin_functions: builtin_functions.into_boxed_slice(),
            ecmascript_functions: ecmascript_functions.into_boxed_slice(),
            dates: dates.into_boxed_slice(),
            numbers: numbers.into_boxed_slice(),
            objects: objects.into_boxed_slice(),
            regexps: regexps.into_boxed_slice(),
            strings: strings.into_boxed_slice(),
            symbols: symbols.into_boxed_slice(),
        }
    }
}

impl WorkQueues {
    pub fn new(heap: &Heap) -> Self {
        Self {
            modules: Vec::with_capacity(heap.modules.len() / 4),
            scripts: Vec::with_capacity(heap.scripts.len() / 4),
            realms: Vec::with_capacity(heap.realms.len() / 4),
            declarative_environments: Vec::with_capacity(heap.environments.declarative.len() / 4),
            function_environments: Vec::with_capacity(heap.environments.function.len() / 4),
            global_environments: Vec::with_capacity(heap.environments.global.len() / 4),
            object_environments: Vec::with_capacity(heap.environments.object.len() / 4),
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
            bound_functions: Vec::with_capacity(heap.bound_functions.len() / 4),
            builtin_functions: Vec::with_capacity(heap.builtin_functions.len() / 4),
            ecmascript_functions: Vec::with_capacity(heap.ecmascript_functions.len() / 4),
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
            Value::BoundFunction(_idx) => todo!(),
            Value::BuiltinFunction(_idx) => todo!(),
            Value::ECMAScriptFunction(_idx) => todo!(),
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

    pub fn push_elements_vector(&mut self, vec: &ElementsVector) {
        match vec.cap {
            ElementArrayKey::E4 => self.e_2_4.push((vec.elements_index, vec.len)),
            ElementArrayKey::E6 => self.e_2_6.push((vec.elements_index, vec.len)),
            ElementArrayKey::E8 => self.e_2_8.push((vec.elements_index, vec.len)),
            ElementArrayKey::E10 => self.e_2_10.push((vec.elements_index, vec.len)),
            ElementArrayKey::E12 => self.e_2_12.push((vec.elements_index, vec.len)),
            ElementArrayKey::E16 => self.e_2_16.push((vec.elements_index, vec.len)),
            ElementArrayKey::E24 => self.e_2_24.push((vec.elements_index, vec.len)),
            ElementArrayKey::E32 => self.e_2_32.push((vec.elements_index, vec.len)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
            && self.scripts.is_empty()
            && self.realms.is_empty()
            && self.declarative_environments.is_empty()
            && self.function_environments.is_empty()
            && self.object_environments.is_empty()
            && self.e_2_4.is_empty()
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
            && self.dates.is_empty()
            && self.bound_functions.is_empty()
            && self.builtin_functions.is_empty()
            && self.ecmascript_functions.is_empty()
            && self.numbers.is_empty()
            && self.objects.is_empty()
            && self.regexps.is_empty()
            && self.strings.is_empty()
            && self.symbols.is_empty()
    }
}

#[derive(Debug)]
pub(crate) struct CompactionList {
    indexes: Box<[u32]>,
    shifts: Box<[u32]>,
}

impl CompactionList {
    pub fn get_shift_for_index(&self, index: u32) -> u32 {
        self.indexes
            .iter()
            .enumerate()
            .rev()
            .find(|(_, candidate)| **candidate <= index)
            .map(|(index, _)| *self.shifts.get(index).unwrap())
            .unwrap_or(0)
    }

    fn build(indexes: Vec<u32>, shifts: Vec<u32>) -> Self {
        assert_eq!(indexes.len(), shifts.len());
        Self {
            indexes: indexes.into_boxed_slice(),
            shifts: shifts.into_boxed_slice(),
        }
    }

    pub(crate) fn from_mark_bits(marks: &[bool]) -> Self {
        let mut builder = CompactionListBuilder::default();
        marks.iter().for_each(|bit| {
            if *bit {
                builder.mark_used();
            } else {
                builder.mark_unused();
            }
        });
        builder.done()
    }

    pub(crate) fn from_mark_u8s(marks: &[(bool, u8)]) -> Self {
        let mut builder = CompactionListBuilder::default();
        marks.iter().for_each(|mark| {
            if mark.0 {
                builder.mark_used();
            } else {
                builder.mark_unused();
            }
        });
        builder.done()
    }

    pub(crate) fn from_mark_u16s(marks: &[(bool, u16)]) -> Self {
        let mut builder = CompactionListBuilder::default();
        marks.iter().for_each(|mark| {
            if mark.0 {
                builder.mark_used();
            } else {
                builder.mark_unused();
            }
        });
        builder.done()
    }

    pub(crate) fn from_mark_u32s(marks: &[(bool, u32)]) -> Self {
        let mut builder = CompactionListBuilder::default();
        marks.iter().for_each(|mark| {
            if mark.0 {
                builder.mark_used();
            } else {
                builder.mark_unused();
            }
        });
        builder.done()
    }
}

#[derive(Debug)]
pub(crate) struct CompactionListBuilder {
    indexes: Vec<u32>,
    shifts: Vec<u32>,
    current_index: u32,
    current_shift: u32,
    current_used: bool,
    current_unused_start_index: u32,
}

impl CompactionListBuilder {
    fn push_index_with_shift(&mut self, index: u32, shift: u32) {
        assert_eq!(self.shifts.len(), self.indexes.len());
        assert!(self.indexes.is_empty() || *self.indexes.last().unwrap() < index);
        assert!(self.shifts.is_empty() || *self.shifts.last().unwrap() < shift);
        self.shifts.push(shift);
        self.indexes.push(index);
    }

    pub fn mark_used(&mut self) {
        if !self.current_used {
            let shift_start_index = if self.current_unused_start_index == 0 {
                self.current_index
            } else {
                self.current_unused_start_index
            };
            self.push_index_with_shift(shift_start_index, self.current_shift);
            self.current_used = true;
        }
        self.current_index += 1;
    }

    pub fn mark_unused(&mut self) {
        if self.current_used {
            self.current_unused_start_index = self.current_index;
            self.current_used = false;
        }
        self.current_shift += 1;
        self.current_index += 1;
    }

    pub fn done(self) -> CompactionList {
        CompactionList::build(self.indexes, self.shifts)
    }
}

impl Default for CompactionListBuilder {
    fn default() -> Self {
        Self {
            indexes: Vec::with_capacity(16),
            shifts: Vec::with_capacity(16),
            current_index: 0,
            current_shift: 0,
            current_used: true,
            current_unused_start_index: 0,
        }
    }
}

pub(crate) struct CompactionLists {
    pub modules: CompactionList,
    pub scripts: CompactionList,
    pub realms: CompactionList,
    pub declarative_environments: CompactionList,
    pub function_environments: CompactionList,
    pub global_environments: CompactionList,
    pub object_environments: CompactionList,
    pub e_2_4: CompactionList,
    pub e_2_6: CompactionList,
    pub e_2_8: CompactionList,
    pub e_2_10: CompactionList,
    pub e_2_12: CompactionList,
    pub e_2_16: CompactionList,
    pub e_2_24: CompactionList,
    pub e_2_32: CompactionList,
    pub arrays: CompactionList,
    pub array_buffers: CompactionList,
    pub bigints: CompactionList,
    pub bound_functions: CompactionList,
    pub builtin_functions: CompactionList,
    pub ecmascript_functions: CompactionList,
    pub dates: CompactionList,
    pub errors: CompactionList,
    pub numbers: CompactionList,
    pub objects: CompactionList,
    pub regexps: CompactionList,
    pub strings: CompactionList,
    pub symbols: CompactionList,
}

impl CompactionLists {
    pub fn create_from_bits(bits: &HeapBits) -> Self {
        // TODO: Instead of each list creating its own Vecs, this
        // could instead be a singular Vec segmented into slices.
        // The total number of vector items needed for compactions can
        // be estimated from bits.len() / 2 - bits_marked. If only one bit
        // is marked then two compaction parts can exist. If only one bit
        // is unmarked then two compaction parts can exist. If exactly half
        // of bits are marked or unmarked then bits.len() / 2 number of compaction
        // areas can exist. We can use this mathematical bound to estimate a good
        // vector allocation.
        Self {
            modules: CompactionList::from_mark_bits(&bits.modules),
            scripts: CompactionList::from_mark_bits(&bits.scripts),
            realms: CompactionList::from_mark_bits(&bits.realms),
            declarative_environments: CompactionList::from_mark_bits(
                &bits.declarative_environments,
            ),
            function_environments: CompactionList::from_mark_bits(&bits.function_environments),
            global_environments: CompactionList::from_mark_bits(&bits.global_environments),
            object_environments: CompactionList::from_mark_bits(&bits.object_environments),
            e_2_4: CompactionList::from_mark_u8s(&bits.e_2_4),
            e_2_6: CompactionList::from_mark_u8s(&bits.e_2_6),
            e_2_8: CompactionList::from_mark_u8s(&bits.e_2_8),
            e_2_10: CompactionList::from_mark_u16s(&bits.e_2_10),
            e_2_12: CompactionList::from_mark_u16s(&bits.e_2_12),
            e_2_16: CompactionList::from_mark_u16s(&bits.e_2_16),
            e_2_24: CompactionList::from_mark_u32s(&bits.e_2_24),
            e_2_32: CompactionList::from_mark_u32s(&bits.e_2_32),
            arrays: CompactionList::from_mark_bits(&bits.arrays),
            array_buffers: CompactionList::from_mark_bits(&bits.array_buffers),
            bigints: CompactionList::from_mark_bits(&bits.bigints),
            bound_functions: CompactionList::from_mark_bits(&bits.bound_functions),
            builtin_functions: CompactionList::from_mark_bits(&bits.builtin_functions),
            ecmascript_functions: CompactionList::from_mark_bits(&bits.ecmascript_functions),
            dates: CompactionList::from_mark_bits(&bits.dates),
            errors: CompactionList::from_mark_bits(&bits.errors),
            numbers: CompactionList::from_mark_bits(&bits.numbers),
            objects: CompactionList::from_mark_bits(&bits.objects),
            regexps: CompactionList::from_mark_bits(&bits.regexps),
            strings: CompactionList::from_mark_bits(&bits.strings),
            symbols: CompactionList::from_mark_bits(&bits.symbols),
        }
    }
}

pub(crate) trait HeapMarkAndSweep<Data>
where
    Data: ?Sized,
{
    /// Mark all Heap references contained in self
    ///
    /// To mark a HeapIndex, push it into the relevant queue in
    /// WorkQueues.
    #[allow(unused_variables)]
    fn mark_values(&self, queues: &mut WorkQueues, data: impl BorrowMut<Data>);

    /// Handle potential sweep of and update Heap references in self
    ///
    /// Sweeping of self is needed for Heap vectors: They must compact
    /// according to the `compactions` parameter. Additionally, any
    /// Heap references in self must be updated according to the
    /// compactions list.
    #[allow(unused_variables)]
    fn sweep_values(&mut self, compactions: &CompactionLists, data: impl Borrow<Data>);
}

impl<T, Data> HeapMarkAndSweep<Data> for &T
where
    T: HeapMarkAndSweep<Data>,
{
    fn mark_values(&self, queues: &mut WorkQueues, data: impl BorrowMut<Data>) {
        (*self).mark_values(queues, data);
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists, _data: impl Borrow<Data>) {
        unreachable!();
    }
}

impl<T, Data> HeapMarkAndSweep<Data> for Option<T>
where
    T: HeapMarkAndSweep<Data>,
{
    fn mark_values(&self, queues: &mut WorkQueues, data: impl BorrowMut<Data>) {
        if let Some(content) = self {
            content.mark_values(queues, data);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, data: impl Borrow<Data>) {
        if let Some(content) = self {
            content.sweep_values(compactions, data);
        }
    }
}

impl<T, const N: usize> HeapMarkAndSweep<u32> for [T; N]
where
    T: HeapMarkAndSweep<()>,
{
    fn mark_values(&self, queues: &mut WorkQueues, length: impl BorrowMut<u32>) {
        let length: u32 = *length.borrow();

        self.as_slice()[..length as usize].iter().for_each(|value| {
            value.mark_values(queues, ());
        });
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, length: impl Borrow<u32>) {
        let length: u32 = *length.borrow();
        if length == 0 {
            return;
        }
        self.as_mut_slice()[..length as usize]
            .iter_mut()
            .for_each(|value| {
                value.sweep_values(compactions, ());
            });
    }
}

pub(crate) fn sweep_heap_vector_values<T: HeapMarkAndSweep<()>>(
    vec: &mut Vec<T>,
    compactions: &CompactionLists,
    bits: &[bool],
) {
    assert_eq!(vec.len(), bits.len());
    let mut iter = bits.iter();
    vec.retain_mut(|item| {
        if *iter.next().unwrap() {
            item.sweep_values(compactions, ());
            true
        } else {
            false
        }
    });
}

pub(crate) fn sweep_heap_u8_elements_vector_values<const N: usize>(
    vec: &mut Vec<Option<[Option<Value>; N]>>,
    compactions: &CompactionLists,
    u8s: &[(bool, u8)],
) {
    assert_eq!(vec.len(), u8s.len());
    let mut iter = u8s.iter();
    vec.retain_mut(|item| {
        let (mark, length) = iter.next().unwrap();
        if *mark {
            item.sweep_values(compactions, *length as u32);
            true
        } else {
            false
        }
    });
}

pub(crate) fn sweep_heap_u16_elements_vector_values<const N: usize>(
    vec: &mut Vec<Option<[Option<Value>; N]>>,
    compactions: &CompactionLists,
    u16s: &[(bool, u16)],
) {
    assert_eq!(vec.len(), u16s.len());
    let mut iter = u16s.iter();
    vec.retain_mut(|item| {
        let (mark, length) = iter.next().unwrap();
        if *mark {
            item.sweep_values(compactions, *length as u32);
            true
        } else {
            false
        }
    });
}

pub(crate) fn sweep_heap_u32_elements_vector_values<const N: usize>(
    vec: &mut Vec<Option<[Option<Value>; N]>>,
    compactions: &CompactionLists,
    u32s: &[(bool, u32)],
) {
    assert_eq!(vec.len(), u32s.len());
    let mut iter = u32s.iter();
    vec.retain_mut(|item| {
        let (mark, length) = iter.next().unwrap();
        if *mark {
            item.sweep_values(compactions, *length);
            true
        } else {
            false
        }
    });
}

impl HeapMarkAndSweep<()> for ArrayIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.arrays.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.arrays.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for ArrayBufferIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.array_buffers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self =
            Self::from_u32(self_index - compactions.array_buffers.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for BigIntIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.bigints.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.bigints.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for BoundFunctionIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.bound_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(
            self_index - compactions.bound_functions.get_shift_for_index(self_index),
        );
    }
}

impl HeapMarkAndSweep<()> for BuiltinFunctionIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.builtin_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(
            self_index
                - compactions
                    .builtin_functions
                    .get_shift_for_index(self_index),
        );
    }
}

impl HeapMarkAndSweep<()> for DateIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.dates.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.dates.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for ECMAScriptFunctionIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.ecmascript_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(
            self_index
                - compactions
                    .ecmascript_functions
                    .get_shift_for_index(self_index),
        );
    }
}

impl HeapMarkAndSweep<()> for ErrorIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.errors.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.errors.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for NumberIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.numbers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.numbers.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for ObjectIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.objects.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.objects.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for RegExpIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.regexps.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.regexps.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for StringIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.strings.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.strings.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for SymbolIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.symbols.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.symbols.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for Value {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        println!("Marking Value");
        match self {
            Value::Undefined
            | Value::Null
            | Value::Boolean(_)
            | Value::SmallString(_)
            | Value::Integer(_)
            | Value::Float(_)
            | Value::SmallBigInt(_) => {
                // Stack values: Nothing to mark
            }
            Value::String(idx) => idx.mark_values(queues, ()),
            Value::Symbol(idx) => idx.mark_values(queues, ()),
            Value::Number(idx) => idx.mark_values(queues, ()),
            Value::BigInt(idx) => idx.mark_values(queues, ()),
            Value::Object(idx) => idx.mark_values(queues, ()),
            Value::Array(idx) => idx.mark_values(queues, ()),
            Value::ArrayBuffer(idx) => idx.mark_values(queues, ()),
            Value::Date(idx) => idx.mark_values(queues, ()),
            Value::Error(idx) => idx.mark_values(queues, ()),
            Value::BoundFunction(idx) => idx.mark_values(queues, ()),
            Value::BuiltinFunction(idx) => idx.mark_values(queues, ()),
            Value::ECMAScriptFunction(idx) => idx.mark_values(queues, ()),
            Value::RegExp(idx) => idx.mark_values(queues, ()),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        match self {
            Value::Undefined
            | Value::Null
            | Value::Boolean(_)
            | Value::SmallString(_)
            | Value::Integer(_)
            | Value::Float(_)
            | Value::SmallBigInt(_) => {
                // Stack values: Nothing to sweep
            }
            Value::String(idx) => idx.sweep_values(compactions, ()),
            Value::Symbol(idx) => idx.sweep_values(compactions, ()),
            Value::Number(idx) => idx.sweep_values(compactions, ()),
            Value::BigInt(idx) => idx.sweep_values(compactions, ()),
            Value::Object(idx) => idx.sweep_values(compactions, ()),
            Value::Array(idx) => idx.sweep_values(compactions, ()),
            Value::ArrayBuffer(idx) => idx.sweep_values(compactions, ()),
            Value::Date(idx) => idx.sweep_values(compactions, ()),
            Value::Error(idx) => idx.sweep_values(compactions, ()),
            Value::BoundFunction(idx) => idx.sweep_values(compactions, ()),
            Value::BuiltinFunction(idx) => idx.sweep_values(compactions, ()),
            Value::ECMAScriptFunction(idx) => idx.sweep_values(compactions, ()),
            Value::RegExp(idx) => idx.sweep_values(compactions, ()),
        }
    }
}

impl HeapMarkAndSweep<()> for Function {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        match self {
            Function::BoundFunction(idx) => idx.mark_values(queues, ()),
            Function::BuiltinFunction(idx) => idx.mark_values(queues, ()),
            Function::ECMAScriptFunction(idx) => idx.mark_values(queues, ()),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        match self {
            Function::BoundFunction(idx) => idx.sweep_values(compactions, ()),
            Function::BuiltinFunction(idx) => idx.sweep_values(compactions, ()),
            Function::ECMAScriptFunction(idx) => idx.sweep_values(compactions, ()),
        }
    }
}

impl HeapMarkAndSweep<()> for Number {
    fn mark_values(&self, queues: &mut WorkQueues, data: impl BorrowMut<()>) {
        if let Self::Number(idx) = self {
            idx.mark_values(queues, data);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, data: impl Borrow<()>) {
        if let Self::Number(idx) = self {
            idx.sweep_values(compactions, data);
        }
    }
}

impl HeapMarkAndSweep<()> for Object {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        println!("Marking Object");
        match self {
            Object::Object(idx) => idx.mark_values(queues, ()),
            Object::Array(idx) => idx.mark_values(queues, ()),
            Object::ArrayBuffer(idx) => idx.mark_values(queues, ()),
            Object::BoundFunction(_) => todo!(),
            Object::BuiltinFunction(_) => todo!(),
            Object::ECMAScriptFunction(_) => todo!(),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        match self {
            Self::Object(idx) => idx.sweep_values(compactions, ()),
            Self::Array(idx) => idx.sweep_values(compactions, ()),
            _ => todo!(),
        }
    }
}

impl HeapMarkAndSweep<()> for String {
    fn mark_values(&self, queues: &mut WorkQueues, data: impl BorrowMut<()>) {
        if let Self::String(idx) = self {
            idx.mark_values(queues, data);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, data: impl Borrow<()>) {
        if let Self::String(idx) = self {
            idx.sweep_values(compactions, data);
        }
    }
}

impl HeapMarkAndSweep<()> for ElementsVector {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        match self.cap {
            ElementArrayKey::E4 => queues.e_2_4.push((self.elements_index, self.len)),
            ElementArrayKey::E6 => queues.e_2_6.push((self.elements_index, self.len)),
            ElementArrayKey::E8 => queues.e_2_8.push((self.elements_index, self.len)),
            ElementArrayKey::E10 => queues.e_2_10.push((self.elements_index, self.len)),
            ElementArrayKey::E12 => queues.e_2_12.push((self.elements_index, self.len)),
            ElementArrayKey::E16 => queues.e_2_16.push((self.elements_index, self.len)),
            ElementArrayKey::E24 => queues.e_2_24.push((self.elements_index, self.len)),
            ElementArrayKey::E32 => queues.e_2_32.push((self.elements_index, self.len)),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.elements_index.into_u32();
        let shift = match self.cap {
            ElementArrayKey::E4 => compactions.e_2_4.get_shift_for_index(self_index),
            ElementArrayKey::E6 => compactions.e_2_6.get_shift_for_index(self_index),
            ElementArrayKey::E8 => compactions.e_2_8.get_shift_for_index(self_index),
            ElementArrayKey::E10 => compactions.e_2_10.get_shift_for_index(self_index),
            ElementArrayKey::E12 => compactions.e_2_12.get_shift_for_index(self_index),
            ElementArrayKey::E16 => compactions.e_2_16.get_shift_for_index(self_index),
            ElementArrayKey::E24 => compactions.e_2_24.get_shift_for_index(self_index),
            ElementArrayKey::E32 => compactions.e_2_32.get_shift_for_index(self_index),
        };
        self.elements_index = ElementIndex::from_u32(self_index - shift);
    }
}

impl HeapMarkAndSweep<()> for ArrayHeapData {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.elements.mark_values(queues, ());
        self.object_index.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.elements.sweep_values(compactions, ());
        self.object_index.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for ArrayBufferHeapData {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.object_index.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.object_index.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for BigIntHeapData {
    fn mark_values(&self, _queues: &mut WorkQueues, _data: impl BorrowMut<()>) {}

    fn sweep_values(&mut self, _compactions: &CompactionLists, _data: impl Borrow<()>) {}
}

impl HeapMarkAndSweep<()> for BoundFunctionHeapData {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.function.mark_values(queues, ());
        self.object_index.mark_values(queues, ());
        self.bound_values.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.function.sweep_values(compactions, ());
        self.object_index.sweep_values(compactions, ());
        self.bound_values.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for BuiltinFunctionHeapData {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.initial_name.mark_values(queues, ());
        self.object_index.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.initial_name.sweep_values(compactions, ());
        self.object_index.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for ECMAScriptFunctionHeapData {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.initial_name.mark_values(queues, ());
        self.object_index.mark_values(queues, ());

        self.ecmascript_function.environment.mark_values(queues, ());
        self.ecmascript_function
            .private_environment
            .mark_values(queues, ());
        self.ecmascript_function.realm.mark_values(queues, ());
        self.ecmascript_function
            .script_or_module
            .mark_values(queues, ());
        self.ecmascript_function.home_object.mark_values(queues, ());
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists, _data: impl Borrow<()>) {
        todo!()
    }
}

impl HeapMarkAndSweep<()> for DateHeapData {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.object_index.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.object_index.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for ErrorHeapData {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.object_index.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.object_index.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for ObjectHeapData {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.keys.mark_values(queues, ());
        self.values.mark_values(queues, ());
        self.prototype.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.keys.sweep_values(compactions, ());
        self.values.sweep_values(compactions, ());
        self.prototype.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for NumberHeapData {
    fn mark_values(&self, _queues: &mut WorkQueues, _data: impl BorrowMut<()>) {}

    fn sweep_values(&mut self, _compactions: &CompactionLists, _data: impl Borrow<()>) {}
}

impl HeapMarkAndSweep<()> for RegExpHeapData {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.object_index.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.object_index.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for StringHeapData {
    fn mark_values(&self, _queues: &mut WorkQueues, _data: impl BorrowMut<()>) {}

    fn sweep_values(&mut self, _compactions: &CompactionLists, _data: impl Borrow<()>) {}
}

impl HeapMarkAndSweep<()> for SymbolHeapData {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.descriptor.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.descriptor.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for ModuleIdentifier {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.modules.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.modules.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for Module {
    fn mark_values(&self, _queues: &mut WorkQueues, _data: impl BorrowMut<()>) {}

    fn sweep_values(&mut self, _compactions: &CompactionLists, _data: impl Borrow<()>) {}
}

impl HeapMarkAndSweep<()> for RealmIdentifier {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.realms.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.realms.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for Realm {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.intrinsics().mark_values(queues, ());
        self.global_env.mark_values(queues, ());
        self.global_object.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.intrinsics().sweep_values(compactions, ());
        self.global_env.sweep_values(compactions, ());
        self.global_object.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for Intrinsics {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.array().mark_values(queues, ());
        self.array_prototype().mark_values(queues, ());
        self.array_buffer().mark_values(queues, ());
        self.array_buffer_prototype().mark_values(queues, ());
        self.big_int().mark_values(queues, ());
        self.big_int_prototype().mark_values(queues, ());
        self.boolean().mark_values(queues, ());
        self.boolean_prototype().mark_values(queues, ());
        self.error().mark_values(queues, ());
        self.error_prototype().mark_values(queues, ());
        self.eval().mark_values(queues, ());
        self.eval_error().mark_values(queues, ());
        self.eval_error_prototype().mark_values(queues, ());
        self.function().mark_values(queues, ());
        self.function_prototype().mark_values(queues, ());
        self.is_finite().mark_values(queues, ());
        self.is_nan().mark_values(queues, ());
        self.math().mark_values(queues, ());
        self.number().mark_values(queues, ());
        self.number_prototype().mark_values(queues, ());
        self.object().mark_values(queues, ());
        self.object_prototype().mark_values(queues, ());
        self.object_prototype_to_string().mark_values(queues, ());
        self.range_error().mark_values(queues, ());
        self.range_error_prototype().mark_values(queues, ());
        self.reference_error().mark_values(queues, ());
        self.reference_error_prototype().mark_values(queues, ());
        self.reflect().mark_values(queues, ());
        self.string().mark_values(queues, ());
        self.string_prototype().mark_values(queues, ());
        self.symbol().mark_values(queues, ());
        self.symbol_prototype().mark_values(queues, ());
        self.syntax_error().mark_values(queues, ());
        self.syntax_error_prototype().mark_values(queues, ());
        self.throw_type_error().mark_values(queues, ());
        self.type_error().mark_values(queues, ());
        self.type_error_prototype().mark_values(queues, ());
        self.uri_error().mark_values(queues, ());
        self.uri_error_prototype().mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.array.sweep_values(compactions, ());
        self.array_prototype.sweep_values(compactions, ());
        self.array_buffer.sweep_values(compactions, ());
        self.array_buffer_prototype.sweep_values(compactions, ());
        self.big_int.sweep_values(compactions, ());
        self.big_int_prototype.sweep_values(compactions, ());
        self.boolean.sweep_values(compactions, ());
        self.boolean_prototype.sweep_values(compactions, ());
        self.error.sweep_values(compactions, ());
        self.error_prototype.sweep_values(compactions, ());
        self.eval.sweep_values(compactions, ());
        self.eval_error.sweep_values(compactions, ());
        self.eval_error_prototype.sweep_values(compactions, ());
        self.function.sweep_values(compactions, ());
        self.function_prototype.sweep_values(compactions, ());
        self.is_finite.sweep_values(compactions, ());
        self.is_nan.sweep_values(compactions, ());
        self.math.sweep_values(compactions, ());
        self.number.sweep_values(compactions, ());
        self.number_prototype.sweep_values(compactions, ());
        self.object.sweep_values(compactions, ());
        self.object_prototype.sweep_values(compactions, ());
        self.object_prototype_to_string
            .sweep_values(compactions, ());
        self.range_error.sweep_values(compactions, ());
        self.range_error_prototype.sweep_values(compactions, ());
        self.reference_error.sweep_values(compactions, ());
        self.reference_error_prototype.sweep_values(compactions, ());
        self.reflect.sweep_values(compactions, ());
        self.string.sweep_values(compactions, ());
        self.string_prototype.sweep_values(compactions, ());
        self.symbol.sweep_values(compactions, ());
        self.symbol_prototype.sweep_values(compactions, ());
        self.syntax_error.sweep_values(compactions, ());
        self.syntax_error_prototype.sweep_values(compactions, ());
        self.throw_type_error.sweep_values(compactions, ());
        self.type_error.sweep_values(compactions, ());
        self.type_error_prototype.sweep_values(compactions, ());
        self.uri_error.sweep_values(compactions, ());
        self.uri_error_prototype.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for ScriptIdentifier {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.scripts.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(self_index - compactions.scripts.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep<()> for Script {
    fn mark_values(&self, queues: &mut WorkQueues, data: impl BorrowMut<()>) {
        self.realm.mark_values(queues, data);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, data: impl Borrow<()>) {
        self.realm.sweep_values(compactions, data);
    }
}

impl HeapMarkAndSweep<()> for ScriptOrModule {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        match self {
            ScriptOrModule::Script(idx) => idx.mark_values(queues, ()),
            ScriptOrModule::Module(idx) => idx.mark_values(queues, ()),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        match self {
            ScriptOrModule::Script(idx) => idx.sweep_values(compactions, ()),
            ScriptOrModule::Module(idx) => idx.sweep_values(compactions, ()),
        }
    }
}

impl HeapMarkAndSweep<()> for DeclarativeEnvironmentIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.declarative_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(
            self_index
                - compactions
                    .declarative_environments
                    .get_shift_for_index(self_index),
        );
    }
}

impl HeapMarkAndSweep<()> for FunctionEnvironmentIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.function_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(
            self_index
                - compactions
                    .function_environments
                    .get_shift_for_index(self_index),
        );
    }
}

impl HeapMarkAndSweep<()> for GlobalEnvironmentIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.global_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(
            self_index
                - compactions
                    .global_environments
                    .get_shift_for_index(self_index),
        );
    }
}

impl HeapMarkAndSweep<()> for ObjectEnvironmentIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        queues.object_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        let self_index = self.into_u32();
        *self = Self::from_u32(
            self_index
                - compactions
                    .object_environments
                    .get_shift_for_index(self_index),
        );
    }
}

impl HeapMarkAndSweep<()> for PrivateEnvironmentIndex {
    fn mark_values(&self, _queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        todo!()
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists, _data: impl Borrow<()>) {
        todo!()
    }
}

impl HeapMarkAndSweep<()> for EnvironmentIndex {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        match self {
            EnvironmentIndex::Declarative(idx) => idx.mark_values(queues, ()),
            EnvironmentIndex::Function(idx) => idx.mark_values(queues, ()),
            EnvironmentIndex::Global(idx) => idx.mark_values(queues, ()),
            EnvironmentIndex::Object(idx) => idx.mark_values(queues, ()),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        match self {
            EnvironmentIndex::Declarative(idx) => idx.sweep_values(compactions, ()),
            EnvironmentIndex::Function(idx) => idx.sweep_values(compactions, ()),
            EnvironmentIndex::Global(idx) => idx.sweep_values(compactions, ()),
            EnvironmentIndex::Object(idx) => idx.sweep_values(compactions, ()),
        }
    }
}

impl HeapMarkAndSweep<()> for DeclarativeEnvironment {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.outer_env.mark_values(queues, ());
        for binding in self.bindings.values() {
            binding.value.mark_values(queues, ());
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.outer_env.sweep_values(compactions, ());
        for binding in self.bindings.values_mut() {
            binding.value.sweep_values(compactions, ());
        }
    }
}

impl HeapMarkAndSweep<()> for FunctionEnvironment {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.declarative_environment.mark_values(queues, ());
        self.function_object.mark_values(queues, ());
        self.new_target.mark_values(queues, ());
        self.this_value.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.declarative_environment.sweep_values(compactions, ());
        self.function_object.sweep_values(compactions, ());
        self.new_target.sweep_values(compactions, ());
        self.this_value.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for GlobalEnvironment {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.declarative_record.mark_values(queues, ());
        self.global_this_value.mark_values(queues, ());
        self.object_record.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.declarative_record.sweep_values(compactions, ());
        self.global_this_value.sweep_values(compactions, ());
        self.object_record.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for ObjectEnvironment {
    fn mark_values(&self, queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        self.outer_env.mark_values(queues, ());
        self.binding_object.mark_values(queues, ());
    }

    fn sweep_values(&mut self, compactions: &CompactionLists, _data: impl Borrow<()>) {
        self.outer_env.sweep_values(compactions, ());
        self.binding_object.sweep_values(compactions, ());
    }
}

impl HeapMarkAndSweep<()> for PrivateEnvironment {
    fn mark_values(&self, _queues: &mut WorkQueues, _data: impl BorrowMut<()>) {
        todo!()
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists, _data: impl Borrow<()>) {
        todo!()
    }
}
