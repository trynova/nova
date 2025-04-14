// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod element_array;
mod heap_bits;
mod heap_constants;
pub(crate) mod heap_gc;
pub mod indexes;
mod object_entry;

use core::{cell::RefCell, ops::Index};

pub(crate) use self::heap_constants::{
    IntrinsicConstructorIndexes, IntrinsicFunctionIndexes, IntrinsicObjectIndexes,
    IntrinsicPrimitiveObjectIndexes, LAST_WELL_KNOWN_SYMBOL_INDEX, WellKnownSymbolIndexes,
    intrinsic_function_count, intrinsic_object_count, intrinsic_primitive_object_count,
};
#[cfg(test)]
pub(crate) use self::heap_constants::{
    LAST_INTRINSIC_CONSTRUCTOR_INDEX, LAST_INTRINSIC_FUNCTION_INDEX, LAST_INTRINSIC_OBJECT_INDEX,
};
pub(crate) use self::object_entry::{ObjectEntry, ObjectEntryPropertyDescriptor};
use self::{
    element_array::{
        ElementArray2Pow4, ElementArray2Pow6, ElementArray2Pow8, ElementArray2Pow10,
        ElementArray2Pow12, ElementArray2Pow16, ElementArray2Pow24, ElementArray2Pow32,
        ElementArrays,
    },
    indexes::{NumberIndex, ObjectIndex},
};
#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::data::DateHeapData;
#[cfg(feature = "regexp")]
use crate::ecmascript::builtins::regexp::RegExpHeapData;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::data::SharedArrayBufferHeapData;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::builtins::{
    ArrayBufferHeapData,
    data_view::{DataView, data::DataViewHeapData},
    typed_array::{TypedArray, data::TypedArrayHeapData},
};
#[cfg(feature = "set")]
use crate::ecmascript::builtins::{
    keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIteratorHeapData,
    set::data::SetHeapData,
};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{
    weak_map::data::WeakMapHeapData, weak_ref::data::WeakRefHeapData,
    weak_set::data::WeakSetHeapData,
};
use crate::{
    ecmascript::{
        builtins::{
            ArrayBuffer, ArrayHeapData,
            array_buffer::DetachKey,
            async_generator_objects::AsyncGeneratorHeapData,
            control_abstraction_objects::{
                async_function_objects::await_reaction::AwaitReaction,
                generator_objects::GeneratorHeapData,
                promise_objects::promise_abstract_operations::{
                    promise_reaction_records::PromiseReactionRecord,
                    promise_resolving_functions::PromiseResolvingFunctionHeapData,
                },
            },
            embedder_object::data::EmbedderObjectHeapData,
            error::ErrorHeapData,
            finalization_registry::data::FinalizationRegistryHeapData,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIteratorHeapData,
            keyed_collections::map_objects::map_iterator_objects::map_iterator::MapIteratorHeapData,
            map::data::MapHeapData,
            module::{Module, data::ModuleHeapData},
            primitive_objects::PrimitiveObjectHeapData,
            promise::data::PromiseHeapData,
            proxy::data::ProxyHeapData,
        },
        execution::{Environments, Realm, RealmRecord},
        scripts_and_modules::{
            script::{Script, ScriptRecord},
            source_code::SourceCodeHeapData,
        },
        types::{
            BUILTIN_STRINGS_LIST, BigIntHeapData, BoundFunctionHeapData,
            BuiltinConstructorHeapData, BuiltinFunctionHeapData, ECMAScriptFunctionHeapData,
            HeapNumber, HeapString, NumberHeapData, Object, ObjectHeapData, OrdinaryObject, String,
            StringHeapData, SymbolHeapData, bigint::HeapBigInt,
        },
    },
    engine::{
        ExecutableHeapData,
        context::{Bindable, NoGcScope},
        rootable::HeapRootData,
    },
};
#[cfg(feature = "array-buffer")]
use ahash::AHashMap;
use hashbrown::HashTable;
pub(crate) use heap_bits::{CompactionLists, HeapMarkAndSweep, WorkQueues};
use wtf8::Wtf8;

