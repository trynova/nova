use super::{
    element_array::{ElementArrayKey, ElementsVector},
    indexes::{
        ArrayBufferIndex, ArrayIndex, BigIntIndex, BoundFunctionIndex, BuiltinFunctionIndex,
        DateIndex, ECMAScriptFunctionIndex, ElementIndex, ErrorIndex, NumberIndex, ObjectIndex,
        RegExpIndex, StringIndex, SymbolIndex,
    },
    Heap, ObjectHeapData,
};
use crate::ecmascript::{
    execution::{
        DeclarativeEnvironmentIndex, FunctionEnvironmentIndex, GlobalEnvironmentIndex,
        ObjectEnvironmentIndex, RealmIdentifier,
    },
    scripts_and_modules::{module::ModuleIdentifier, script::ScriptIdentifier},
    types::{Object, Value},
};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct HeapBits {
    pub modules: Box<[AtomicBool]>,
    pub scripts: Box<[AtomicBool]>,
    pub realms: Box<[AtomicBool]>,
    pub declarative_environments: Box<[AtomicBool]>,
    pub function_environments: Box<[AtomicBool]>,
    pub global_environments: Box<[AtomicBool]>,
    pub object_environments: Box<[AtomicBool]>,
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
    pub bound_functions: Box<[AtomicBool]>,
    pub builtin_functions: Box<[AtomicBool]>,
    pub ecmascript_functions: Box<[AtomicBool]>,
    pub dates: Box<[AtomicBool]>,
    pub errors: Box<[AtomicBool]>,
    pub numbers: Box<[AtomicBool]>,
    pub objects: Box<[AtomicBool]>,
    pub regexps: Box<[AtomicBool]>,
    pub strings: Box<[AtomicBool]>,
    pub symbols: Box<[AtomicBool]>,
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
        let mut modules = Vec::with_capacity(heap.modules.len());
        modules.resize_with(modules.capacity(), || AtomicBool::new(false));
        let mut scripts = Vec::with_capacity(heap.scripts.len());
        scripts.resize_with(scripts.capacity(), || AtomicBool::new(false));
        let mut realms = Vec::with_capacity(heap.realms.len());
        realms.resize_with(realms.capacity(), || AtomicBool::new(false));
        let mut declarative_environments = Vec::with_capacity(heap.environments.declarative.len());
        declarative_environments.resize_with(declarative_environments.capacity(), || {
            AtomicBool::new(false)
        });
        let mut function_environments = Vec::with_capacity(heap.environments.function.len());
        function_environments
            .resize_with(function_environments.capacity(), || AtomicBool::new(false));
        let mut global_environments = Vec::with_capacity(heap.environments.global.len());
        global_environments.resize_with(global_environments.capacity(), || AtomicBool::new(false));
        let mut object_environments = Vec::with_capacity(heap.environments.object.len());
        object_environments.resize_with(object_environments.capacity(), || AtomicBool::new(false));
        let mut e_2_4 = Vec::with_capacity(heap.elements.e2pow4.values.len());
        e_2_4.resize_with(e_2_4.capacity(), || AtomicBool::new(false));
        let mut e_2_6 = Vec::with_capacity(heap.elements.e2pow6.values.len());
        e_2_6.resize_with(e_2_6.capacity(), || AtomicBool::new(false));
        let mut e_2_8 = Vec::with_capacity(heap.elements.e2pow8.values.len());
        e_2_8.resize_with(e_2_8.capacity(), || AtomicBool::new(false));
        let mut e_2_10 = Vec::with_capacity(heap.elements.e2pow10.values.len());
        e_2_10.resize_with(e_2_10.capacity(), || AtomicBool::new(false));
        let mut e_2_12 = Vec::with_capacity(heap.elements.e2pow12.values.len());
        e_2_12.resize_with(e_2_12.capacity(), || AtomicBool::new(false));
        let mut e_2_16 = Vec::with_capacity(heap.elements.e2pow16.values.len());
        e_2_16.resize_with(e_2_16.capacity(), || AtomicBool::new(false));
        let mut e_2_24 = Vec::with_capacity(heap.elements.e2pow24.values.len());
        e_2_24.resize_with(e_2_24.capacity(), || AtomicBool::new(false));
        let mut e_2_32 = Vec::with_capacity(heap.elements.e2pow32.values.len());
        e_2_32.resize_with(e_2_32.capacity(), || AtomicBool::new(false));
        let mut arrays = Vec::with_capacity(heap.arrays.len());
        arrays.resize_with(arrays.capacity(), || AtomicBool::new(false));
        let mut array_buffers = Vec::with_capacity(heap.array_buffers.len());
        array_buffers.resize_with(array_buffers.capacity(), || AtomicBool::new(false));
        let mut bigints = Vec::with_capacity(heap.bigints.len());
        bigints.resize_with(bigints.capacity(), || AtomicBool::new(false));
        let mut errors = Vec::with_capacity(heap.errors.len());
        errors.resize_with(errors.capacity(), || AtomicBool::new(false));
        let mut bound_functions = Vec::with_capacity(heap.bound_functions.len());
        bound_functions.resize_with(bound_functions.capacity(), || AtomicBool::new(false));
        let mut builtin_functions = Vec::with_capacity(heap.builtin_functions.len());
        builtin_functions.resize_with(builtin_functions.capacity(), || AtomicBool::new(false));
        let mut ecmascript_functions = Vec::with_capacity(heap.ecmascript_functions.len());
        ecmascript_functions
            .resize_with(ecmascript_functions.capacity(), || AtomicBool::new(false));
        let mut dates = Vec::with_capacity(heap.dates.len());
        dates.resize_with(dates.capacity(), || AtomicBool::new(false));
        let mut numbers = Vec::with_capacity(heap.numbers.len());
        numbers.resize_with(numbers.capacity(), || AtomicBool::new(false));
        let mut objects = Vec::with_capacity(heap.objects.len());
        objects.resize_with(objects.capacity(), || AtomicBool::new(false));
        let mut regexps = Vec::with_capacity(heap.regexps.len());
        regexps.resize_with(regexps.capacity(), || AtomicBool::new(false));
        let mut strings = Vec::with_capacity(heap.strings.len());
        strings.resize_with(strings.capacity(), || AtomicBool::new(false));
        let mut symbols = Vec::with_capacity(heap.symbols.len());
        symbols.resize_with(symbols.capacity(), || AtomicBool::new(false));
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

    pub(crate) fn from_mark_bits(marks: &[AtomicBool]) -> Self {
        let mut builder = CompactionListBuilder::default();
        marks.iter().for_each(|bit| {
            if bit.load(Ordering::Relaxed) {
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
            e_2_4: CompactionList::from_mark_bits(&bits.e_2_4),
            e_2_6: CompactionList::from_mark_bits(&bits.e_2_6),
            e_2_8: CompactionList::from_mark_bits(&bits.e_2_8),
            e_2_10: CompactionList::from_mark_bits(&bits.e_2_10),
            e_2_12: CompactionList::from_mark_bits(&bits.e_2_12),
            e_2_16: CompactionList::from_mark_bits(&bits.e_2_16),
            e_2_24: CompactionList::from_mark_bits(&bits.e_2_24),
            e_2_32: CompactionList::from_mark_bits(&bits.e_2_32),
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

pub(crate) trait HeapCompaction {
    fn compact_self_values(&mut self, compactions: &CompactionLists);
}

impl<T> HeapCompaction for Option<T>
where
    T: HeapCompaction,
{
    fn compact_self_values(&mut self, compactions: &CompactionLists) {
        if let Some(content) = self {
            content.compact_self_values(compactions);
        }
    }
}

impl HeapCompaction for ArrayIndex {
    fn compact_self_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.into_u32_index();
        *self = ArrayIndex::from_u32_index(
            self_index - compactions.arrays.get_shift_for_index(self_index),
        );
    }
}

impl HeapCompaction for NumberIndex {
    fn compact_self_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.into_u32_index();
        *self = NumberIndex::from_u32_index(
            self_index - compactions.numbers.get_shift_for_index(self_index),
        );
    }
}

impl HeapCompaction for ObjectIndex {
    fn compact_self_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.into_u32_index();
        *self = ObjectIndex::from_u32_index(
            self_index - compactions.objects.get_shift_for_index(self_index),
        );
    }
}

impl HeapCompaction for StringIndex {
    fn compact_self_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.into_u32_index();
        *self = StringIndex::from_u32_index(
            self_index - compactions.strings.get_shift_for_index(self_index),
        );
    }
}

impl HeapCompaction for Value {
    fn compact_self_values(&mut self, compactions: &CompactionLists) {
        match self {
            Self::Array(idx) => {
                idx.compact_self_values(compactions);
            }
            Self::Number(idx) => {
                idx.compact_self_values(compactions);
            }
            Self::Object(idx) => {
                idx.compact_self_values(compactions);
            }
            Self::String(idx) => {
                idx.compact_self_values(compactions);
            }
            _ => todo!(),
        }
    }
}

impl HeapCompaction for Object {
    fn compact_self_values(&mut self, compactions: &CompactionLists) {
        match self {
            Self::Object(idx) => idx.compact_self_values(compactions),
            Self::Array(idx) => idx.compact_self_values(compactions),
            _ => todo!(),
        }
    }
}

impl HeapCompaction for ElementsVector {
    fn compact_self_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.elements_index.into_u32_index();
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
        self.elements_index = ElementIndex::from_u32_index(self_index - shift);
    }
}

impl HeapCompaction for ObjectHeapData {
    fn compact_self_values(&mut self, compactions: &CompactionLists) {
        self.keys.compact_self_values(compactions);
        self.values.compact_self_values(compactions);
        self.prototype.compact_self_values(compactions);
    }
}
