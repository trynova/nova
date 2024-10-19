use std::num::NonZeroU32;

use ahash::AHashMap;

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#[cfg(feature = "array-buffer")]
use super::indexes::TypedArrayIndex;
use super::{
    element_array::{ElementArrayKey, ElementDescriptor, ElementsVector},
    indexes::{BaseIndex, ElementIndex},
    Heap,
};
#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::Date;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::builtins::{data_view::DataView, ArrayBuffer};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
use crate::ecmascript::{
    builtins::{
        bound_function::BoundFunction,
        control_abstraction_objects::{
            async_function_objects::await_reaction::AwaitReactionIdentifier,
            generator_objects::Generator,
            promise_objects::promise_abstract_operations::{
                promise_reaction_records::PromiseReaction,
                promise_resolving_functions::BuiltinPromiseResolvingFunction,
            },
        },
        embedder_object::EmbedderObject,
        error::Error,
        finalization_registry::FinalizationRegistry,
        indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIterator,
        keyed_collections::{
            map_objects::map_iterator_objects::map_iterator::MapIterator,
            set_objects::set_iterator_objects::set_iterator::SetIterator,
        },
        map::Map,
        module::Module,
        primitive_objects::PrimitiveObject,
        promise::Promise,
        proxy::Proxy,
        regexp::RegExp,
        set::Set,
        Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
    },
    execution::{
        DeclarativeEnvironmentIndex, EnvironmentIndex, FunctionEnvironmentIndex,
        GlobalEnvironmentIndex, ObjectEnvironmentIndex, RealmIdentifier,
    },
    scripts_and_modules::{script::ScriptIdentifier, source_code::SourceCode},
    types::{
        bigint::HeapBigInt, HeapNumber, HeapString, OrdinaryObject, Symbol, Value,
        BUILTIN_STRINGS_LIST,
    },
};
use crate::engine::Executable;

#[derive(Debug)]
pub struct HeapBits {
    #[cfg(feature = "array-buffer")]
    pub array_buffers: Box<[bool]>,
    pub arrays: Box<[bool]>,
    pub array_iterators: Box<[bool]>,
    pub await_reactions: Box<[bool]>,
    pub bigints: Box<[bool]>,
    pub bound_functions: Box<[bool]>,
    pub builtin_constructors: Box<[bool]>,
    pub builtin_functions: Box<[bool]>,
    #[cfg(feature = "array-buffer")]
    pub data_views: Box<[bool]>,
    #[cfg(feature = "date")]
    pub dates: Box<[bool]>,
    pub declarative_environments: Box<[bool]>,
    pub e_2_10: Box<[(bool, u16)]>,
    pub e_2_12: Box<[(bool, u16)]>,
    pub e_2_16: Box<[(bool, u16)]>,
    pub e_2_24: Box<[(bool, u32)]>,
    pub e_2_32: Box<[(bool, u32)]>,
    pub e_2_4: Box<[(bool, u8)]>,
    pub e_2_6: Box<[(bool, u8)]>,
    pub e_2_8: Box<[(bool, u8)]>,
    pub ecmascript_functions: Box<[bool]>,
    pub embedder_objects: Box<[bool]>,
    pub errors: Box<[bool]>,
    pub executables: Box<[bool]>,
    pub source_codes: Box<[bool]>,
    pub finalization_registrys: Box<[bool]>,
    pub function_environments: Box<[bool]>,
    pub generators: Box<[bool]>,
    pub global_environments: Box<[bool]>,
    pub maps: Box<[bool]>,
    pub map_iterators: Box<[bool]>,
    pub modules: Box<[bool]>,
    pub numbers: Box<[bool]>,
    pub object_environments: Box<[bool]>,
    pub objects: Box<[bool]>,
    pub primitive_objects: Box<[bool]>,
    pub promise_reaction_records: Box<[bool]>,
    pub promise_resolving_functions: Box<[bool]>,
    pub promises: Box<[bool]>,
    pub proxys: Box<[bool]>,
    pub realms: Box<[bool]>,
    pub regexps: Box<[bool]>,
    pub scripts: Box<[bool]>,
    pub sets: Box<[bool]>,
    pub set_iterators: Box<[bool]>,
    #[cfg(feature = "shared-array-buffer")]
    pub shared_array_buffers: Box<[bool]>,
    pub strings: Box<[bool]>,
    pub symbols: Box<[bool]>,
    #[cfg(feature = "array-buffer")]
    pub typed_arrays: Box<[bool]>,
    #[cfg(feature = "weak-refs")]
    pub weak_maps: Box<[bool]>,
    #[cfg(feature = "weak-refs")]
    pub weak_refs: Box<[bool]>,
    #[cfg(feature = "weak-refs")]
    pub weak_sets: Box<[bool]>,
}