#[derive(Debug)]
pub struct Heap {
    #[cfg(feature = "array-buffer")]
    pub array_buffers: Vec<Option<ArrayBufferHeapData<'static>>>,
    #[cfg(feature = "array-buffer")]
    pub array_buffer_detach_keys: AHashMap<ArrayBuffer<'static>, DetachKey>,
    pub arrays: Vec<Option<ArrayHeapData<'static>>>,
    pub array_iterators: Vec<Option<ArrayIteratorHeapData<'static>>>,
    pub async_generators: Vec<Option<AsyncGeneratorHeapData<'static>>>,
    pub(crate) await_reactions: Vec<Option<AwaitReaction<'static>>>,
    pub bigints: Vec<Option<BigIntHeapData>>,
    pub bound_functions: Vec<Option<BoundFunctionHeapData<'static>>>,
    pub builtin_constructors: Vec<Option<BuiltinConstructorHeapData<'static>>>,
    pub builtin_functions: Vec<Option<BuiltinFunctionHeapData<'static>>>,
    #[cfg(feature = "array-buffer")]
    pub data_views: Vec<Option<DataViewHeapData<'static>>>,
    #[cfg(feature = "array-buffer")]
    pub data_view_byte_lengths: AHashMap<DataView<'static>, usize>,
    #[cfg(feature = "array-buffer")]
    pub data_view_byte_offsets: AHashMap<DataView<'static>, usize>,
    #[cfg(feature = "date")]
    pub dates: Vec<Option<DateHeapData<'static>>>,
    pub ecmascript_functions: Vec<Option<ECMAScriptFunctionHeapData<'static>>>,
    /// ElementsArrays is where all element arrays live;
    /// Element arrays are static arrays of Values plus
    /// a HashMap of possible property descriptors.
    pub elements: ElementArrays,
    pub embedder_objects: Vec<Option<EmbedderObjectHeapData>>,
    pub environments: Environments,
    pub errors: Vec<Option<ErrorHeapData<'static>>>,
    /// Stores compiled bytecodes
    pub(crate) executables: Vec<ExecutableHeapData<'static>>,
    pub finalization_registrys: Vec<Option<FinalizationRegistryHeapData<'static>>>,
    pub generators: Vec<Option<GeneratorHeapData<'static>>>,
    pub(crate) globals: RefCell<Vec<Option<HeapRootData>>>,
    pub maps: Vec<Option<MapHeapData<'static>>>,
    pub map_iterators: Vec<Option<MapIteratorHeapData<'static>>>,
    pub numbers: Vec<Option<NumberHeapData>>,
    pub objects: Vec<Option<ObjectHeapData<'static>>>,
    pub primitive_objects: Vec<Option<PrimitiveObjectHeapData<'static>>>,
    pub promise_reaction_records: Vec<Option<PromiseReactionRecord<'static>>>,
    pub promise_resolving_functions: Vec<Option<PromiseResolvingFunctionHeapData<'static>>>,
    pub promises: Vec<Option<PromiseHeapData<'static>>>,
    pub proxys: Vec<Option<ProxyHeapData<'static>>>,
    pub realms: Vec<Option<RealmRecord<'static>>>,
    #[cfg(feature = "regexp")]
    pub regexps: Vec<Option<RegExpHeapData<'static>>>,
    #[cfg(feature = "set")]
    pub sets: Vec<Option<SetHeapData<'static>>>,
    #[cfg(feature = "set")]
    pub set_iterators: Vec<Option<SetIteratorHeapData<'static>>>,
    #[cfg(feature = "shared-array-buffer")]
    pub shared_array_buffers: Vec<Option<SharedArrayBufferHeapData<'static>>>,
    pub symbols: Vec<Option<SymbolHeapData<'static>>>,
    #[cfg(feature = "array-buffer")]
    pub typed_arrays: Vec<Option<TypedArrayHeapData<'static>>>,
    #[cfg(feature = "array-buffer")]
    pub typed_array_byte_lengths: AHashMap<TypedArray<'static>, usize>,
    #[cfg(feature = "array-buffer")]
    pub typed_array_byte_offsets: AHashMap<TypedArray<'static>, usize>,
    #[cfg(feature = "array-buffer")]
    pub typed_array_array_lengths: AHashMap<TypedArray<'static>, usize>,
    #[cfg(feature = "weak-refs")]
    pub weak_maps: Vec<Option<WeakMapHeapData<'static>>>,
    #[cfg(feature = "weak-refs")]
    pub weak_refs: Vec<Option<WeakRefHeapData<'static>>>,
    #[cfg(feature = "weak-refs")]
    pub weak_sets: Vec<Option<WeakSetHeapData<'static>>>,
    pub modules: Vec<Option<ModuleHeapData<'static>>>,
    pub scripts: Vec<Option<ScriptRecord<'static>>>,
    // Parsed ASTs referred by functions must be dropped after functions.
    // These are held in the SourceCodeHeapData structs.
    pub(crate) source_codes: Vec<Option<SourceCodeHeapData<'static>>>,
    // But: Source code string data is in the string heap. We need to thus drop
    // the strings only after the source ASTs drop.
    pub strings: Vec<Option<StringHeapData>>,
    pub string_lookup_table: HashTable<HeapString<'static>>,
    pub string_hasher: ahash::RandomState,
    /// Counts allocations for garbage collection triggering.
    #[cfg(feature = "interleaved-gc")]
    pub(crate) alloc_counter: usize,
}

