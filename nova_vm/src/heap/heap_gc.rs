// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::thread;

#[cfg(feature = "date")]
use crate::ecmascript::Date;
#[cfg(feature = "temporal")]
use crate::ecmascript::TemporalInstant;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::{ArrayBuffer, DataView, VoidArray};
#[cfg(feature = "regexp")]
use crate::ecmascript::{RegExp, RegExpStringIterator};
#[cfg(feature = "set")]
use crate::ecmascript::{Set, SetIterator};
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::{SharedArrayBuffer, SharedDataView, SharedVoidArray};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::{WeakMap, WeakRef, WeakSet};
#[cfg(feature = "array-buffer")]
use crate::heap::heap_bits::sweep_side_table_values;
use crate::{
    ecmascript::{
        Agent, Array, ArrayIterator, AsyncGenerator, AwaitReaction, BUILTIN_STRINGS_LIST,
        BoundFunction, BuiltinConstructorFunction, BuiltinFunction, BuiltinPromiseFinallyFunction,
        BuiltinPromiseResolvingFunction, DeclarativeEnvironment, ECMAScriptFunction,
        EmbedderObject, Environments, Error, FinalizationRegistry, FunctionEnvironment, Generator,
        GlobalEnvironment, HeapBigInt, HeapNumber, HeapString, Map, MapIterator, Module,
        ModuleEnvironment, ModuleRequest, ObjectEnvironment, ObjectShape, OrdinaryObject,
        PrimitiveObject, PrivateEnvironment, Promise, PromiseGroup, PromiseReaction,
        PropertyLookupCache, Proxy, Realm, Script, SourceCode, SourceTextModule, StringIterator,
        Symbol,
    },
    engine::{Bindable, Executable, GcScope},
    heap::{
        ElementIndex, Heap, HeapIndexHandle, PropertyKeyIndex, WellKnownSymbolIndexes,
        element_array::ElementArrays,
        heap_bits::{
            CompactionLists, HeapBits, HeapMarkAndSweep, WorkQueues, mark_descriptors,
            sweep_heap_elements_vector_descriptors, sweep_heap_soa_vector_values,
            sweep_heap_vector_values, sweep_lookup_table,
        },
    },
    ndt,
};