#[derive(Debug)]
pub(crate) struct WorkQueues {
    #[cfg(feature = "array-buffer")]
    pub array_buffers: Vec<ArrayBuffer>,
    pub arrays: Vec<Array>,
    pub array_iterators: Vec<ArrayIterator>,
    pub await_reactions: Vec<AwaitReactionIdentifier>,
    pub bigints: Vec<HeapBigInt>,
    pub bound_functions: Vec<BoundFunction>,
    pub builtin_constructors: Vec<BuiltinConstructorFunction>,
    pub builtin_functions: Vec<BuiltinFunction>,
    #[cfg(feature = "array-buffer")]
    pub data_views: Vec<DataView>,
    #[cfg(feature = "date")]
    pub dates: Vec<Date>,
    pub declarative_environments: Vec<DeclarativeEnvironmentIndex>,
    pub e_2_10: Vec<(ElementIndex, u32)>,
    pub e_2_12: Vec<(ElementIndex, u32)>,
    pub e_2_16: Vec<(ElementIndex, u32)>,
    pub e_2_24: Vec<(ElementIndex, u32)>,
    pub e_2_32: Vec<(ElementIndex, u32)>,
    pub e_2_4: Vec<(ElementIndex, u32)>,
    pub e_2_6: Vec<(ElementIndex, u32)>,
    pub e_2_8: Vec<(ElementIndex, u32)>,
    pub ecmascript_functions: Vec<ECMAScriptFunction>,
    pub embedder_objects: Vec<EmbedderObject>,
    pub source_codes: Vec<SourceCode>,
    pub errors: Vec<Error>,
    pub executables: Vec<Executable>,
    pub finalization_registrys: Vec<FinalizationRegistry>,
    pub function_environments: Vec<FunctionEnvironmentIndex>,
    pub generators: Vec<Generator>,
    pub global_environments: Vec<GlobalEnvironmentIndex>,
    pub maps: Vec<Map>,
    pub map_iterators: Vec<MapIterator>,
    pub modules: Vec<Module>,
    pub numbers: Vec<HeapNumber>,
    pub object_environments: Vec<ObjectEnvironmentIndex>,
    pub objects: Vec<OrdinaryObject>,
    pub primitive_objects: Vec<PrimitiveObject>,
    pub promises: Vec<Promise>,
    pub promise_reaction_records: Vec<PromiseReaction>,
    pub promise_resolving_functions: Vec<BuiltinPromiseResolvingFunction>,
    pub proxys: Vec<Proxy>,
    pub realms: Vec<RealmIdentifier>,
    pub regexps: Vec<RegExp>,
    pub scripts: Vec<ScriptIdentifier>,
    pub sets: Vec<Set>,
    pub set_iterators: Vec<SetIterator>,
    #[cfg(feature = "shared-array-buffer")]
    pub shared_array_buffers: Vec<SharedArrayBuffer>,
    pub strings: Vec<HeapString>,
    pub symbols: Vec<Symbol>,
    #[cfg(feature = "array-buffer")]
    pub typed_arrays: Vec<TypedArrayIndex>,
    #[cfg(feature = "weak-refs")]
    pub weak_maps: Vec<WeakMap>,
    #[cfg(feature = "weak-refs")]
    pub weak_refs: Vec<WeakRef>,
    #[cfg(feature = "weak-refs")]
    pub weak_sets: Vec<WeakSet>,
}