pub trait CreateHeapData<T, F> {
    /// Creates a [`Value`] from the given data. Allocating the data is **not**
    /// guaranteed.
    fn create(&mut self, data: T) -> F;
}

impl CreateHeapData<&str, String<'static>> for Heap {
    fn create(&mut self, data: &str) -> String<'static> {
        if let Ok(value) = String::try_from(data) {
            value
        } else {
            // SAFETY: String couldn't be represented as a SmallString.
            unsafe { self.alloc_str(data) }
        }
    }
}

impl CreateHeapData<std::string::String, String<'static>> for Heap {
    fn create(&mut self, data: std::string::String) -> String<'static> {
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
            #[cfg(feature = "array-buffer")]
            array_buffers: Vec::with_capacity(1024),
            #[cfg(feature = "array-buffer")]
            array_buffer_detach_keys: AHashMap::with_capacity(0),
            arrays: Vec::with_capacity(1024),
            array_iterators: Vec::with_capacity(256),
            async_generators: Vec::with_capacity(0),
            await_reactions: Vec::with_capacity(1024),
            bigints: Vec::with_capacity(1024),
            bound_functions: Vec::with_capacity(256),
            builtin_constructors: Vec::with_capacity(256),
            builtin_functions: Vec::with_capacity(1024),
            #[cfg(feature = "array-buffer")]
            data_views: Vec::with_capacity(0),
            #[cfg(feature = "array-buffer")]
            data_view_byte_lengths: AHashMap::with_capacity(0),
            #[cfg(feature = "array-buffer")]
            data_view_byte_offsets: AHashMap::with_capacity(0),
            #[cfg(feature = "date")]
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
            executables: Vec::with_capacity(1024),
            source_codes: Vec::with_capacity(0),
            finalization_registrys: Vec::with_capacity(0),
            generators: Vec::with_capacity(1024),
            globals: RefCell::new(Vec::with_capacity(1024)),
            maps: Vec::with_capacity(128),
            map_iterators: Vec::with_capacity(128),
            modules: Vec::with_capacity(0),
            numbers: Vec::with_capacity(1024),
            objects: Vec::with_capacity(1024),
            primitive_objects: Vec::with_capacity(0),
            promise_reaction_records: Vec::with_capacity(0),
            promise_resolving_functions: Vec::with_capacity(0),
            promises: Vec::with_capacity(0),
            proxys: Vec::with_capacity(0),
            realms: Vec::with_capacity(1),
            #[cfg(feature = "regexp")]
            regexps: Vec::with_capacity(1024),
            scripts: Vec::with_capacity(1),
            #[cfg(feature = "set")]
            sets: Vec::with_capacity(128),
            #[cfg(feature = "set")]
            set_iterators: Vec::with_capacity(128),
            #[cfg(feature = "shared-array-buffer")]
            shared_array_buffers: Vec::with_capacity(0),
            strings: Vec::with_capacity(1024),
            string_lookup_table: HashTable::with_capacity(1024),
            string_hasher: ahash::RandomState::new(),
            symbols: Vec::with_capacity(1024),
            #[cfg(feature = "array-buffer")]
            typed_arrays: Vec::with_capacity(0),
            #[cfg(feature = "array-buffer")]
            typed_array_byte_lengths: AHashMap::with_capacity(0),
            #[cfg(feature = "array-buffer")]
            typed_array_byte_offsets: AHashMap::with_capacity(0),
            #[cfg(feature = "array-buffer")]
            typed_array_array_lengths: AHashMap::with_capacity(0),
            #[cfg(feature = "weak-refs")]
            weak_maps: Vec::with_capacity(0),
            #[cfg(feature = "weak-refs")]
            weak_refs: Vec::with_capacity(0),
            #[cfg(feature = "weak-refs")]
            weak_sets: Vec::with_capacity(0),
            #[cfg(feature = "interleaved-gc")]
            alloc_counter: 0,
        };

        for builtin_string in BUILTIN_STRINGS_LIST {
            unsafe { heap.alloc_static_str(builtin_string) };
        }

        heap
    }

    pub(crate) fn add_module<'a>(
        &mut self,
        module: ModuleHeapData,
        _: NoGcScope<'a, '_>,
    ) -> Module<'a> {
        self.modules.push(Some(module.unbind()));
        #[cfg(feature = "interleaved-gc")]
        {
            self.alloc_counter += core::mem::size_of::<Option<ModuleHeapData<'static>>>();
        }
        Module::last(&self.modules)
    }

    pub(crate) fn add_realm<'a>(&mut self, realm: RealmRecord, _: NoGcScope<'a, '_>) -> Realm<'a> {
        self.realms.push(Some(realm.unbind()));
        #[cfg(feature = "interleaved-gc")]
        {
            self.alloc_counter += core::mem::size_of::<Option<RealmRecord<'static>>>();
        }
        Realm::last(&self.realms)
    }

    pub(crate) fn add_script<'a>(
        &mut self,
        script: ScriptRecord,
        _: NoGcScope<'a, '_>,
    ) -> Script<'a> {
        self.scripts.push(Some(script.unbind()));
        #[cfg(feature = "interleaved-gc")]
        {
            self.alloc_counter += core::mem::size_of::<Option<ScriptRecord<'static>>>();
        }
        Script::last(&self.scripts)
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
    pub(crate) unsafe fn alloc_str(&mut self, message: &str) -> String<'static> {
        let found = self.find_equal_string(message);
        match found {
            Ok(string) => string,
            Err(hash) => {
                let data = StringHeapData::from_str(message);
                self.create((data, hash))
            }
        }
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
    unsafe fn alloc_string(&mut self, message: std::string::String) -> String<'static> {
        let found = self.find_equal_string(message.as_str());
        match found {
            Ok(string) => string,
            Err(hash) => {
                let data = StringHeapData::from_string(message);
                self.create((data, hash))
            }
        }
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
    pub(crate) unsafe fn alloc_static_str(&mut self, message: &'static str) -> String<'static> {
        let found = self.find_equal_string(message);
        match found {
            Ok(string) => string,
            Err(hash) => {
                let data = StringHeapData::from_static_str(message);
                self.create((data, hash))
            }
        }
    }

    /// Find existing heap String or return the strings hash.
    fn find_equal_string(&self, message: &str) -> Result<String<'static>, u64> {
        debug_assert!(message.len() > 7);
        let message = Wtf8::from_str(message);
        let hash = self.string_hasher.hash_one(message);
        self.string_lookup_table
            .find(hash, |heap_string| {
                let heap_str = self.strings[heap_string.get_index()]
                    .as_ref()
                    .unwrap()
                    .as_wtf8();
                heap_str == message
            })
            .map(|&heap_string| heap_string.into())
            .ok_or(hash)
    }

    /// Allocate a 64-bit floating point number onto the Agent heap
    ///
    /// # Safety
    ///
    /// The number being allocated must not be representable
    /// as a SmallInteger or f32. All stack-allocated numbers must be
    /// inequal to any heap-allocated number.
    pub unsafe fn alloc_number<'gc>(&mut self, number: f64) -> HeapNumber<'gc> {
        debug_assert!(number.fract() != 0.0 || number as f32 as f64 != number);
        self.numbers.push(Some(number.into()));
        #[cfg(feature = "interleaved-gc")]
        {
            self.alloc_counter += core::mem::size_of::<Option<NumberHeapData>>();
        }
        HeapNumber(NumberIndex::last(&self.numbers))
    }

    pub(crate) fn create_null_object(
        &mut self,
        entries: &[ObjectEntry],
    ) -> OrdinaryObject<'static> {
        let (keys, values) = self.elements.create_object_entries(entries);
        let object_data = ObjectHeapData {
            extensible: true,
            keys: keys.unbind(),
            values: values.unbind(),
            prototype: None,
        };
        self.objects.push(Some(object_data));
        #[cfg(feature = "interleaved-gc")]
        {
            self.alloc_counter += core::mem::size_of::<Option<ObjectHeapData<'static>>>();
        }
        ObjectIndex::last(&self.objects).into()
    }

    pub(crate) fn create_object_with_prototype(
        &mut self,
        prototype: Object,
        entries: &[ObjectEntry],
    ) -> OrdinaryObject<'static> {
        let (keys, values) = self.elements.create_object_entries(entries);
        let object_data = ObjectHeapData {
            extensible: true,
            keys: keys.unbind(),
            values: values.unbind(),
            prototype: Some(prototype.unbind()),
        };
        self.objects.push(Some(object_data));
        #[cfg(feature = "interleaved-gc")]
        {
            self.alloc_counter += core::mem::size_of::<Option<ObjectHeapData<'static>>>();
        }
        ObjectIndex::last(&self.objects).into()
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

