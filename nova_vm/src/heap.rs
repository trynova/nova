// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod element_array;
mod heap_bits;
mod heap_constants;
mod heap_gc;
pub mod indexes;
mod object_entry;

pub(crate) use self::heap_constants::{
    intrinsic_function_count, intrinsic_object_count, intrinsic_primitive_object_count,
    IntrinsicConstructorIndexes, IntrinsicFunctionIndexes, IntrinsicObjectIndexes,
    IntrinsicPrimitiveObjectIndexes, WellKnownSymbolIndexes,
};
#[cfg(test)]
pub(crate) use self::heap_constants::{
    LAST_INTRINSIC_CONSTRUCTOR_INDEX, LAST_INTRINSIC_FUNCTION_INDEX, LAST_INTRINSIC_OBJECT_INDEX,
    LAST_WELL_KNOWN_SYMBOL_INDEX,
};
pub(crate) use self::object_entry::{ObjectEntry, ObjectEntryPropertyDescriptor};
use self::{
    element_array::{
        ElementArray2Pow10, ElementArray2Pow12, ElementArray2Pow16, ElementArray2Pow24,
        ElementArray2Pow32, ElementArray2Pow4, ElementArray2Pow6, ElementArray2Pow8, ElementArrays,
    },
    indexes::{NumberIndex, ObjectIndex, StringIndex},
};
use crate::ecmascript::{
    builtins::{
        control_abstraction_objects::promise_objects::promise_abstract_operations::{
            promise_reaction_records::PromiseReactionRecord,
            promise_resolving_functions::PromiseResolvingFunctionHeapData,
        },
        data_view::data::DataViewHeapData,
        date::data::DateHeapData,
        embedder_object::data::EmbedderObjectHeapData,
        error::ErrorHeapData,
        finalization_registry::data::FinalizationRegistryHeapData,
        map::data::MapHeapData,
        module::data::ModuleHeapData,
        primitive_objects::PrimitiveObjectHeapData,
        promise::data::PromiseHeapData,
        proxy::data::ProxyHeapData,
        regexp::RegExpHeapData,
        set::data::SetHeapData,
        shared_array_buffer::data::SharedArrayBufferHeapData,
        typed_array::data::TypedArrayHeapData,
        weak_map::data::WeakMapHeapData,
        weak_ref::data::WeakRefHeapData,
        weak_set::data::WeakSetHeapData,
    },
    scripts_and_modules::eval_source::EvalSourceHeapData,
    types::{HeapNumber, HeapString, OrdinaryObject, BUILTIN_STRINGS_LIST},
};
use crate::ecmascript::{
    builtins::{ArrayBufferHeapData, ArrayHeapData},
    execution::{Environments, Realm, RealmIdentifier},
    scripts_and_modules::{
        module::ModuleIdentifier,
        script::{Script, ScriptIdentifier},
    },
    types::{
        BigIntHeapData, BoundFunctionHeapData, BuiltinFunctionHeapData, ECMAScriptFunctionHeapData,
        NumberHeapData, Object, ObjectHeapData, String, StringHeapData, SymbolHeapData, Value,
    },
};
pub(crate) use heap_bits::{CompactionLists, HeapMarkAndSweep, WorkQueues};

#[derive(Debug)]
pub struct Heap {
    pub array_buffers: Vec<Option<ArrayBufferHeapData>>,
    pub arrays: Vec<Option<ArrayHeapData>>,
    pub bigints: Vec<Option<BigIntHeapData>>,
    pub bound_functions: Vec<Option<BoundFunctionHeapData>>,
    pub builtin_functions: Vec<Option<BuiltinFunctionHeapData>>,
    pub data_views: Vec<Option<DataViewHeapData>>,
    pub dates: Vec<Option<DateHeapData>>,
    pub ecmascript_functions: Vec<Option<ECMAScriptFunctionHeapData>>,
    /// ElementsArrays is where all element arrays live;
    /// Element arrays are static arrays of Values plus
    /// a HashMap of possible property descriptors.
    pub elements: ElementArrays,
    pub embedder_objects: Vec<Option<EmbedderObjectHeapData>>,
    pub environments: Environments,
    pub errors: Vec<Option<ErrorHeapData>>,
    pub(crate) eval_sources: Vec<Option<EvalSourceHeapData>>,
    pub finalization_registrys: Vec<Option<FinalizationRegistryHeapData>>,
    pub globals: Vec<Value>,
    pub maps: Vec<Option<MapHeapData>>,
    pub modules: Vec<Option<ModuleHeapData>>,
    pub numbers: Vec<Option<NumberHeapData>>,
    pub objects: Vec<Option<ObjectHeapData>>,
    pub primitive_objects: Vec<Option<PrimitiveObjectHeapData>>,
    pub promise_reaction_records: Vec<Option<PromiseReactionRecord>>,
    pub promise_resolving_functions: Vec<Option<PromiseResolvingFunctionHeapData>>,
    pub promises: Vec<Option<PromiseHeapData>>,
    pub proxys: Vec<Option<ProxyHeapData>>,
    pub realms: Vec<Option<Realm>>,
    pub regexps: Vec<Option<RegExpHeapData>>,
    pub scripts: Vec<Option<Script>>,
    pub sets: Vec<Option<SetHeapData>>,
    pub shared_array_buffers: Vec<Option<SharedArrayBufferHeapData>>,
    pub strings: Vec<Option<StringHeapData>>,
    pub symbols: Vec<Option<SymbolHeapData>>,
    pub typed_arrays: Vec<Option<TypedArrayHeapData>>,
    pub weak_maps: Vec<Option<WeakMapHeapData>>,
    pub weak_refs: Vec<Option<WeakRefHeapData>>,
    pub weak_sets: Vec<Option<WeakSetHeapData>>,
}