impl HeapBits {
    pub fn new(heap: &Heap) -> Self {
        #[cfg(feature = "array-buffer")]
        let array_buffers = vec![false; heap.array_buffers.len()];
        let arrays = vec![false; heap.arrays.len()];
        let array_iterators = vec![false; heap.array_iterators.len()];
        let await_reactions = vec![false; heap.await_reactions.len()];
        let bigints = vec![false; heap.bigints.len()];
        let bound_functions = vec![false; heap.bound_functions.len()];
        let builtin_constructors = vec![false; heap.builtin_constructors.len()];
        let builtin_functions = vec![false; heap.builtin_functions.len()];
        #[cfg(feature = "array-buffer")]
        let data_views = vec![false; heap.data_views.len()];
        #[cfg(feature = "date")]
        let dates = vec![false; heap.dates.len()];
        let declarative_environments = vec![false; heap.environments.declarative.len()];
        let e_2_10 = vec![(false, 0u16); heap.elements.e2pow10.values.len()];
        let e_2_12 = vec![(false, 0u16); heap.elements.e2pow12.values.len()];
        let e_2_16 = vec![(false, 0u16); heap.elements.e2pow16.values.len()];
        let e_2_24 = vec![(false, 0u32); heap.elements.e2pow24.values.len()];
        let e_2_32 = vec![(false, 0u32); heap.elements.e2pow32.values.len()];
        let e_2_4 = vec![(false, 0u8); heap.elements.e2pow4.values.len()];
        let e_2_6 = vec![(false, 0u8); heap.elements.e2pow6.values.len()];
        let e_2_8 = vec![(false, 0u8); heap.elements.e2pow8.values.len()];
        let ecmascript_functions = vec![false; heap.ecmascript_functions.len()];
        let embedder_objects = vec![false; heap.embedder_objects.len()];
        let errors = vec![false; heap.errors.len()];
        let executables = vec![false; heap.executables.len()];
        let source_codes = vec![false; heap.source_codes.len()];
        let finalization_registrys = vec![false; heap.finalization_registrys.len()];
        let function_environments = vec![false; heap.environments.function.len()];
        let generators = vec![false; heap.generators.len()];
        let global_environments = vec![false; heap.environments.global.len()];
        let maps = vec![false; heap.maps.len()];
        let map_iterators = vec![false; heap.map_iterators.len()];
        let modules = vec![false; heap.modules.len()];
        let numbers = vec![false; heap.numbers.len()];
        let object_environments = vec![false; heap.environments.object.len()];
        let objects = vec![false; heap.objects.len()];
        let primitive_objects = vec![false; heap.primitive_objects.len()];
        let promise_reaction_records = vec![false; heap.promise_reaction_records.len()];
        let promise_resolving_functions = vec![false; heap.promise_resolving_functions.len()];
        let promises = vec![false; heap.promises.len()];
        let proxys = vec![false; heap.proxys.len()];
        let realms = vec![false; heap.realms.len()];
        let regexps = vec![false; heap.regexps.len()];
        let scripts = vec![false; heap.scripts.len()];
        let sets = vec![false; heap.sets.len()];
        let set_iterators = vec![false; heap.set_iterators.len()];
        #[cfg(feature = "shared-array-buffer")]
        let shared_array_buffers = vec![false; heap.shared_array_buffers.len()];
        let strings = vec![false; heap.strings.len()];
        let symbols = vec![false; heap.symbols.len()];
        #[cfg(feature = "array-buffer")]
        let typed_arrays = vec![false; heap.typed_arrays.len()];
        #[cfg(feature = "weak-refs")]
        let weak_maps = vec![false; heap.weak_maps.len()];
        #[cfg(feature = "weak-refs")]
        let weak_refs = vec![false; heap.weak_refs.len()];
        #[cfg(feature = "weak-refs")]
        let weak_sets = vec![false; heap.weak_sets.len()];
        Self {
            #[cfg(feature = "array-buffer")]
            array_buffers: array_buffers.into_boxed_slice(),
            arrays: arrays.into_boxed_slice(),
            array_iterators: array_iterators.into_boxed_slice(),
            await_reactions: await_reactions.into_boxed_slice(),
            bigints: bigints.into_boxed_slice(),
            bound_functions: bound_functions.into_boxed_slice(),
            builtin_constructors: builtin_constructors.into_boxed_slice(),
            builtin_functions: builtin_functions.into_boxed_slice(),
            #[cfg(feature = "array-buffer")]
            data_views: data_views.into_boxed_slice(),
            #[cfg(feature = "date")]
            dates: dates.into_boxed_slice(),
            declarative_environments: declarative_environments.into_boxed_slice(),
            e_2_10: e_2_10.into_boxed_slice(),
            e_2_12: e_2_12.into_boxed_slice(),
            e_2_16: e_2_16.into_boxed_slice(),
            e_2_24: e_2_24.into_boxed_slice(),
            e_2_32: e_2_32.into_boxed_slice(),
            e_2_4: e_2_4.into_boxed_slice(),
            e_2_6: e_2_6.into_boxed_slice(),
            e_2_8: e_2_8.into_boxed_slice(),
            ecmascript_functions: ecmascript_functions.into_boxed_slice(),
            embedder_objects: embedder_objects.into_boxed_slice(),
            errors: errors.into_boxed_slice(),
            executables: executables.into_boxed_slice(),
            source_codes: source_codes.into_boxed_slice(),
            finalization_registrys: finalization_registrys.into_boxed_slice(),
            function_environments: function_environments.into_boxed_slice(),
            generators: generators.into_boxed_slice(),
            global_environments: global_environments.into_boxed_slice(),
            maps: maps.into_boxed_slice(),
            map_iterators: map_iterators.into_boxed_slice(),
            modules: modules.into_boxed_slice(),
            numbers: numbers.into_boxed_slice(),
            object_environments: object_environments.into_boxed_slice(),
            objects: objects.into_boxed_slice(),
            primitive_objects: primitive_objects.into_boxed_slice(),
            promise_reaction_records: promise_reaction_records.into_boxed_slice(),
            promise_resolving_functions: promise_resolving_functions.into_boxed_slice(),
            promises: promises.into_boxed_slice(),
            proxys: proxys.into_boxed_slice(),
            realms: realms.into_boxed_slice(),
            regexps: regexps.into_boxed_slice(),
            scripts: scripts.into_boxed_slice(),
            sets: sets.into_boxed_slice(),
            set_iterators: set_iterators.into_boxed_slice(),
            #[cfg(feature = "shared-array-buffer")]
            shared_array_buffers: shared_array_buffers.into_boxed_slice(),
            strings: strings.into_boxed_slice(),
            symbols: symbols.into_boxed_slice(),
            #[cfg(feature = "array-buffer")]
            typed_arrays: typed_arrays.into_boxed_slice(),
            #[cfg(feature = "weak-refs")]
            weak_maps: weak_maps.into_boxed_slice(),
            #[cfg(feature = "weak-refs")]
            weak_refs: weak_refs.into_boxed_slice(),
            #[cfg(feature = "weak-refs")]
            weak_sets: weak_sets.into_boxed_slice(),
        }
    }
}