pub(crate) fn heap_gc(agent: &mut Agent, root_realms: &mut [Option<Realm<'static>>], gc: GcScope) {
    ndt::gc_start!(|| ());

    let mut bits = HeapBits::new(&agent.heap);
    bits.strings
        .mark_range(0..(BUILTIN_STRINGS_LIST.len() as u32), &mut bits.bits);
    bits.symbols.mark_range(
        0..(WellKnownSymbolIndexes::Unscopables as u32),
        &mut bits.bits,
    );
    let mut queues = WorkQueues::new(&agent.heap, &bits);
    root_realms.iter().for_each(|realm| {
        if let Some(realm) = realm {
            queues.realms.push(realm.unbind());
        }
    });
    queues.object_shapes.push(ObjectShape::NULL);
    agent.heap.prototype_shapes.mark_values(&mut queues);
    agent.heap.caches.mark_values(&mut queues);
    agent.mark_values(&mut queues);
    let mut has_finalization_registrys = false;

    while !queues.is_empty() {
        let Heap {
            #[cfg(feature = "array-buffer")]
            array_buffers,
            #[cfg(feature = "array-buffer")]
                array_buffer_detach_keys: _,
            arrays,
            array_iterators,
            async_generators,
            await_reactions,
            bigints,
            bound_functions,
            builtin_constructors,
            builtin_functions,
            caches,
            #[cfg(feature = "date")]
            dates,
            #[cfg(feature = "temporal")]
            instants,
            ecmascript_functions,
            elements,
            embedder_objects,
            environments,
            errors,
            executables,
            source_codes,
            finalization_registrys,
            generators,
            globals: _,
            maps,
            map_iterators,
            modules,
            module_request_records,
            numbers,
            object_shapes,
            object_shape_transitions,
            prototype_shapes: _,
            objects,
            primitive_objects,
            promise_reaction_records,
            promise_resolving_functions,
            promise_finally_functions,
            promises,
            promise_group_records,
            proxies,
            realms,
            #[cfg(feature = "regexp")]
            regexps,
            #[cfg(feature = "regexp")]
            regexp_string_iterators,
            scripts,
            #[cfg(feature = "set")]
            sets,
            #[cfg(feature = "set")]
            set_iterators,
            #[cfg(feature = "shared-array-buffer")]
            shared_array_buffers,
            source_text_module_records,
            string_iterators,
            strings,
            string_lookup_table: _,
            string_hasher: _,
            symbols,
            #[cfg(feature = "array-buffer")]
            typed_arrays,
            #[cfg(feature = "array-buffer")]
                typed_array_byte_lengths: _,
            #[cfg(feature = "array-buffer")]
                typed_array_byte_offsets: _,
            #[cfg(feature = "array-buffer")]
                typed_array_array_lengths: _,
            #[cfg(feature = "array-buffer")]
            data_views,
            #[cfg(feature = "array-buffer")]
                data_view_byte_lengths: _,
            #[cfg(feature = "array-buffer")]
                data_view_byte_offsets: _,
            #[cfg(feature = "shared-array-buffer")]
            shared_typed_arrays,
            #[cfg(feature = "shared-array-buffer")]
                shared_typed_array_byte_lengths: _,
            #[cfg(feature = "shared-array-buffer")]
                shared_typed_array_byte_offsets: _,
            #[cfg(feature = "shared-array-buffer")]
                shared_typed_array_array_lengths: _,
            #[cfg(feature = "shared-array-buffer")]
            shared_data_views,
            #[cfg(feature = "shared-array-buffer")]
                shared_data_view_byte_lengths: _,
            #[cfg(feature = "shared-array-buffer")]
                shared_data_view_byte_offsets: _,
            #[cfg(feature = "weak-refs")]
            weak_maps,
            #[cfg(feature = "weak-refs")]
            weak_refs,
            #[cfg(feature = "weak-refs")]
            weak_sets,
            alloc_counter: _,
        } = &agent.heap;
        let Environments {
            declarative: declarative_environments,
            function: function_environments,
            global: global_environments,
            module: module_environments,
            object: object_environments,
            private: private_environments,
        } = environments;
        let ElementArrays {
            e2pow1,
            e2pow2,
            e2pow3,
            e2pow4,
            e2pow6,
            e2pow8,
            e2pow10,
            e2pow12,
            e2pow16,
            e2pow24,
            e2pow32,
            k2pow1,
            k2pow2,
            k2pow3,
            k2pow4,
            k2pow6,
            k2pow8,
            k2pow10,
            k2pow12,
            k2pow16,
            k2pow24,
            k2pow32,
        } = elements;

        if !queues.modules.is_empty() {
            let mut module_marks: Box<[Module]> = queues.modules.drain(..).collect();
            module_marks.sort();
            module_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.modules.set_bit(index, &bits.bits) {
                    // Did mark.
                    modules.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.scripts.is_empty() {
            let mut script_marks: Box<[Script]> = queues.scripts.drain(..).collect();
            script_marks.sort();
            script_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.scripts.set_bit(index, &bits.bits) {
                    // Did mark.
                    scripts.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.realms.is_empty() {
            let mut realm_marks: Box<[Realm]> = queues.realms.drain(..).collect();
            realm_marks.sort();
            realm_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.realms.set_bit(index, &bits.bits) {
                    // Did mark.
                    realms.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.declarative_environments.is_empty() {
            let mut declarative_environment_marks: Box<[DeclarativeEnvironment]> =
                queues.declarative_environments.drain(..).collect();
            declarative_environment_marks.sort();
            declarative_environment_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.declarative_environments.set_bit(index, &bits.bits) {
                    // Did mark.
                    declarative_environments.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.function_environments.is_empty() {
            let mut function_environment_marks: Box<[FunctionEnvironment]> =
                queues.function_environments.drain(..).collect();
            function_environment_marks.sort();
            function_environment_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.function_environments.set_bit(index, &bits.bits) {
                    // Did mark.
                    function_environments.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.global_environments.is_empty() {
            let mut global_environment_marks: Box<[GlobalEnvironment]> =
                queues.global_environments.drain(..).collect();
            global_environment_marks.sort();
            global_environment_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.global_environments.set_bit(index, &bits.bits) {
                    // Did mark.
                    global_environments.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.module_environments.is_empty() {
            let mut module_environment_marks: Box<[ModuleEnvironment]> =
                queues.module_environments.drain(..).collect();
            module_environment_marks.sort();
            module_environment_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.module_environments.set_bit(index, &bits.bits) {
                    // Did mark.
                    module_environments.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.object_environments.is_empty() {
            let mut object_environment_marks: Box<[ObjectEnvironment]> =
                queues.object_environments.drain(..).collect();
            object_environment_marks.sort();
            object_environment_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.object_environments.set_bit(index, &bits.bits) {
                    // Did mark.
                    object_environments.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.private_environments.is_empty() {
            let mut private_environment_marks: Box<[PrivateEnvironment]> =
                queues.private_environments.drain(..).collect();
            private_environment_marks.sort();
            private_environment_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.private_environments.set_bit(index, &bits.bits) {
                    // Did mark.
                    private_environments.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.pending_ephemerons.is_empty() {
            queues.pending_ephemerons.sort_by_key(|(key, _)| *key);
            let new_values_to_mark = queues
                .pending_ephemerons
                .extract_if(.., |(key, _)| queues.bits.is_marked(key))
                .map(|(_, value)| value)
                .collect::<Vec<_>>();
            for value in new_values_to_mark {
                value.mark_values(&mut queues);
            }
        }

        if !queues.arrays.is_empty() {
            let mut array_marks: Box<[Array]> = queues.arrays.drain(..).collect();
            array_marks.sort();
            array_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.arrays.set_bit(index, &bits.bits) {
                    // Did mark.
                    arrays.get(index as u32).mark_values(&mut queues);
                }
            });
        }

        #[cfg(feature = "array-buffer")]
        {
            if !queues.array_buffers.is_empty() {
                let mut array_buffer_marks: Box<[ArrayBuffer]> =
                    queues.array_buffers.drain(..).collect();
                array_buffer_marks.sort();
                array_buffer_marks.iter().for_each(|&idx| {
                    let index = idx.get_index();
                    if bits.array_buffers.set_bit(index, &bits.bits) {
                        // Did mark.
                        array_buffers.get(index).mark_values(&mut queues);
                    }
                });
            }
        }

        if !queues.array_iterators.is_empty() {
            let mut array_iterator_marks: Box<[ArrayIterator]> =
                queues.array_iterators.drain(..).collect();
            array_iterator_marks.sort();
            array_iterator_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.array_iterators.set_bit(index, &bits.bits) {
                    // Did mark.
                    array_iterators.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.async_generators.is_empty() {
            let mut async_generator_marks: Box<[AsyncGenerator]> =
                queues.async_generators.drain(..).collect();
            async_generator_marks.sort();
            async_generator_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.async_generators.set_bit(index, &bits.bits) {
                    // Did mark.
                    async_generators.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.await_reactions.is_empty() {
            let mut await_reaction_marks: Box<[AwaitReaction]> =
                queues.await_reactions.drain(..).collect();
            await_reaction_marks.sort();
            await_reaction_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.await_reactions.set_bit(index, &bits.bits) {
                    // Did mark.
                    await_reactions.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.bigints.is_empty() {
            let mut bigint_marks: Box<[HeapBigInt]> = queues.bigints.drain(..).collect();
            bigint_marks.sort();
            bigint_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.bigints.set_bit(index, &bits.bits) {
                    // Did mark.
                    bigints.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.bound_functions.is_empty() {
            let mut bound_function_marks: Box<[BoundFunction]> =
                queues.bound_functions.drain(..).collect();
            bound_function_marks.sort();
            bound_function_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.bound_functions.set_bit(index, &bits.bits) {
                    // Did mark.
                    bound_functions.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.ecmascript_functions.is_empty() {
            let mut ecmascript_function_marks: Box<[ECMAScriptFunction]> =
                queues.ecmascript_functions.drain(..).collect();
            ecmascript_function_marks.sort();
            ecmascript_function_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.ecmascript_functions.set_bit(index, &bits.bits) {
                    // Did mark.
                    ecmascript_functions.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.errors.is_empty() {
            let mut error_marks: Box<[Error]> = queues.errors.drain(..).collect();
            error_marks.sort();
            error_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.errors.set_bit(index, &bits.bits) {
                    // Did mark.
                    errors.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.executables.is_empty() {
            let mut executable_marks: Box<[Executable]> = queues.executables.drain(..).collect();
            executable_marks.sort();
            executable_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.executables.set_bit(index, &bits.bits) {
                    // Did mark.
                    executables.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.source_codes.is_empty() {
            let mut source_code_marks: Box<[SourceCode]> = queues.source_codes.drain(..).collect();
            source_code_marks.sort();
            source_code_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.source_codes.set_bit(index, &bits.bits) {
                    // Did mark.
                    source_codes.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.builtin_constructors.is_empty() {
            let mut builtin_constructors_marks: Box<[BuiltinConstructorFunction]> =
                queues.builtin_constructors.drain(..).collect();
            builtin_constructors_marks.sort();
            builtin_constructors_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.builtin_constructors.set_bit(index, &bits.bits) {
                    // Did mark.
                    builtin_constructors.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.builtin_functions.is_empty() {
            let mut builtin_functions_marks: Box<[BuiltinFunction]> =
                queues.builtin_functions.drain(..).collect();
            builtin_functions_marks.sort();
            builtin_functions_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.builtin_functions.set_bit(index, &bits.bits) {
                    // Did mark.
                    builtin_functions.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.caches.is_empty() {
            let mut caches_marks: Box<[PropertyLookupCache]> = queues.caches.drain(..).collect();
            caches_marks.sort();
            caches_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.caches.set_bit(index, &bits.bits) {
                    // Did mark.
                    caches.mark_cache(index, &mut queues);
                }
            });
        }
        #[cfg(feature = "array-buffer")]
        if !queues.data_views.is_empty() {
            let mut data_view_marks: Box<[DataView]> = queues.data_views.drain(..).collect();
            data_view_marks.sort();
            data_view_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.data_views.set_bit(index, &bits.bits) {
                    // Did mark.
                    data_views.get(index).mark_values(&mut queues);
                }
            });
        }

        #[cfg(feature = "date")]
        if !queues.dates.is_empty() {
            let mut date_marks: Box<[Date]> = queues.dates.drain(..).collect();
            date_marks.sort();
            date_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.dates.set_bit(index, &bits.bits) {
                    // Did mark.
                    dates.get(index).mark_values(&mut queues);
                }
            });
        }
        #[cfg(feature = "temporal")]
        {
            let mut instant_marks: Box<[TemporalInstant]> = queues.instants.drain(..).collect();
            instant_marks.sort();
            instant_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if let Some(marked) = bits.instants.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    instants.get(index).mark_values(&mut queues);
                }
            });
        }

        if !queues.embedder_objects.is_empty() {
            let mut embedder_object_marks: Box<[EmbedderObject]> =
                queues.embedder_objects.drain(..).collect();
            embedder_object_marks.sort();
            embedder_object_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.embedder_objects.set_bit(index, &bits.bits) {
                    // Did mark.
                    embedder_objects.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.finalization_registrys.is_empty() {
            let mut finalization_registry_marks: Box<[FinalizationRegistry]> =
                queues.finalization_registrys.drain(..).collect();
            finalization_registry_marks.sort();
            if !finalization_registry_marks.is_empty() {
                has_finalization_registrys = true;
            }
            finalization_registry_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.finalization_registrys.set_bit(index, &bits.bits) {
                    // Did mark.

                    finalization_registrys
                        .get(index as u32)
                        .mark_values(&mut queues);
                }
            });
        }
        if !queues.generators.is_empty() {
            let mut generator_marks: Box<[Generator]> = queues.generators.drain(..).collect();
            generator_marks.sort();
            generator_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.generators.set_bit(index, &bits.bits) {
                    // Did mark.
                    generators.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.object_shapes.is_empty() {
            let mut object_marks: Box<[ObjectShape]> = queues.object_shapes.drain(..).collect();
            object_marks.sort();
            object_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.object_shapes.set_bit(index, &bits.bits) {
                    // Did mark.
                    object_shapes.get(index).mark_values(&mut queues);
                    object_shape_transitions.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.objects.is_empty() {
            let mut object_marks: Box<[OrdinaryObject]> = queues.objects.drain(..).collect();
            object_marks.sort();
            object_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.objects.set_bit(index, &bits.bits) {
                    // Did mark.
                    if let Some(rec) = objects.get(index) {
                        rec.mark_values(&mut queues, &agent.heap.object_shapes);
                    }
                }
            });
        }
        if !queues.promises.is_empty() {
            let mut promise_marks: Box<[Promise]> = queues.promises.drain(..).collect();
            promise_marks.sort();
            promise_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.promises.set_bit(index, &bits.bits) {
                    // Did mark.
                    promises.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.promise_reaction_records.is_empty() {
            let mut promise_reaction_record_marks: Box<[PromiseReaction]> =
                queues.promise_reaction_records.drain(..).collect();
            promise_reaction_record_marks.sort();
            promise_reaction_record_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.promise_reaction_records.set_bit(index, &bits.bits) {
                    // Did mark.
                    promise_reaction_records.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.promise_group_records.is_empty() {
            let mut promise_group_record_marks: Box<[PromiseGroup]> =
                queues.promise_group_records.drain(..).collect();
            promise_group_record_marks.sort();
            promise_group_record_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.promise_group_records.set_bit(index, &bits.bits) {
                    // Did mark.
                    promise_group_records.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.promise_resolving_functions.is_empty() {
            let mut promise_resolving_function_marks: Box<[BuiltinPromiseResolvingFunction]> =
                queues.promise_resolving_functions.drain(..).collect();
            promise_resolving_function_marks.sort();
            promise_resolving_function_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.promise_resolving_functions.set_bit(index, &bits.bits) {
                    // Did mark.
                    promise_resolving_functions
                        .get(index)
                        .mark_values(&mut queues);
                }
            });
        }
        if !queues.promise_finally_functions.is_empty() {
            let mut promise_finally_function_marks: Box<[BuiltinPromiseFinallyFunction]> =
                queues.promise_finally_functions.drain(..).collect();
            promise_finally_function_marks.sort();
            promise_finally_function_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.promise_finally_functions.set_bit(index, &bits.bits) {
                    // Did mark.
                    promise_finally_functions
                        .get(index)
                        .mark_values(&mut queues);
                }
            });
        }
        if !queues.proxies.is_empty() {
            let mut proxy_marks: Box<[Proxy]> = queues.proxies.drain(..).collect();
            proxy_marks.sort();
            proxy_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.proxies.set_bit(index, &bits.bits) {
                    // Did mark.
                    proxies.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.maps.is_empty() {
            let mut map_marks: Box<[Map]> = queues.maps.drain(..).collect();
            map_marks.sort();
            map_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.maps.set_bit(index, &bits.bits) {
                    // Did mark.
                    maps.get(index as u32).mark_values(&mut queues);
                }
            });
        }
        if !queues.map_iterators.is_empty() {
            let mut map_iterator_marks: Box<[MapIterator]> =
                queues.map_iterators.drain(..).collect();
            map_iterator_marks.sort();
            map_iterator_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.map_iterators.set_bit(index, &bits.bits) {
                    // Did mark.
                    map_iterators.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.module_request_records.is_empty() {
            let mut module_request_record_marks: Box<[ModuleRequest]> =
                queues.module_request_records.drain(..).collect();
            module_request_record_marks.sort();
            module_request_record_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.module_request_records.set_bit(index, &bits.bits) {
                    // Did mark.
                    module_request_records.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.numbers.is_empty() {
            let mut number_marks: Box<[HeapNumber]> = queues.numbers.drain(..).collect();
            number_marks.sort();
            number_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.numbers.set_bit(index, &bits.bits) {
                    // Did mark.
                    numbers.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.primitive_objects.is_empty() {
            let mut primitive_object_marks: Box<[PrimitiveObject]> =
                queues.primitive_objects.drain(..).collect();
            primitive_object_marks.sort();
            primitive_object_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.primitive_objects.set_bit(index, &bits.bits) {
                    // Did mark.
                    primitive_objects.get(index).mark_values(&mut queues);
                }
            });
        }
        #[cfg(feature = "regexp")]
        {
            if !queues.regexps.is_empty() {
                let mut regexp_marks: Box<[RegExp]> = queues.regexps.drain(..).collect();
                regexp_marks.sort();
                regexp_marks.iter().for_each(|&idx| {
                    let index = idx.get_index();
                    if bits.regexps.set_bit(index, &bits.bits) {
                        // Did mark.
                        regexps.get(index).mark_values(&mut queues);
                    }
                });
            }
            if !queues.regexp_string_iterators.is_empty() {
                let mut regexp_string_iterator_marks: Box<[RegExpStringIterator]> =
                    queues.regexp_string_iterators.drain(..).collect();
                regexp_string_iterator_marks.sort();
                regexp_string_iterator_marks.iter().for_each(|&idx| {
                    let index = idx.get_index();
                    if bits.regexp_string_iterators.set_bit(index, &bits.bits) {
                        // Did mark.
                        regexp_string_iterators.get(index).mark_values(&mut queues);
                    }
                });
            }
        }
        #[cfg(feature = "set")]
        {
            if !queues.sets.is_empty() {
                let mut set_marks: Box<[Set]> = queues.sets.drain(..).collect();
                set_marks.sort();
                set_marks.iter().for_each(|&idx| {
                    let index = idx.get_index();
                    if bits.sets.set_bit(index, &bits.bits) {
                        // Did mark.
                        sets.get(index as u32).mark_values(&mut queues);
                    }
                });
            }

            if !queues.set_iterators.is_empty() {
                let mut set_iterator_marks: Box<[SetIterator]> =
                    queues.set_iterators.drain(..).collect();
                set_iterator_marks.sort();
                set_iterator_marks.iter().for_each(|&idx| {
                    let index = idx.get_index();
                    if bits.set_iterators.set_bit(index, &bits.bits) {
                        // Did mark.
                        set_iterators.get(index).mark_values(&mut queues);
                    }
                });
            }
        }
        #[cfg(feature = "shared-array-buffer")]
        {
            if !queues.shared_array_buffers.is_empty() {
                let mut shared_array_buffer_marks: Box<[SharedArrayBuffer]> =
                    queues.shared_array_buffers.drain(..).collect();
                shared_array_buffer_marks.sort();
                shared_array_buffer_marks.iter().for_each(|&idx| {
                    let index = idx.get_index();
                    if bits.shared_array_buffers.set_bit(index, &bits.bits) {
                        // Did mark.
                        shared_array_buffers.get(index).mark_values(&mut queues);
                    }
                });
            }
            if !queues.shared_data_views.is_empty() {
                let mut shared_data_view_marks: Box<[SharedDataView]> =
                    queues.shared_data_views.drain(..).collect();
                shared_data_view_marks.sort();
                shared_data_view_marks.iter().for_each(|&idx| {
                    let index = idx.get_index();
                    if bits.shared_data_views.set_bit(index, &bits.bits) {
                        // Did mark.
                        shared_data_views.get(index).mark_values(&mut queues);
                    }
                });
            }
            if !queues.shared_typed_arrays.is_empty() {
                let mut shared_typed_array_marks: Box<[SharedVoidArray]> =
                    queues.shared_typed_arrays.drain(..).collect();
                shared_typed_array_marks.sort();
                shared_typed_array_marks.iter().for_each(|&idx| {
                    let index = idx.get_index();
                    if bits.shared_typed_arrays.set_bit(index, &bits.bits) {
                        // Did mark.
                        shared_typed_arrays.get(index).mark_values(&mut queues);
                    }
                });
            }
        }
        if !queues.source_text_module_records.is_empty() {
            let mut source_text_module_record_marks: Box<[SourceTextModule]> =
                queues.source_text_module_records.drain(..).collect();
            source_text_module_record_marks.sort();
            source_text_module_record_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.source_text_module_records.set_bit(index, &bits.bits) {
                    // Did mark.
                    source_text_module_records
                        .get(index)
                        .mark_values(&mut queues);
                }
            });
        }
        if !queues.string_iterators.is_empty() {
            let mut string_generator_marks: Box<[StringIterator]> =
                queues.string_iterators.drain(..).collect();
            string_generator_marks.sort();
            string_generator_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.string_iterators.set_bit(index, &bits.bits) {
                    // Did mark.
                    string_iterators.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.strings.is_empty() {
            let mut string_marks: Box<[HeapString]> = queues.strings.drain(..).collect();
            string_marks.sort();
            string_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.strings.set_bit(index, &bits.bits) {
                    // Did mark.
                    strings.get(index).mark_values(&mut queues);
                }
            });
        }
        if !queues.symbols.is_empty() {
            let mut symbol_marks: Box<[Symbol]> = queues.symbols.drain(..).collect();
            symbol_marks.sort();
            symbol_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.symbols.set_bit(index, &bits.bits) {
                    // Did mark.
                    symbols.get(index).mark_values(&mut queues);
                }
            });
        }
        #[cfg(feature = "array-buffer")]
        if !queues.typed_arrays.is_empty() {
            let mut typed_arrays_marks: Box<[VoidArray]> = queues.typed_arrays.drain(..).collect();
            typed_arrays_marks.sort();
            typed_arrays_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.typed_arrays.set_bit(index, &bits.bits) {
                    // Did mark.
                    typed_arrays.get(index).mark_values(&mut queues);
                }
            });
        }
        #[cfg(feature = "weak-refs")]
        {
            if !queues.weak_maps.is_empty() {
                let mut weak_map_marks: Box<[WeakMap]> = queues.weak_maps.drain(..).collect();
                weak_map_marks.sort();
                weak_map_marks.iter().for_each(|&idx| {
                    let index = idx.get_index();
                    if bits.weak_maps.set_bit(index, &bits.bits) {
                        // Did mark.
                        weak_maps.get(index).mark_values(&mut queues);
                    }
                });
            }
            if !queues.weak_refs.is_empty() {
                let mut weak_ref_marks: Box<[WeakRef]> = queues.weak_refs.drain(..).collect();
                weak_ref_marks.sort();
                weak_ref_marks.iter().for_each(|&idx| {
                    let index = idx.get_index();
                    if bits.weak_refs.set_bit(index, &bits.bits) {
                        // Did mark.
                        weak_refs.get(index).mark_values(&mut queues);
                    }
                });
            }
            if !queues.weak_sets.is_empty() {
                let mut weak_set_marks: Box<[WeakSet]> = queues.weak_sets.drain(..).collect();
                weak_set_marks.sort();
                weak_set_marks.iter().for_each(|&idx| {
                    let index = idx.get_index();
                    if bits.weak_sets.set_bit(index, &bits.bits) {
                        // Did mark.
                        weak_sets.get(index).mark_values(&mut queues);
                    }
                });
            }
        }

        if !queues.e_2_1.is_empty() {
            let mut e_2_1_marks: Box<[ElementIndex]> = queues.e_2_1.drain(..).collect();
            e_2_1_marks.sort();
            e_2_1_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.e_2_1.set_bit(index, &bits.bits) {
                    if let Some(descriptors) = e2pow1.descriptors.get(&idx) {
                        mark_descriptors(descriptors, &mut queues);
                    }
                    e2pow1.values.get(index).mark_values(&mut queues);
                } else {
                    panic!("ElementsVector was not unique");
                }
            });
        }
        if !queues.e_2_2.is_empty() {
            let mut e_2_2_marks: Box<[ElementIndex]> = queues.e_2_2.drain(..).collect();
            e_2_2_marks.sort();
            e_2_2_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.e_2_2.set_bit(index, &bits.bits) {
                    if let Some(descriptors) = e2pow2.descriptors.get(&idx) {
                        mark_descriptors(descriptors, &mut queues);
                    }
                    e2pow2.values.get(index).mark_values(&mut queues);
                } else {
                    panic!("ElementsVector was not unique");
                }
            });
        }
        if !queues.e_2_3.is_empty() {
            let mut e_2_3_marks: Box<[ElementIndex]> = queues.e_2_3.drain(..).collect();
            e_2_3_marks.sort();
            e_2_3_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.e_2_3.set_bit(index, &bits.bits) {
                    if let Some(descriptors) = e2pow3.descriptors.get(&idx) {
                        mark_descriptors(descriptors, &mut queues);
                    }
                    e2pow3.values.get(index).mark_values(&mut queues);
                } else {
                    panic!("ElementsVector was not unique");
                }
            });
        }
        if !queues.e_2_4.is_empty() {
            let mut e_2_4_marks: Box<[ElementIndex]> = queues.e_2_4.drain(..).collect();
            e_2_4_marks.sort();
            e_2_4_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.e_2_4.set_bit(index, &bits.bits) {
                    if let Some(descriptors) = e2pow4.descriptors.get(&idx) {
                        mark_descriptors(descriptors, &mut queues);
                    }
                    e2pow4.values.get(index).mark_values(&mut queues);
                } else {
                    panic!("ElementsVector was not unique");
                }
            });
        }
        if !queues.e_2_6.is_empty() {
            let mut e_2_6_marks: Box<[ElementIndex]> = queues.e_2_6.drain(..).collect();
            e_2_6_marks.sort();
            e_2_6_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.e_2_6.set_bit(index, &bits.bits) {
                    if let Some(descriptors) = e2pow6.descriptors.get(&idx) {
                        mark_descriptors(descriptors, &mut queues);
                    }
                    e2pow6.values.get(index).mark_values(&mut queues);
                } else {
                    panic!("ElementsVector was not unique");
                }
            });
        }
        if !queues.e_2_8.is_empty() {
            let mut e_2_8_marks: Box<[ElementIndex]> = queues.e_2_8.drain(..).collect();
            e_2_8_marks.sort();
            e_2_8_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.e_2_8.set_bit(index, &bits.bits) {
                    if let Some(descriptors) = e2pow8.descriptors.get(&idx) {
                        mark_descriptors(descriptors, &mut queues);
                    }
                    e2pow8.values.get(index).mark_values(&mut queues);
                } else {
                    panic!("ElementsVector was not unique");
                }
            });
        }
        if !queues.e_2_10.is_empty() {
            let mut e_2_10_marks: Box<[ElementIndex]> = queues.e_2_10.drain(..).collect();
            e_2_10_marks.sort();
            e_2_10_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.e_2_10.set_bit(index, &bits.bits) {
                    if let Some(descriptors) = e2pow10.descriptors.get(&idx) {
                        mark_descriptors(descriptors, &mut queues);
                    }
                    e2pow10.values.get(index).mark_values(&mut queues);
                } else {
                    panic!("ElementsVector was not unique");
                }
            });
        }
        if !queues.e_2_12.is_empty() {
            let mut e_2_12_marks: Box<[ElementIndex]> = queues.e_2_12.drain(..).collect();
            e_2_12_marks.sort();
            e_2_12_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.e_2_12.set_bit(index, &bits.bits) {
                    if let Some(descriptors) = e2pow12.descriptors.get(&idx) {
                        mark_descriptors(descriptors, &mut queues);
                    }
                    e2pow12.values.get(index).mark_values(&mut queues);
                } else {
                    panic!("ElementsVector was not unique");
                }
            });
        }
        if !queues.e_2_16.is_empty() {
            let mut e_2_16_marks: Box<[ElementIndex]> = queues.e_2_16.drain(..).collect();
            e_2_16_marks.sort();
            e_2_16_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.e_2_16.set_bit(index, &bits.bits) {
                    if let Some(descriptors) = e2pow16.descriptors.get(&idx) {
                        mark_descriptors(descriptors, &mut queues);
                    }
                    e2pow16.values.get(index).mark_values(&mut queues);
                } else {
                    panic!("ElementsVector was not unique");
                }
            });
        }
        if !queues.e_2_24.is_empty() {
            let mut e_2_24_marks: Box<[ElementIndex]> = queues.e_2_24.drain(..).collect();
            e_2_24_marks.sort();
            e_2_24_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.e_2_24.set_bit(index, &bits.bits) {
                    if let Some(descriptors) = e2pow24.descriptors.get(&idx) {
                        mark_descriptors(descriptors, &mut queues);
                    }
                    e2pow24.values.get(index).mark_values(&mut queues);
                } else {
                    panic!("ElementsVector was not unique");
                }
            });
        }
        if !queues.e_2_32.is_empty() {
            let mut e_2_32_marks: Box<[ElementIndex]> = queues.e_2_32.drain(..).collect();
            e_2_32_marks.sort();
            e_2_32_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.e_2_32.set_bit(index, &bits.bits) {
                    if let Some(descriptors) = e2pow32.descriptors.get(&idx) {
                        mark_descriptors(descriptors, &mut queues);
                    }
                    e2pow32.values.get(index).mark_values(&mut queues);
                } else {
                    panic!("ElementsVector was not unique");
                }
            });
        }

        if !queues.k_2_4.is_empty() {
            let mut k_2_4_marks: Box<[PropertyKeyIndex]> = queues.k_2_4.drain(..).collect();
            k_2_4_marks.sort();
            k_2_4_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.k_2_4.set_bit(index, &bits.bits) {
                    k2pow4.keys.get(index).mark_values(&mut queues)
                }
            });
        }
        if !queues.k_2_1.is_empty() {
            let mut k_2_1_marks: Box<[PropertyKeyIndex]> = queues.k_2_1.drain(..).collect();
            k_2_1_marks.sort();
            k_2_1_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.k_2_1.set_bit(index, &bits.bits) {
                    k2pow1.keys.get(index).mark_values(&mut queues)
                }
            });
        }
        if !queues.k_2_2.is_empty() {
            let mut k_2_2_marks: Box<[PropertyKeyIndex]> = queues.k_2_2.drain(..).collect();
            k_2_2_marks.sort();
            k_2_2_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.k_2_2.set_bit(index, &bits.bits) {
                    k2pow2.keys.get(index).mark_values(&mut queues)
                }
            });
        }
        if !queues.k_2_3.is_empty() {
            let mut k_2_3_marks: Box<[PropertyKeyIndex]> = queues.k_2_3.drain(..).collect();
            k_2_3_marks.sort();
            k_2_3_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.k_2_3.set_bit(index, &bits.bits) {
                    k2pow3.keys.get(index).mark_values(&mut queues)
                }
            });
        }
        if !queues.k_2_6.is_empty() {
            let mut k_2_6_marks: Box<[PropertyKeyIndex]> = queues.k_2_6.drain(..).collect();
            k_2_6_marks.sort();
            k_2_6_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.k_2_6.set_bit(index, &bits.bits) {
                    k2pow6.keys.get(index).mark_values(&mut queues)
                }
            });
        }
        if !queues.k_2_8.is_empty() {
            let mut k_2_8_marks: Box<[PropertyKeyIndex]> = queues.k_2_8.drain(..).collect();
            k_2_8_marks.sort();
            k_2_8_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.k_2_8.set_bit(index, &bits.bits) {
                    k2pow8.keys.get(index).mark_values(&mut queues)
                }
            });
        }
        if !queues.k_2_10.is_empty() {
            let mut k_2_10_marks: Box<[PropertyKeyIndex]> = queues.k_2_10.drain(..).collect();
            k_2_10_marks.sort();
            k_2_10_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.k_2_10.set_bit(index, &bits.bits) {
                    k2pow10.keys.get(index).mark_values(&mut queues)
                }
            });
        }
        if !queues.k_2_12.is_empty() {
            let mut k_2_12_marks: Box<[PropertyKeyIndex]> = queues.k_2_12.drain(..).collect();
            k_2_12_marks.sort();
            k_2_12_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.k_2_12.set_bit(index, &bits.bits) {
                    k2pow12.keys.get(index).mark_values(&mut queues)
                }
            });
        }
        if !queues.k_2_16.is_empty() {
            let mut k_2_16_marks: Box<[PropertyKeyIndex]> = queues.k_2_16.drain(..).collect();
            k_2_16_marks.sort();
            k_2_16_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.k_2_16.set_bit(index, &bits.bits) {
                    k2pow16.keys.get(index).mark_values(&mut queues)
                }
            });
        }
        if !queues.k_2_24.is_empty() {
            let mut k_2_24_marks: Box<[PropertyKeyIndex]> = queues.k_2_24.drain(..).collect();
            k_2_24_marks.sort();
            k_2_24_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.k_2_24.set_bit(index, &bits.bits) {
                    k2pow24.keys.get(index).mark_values(&mut queues)
                }
            });
        }
        if !queues.k_2_32.is_empty() {
            let mut k_2_32_marks: Box<[PropertyKeyIndex]> = queues.k_2_32.drain(..).collect();
            k_2_32_marks.sort();
            k_2_32_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if bits.k_2_32.set_bit(index, &bits.bits) {
                    k2pow32.keys.get(index).mark_values(&mut queues)
                }
            });
        }
    }

    sweep(agent, &bits, root_realms, gc);
    if has_finalization_registrys {
        FinalizationRegistry::enqueue_cleanup_jobs(agent);
    }
    ndt::gc_done!(|| ());
}