/// A partial view to the Agent's heap that allows accessing primitive value
/// heap data.
pub(crate) struct PrimitiveHeap<'a> {
    pub(crate) bigints: &'a Vec<Option<BigIntHeapData>>,
    pub(crate) numbers: &'a Vec<Option<NumberHeapData>>,
    pub(crate) strings: &'a Vec<Option<StringHeapData>>,
}

impl PrimitiveHeap<'_> {
    pub(crate) fn new<'a>(
        bigints: &'a Vec<Option<BigIntHeapData>>,
        numbers: &'a Vec<Option<NumberHeapData>>,
        strings: &'a Vec<Option<StringHeapData>>,
    ) -> PrimitiveHeap<'a> {
        PrimitiveHeap {
            bigints,
            numbers,
            strings,
        }
    }
}

/// Helper trait for primitive heap data indexing.
pub(crate) trait PrimitiveHeapIndexable:
    Index<HeapNumber<'static>, Output = f64>
    + Index<HeapString<'static>, Output = StringHeapData>
    + Index<HeapBigInt<'static>, Output = BigIntHeapData>
{
}

impl PrimitiveHeapIndexable for PrimitiveHeap<'_> {}

#[test]
fn init_heap() {
    let heap = Heap::new();
    println!("{:#?}", heap);
}