impl WorkQueues {
    pub fn new(heap: &Heap) -> Self {
        Self {
            #[cfg(feature = "array-buffer")]
            array_buffers: Vec::with_capacity(heap.array_buffers.len() / 4),
            arrays: Vec::with_capacity(heap.arrays.len() / 4),
            array_iterators: Vec::with_capacity(heap.array_iterators.len() / 4),
            await_reactions: Vec::with_capacity(heap.await_reactions.len() / 4),
            bigints: Vec::with_capacity(heap.bigints.len() / 4),
            bound_functions: Vec::with_capacity(heap.bound_functions.len() / 4),
            builtin_constructors: Vec::with_capacity(heap.builtin_constructors.len() / 4),
            builtin_functions: Vec::with_capacity(heap.builtin_functions.len() / 4),
            #[cfg(feature = "array-buffer")]
            data_views: Vec::with_capacity(heap.data_views.len() / 4),
            #[cfg(feature = "date")]
            dates: Vec::with_capacity(heap.dates.len() / 4),
            declarative_environments: Vec::with_capacity(heap.environments.declarative.len() / 4),
            e_2_10: Vec::with_capacity(heap.elements.e2pow10.values.len() / 4),
            e_2_12: Vec::with_capacity(heap.elements.e2pow12.values.len() / 4),
            e_2_16: Vec::with_capacity(heap.elements.e2pow16.values.len() / 4),
            e_2_24: Vec::with_capacity(heap.elements.e2pow24.values.len() / 4),
            e_2_32: Vec::with_capacity(heap.elements.e2pow32.values.len() / 4),
            e_2_4: Vec::with_capacity(heap.elements.e2pow4.values.len() / 4),
            e_2_6: Vec::with_capacity(heap.elements.e2pow6.values.len() / 4),
            e_2_8: Vec::with_capacity(heap.elements.e2pow8.values.len() / 4),
            ecmascript_functions: Vec::with_capacity(heap.ecmascript_functions.len() / 4),
            embedder_objects: Vec::with_capacity(heap.embedder_objects.len() / 4),
            errors: Vec::with_capacity(heap.errors.len() / 4),
            executables: Vec::with_capacity(heap.executables.len() / 4),
            source_codes: Vec::with_capacity(heap.source_codes.len() / 4),
            finalization_registrys: Vec::with_capacity(heap.finalization_registrys.len() / 4),
            function_environments: Vec::with_capacity(heap.environments.function.len() / 4),
            generators: Vec::with_capacity(heap.generators.len() / 4),
            global_environments: Vec::with_capacity(heap.environments.global.len() / 4),
            maps: Vec::with_capacity(heap.maps.len() / 4),
            map_iterators: Vec::with_capacity(heap.map_iterators.len() / 4),
            modules: Vec::with_capacity(heap.modules.len() / 4),
            numbers: Vec::with_capacity(heap.numbers.len() / 4),
            object_environments: Vec::with_capacity(heap.environments.object.len() / 4),
            objects: Vec::with_capacity(heap.objects.len() / 4),
            primitive_objects: Vec::with_capacity(heap.primitive_objects.len() / 4),
            promise_reaction_records: Vec::with_capacity(heap.promise_reaction_records.len() / 4),
            promise_resolving_functions: Vec::with_capacity(
                heap.promise_resolving_functions.len() / 4,
            ),
            promises: Vec::with_capacity(heap.promises.len() / 4),
            proxys: Vec::with_capacity(heap.proxys.len() / 4),
            realms: Vec::with_capacity(heap.realms.len() / 4),
            regexps: Vec::with_capacity(heap.regexps.len() / 4),
            scripts: Vec::with_capacity(heap.scripts.len() / 4),
            sets: Vec::with_capacity(heap.sets.len() / 4),
            set_iterators: Vec::with_capacity(heap.set_iterators.len() / 4),
            #[cfg(feature = "shared-array-buffer")]
            shared_array_buffers: Vec::with_capacity(heap.shared_array_buffers.len() / 4),
            strings: Vec::with_capacity((heap.strings.len() / 4).max(BUILTIN_STRINGS_LIST.len())),
            symbols: Vec::with_capacity((heap.symbols.len() / 4).max(13)),
            #[cfg(feature = "array-buffer")]
            typed_arrays: Vec::with_capacity(heap.typed_arrays.len() / 4),
            #[cfg(feature = "weak-refs")]
            weak_maps: Vec::with_capacity(heap.weak_maps.len() / 4),
            #[cfg(feature = "weak-refs")]
            weak_refs: Vec::with_capacity(heap.weak_refs.len() / 4),
            #[cfg(feature = "weak-refs")]
            weak_sets: Vec::with_capacity(heap.weak_sets.len() / 4),
        }
    }