pub trait CreateHeapData<T, F> {
    /// Creates a [`Value`] from the given data. Allocating the data is **not**
    /// guaranteed.
    fn create(&mut self, data: T) -> F;
}

impl CreateHeapData<&str, String> for Heap {
    fn create(&mut self, data: &str) -> String {
        if let Ok(value) = String::try_from(data) {
            value
        } else {
            // SAFETY: String couldn't be represented as a SmallString.
            unsafe { self.alloc_str(data) }
        }
    }
}

impl CreateHeapData<std::string::String, String> for Heap {
    fn create(&mut self, data: std::string::String) -> String {
        if let Ok(value) = String::try_from(data.as_str()) {
            value
        } else {
            // SAFETY: String couldn't be represented as a SmallString.
            unsafe { self.alloc_string(data) }
        }
    }
}

impl Heap {
    pub fn new() -> Heap {
        let mut heap = Heap {
            array_buffers: Vec::with_capacity(1024),
            arrays: Vec::with_capacity(1024),
            bigints: Vec::with_capacity(1024),
            bound_functions: Vec::with_capacity(256),
            builtin_functions: Vec::with_capacity(1024),
            data_views: Vec::with_capacity(0),
            dates: Vec::with_capacity(1024),
            ecmascript_functions: Vec::with_capacity(1024),
            elements: ElementArrays {
                e2pow4: ElementArray2Pow4::with_capacity(1024),
                e2pow6: ElementArray2Pow6::with_capacity(1024),
                e2pow8: ElementArray2Pow8::default(),
                e2pow10: ElementArray2Pow10::default(),
                e2pow12: ElementArray2Pow12::default(),
                e2pow16: ElementArray2Pow16::default(),
                e2pow24: ElementArray2Pow24::default(),
                e2pow32: ElementArray2Pow32::default(),
            },
            embedder_objects: Vec::with_capacity(0),
            environments: Default::default(),
            errors: Vec::with_capacity(1024),
            finalization_registrys: Vec::with_capacity(0),
            globals: Vec::with_capacity(1024),
            maps: Vec::with_capacity(128),
            numbers: Vec::with_capacity(1024),
            objects: Vec::with_capacity(1024),
            primitive_objects: Vec::with_capacity(0),
            promise_reaction_records: Vec::with_capacity(0),
            promise_resolving_functions: Vec::with_capacity(0),
            promises: Vec::with_capacity(0),
            proxys: Vec::with_capacity(0),
            realms: Vec::with_capacity(1),
            regexps: Vec::with_capacity(1024),
            sets: Vec::with_capacity(128),
            shared_array_buffers: Vec::with_capacity(0),
            strings: Vec::with_capacity(1024),
            symbols: Vec::with_capacity(1024),
            typed_arrays: Vec::with_capacity(0),
            weak_maps: Vec::with_capacity(0),
            weak_refs: Vec::with_capacity(0),
            weak_sets: Vec::with_capacity(0),
            // Drop scripts, modules, and eval sources last to ensure that all
            // objects referring to them have dropped first.
            eval_sources: Vec::with_capacity(0),
            modules: Vec::with_capacity(0),
            scripts: Vec::with_capacity(1),
        };

        heap.strings.extend_from_slice(
            &BUILTIN_STRINGS_LIST
                .map(|builtin_string| Some(StringHeapData::from_static_str(builtin_string))),
        );

        heap
    }