// NOTE: This is the one true use of the `GcScope` which is why we allow a lint
// exception here. For future reference see [this comment](https://github.com/trynova/nova/pull/913#discussion_r2616482397).
#[allow(unknown_lints, can_use_no_gc_scope)]
fn sweep(
    agent: &mut Agent,
    bits: &HeapBits,
    root_realms: &mut [Option<Realm<'static>>],
    _: GcScope,
) {
    let compactions = CompactionLists::create_from_bits(bits);

    for realm in root_realms {
        realm.sweep_values(&compactions);
    }

    agent.sweep_values(&compactions);

    let Heap {
        #[cfg(feature = "array-buffer")]
        array_buffers,
        #[cfg(feature = "array-buffer")]
        array_buffer_detach_keys,
        arrays,
        array_iterators,
        async_generators,
        await_reactions,
        bigints,
        bound_functions,
        builtin_constructors,
        builtin_functions,
        caches,
        #[cfg(feature = "date")]
        dates,
        #[cfg(feature = "temporal")]
        instants,
        ecmascript_functions,
        elements,
        embedder_objects,
        environments,
        errors,
        executables,
        source_codes,
        finalization_registrys,
        generators,
        globals,
        maps,
        map_iterators,
        modules,
        module_request_records,
        numbers,
        object_shapes,
        object_shape_transitions,
        prototype_shapes,
        objects,
        primitive_objects,
        promise_reaction_records,
        promise_resolving_functions,
        promise_finally_functions,
        promises,
        promise_group_records,
        proxies,
        realms,
        #[cfg(feature = "regexp")]
        regexps,
        #[cfg(feature = "regexp")]
        regexp_string_iterators,
        scripts,
        #[cfg(feature = "set")]
        sets,
        #[cfg(feature = "set")]
        set_iterators,
        #[cfg(feature = "shared-array-buffer")]
        shared_array_buffers,
        source_text_module_records,
        string_iterators,
        strings,
        string_lookup_table,
        string_hasher: _,
        symbols,
        #[cfg(feature = "array-buffer")]
        typed_arrays,
        #[cfg(feature = "array-buffer")]
        typed_array_byte_lengths,
        #[cfg(feature = "array-buffer")]
        typed_array_byte_offsets,
        #[cfg(feature = "array-buffer")]
        typed_array_array_lengths,
        #[cfg(feature = "array-buffer")]
        data_views,
        #[cfg(feature = "array-buffer")]
        data_view_byte_lengths,
        #[cfg(feature = "array-buffer")]
        data_view_byte_offsets,
        #[cfg(feature = "shared-array-buffer")]
        shared_typed_arrays,
        #[cfg(feature = "shared-array-buffer")]
        shared_typed_array_byte_lengths,
        #[cfg(feature = "shared-array-buffer")]
        shared_typed_array_byte_offsets,
        #[cfg(feature = "shared-array-buffer")]
        shared_typed_array_array_lengths,
        #[cfg(feature = "shared-array-buffer")]
        shared_data_views,
        #[cfg(feature = "shared-array-buffer")]
        shared_data_view_byte_lengths,
        #[cfg(feature = "shared-array-buffer")]
        shared_data_view_byte_offsets,
        #[cfg(feature = "weak-refs")]
        weak_maps,
        #[cfg(feature = "weak-refs")]
        weak_refs,
        #[cfg(feature = "weak-refs")]
        weak_sets,
        alloc_counter,
    } = &mut agent.heap;
    // Reset the allocation counter.
    *alloc_counter = 0;
    let Environments {
        declarative,
        function,
        global,
        module,
        object,
        private,
    } = environments;
    let ElementArrays {
        e2pow1,
        e2pow2,
        e2pow3,
        e2pow4,
        e2pow6,
        e2pow8,
        e2pow10,
        e2pow12,
        e2pow16,
        e2pow24,
        e2pow32,
        k2pow1,
        k2pow2,
        k2pow3,
        k2pow4,
        k2pow6,
        k2pow8,
        k2pow10,
        k2pow12,
        k2pow16,
        k2pow24,
        k2pow32,
    } = elements;

    let mut globals = globals.borrow_mut();
    let globals_iter = globals.iter_mut();
    thread::scope(|s| {
        s.spawn(|| {
            prototype_shapes.sweep_values(&compactions);
        });

        s.spawn(|| {
            caches.sweep_cache(&compactions, &bits.caches, &bits.bits);
            caches.sweep_values(&compactions);
        });

        s.spawn(|| {
            for value in globals_iter {
                value.sweep_values(&compactions);
            }
        });
        if !e2pow1.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow1.descriptors,
                    &compactions,
                    &compactions.e_2_1,
                    &bits.e_2_1,
                    &bits.bits,
                );
                sweep_heap_vector_values(&mut e2pow1.values, &compactions, &bits.e_2_1, &bits.bits);
            });
        }
        if !e2pow2.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow2.descriptors,
                    &compactions,
                    &compactions.e_2_2,
                    &bits.e_2_2,
                    &bits.bits,
                );
                sweep_heap_vector_values(&mut e2pow2.values, &compactions, &bits.e_2_2, &bits.bits);
            });
        }
        if !e2pow3.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow3.descriptors,
                    &compactions,
                    &compactions.e_2_3,
                    &bits.e_2_3,
                    &bits.bits,
                );
                sweep_heap_vector_values(&mut e2pow3.values, &compactions, &bits.e_2_3, &bits.bits);
            });
        }
        if !e2pow4.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow4.descriptors,
                    &compactions,
                    &compactions.e_2_4,
                    &bits.e_2_4,
                    &bits.bits,
                );
                sweep_heap_vector_values(&mut e2pow4.values, &compactions, &bits.e_2_4, &bits.bits);
            });
        }
        if !e2pow6.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow6.descriptors,
                    &compactions,
                    &compactions.e_2_6,
                    &bits.e_2_6,
                    &bits.bits,
                );
                sweep_heap_vector_values(&mut e2pow6.values, &compactions, &bits.e_2_6, &bits.bits);
            });
        }
        if !e2pow8.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow8.descriptors,
                    &compactions,
                    &compactions.e_2_8,
                    &bits.e_2_8,
                    &bits.bits,
                );
                sweep_heap_vector_values(&mut e2pow8.values, &compactions, &bits.e_2_8, &bits.bits);
            });
        }
        if !e2pow10.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow10.descriptors,
                    &compactions,
                    &compactions.e_2_10,
                    &bits.e_2_10,
                    &bits.bits,
                );
                sweep_heap_vector_values(
                    &mut e2pow10.values,
                    &compactions,
                    &bits.e_2_10,
                    &bits.bits,
                );
            });
        }
        if !e2pow12.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow12.descriptors,
                    &compactions,
                    &compactions.e_2_12,
                    &bits.e_2_12,
                    &bits.bits,
                );
                sweep_heap_vector_values(
                    &mut e2pow12.values,
                    &compactions,
                    &bits.e_2_12,
                    &bits.bits,
                );
            });
        }
        if !e2pow16.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow16.descriptors,
                    &compactions,
                    &compactions.e_2_16,
                    &bits.e_2_16,
                    &bits.bits,
                );
                sweep_heap_vector_values(
                    &mut e2pow16.values,
                    &compactions,
                    &bits.e_2_16,
                    &bits.bits,
                );
            });
        }
        if !e2pow24.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow24.descriptors,
                    &compactions,
                    &compactions.e_2_24,
                    &bits.e_2_24,
                    &bits.bits,
                );
                sweep_heap_vector_values(
                    &mut e2pow24.values,
                    &compactions,
                    &bits.e_2_24,
                    &bits.bits,
                );
            });
        }
        if !e2pow32.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow32.descriptors,
                    &compactions,
                    &compactions.e_2_32,
                    &bits.e_2_32,
                    &bits.bits,
                );
                sweep_heap_vector_values(
                    &mut e2pow32.values,
                    &compactions,
                    &bits.e_2_32,
                    &bits.bits,
                );
            });
        }
        if !k2pow1.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(&mut k2pow1.keys, &compactions, &bits.k_2_1, &bits.bits);
            });
        }
        if !k2pow2.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(&mut k2pow2.keys, &compactions, &bits.k_2_2, &bits.bits);
            });
        }
        if !k2pow3.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(&mut k2pow3.keys, &compactions, &bits.k_2_3, &bits.bits);
            });
        }
        if !k2pow4.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(&mut k2pow4.keys, &compactions, &bits.k_2_4, &bits.bits);
            });
        }
        if !k2pow6.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(&mut k2pow6.keys, &compactions, &bits.k_2_6, &bits.bits);
            });
        }
        if !k2pow8.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(&mut k2pow8.keys, &compactions, &bits.k_2_8, &bits.bits);
            });
        }
        if !k2pow10.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(&mut k2pow10.keys, &compactions, &bits.k_2_10, &bits.bits);
            });
        }
        if !k2pow12.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(&mut k2pow12.keys, &compactions, &bits.k_2_12, &bits.bits);
            });
        }
        if !k2pow16.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(&mut k2pow16.keys, &compactions, &bits.k_2_16, &bits.bits);
            });
        }
        if !k2pow24.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(&mut k2pow24.keys, &compactions, &bits.k_2_24, &bits.bits);
            });
        }
        if !k2pow32.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(&mut k2pow32.keys, &compactions, &bits.k_2_32, &bits.bits);
            });
        }
        #[cfg(feature = "array-buffer")]
        if !array_buffers.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    array_buffers,
                    &compactions,
                    &bits.array_buffers,
                    &bits.bits,
                );
                sweep_side_table_values(array_buffer_detach_keys, &compactions);
            });
        }
        if !arrays.is_empty() {
            s.spawn(|| {
                sweep_heap_soa_vector_values(arrays, &compactions, &bits.arrays, &bits.bits);
            });
        }
        if !array_iterators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    array_iterators,
                    &compactions,
                    &bits.array_iterators,
                    &bits.bits,
                );
            });
        }
        if !async_generators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    async_generators,
                    &compactions,
                    &bits.async_generators,
                    &bits.bits,
                );
            });
        }
        if !await_reactions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    await_reactions,
                    &compactions,
                    &bits.await_reactions,
                    &bits.bits,
                );
            });
        }
        if !bigints.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(bigints, &compactions, &bits.bigints, &bits.bits);
            });
        }
        if !bound_functions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    bound_functions,
                    &compactions,
                    &bits.bound_functions,
                    &bits.bits,
                );
            });
        }
        if !builtin_constructors.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    builtin_constructors,
                    &compactions,
                    &bits.builtin_constructors,
                    &bits.bits,
                );
            });
        }
        if !builtin_functions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    builtin_functions,
                    &compactions,
                    &bits.builtin_functions,
                    &bits.bits,
                );
            });
        }
        #[cfg(feature = "array-buffer")]
        if !data_views.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(data_views, &compactions, &bits.data_views, &bits.bits);
                sweep_side_table_values(data_view_byte_lengths, &compactions);
                sweep_side_table_values(data_view_byte_offsets, &compactions);
            });
        }
        #[cfg(feature = "shared-array-buffer")]
        if !shared_data_views.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    shared_data_views,
                    &compactions,
                    &bits.shared_data_views,
                    &bits.bits,
                );
                sweep_side_table_values(shared_data_view_byte_lengths, &compactions);
                sweep_side_table_values(shared_data_view_byte_offsets, &compactions);
            });
        }
        #[cfg(feature = "date")]
        if !dates.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(dates, &compactions, &bits.dates, &bits.bits);
            });
        }
        #[cfg(feature = "temporal")]
        if !instants.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(instants, &compactions, &bits.instants, &bits.bits);
            });
        }
        if !declarative.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    declarative,
                    &compactions,
                    &bits.declarative_environments,
                    &bits.bits,
                );
            });
        }
        if !ecmascript_functions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    ecmascript_functions,
                    &compactions,
                    &bits.ecmascript_functions,
                    &bits.bits,
                );
            });
        }
        if !embedder_objects.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    embedder_objects,
                    &compactions,
                    &bits.embedder_objects,
                    &bits.bits,
                );
            });
        }
        if !errors.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(errors, &compactions, &bits.errors, &bits.bits);
            });
        }
        if !executables.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(executables, &compactions, &bits.executables, &bits.bits);
            });
        }
        if !finalization_registrys.is_empty() {
            s.spawn(|| {
                sweep_heap_soa_vector_values(
                    finalization_registrys,
                    &compactions,
                    &bits.finalization_registrys,
                    &bits.bits,
                );
            });
        }
        if !function.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    function,
                    &compactions,
                    &bits.function_environments,
                    &bits.bits,
                );
            });
        }
        if !generators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(generators, &compactions, &bits.generators, &bits.bits);
            });
        }
        if !global.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    global,
                    &compactions,
                    &bits.global_environments,
                    &bits.bits,
                );
            });
        }
        if !module.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    module,
                    &compactions,
                    &bits.module_environments,
                    &bits.bits,
                );
            });
        }
        if !maps.is_empty() {
            s.spawn(|| {
                sweep_heap_soa_vector_values(maps, &compactions, &bits.maps, &bits.bits);
            });
        }
        if !map_iterators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    map_iterators,
                    &compactions,
                    &bits.map_iterators,
                    &bits.bits,
                );
            });
        }
        if !modules.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(modules, &compactions, &bits.modules, &bits.bits);
            });
        }
        if !module_request_records.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    module_request_records,
                    &compactions,
                    &bits.module_request_records,
                    &bits.bits,
                );
            });
        }
        if !numbers.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(numbers, &compactions, &bits.numbers, &bits.bits);
            });
        }
        if !object.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    object,
                    &compactions,
                    &bits.object_environments,
                    &bits.bits,
                );
            });
        }
        if !private.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    private,
                    &compactions,
                    &bits.private_environments,
                    &bits.bits,
                );
            });
        }
        if !object_shapes.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    object_shape_transitions,
                    &compactions,
                    &bits.object_shapes,
                    &bits.bits,
                );
            });
        }
        if !object_shapes.is_empty() || !objects.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    object_shapes,
                    &compactions,
                    &bits.object_shapes,
                    &bits.bits,
                );
                assert_eq!(objects.len(), bits.objects.len());
                let mut iter = bits.objects.iter(&bits.bits);
                objects.retain_mut(|item| {
                    let do_retain = iter.next().unwrap();
                    if do_retain {
                        item.sweep_values(&compactions, object_shapes);
                        true
                    } else {
                        false
                    }
                });
            });
        }
        if !primitive_objects.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    primitive_objects,
                    &compactions,
                    &bits.primitive_objects,
                    &bits.bits,
                );
            });
        }
        if !promise_reaction_records.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    promise_reaction_records,
                    &compactions,
                    &bits.promise_reaction_records,
                    &bits.bits,
                );
            });
        }
        if !promise_resolving_functions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    promise_resolving_functions,
                    &compactions,
                    &bits.promise_resolving_functions,
                    &bits.bits,
                );
            });
        }
        if !promise_finally_functions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    promise_finally_functions,
                    &compactions,
                    &bits.promise_finally_functions,
                    &bits.bits,
                );
            });
        }
        if !promises.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(promises, &compactions, &bits.promises, &bits.bits);
            });
        }
        if !promise_group_records.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    promise_group_records,
                    &compactions,
                    &bits.promise_group_records,
                    &bits.bits,
                );
            });
        }
        if !proxies.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(proxies, &compactions, &bits.proxies, &bits.bits);
            });
        }
        if !realms.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(realms, &compactions, &bits.realms, &bits.bits);
            });
        }
        #[cfg(feature = "regexp")]
        if !regexps.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(regexps, &compactions, &bits.regexps, &bits.bits);
            });
        }
        #[cfg(feature = "regexp")]
        if !regexp_string_iterators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    regexp_string_iterators,
                    &compactions,
                    &bits.regexp_string_iterators,
                    &bits.bits,
                );
            });
        }
        if !scripts.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(scripts, &compactions, &bits.scripts, &bits.bits);
            });
        }
        #[cfg(feature = "set")]
        if !sets.is_empty() {
            s.spawn(|| {
                sweep_heap_soa_vector_values(sets, &compactions, &bits.sets, &bits.bits);
            });
        }
        #[cfg(feature = "set")]
        if !set_iterators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    set_iterators,
                    &compactions,
                    &bits.set_iterators,
                    &bits.bits,
                );
            });
        }
        #[cfg(feature = "shared-array-buffer")]
        if !shared_array_buffers.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    shared_array_buffers,
                    &compactions,
                    &bits.shared_array_buffers,
                    &bits.bits,
                );
            });
        }
        if !source_text_module_records.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    source_text_module_records,
                    &compactions,
                    &bits.source_text_module_records,
                    &bits.bits,
                );
            });
        }
        if !source_codes.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    source_codes,
                    &compactions,
                    &bits.source_codes,
                    &bits.bits,
                );
            });
        }
        if !string_iterators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    string_iterators,
                    &compactions,
                    &bits.string_iterators,
                    &bits.bits,
                );
            });
        }
        if !strings.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(strings, &compactions, &bits.strings, &bits.bits);
                sweep_lookup_table(string_lookup_table, &compactions);
            });
        }
        if !symbols.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(symbols, &compactions, &bits.symbols, &bits.bits);
            });
        }
        #[cfg(feature = "array-buffer")]
        if !typed_arrays.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    typed_arrays,
                    &compactions,
                    &bits.typed_arrays,
                    &bits.bits,
                );
                sweep_side_table_values(typed_array_byte_lengths, &compactions);
                sweep_side_table_values(typed_array_byte_offsets, &compactions);
                sweep_side_table_values(typed_array_array_lengths, &compactions);
            });
        }
        #[cfg(feature = "shared-array-buffer")]
        if !shared_typed_arrays.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    shared_typed_arrays,
                    &compactions,
                    &bits.shared_typed_arrays,
                    &bits.bits,
                );
                sweep_side_table_values(shared_typed_array_byte_lengths, &compactions);
                sweep_side_table_values(shared_typed_array_byte_offsets, &compactions);
                sweep_side_table_values(shared_typed_array_array_lengths, &compactions);
            });
        }
        #[cfg(feature = "weak-refs")]
        if !weak_maps.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(weak_maps, &compactions, &bits.weak_maps, &bits.bits);
            });
        }
        #[cfg(feature = "weak-refs")]
        if !weak_refs.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(weak_refs, &compactions, &bits.weak_refs, &bits.bits);
            });
        }
        #[cfg(feature = "weak-refs")]
        if !weak_sets.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(weak_sets, &compactions, &bits.weak_sets, &bits.bits);
            });
        }
    });
}

#[test]
fn test_heap_gc() {
    use crate::engine::GcScope;
    use crate::{
        ecmascript::{DefaultHostHooks, Options},
        engine::HeapRootData,
    };

    let mut agent = Agent::new(Options::default(), &DefaultHostHooks);

    let (mut gc, mut scope) = unsafe { GcScope::create_root() };
    let mut gc = GcScope::new(&mut gc, &mut scope);
    assert!(agent.heap.objects.is_empty());
    let obj = HeapRootData::Object(
        OrdinaryObject::create_object(&mut agent, None, &[]).expect("Should perform GC here"),
    );
    agent.heap.globals.borrow_mut().push(obj);
    heap_gc(&mut agent, &mut [], gc.reborrow());

    assert_eq!(agent.heap.objects.len(), 1);
    assert_eq!(agent.heap.elements.e2pow4.values.len(), 0);
    assert!(agent.heap.globals.borrow().last().is_some());
    println!(
        "Global #1: {:#?}",
        agent.heap.globals.borrow().last().unwrap()
    );
}