    pub fn push_elements_vector(&mut self, vec: &ElementsVector) {
        match vec.cap {
            ElementArrayKey::Empty => {}
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

    pub fn push_environment_index(&mut self, value: EnvironmentIndex) {
        match value {
            EnvironmentIndex::Declarative(idx) => self.declarative_environments.push(idx),
            EnvironmentIndex::Function(idx) => self.function_environments.push(idx),
            EnvironmentIndex::Global(idx) => self.global_environments.push(idx),
            EnvironmentIndex::Object(idx) => self.object_environments.push(idx),
        }
    }

    pub fn is_empty(&self) -> bool {
        let Self {
            #[cfg(feature = "array-buffer")]
            array_buffers,
            arrays,
            array_iterators,
            await_reactions,
            bigints,
            bound_functions,
            builtin_constructors,
            builtin_functions,
            #[cfg(feature = "array-buffer")]
            data_views,
            #[cfg(feature = "date")]
            dates,
            declarative_environments,
            e_2_10,
            e_2_12,
            e_2_16,
            e_2_24,
            e_2_32,
            e_2_4,
            e_2_6,
            e_2_8,
            ecmascript_functions,
            embedder_objects,
            source_codes,
            errors,
            executables,
            finalization_registrys,
            function_environments,
            generators,
            global_environments,
            maps,
            map_iterators,
            modules,
            numbers,
            object_environments,
            objects,
            primitive_objects,
            promises,
            promise_reaction_records,
            promise_resolving_functions,
            proxys,
            realms,
            regexps,
            scripts,
            sets,
            set_iterators,
            #[cfg(feature = "shared-array-buffer")]
            shared_array_buffers,
            strings,
            symbols,
            #[cfg(feature = "array-buffer")]
            typed_arrays,
            #[cfg(feature = "weak-refs")]
            weak_maps,
            #[cfg(feature = "weak-refs")]
            weak_refs,
            #[cfg(feature = "weak-refs")]
            weak_sets,
        } = self;

        #[cfg(not(feature = "date"))]
        let dates: &[bool; 0] = &[];
        #[cfg(not(feature = "array-buffer"))]
        let data_views: &[bool; 0] = &[];
        #[cfg(not(feature = "array-buffer"))]
        let array_buffers: &[bool; 0] = &[];
        #[cfg(not(feature = "array-buffer"))]
        let typed_arrays: &[bool; 0] = &[];
        #[cfg(not(feature = "shared-array-buffer"))]
        let shared_array_buffers: &[bool; 0] = &[];
        #[cfg(not(feature = "weak-refs"))]
        let weak_maps: &[bool; 0] = &[];
        #[cfg(not(feature = "weak-refs"))]
        let weak_refs: &[bool; 0] = &[];
        #[cfg(not(feature = "weak-refs"))]
        let weak_sets: &[bool; 0] = &[];

        array_buffers.is_empty()
            && arrays.is_empty()
            && array_iterators.is_empty()
            && await_reactions.is_empty()
            && bigints.is_empty()
            && bound_functions.is_empty()
            && builtin_constructors.is_empty()
            && builtin_functions.is_empty()
            && data_views.is_empty()
            && dates.is_empty()
            && declarative_environments.is_empty()
            && e_2_10.is_empty()
            && e_2_12.is_empty()
            && e_2_16.is_empty()
            && e_2_24.is_empty()
            && e_2_32.is_empty()
            && e_2_4.is_empty()
            && e_2_6.is_empty()
            && e_2_8.is_empty()
            && ecmascript_functions.is_empty()
            && embedder_objects.is_empty()
            && errors.is_empty()
            && executables.is_empty()
            && source_codes.is_empty()
            && finalization_registrys.is_empty()
            && function_environments.is_empty()
            && generators.is_empty()
            && global_environments.is_empty()
            && maps.is_empty()
            && map_iterators.is_empty()
            && modules.is_empty()
            && numbers.is_empty()
            && object_environments.is_empty()
            && objects.is_empty()
            && primitive_objects.is_empty()
            && promise_reaction_records.is_empty()
            && promise_resolving_functions.is_empty()
            && promises.is_empty()
            && proxys.is_empty()
            && realms.is_empty()
            && regexps.is_empty()
            && scripts.is_empty()
            && sets.is_empty()
            && set_iterators.is_empty()
            && shared_array_buffers.is_empty()
            && strings.is_empty()
            && symbols.is_empty()
            && typed_arrays.is_empty()
            && weak_maps.is_empty()
            && weak_refs.is_empty()
            && weak_sets.is_empty()
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

    pub(crate) fn shift_index<T: ?Sized>(&self, index: &mut BaseIndex<T>) {
        let base_index = index.into_u32_index();
        *index = BaseIndex::from_u32_index(base_index - self.get_shift_for_index(base_index));
    }

    pub(crate) fn shift_u32_index(&self, index: &mut u32) {
        *index -= self.get_shift_for_index(*index);
    }

    pub(crate) fn shift_non_zero_u32_index(&self, index: &mut NonZeroU32) {
        // 1-indexed value
        let base_index: u32 = (*index).into();
        // 0-indexed value
        let base_index = base_index - 1;
        let shifted_base_index = base_index - self.get_shift_for_index(base_index);
        // SAFETY: Shifted base index can be 0, adding 1 makes it non-zero.
        *index = unsafe { NonZeroU32::new_unchecked(shifted_base_index + 1) };
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
    #[cfg(feature = "array-buffer")]
    pub array_buffers: CompactionList,
    pub arrays: CompactionList,
    pub array_iterators: CompactionList,
    pub await_reactions: CompactionList,
    pub bigints: CompactionList,
    pub bound_functions: CompactionList,
    pub builtin_constructors: CompactionList,
    pub builtin_functions: CompactionList,
    #[cfg(feature = "array-buffer")]
    pub data_views: CompactionList,
    #[cfg(feature = "date")]
    pub dates: CompactionList,
    pub declarative_environments: CompactionList,
    pub e_2_10: CompactionList,
    pub e_2_12: CompactionList,
    pub e_2_16: CompactionList,
    pub e_2_24: CompactionList,
    pub e_2_32: CompactionList,
    pub e_2_4: CompactionList,
    pub e_2_6: CompactionList,
    pub e_2_8: CompactionList,
    pub ecmascript_functions: CompactionList,
    pub embedder_objects: CompactionList,
    pub source_codes: CompactionList,
    pub errors: CompactionList,
    pub executables: CompactionList,
    pub finalization_registrys: CompactionList,
    pub function_environments: CompactionList,
    pub generators: CompactionList,
    pub global_environments: CompactionList,
    pub maps: CompactionList,
    pub map_iterators: CompactionList,
    pub modules: CompactionList,
    pub numbers: CompactionList,
    pub object_environments: CompactionList,
    pub objects: CompactionList,
    pub primitive_objects: CompactionList,
    pub promise_reaction_records: CompactionList,
    pub promise_resolving_functions: CompactionList,
    pub promises: CompactionList,
    pub proxys: CompactionList,
    pub realms: CompactionList,
    pub regexps: CompactionList,
    pub scripts: CompactionList,
    pub sets: CompactionList,
    pub set_iterators: CompactionList,
    #[cfg(feature = "shared-array-buffer")]
    pub shared_array_buffers: CompactionList,
    pub strings: CompactionList,
    pub symbols: CompactionList,
    #[cfg(feature = "array-buffer")]
    pub typed_arrays: CompactionList,
    #[cfg(feature = "weak-refs")]
    pub weak_maps: CompactionList,
    #[cfg(feature = "weak-refs")]
    pub weak_refs: CompactionList,
    #[cfg(feature = "weak-refs")]
    pub weak_sets: CompactionList,
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
            #[cfg(feature = "array-buffer")]
            array_buffers: CompactionList::from_mark_bits(&bits.array_buffers),
            array_iterators: CompactionList::from_mark_bits(&bits.array_iterators),
            await_reactions: CompactionList::from_mark_bits(&bits.await_reactions),
            bigints: CompactionList::from_mark_bits(&bits.bigints),
            bound_functions: CompactionList::from_mark_bits(&bits.bound_functions),
            builtin_constructors: CompactionList::from_mark_bits(&bits.builtin_constructors),
            builtin_functions: CompactionList::from_mark_bits(&bits.builtin_functions),
            ecmascript_functions: CompactionList::from_mark_bits(&bits.ecmascript_functions),
            embedder_objects: CompactionList::from_mark_bits(&bits.embedder_objects),
            generators: CompactionList::from_mark_bits(&bits.generators),
            source_codes: CompactionList::from_mark_bits(&bits.source_codes),
            #[cfg(feature = "date")]
            dates: CompactionList::from_mark_bits(&bits.dates),
            errors: CompactionList::from_mark_bits(&bits.errors),
            executables: CompactionList::from_mark_bits(&bits.executables),
            maps: CompactionList::from_mark_bits(&bits.maps),
            map_iterators: CompactionList::from_mark_bits(&bits.map_iterators),
            numbers: CompactionList::from_mark_bits(&bits.numbers),
            objects: CompactionList::from_mark_bits(&bits.objects),
            promise_reaction_records: CompactionList::from_mark_bits(
                &bits.promise_reaction_records,
            ),
            promise_resolving_functions: CompactionList::from_mark_bits(
                &bits.promise_resolving_functions,
            ),
            promises: CompactionList::from_mark_bits(&bits.promises),
            primitive_objects: CompactionList::from_mark_bits(&bits.primitive_objects),
            regexps: CompactionList::from_mark_bits(&bits.regexps),
            sets: CompactionList::from_mark_bits(&bits.sets),
            set_iterators: CompactionList::from_mark_bits(&bits.set_iterators),
            strings: CompactionList::from_mark_bits(&bits.strings),
            #[cfg(feature = "shared-array-buffer")]
            shared_array_buffers: CompactionList::from_mark_bits(&bits.shared_array_buffers),
            symbols: CompactionList::from_mark_bits(&bits.symbols),
            #[cfg(feature = "array-buffer")]
            data_views: CompactionList::from_mark_bits(&bits.data_views),
            finalization_registrys: CompactionList::from_mark_bits(&bits.finalization_registrys),
            proxys: CompactionList::from_mark_bits(&bits.proxys),
            #[cfg(feature = "weak-refs")]
            weak_maps: CompactionList::from_mark_bits(&bits.weak_maps),
            #[cfg(feature = "weak-refs")]
            weak_refs: CompactionList::from_mark_bits(&bits.weak_refs),
            #[cfg(feature = "weak-refs")]
            weak_sets: CompactionList::from_mark_bits(&bits.weak_sets),
            #[cfg(feature = "array-buffer")]
            typed_arrays: CompactionList::from_mark_bits(&bits.typed_arrays),
        }
    }
}

pub(crate) trait HeapMarkAndSweep {
    /// Mark all Heap references contained in self
    ///
    /// To mark a HeapIndex, push it into the relevant queue in
    /// WorkQueues.
    #[allow(unused_variables)]
    fn mark_values(&self, queues: &mut WorkQueues);

    /// Handle potential sweep of and update Heap references in self
    ///
    /// Sweeping of self is needed for Heap vectors: They must compact
    /// according to the `compactions` parameter. Additionally, any
    /// Heap references in self must be updated according to the
    /// compactions list.
    #[allow(unused_variables)]
    fn sweep_values(&mut self, compactions: &CompactionLists);
}

impl<T> HeapMarkAndSweep for &T
where
    T: HeapMarkAndSweep,
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        (*self).mark_values(queues);
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        unreachable!();
    }
}