    pub(crate) fn add_module(&mut self, module: ModuleHeapData) -> ModuleIdentifier {
        self.modules.push(Some(module));
        ModuleIdentifier::last(&self.modules)
    }

    pub(crate) fn add_realm(&mut self, realm: Realm) -> RealmIdentifier {
        self.realms.push(Some(realm));
        RealmIdentifier::last(&self.realms)
    }

    pub(crate) fn add_script(&mut self, script: Script) -> ScriptIdentifier {
        self.scripts.push(Some(script));
        ScriptIdentifier::last(&self.scripts)
    }

    /// Allocate a string onto the Agent heap
    ///
    /// This method will currently iterate through all heap strings to look for
    /// a possible matching string and if found will return its HeapString
    /// instead of allocating a copy.
    ///
    /// # Safety
    ///
    /// The string being allocated must not be representable as a
    /// SmallString. All SmallStrings must be kept on the stack to ensure that
    /// comparison between heap allocated strings and SmallStrings can be
    /// guaranteed to never equal true.
    pub(crate) unsafe fn alloc_str(&mut self, message: &str) -> String {
        let found = self.find_equal_string(message);
        if let Some(idx) = found {
            return idx;
        }
        let data = StringHeapData::from_str(message);
        self.create(data)
    }

    /// Allocate a static string onto the Agent heap
    ///
    /// This method will currently iterate through all heap strings to look for
    /// a possible matching string and if found will return its HeapString
    /// instead of allocating a copy.
    ///
    /// # Safety
    ///
    /// The string being allocated must not be representable as a
    /// SmallString. All SmallStrings must be kept on the stack to ensure that
    /// comparison between heap allocated strings and SmallStrings can be
    /// guaranteed to never equal true.
    unsafe fn alloc_string(&mut self, message: std::string::String) -> String {
        let found = self.find_equal_string(message.as_str());
        if let Some(idx) = found {
            return idx;
        }
        let data = StringHeapData::from_string(message);
        self.create(data)
    }

    /// Allocate a static string onto the Agent heap
    ///
    /// This method will currently iterate through all heap strings to look for
    /// a possible matching string and if found will return its HeapString
    /// instead of allocating a copy.
    ///
    /// # Safety
    ///
    /// The string being allocated must not be representable as a
    /// SmallString. All SmallStrings must be kept on the stack to ensure that
    /// comparison between heap allocated strings and SmallStrings can be
    /// guaranteed to never equal true.
    pub(crate) unsafe fn alloc_static_str(&mut self, message: &'static str) -> String {
        let found = self.find_equal_string(message);
        if let Some(idx) = found {
            return idx;
        }
        let data = StringHeapData::from_static_str(message);
        self.create(data)
    }

    fn find_equal_string(&self, message: &str) -> Option<String> {
        debug_assert!(message.len() > 7);
        self.strings
            .iter()
            .position(|opt| opt.as_ref().map_or(false, |data| data.as_str() == message))
            .map(|found_index| HeapString(StringIndex::from_index(found_index)).into())
    }

    /// Allocate a 64-bit floating point number onto the Agent heap
    ///
    /// # Safety
    ///
    /// The number being allocated must not be representable
    /// as a SmallInteger or f32. All stack-allocated numbers must be
    /// inequal to any heap-allocated number.
    pub unsafe fn alloc_number(&mut self, number: f64) -> HeapNumber {
        debug_assert!(number.fract() != 0.0 || number as f32 as f64 != number);
        self.numbers.push(Some(number.into()));
        HeapNumber(NumberIndex::last(&self.numbers))
    }

    pub(crate) fn create_null_object(&mut self, entries: &[ObjectEntry]) -> OrdinaryObject {
        let (keys, values) = self.elements.create_object_entries(entries);
        let object_data = ObjectHeapData {
            extensible: true,
            keys,
            values,
            prototype: None,
        };
        self.objects.push(Some(object_data));
        ObjectIndex::last(&self.objects).into()
    }

    pub(crate) fn create_object_with_prototype(
        &mut self,
        prototype: Object,
        entries: &[ObjectEntry],
    ) -> OrdinaryObject {
        let (keys, values) = self.elements.create_object_entries(entries);
        let object_data = ObjectHeapData {
            extensible: true,
            keys,
            values,
            prototype: Some(prototype),
        };
        self.objects.push(Some(object_data));
        ObjectIndex::last(&self.objects).into()
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn init_heap() {
    let heap = Heap::new();
    println!("{:#?}", heap);
}