impl<T> HeapMarkAndSweep for Option<T>
where
    T: HeapMarkAndSweep,
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        if let Some(content) = self {
            content.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        if let Some(content) = self {
            content.sweep_values(compactions);
        }
    }
}

impl<T> HeapMarkAndSweep for Box<[T]>
where
    T: HeapMarkAndSweep,
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.iter().for_each(|entry| entry.mark_values(queues));
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.iter_mut()
            .for_each(|entry| entry.sweep_values(compactions))
    }
}

impl<T> HeapMarkAndSweep for &[T]
where
    T: HeapMarkAndSweep,
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.iter().for_each(|entry| entry.mark_values(queues));
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        panic!();
    }
}

impl<T> HeapMarkAndSweep for &mut [T]
where
    T: HeapMarkAndSweep,
{
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.iter().for_each(|entry| entry.mark_values(queues))
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.iter_mut()
            .for_each(|entry| entry.sweep_values(compactions))
    }
}

pub(crate) fn mark_array_with_u32_length<T: HeapMarkAndSweep, const N: usize>(
    array: &Option<[T; N]>,
    queues: &mut WorkQueues,
    length: u32,
) {
    array.as_ref().unwrap()[..length as usize]
        .iter()
        .for_each(|value| {
            value.mark_values(queues);
        });
}

pub(crate) fn mark_descriptors(
    descriptors: &AHashMap<u32, ElementDescriptor>,
    queues: &mut WorkQueues,
) {
    for descriptor in descriptors.values() {
        descriptor.mark_values(queues);
    }
}

fn sweep_array_with_u32_length<T: HeapMarkAndSweep, const N: usize>(
    array: &mut Option<[T; N]>,
    compactions: &CompactionLists,
    length: u32,
) {
    if length == 0 {
        return;
    }
    array.as_mut().unwrap()[..length as usize]
        .iter_mut()
        .for_each(|value| {
            value.sweep_values(compactions);
        });
}

pub(crate) fn sweep_heap_vector_values<T: HeapMarkAndSweep + std::fmt::Debug>(
    vec: &mut Vec<T>,
    compactions: &CompactionLists,
    bits: &[bool],
) {
    assert_eq!(vec.len(), bits.len());
    let mut iter = bits.iter();
    vec.retain_mut(|item| {
        let do_retain = iter.next().unwrap();
        if *do_retain {
            item.sweep_values(compactions);
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
            sweep_array_with_u32_length(item, compactions, *length as u32);
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
            sweep_array_with_u32_length(item, compactions, *length as u32);
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
            sweep_array_with_u32_length(item, compactions, *length);
            true
        } else {
            false
        }
    });
}

pub(crate) fn sweep_heap_elements_vector_descriptors<T>(
    descriptors: &mut AHashMap<ElementIndex, AHashMap<u32, ElementDescriptor>>,
    compactions: &CompactionLists,
    self_compactions: &CompactionList,
    marks: &[(bool, T)],
) {
    let mut keys_to_remove = Vec::with_capacity(marks.len() / 4);
    let mut keys_to_reassign = Vec::with_capacity(marks.len() / 4);
    for (key, descriptor) in descriptors.iter_mut() {
        let old_key = *key;
        if !marks.get(key.into_index()).unwrap().0 {
            keys_to_remove.push(old_key);
        } else {
            for descriptor in descriptor.values_mut() {
                descriptor.sweep_values(compactions);
            }
            let mut new_key = old_key;
            self_compactions.shift_index(&mut new_key);
            if new_key != old_key {
                keys_to_reassign.push((old_key, new_key));
            }
        }
    }
    keys_to_remove.sort();
    keys_to_reassign.sort();
    for old_key in keys_to_remove.iter() {
        descriptors.remove(old_key);
    }
    for (old_key, new_key) in keys_to_reassign {
        // SAFETY: The old key came from iterating descriptors, and the same
        // key cannot appear in both keys to remove and keys to reassign. Thus
        // the key must necessarily exist in the descriptors hash map.
        let descriptor = unsafe { descriptors.remove(&old_key).unwrap_unchecked() };
        descriptors.insert(new_key, descriptor);
    }
}
