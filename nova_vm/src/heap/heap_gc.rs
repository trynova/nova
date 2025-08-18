// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::thread;

use super::{
    Heap, WellKnownSymbolIndexes,
    element_array::ElementArrays,
    heap_bits::{
        CompactionLists, HeapBits, HeapMarkAndSweep, WorkQueues, mark_array_with_u32_length,
        mark_descriptors, mark_optional_array_with_u32_length,
        sweep_heap_elements_vector_descriptors, sweep_heap_u8_elements_vector_values,
        sweep_heap_u8_property_key_vector, sweep_heap_u16_elements_vector_values,
        sweep_heap_u16_property_key_vector, sweep_heap_u32_elements_vector_values,
        sweep_heap_u32_property_key_vector, sweep_heap_vector_values, sweep_lookup_table,
    },
    indexes::{ElementIndex, PropertyKeyIndex, StringIndex},
};
#[cfg(feature = "array-buffer")]
use super::{heap_bits::sweep_side_table_values, indexes::TypedArrayIndex};
#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::Date;
#[cfg(feature = "regexp")]
use crate::ecmascript::builtins::regexp::RegExp;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::builtins::{ArrayBuffer, data_view::DataView};
#[cfg(feature = "set")]
use crate::ecmascript::builtins::{
    keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIterator, set::Set,
};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
use crate::{
    ecmascript::{
        builtins::{
            Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
            async_generator_objects::AsyncGenerator,
            bound_function::BoundFunction,
            control_abstraction_objects::{
                async_function_objects::await_reaction::AwaitReaction,
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
            keyed_collections::map_objects::map_iterator_objects::map_iterator::MapIterator,
            map::Map,
            module::Module,
            ordinary::{caches::PropertyLookupCache, shape::ObjectShape},
            primitive_objects::PrimitiveObject,
            promise::Promise,
            promise_objects::promise_abstract_operations::promise_finally_functions::BuiltinPromiseFinallyFunction,
            proxy::Proxy,
            text_processing::string_objects::string_iterator_objects::StringIterator,
        },
        execution::{
            Agent, DeclarativeEnvironment, Environments, FunctionEnvironment, GlobalEnvironment,
            ModuleEnvironment, ObjectEnvironment, Realm,
        },
        scripts_and_modules::{
            module::module_semantics::{
                ModuleRequest, source_text_module_records::SourceTextModule,
            },
            script::Script,
            source_code::SourceCode,
        },
        types::{
            BUILTIN_STRINGS_LIST, HeapNumber, HeapString, OrdinaryObject, Symbol,
            bigint::HeapBigInt,
        },
    },
    engine::{
        Executable,
        context::{Bindable, GcScope},
    },
};

pub fn heap_gc(agent: &mut Agent, root_realms: &mut [Option<Realm<'static>>], gc: GcScope) {
    let mut bits = HeapBits::new(&agent.heap);
    let mut queues = WorkQueues::new(&agent.heap);

    root_realms.iter().for_each(|realm| {
        if let Some(realm) = realm {
            queues.realms.push(realm.unbind());
        }
    });

    queues.strings.extend(
        (0..BUILTIN_STRINGS_LIST.len()).map(|index| HeapString(StringIndex::from_index(index))),
    );
    queues.symbols.extend_from_slice(&[
        WellKnownSymbolIndexes::AsyncIterator.into(),
        WellKnownSymbolIndexes::HasInstance.into(),
        WellKnownSymbolIndexes::IsConcatSpreadable.into(),
        WellKnownSymbolIndexes::Iterator.into(),
        WellKnownSymbolIndexes::Match.into(),
        WellKnownSymbolIndexes::MatchAll.into(),
        WellKnownSymbolIndexes::Replace.into(),
        WellKnownSymbolIndexes::Search.into(),
        WellKnownSymbolIndexes::Species.into(),
        WellKnownSymbolIndexes::Split.into(),
        WellKnownSymbolIndexes::ToPrimitive.into(),
        WellKnownSymbolIndexes::ToStringTag.into(),
        WellKnownSymbolIndexes::Unscopables.into(),
    ]);
    queues.object_shapes.push(ObjectShape::NULL);
    agent.mark_values(&mut queues);

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
            #[cfg(feature = "array-buffer")]
            data_views,
            #[cfg(feature = "array-buffer")]
                data_view_byte_lengths: _,
            #[cfg(feature = "array-buffer")]
                data_view_byte_offsets: _,
            #[cfg(feature = "date")]
            dates,
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
            prototype_shapes,
            objects,
            primitive_objects,
            promise_reaction_records,
            promise_resolving_functions,
            promise_finally_functions,
            promises,
            proxys,
            realms,
            #[cfg(feature = "regexp")]
            regexps,
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
            private: _private_environments,
        } = environments;
        let ElementArrays {
            e2pow4,
            e2pow6,
            e2pow8,
            e2pow10,
            e2pow12,
            e2pow16,
            e2pow24,
            e2pow32,
            k2pow4,
            k2pow6,
            k2pow8,
            k2pow10,
            k2pow12,
            k2pow16,
            k2pow24,
            k2pow32,
        } = elements;

        prototype_shapes.mark_values(&mut queues);
        caches.mark_values(&mut queues);

        let mut module_marks: Box<[Module]> = queues.modules.drain(..).collect();
        module_marks.sort();
        module_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.modules.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                modules.get(index).mark_values(&mut queues);
            }
        });
        let mut script_marks: Box<[Script]> = queues.scripts.drain(..).collect();
        script_marks.sort();
        script_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.scripts.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                scripts.get(index).mark_values(&mut queues);
            }
        });
        let mut realm_marks: Box<[Realm]> = queues.realms.drain(..).collect();
        realm_marks.sort();
        realm_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.realms.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                realms.get(index).mark_values(&mut queues);
            }
        });

        let mut declarative_environment_marks: Box<[DeclarativeEnvironment]> =
            queues.declarative_environments.drain(..).collect();
        declarative_environment_marks.sort();
        declarative_environment_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.declarative_environments.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                declarative_environments.get(index).mark_values(&mut queues);
            }
        });
        let mut function_environment_marks: Box<[FunctionEnvironment]> =
            queues.function_environments.drain(..).collect();
        function_environment_marks.sort();
        function_environment_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.function_environments.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                function_environments.get(index).mark_values(&mut queues);
            }
        });
        let mut global_environment_marks: Box<[GlobalEnvironment]> =
            queues.global_environments.drain(..).collect();
        global_environment_marks.sort();
        global_environment_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.global_environments.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                global_environments.get(index).mark_values(&mut queues);
            }
        });
        let mut module_environment_marks: Box<[ModuleEnvironment]> =
            queues.module_environments.drain(..).collect();
        module_environment_marks.sort();
        module_environment_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.module_environments.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                module_environments.get(index).mark_values(&mut queues);
            }
        });
        let mut object_environment_marks: Box<[ObjectEnvironment]> =
            queues.object_environments.drain(..).collect();
        object_environment_marks.sort();
        object_environment_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.object_environments.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                object_environments.get(index).mark_values(&mut queues);
            }
        });

        let mut array_marks: Box<[Array]> = queues.arrays.drain(..).collect();
        array_marks.sort();
        array_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.arrays.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                arrays.get(index).mark_values(&mut queues);
            }
        });
        #[cfg(feature = "array-buffer")]
        {
            let mut array_buffer_marks: Box<[ArrayBuffer]> =
                queues.array_buffers.drain(..).collect();
            array_buffer_marks.sort();
            array_buffer_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if let Some(marked) = bits.array_buffers.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    array_buffers.get(index).mark_values(&mut queues);
                }
            });
        }
        let mut array_iterator_marks: Box<[ArrayIterator]> =
            queues.array_iterators.drain(..).collect();
        array_iterator_marks.sort();
        array_iterator_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.array_iterators.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                array_iterators.get(index).mark_values(&mut queues);
            }
        });
        let mut async_generator_marks: Box<[AsyncGenerator]> =
            queues.async_generators.drain(..).collect();
        async_generator_marks.sort();
        async_generator_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.async_generators.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                async_generators.get(index).mark_values(&mut queues);
            }
        });
        let mut await_reaction_marks: Box<[AwaitReaction]> =
            queues.await_reactions.drain(..).collect();
        await_reaction_marks.sort();
        await_reaction_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.await_reactions.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                await_reactions.get(index).mark_values(&mut queues);
            }
        });
        let mut bigint_marks: Box<[HeapBigInt]> = queues.bigints.drain(..).collect();
        bigint_marks.sort();
        bigint_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.bigints.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                bigints.get(index).mark_values(&mut queues);
            }
        });
        let mut bound_function_marks: Box<[BoundFunction]> =
            queues.bound_functions.drain(..).collect();
        bound_function_marks.sort();
        bound_function_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.bound_functions.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                bound_functions.get(index).mark_values(&mut queues);
            }
        });
        let mut ecmascript_function_marks: Box<[ECMAScriptFunction]> =
            queues.ecmascript_functions.drain(..).collect();
        ecmascript_function_marks.sort();
        ecmascript_function_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.ecmascript_functions.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                ecmascript_functions.get(index).mark_values(&mut queues);
            }
        });
        let mut error_marks: Box<[Error]> = queues.errors.drain(..).collect();
        error_marks.sort();
        error_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.errors.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                errors.get(index).mark_values(&mut queues);
            }
        });
        let mut executable_marks: Box<[Executable]> = queues.executables.drain(..).collect();
        executable_marks.sort();
        executable_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.executables.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                executables.get(index).mark_values(&mut queues);
            }
        });
        let mut source_code_marks: Box<[SourceCode]> = queues.source_codes.drain(..).collect();
        source_code_marks.sort();
        source_code_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.source_codes.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                source_codes.get(index).mark_values(&mut queues);
            }
        });
        let mut builtin_constructors_marks: Box<[BuiltinConstructorFunction]> =
            queues.builtin_constructors.drain(..).collect();
        builtin_constructors_marks.sort();
        builtin_constructors_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.builtin_constructors.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                builtin_constructors.get(index).mark_values(&mut queues);
            }
        });
        let mut builtin_functions_marks: Box<[BuiltinFunction]> =
            queues.builtin_functions.drain(..).collect();
        builtin_functions_marks.sort();
        builtin_functions_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.builtin_functions.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                builtin_functions.get(index).mark_values(&mut queues);
            }
        });
        let mut caches_marks: Box<[PropertyLookupCache]> = queues.caches.drain(..).collect();
        caches_marks.sort();
        caches_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.caches.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                caches.mark_cache(index, &mut queues);
            }
        });
        #[cfg(feature = "array-buffer")]
        {
            let mut data_view_marks: Box<[DataView]> = queues.data_views.drain(..).collect();
            data_view_marks.sort();
            data_view_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if let Some(marked) = bits.data_views.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    data_views.get(index).mark_values(&mut queues);
                }
            });
        }
        #[cfg(feature = "date")]
        {
            let mut date_marks: Box<[Date]> = queues.dates.drain(..).collect();
            date_marks.sort();
            date_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if let Some(marked) = bits.dates.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    dates.get(index).mark_values(&mut queues);
                }
            });
        }
        let mut embedder_object_marks: Box<[EmbedderObject]> =
            queues.embedder_objects.drain(..).collect();
        embedder_object_marks.sort();
        embedder_object_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.embedder_objects.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                embedder_objects.get(index).mark_values(&mut queues);
            }
        });
        let mut finalization_registry_marks: Box<[FinalizationRegistry]> =
            queues.finalization_registrys.drain(..).collect();
        finalization_registry_marks.sort();
        finalization_registry_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.finalization_registrys.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                finalization_registrys.get(index).mark_values(&mut queues);
            }
        });
        let mut generator_marks: Box<[Generator]> = queues.generators.drain(..).collect();
        generator_marks.sort();
        generator_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.generators.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                generators.get(index).mark_values(&mut queues);
            }
        });
        let mut object_marks: Box<[ObjectShape]> = queues.object_shapes.drain(..).collect();
        object_marks.sort();
        object_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.object_shapes.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                object_shapes.get(index).mark_values(&mut queues);
                object_shape_transitions.get(index).mark_values(&mut queues);
            }
        });
        let mut object_marks: Box<[OrdinaryObject]> = queues.objects.drain(..).collect();
        object_marks.sort();
        object_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.objects.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                objects.get(index).mark_values(&mut queues);
            }
        });
        let mut promise_marks: Box<[Promise]> = queues.promises.drain(..).collect();
        promise_marks.sort();
        promise_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.promises.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                promises.get(index).mark_values(&mut queues);
            }
        });
        let mut promise_reaction_record_marks: Box<[PromiseReaction]> =
            queues.promise_reaction_records.drain(..).collect();
        promise_reaction_record_marks.sort();
        promise_reaction_record_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.promise_reaction_records.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                promise_reaction_records.get(index).mark_values(&mut queues);
            }
        });
        let mut promise_resolving_function_marks: Box<[BuiltinPromiseResolvingFunction]> =
            queues.promise_resolving_functions.drain(..).collect();
        promise_resolving_function_marks.sort();
        promise_resolving_function_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.promise_resolving_functions.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                promise_resolving_functions
                    .get(index)
                    .mark_values(&mut queues);
            }
        });
        let mut promise_finally_function_marks: Box<[BuiltinPromiseFinallyFunction]> =
            queues.promise_finally_functions.drain(..).collect();
        promise_finally_function_marks.sort();
        promise_finally_function_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.promise_finally_functions.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                promise_finally_functions
                    .get(index)
                    .mark_values(&mut queues);
            }
        });
        let mut proxy_marks: Box<[Proxy]> = queues.proxys.drain(..).collect();
        proxy_marks.sort();
        proxy_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.proxys.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                proxys.get(index).mark_values(&mut queues);
            }
        });
        let mut map_marks: Box<[Map]> = queues.maps.drain(..).collect();
        map_marks.sort();
        map_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.maps.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                maps.get(index).mark_values(&mut queues);
            }
        });
        let mut map_iterator_marks: Box<[MapIterator]> = queues.map_iterators.drain(..).collect();
        map_iterator_marks.sort();
        map_iterator_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.map_iterators.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                map_iterators.get(index).mark_values(&mut queues);
            }
        });
        let mut module_request_record_marks: Box<[ModuleRequest]> =
            queues.module_request_records.drain(..).collect();
        module_request_record_marks.sort();
        module_request_record_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.module_request_records.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                module_request_records.get(index).mark_values(&mut queues);
            }
        });
        let mut number_marks: Box<[HeapNumber]> = queues.numbers.drain(..).collect();
        number_marks.sort();
        number_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.numbers.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                numbers.get(index).mark_values(&mut queues);
            }
        });
        let mut primitive_object_marks: Box<[PrimitiveObject]> =
            queues.primitive_objects.drain(..).collect();
        primitive_object_marks.sort();
        primitive_object_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.primitive_objects.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                primitive_objects.get(index).mark_values(&mut queues);
            }
        });
        #[cfg(feature = "regexp")]
        {
            let mut regexp_marks: Box<[RegExp]> = queues.regexps.drain(..).collect();
            regexp_marks.sort();
            regexp_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if let Some(marked) = bits.regexps.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    regexps.get(index).mark_values(&mut queues);
                }
            });
        }
        #[cfg(feature = "set")]
        {
            let mut set_marks: Box<[Set]> = queues.sets.drain(..).collect();
            set_marks.sort();
            set_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if let Some(marked) = bits.sets.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    sets.get(index).mark_values(&mut queues);
                }
            });

            let mut set_iterator_marks: Box<[SetIterator]> =
                queues.set_iterators.drain(..).collect();
            set_iterator_marks.sort();
            set_iterator_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if let Some(marked) = bits.set_iterators.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    set_iterators.get(index).mark_values(&mut queues);
                }
            });
        }
        #[cfg(feature = "shared-array-buffer")]
        {
            let mut shared_array_buffer_marks: Box<[SharedArrayBuffer]> =
                queues.shared_array_buffers.drain(..).collect();
            shared_array_buffer_marks.sort();
            shared_array_buffer_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if let Some(marked) = bits.shared_array_buffers.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    shared_array_buffers.get(index).mark_values(&mut queues);
                }
            });
        }
        let mut source_text_module_record_marks: Box<[SourceTextModule]> =
            queues.source_text_module_records.drain(..).collect();
        source_text_module_record_marks.sort();
        source_text_module_record_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.source_text_module_records.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                source_text_module_records
                    .get(index)
                    .mark_values(&mut queues);
            }
        });
        let mut string_generator_marks: Box<[StringIterator]> =
            queues.string_iterators.drain(..).collect();
        string_generator_marks.sort();
        string_generator_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.string_iterators.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                string_iterators.get(index).mark_values(&mut queues);
            }
        });
        let mut string_marks: Box<[HeapString]> = queues.strings.drain(..).collect();
        string_marks.sort();
        string_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.strings.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                strings.get(index).mark_values(&mut queues);
            }
        });
        let mut symbol_marks: Box<[Symbol]> = queues.symbols.drain(..).collect();
        symbol_marks.sort();
        symbol_marks.iter().for_each(|&idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.symbols.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                symbols.get(index).mark_values(&mut queues);
            }
        });
        #[cfg(feature = "array-buffer")]
        {
            let mut typed_arrays_marks: Box<[TypedArrayIndex]> =
                queues.typed_arrays.drain(..).collect();
            typed_arrays_marks.sort();
            typed_arrays_marks.iter().for_each(|&idx| {
                let index = idx.into_index();
                if let Some(marked) = bits.typed_arrays.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    typed_arrays.get(index).mark_values(&mut queues);
                }
            });
        }
        #[cfg(feature = "weak-refs")]
        {
            let mut weak_map_marks: Box<[WeakMap]> = queues.weak_maps.drain(..).collect();
            weak_map_marks.sort();
            weak_map_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if let Some(marked) = bits.weak_maps.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    weak_maps.get(index).mark_values(&mut queues);
                }
            });
            let mut weak_ref_marks: Box<[WeakRef]> = queues.weak_refs.drain(..).collect();
            weak_ref_marks.sort();
            weak_ref_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if let Some(marked) = bits.weak_refs.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    weak_refs.get(index).mark_values(&mut queues);
                }
            });
            let mut weak_set_marks: Box<[WeakSet]> = queues.weak_sets.drain(..).collect();
            weak_set_marks.sort();
            weak_set_marks.iter().for_each(|&idx| {
                let index = idx.get_index();
                if let Some(marked) = bits.weak_sets.get_mut(index) {
                    if *marked {
                        // Already marked, ignore
                        return;
                    }
                    *marked = true;
                    weak_sets.get(index).mark_values(&mut queues);
                }
            });
        }

        let mut e_2_4_marks: Box<[(ElementIndex, u32)]> = queues.e_2_4.drain(..).collect();
        e_2_4_marks.sort();
        e_2_4_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.e_2_4.get_mut(index) {
                if *marked {
                    // Already marked, panic: Elements are uniquely owned
                    // and any other reference existing to this entry is a sign of
                    // a GC algorithm bug.
                    panic!("ElementsVector was not unique");
                }
                *marked = true;
                *length = len as u8;
                if let Some(descriptors) = e2pow4.descriptors.get(&idx) {
                    mark_descriptors(descriptors, &mut queues);
                }
                if let Some(array) = e2pow4.values.get(index) {
                    mark_optional_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut e_2_6_marks: Box<[(ElementIndex, u32)]> = queues.e_2_6.drain(..).collect();
        e_2_6_marks.sort();
        e_2_6_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.e_2_6.get_mut(index) {
                if *marked {
                    // Already marked, panic: Elements are uniquely owned
                    // and any other reference existing to this entry is a sign of
                    // a GC algorithm bug.
                    panic!("ElementsVector was not unique");
                }
                *marked = true;
                *length = len as u8;
                if let Some(descriptors) = e2pow6.descriptors.get(&idx) {
                    mark_descriptors(descriptors, &mut queues);
                }
                if let Some(array) = e2pow6.values.get(index) {
                    mark_optional_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut e_2_8_marks: Box<[(ElementIndex, u32)]> = queues.e_2_8.drain(..).collect();
        e_2_8_marks.sort();
        e_2_8_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.e_2_8.get_mut(index) {
                if *marked {
                    // Already marked, panic: Elements are uniquely owned
                    // and any other reference existing to this entry is a sign of
                    // a GC algorithm bug.
                    panic!("ElementsVector was not unique");
                }
                *marked = true;
                *length = len as u8;
                if let Some(descriptors) = e2pow8.descriptors.get(&idx) {
                    mark_descriptors(descriptors, &mut queues);
                }
                if let Some(array) = e2pow8.values.get(index) {
                    mark_optional_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut e_2_10_marks: Box<[(ElementIndex, u32)]> = queues.e_2_10.drain(..).collect();
        e_2_10_marks.sort();
        e_2_10_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.e_2_10.get_mut(index) {
                if *marked {
                    // Already marked, panic: Elements are uniquely owned
                    // and any other reference existing to this entry is a sign of
                    // a GC algorithm bug.
                    panic!("ElementsVector was not unique");
                }
                *marked = true;
                *length = len as u16;
                if let Some(descriptors) = e2pow10.descriptors.get(&idx) {
                    mark_descriptors(descriptors, &mut queues);
                }
                if let Some(array) = e2pow10.values.get(index) {
                    mark_optional_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut e_2_12_marks: Box<[(ElementIndex, u32)]> = queues.e_2_12.drain(..).collect();
        e_2_12_marks.sort();
        e_2_12_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.e_2_12.get_mut(index) {
                if *marked {
                    // Already marked, panic: Elements are uniquely owned
                    // and any other reference existing to this entry is a sign of
                    // a GC algorithm bug.
                    panic!("ElementsVector was not unique");
                }
                *marked = true;
                *length = len as u16;
                if let Some(descriptors) = e2pow12.descriptors.get(&idx) {
                    mark_descriptors(descriptors, &mut queues);
                }
                if let Some(array) = e2pow12.values.get(index) {
                    mark_optional_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut e_2_16_marks: Box<[(ElementIndex, u32)]> = queues.e_2_16.drain(..).collect();
        e_2_16_marks.sort();
        e_2_16_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.e_2_16.get_mut(index) {
                if *marked {
                    // Already marked, panic: Elements are uniquely owned
                    // and any other reference existing to this entry is a sign of
                    // a GC algorithm bug.
                    panic!("ElementsVector was not unique");
                }
                *marked = true;
                *length = len as u16;
                if let Some(descriptors) = e2pow16.descriptors.get(&idx) {
                    mark_descriptors(descriptors, &mut queues);
                }
                if let Some(array) = e2pow16.values.get(index) {
                    mark_optional_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut e_2_24_marks: Box<[(ElementIndex, u32)]> = queues.e_2_24.drain(..).collect();
        e_2_24_marks.sort();
        e_2_24_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.e_2_24.get_mut(index) {
                if *marked {
                    // Already marked, panic: Elements are uniquely owned
                    // and any other reference existing to this entry is a sign of
                    // a GC algorithm bug.
                    panic!("ElementsVector was not unique");
                }
                *marked = true;
                *length = len;
                if let Some(descriptors) = e2pow24.descriptors.get(&idx) {
                    mark_descriptors(descriptors, &mut queues);
                }
                if let Some(array) = e2pow24.values.get(index) {
                    mark_optional_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut e_2_32_marks: Box<[(ElementIndex, u32)]> = queues.e_2_32.drain(..).collect();
        e_2_32_marks.sort();
        e_2_32_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.e_2_32.get_mut(index) {
                if *marked {
                    // Already marked, panic: Elements are uniquely owned
                    // and any other reference existing to this entry is a sign of
                    // a GC algorithm bug.
                    panic!("ElementsVector was not unique");
                }
                *marked = true;
                *length = len;
                if let Some(descriptors) = e2pow32.descriptors.get(&idx) {
                    mark_descriptors(descriptors, &mut queues);
                }
                if let Some(array) = e2pow32.values.get(index) {
                    mark_optional_array_with_u32_length(array, &mut queues, len);
                }
            }
        });

        let mut k_2_4_marks: Box<[(PropertyKeyIndex, u32)]> = queues.k_2_4.drain(..).collect();
        k_2_4_marks.sort();
        k_2_4_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.k_2_4.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                *length = len as u8;
                if let Some(array) = k2pow4.keys.get(index) {
                    mark_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut k_2_6_marks: Box<[(PropertyKeyIndex, u32)]> = queues.k_2_6.drain(..).collect();
        k_2_6_marks.sort();
        k_2_6_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.k_2_6.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                *length = len as u8;
                if let Some(array) = k2pow6.keys.get(index) {
                    mark_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut k_2_8_marks: Box<[(PropertyKeyIndex, u32)]> = queues.k_2_8.drain(..).collect();
        k_2_8_marks.sort();
        k_2_8_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.k_2_8.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                *length = len as u8;
                if let Some(array) = k2pow8.keys.get(index) {
                    mark_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut k_2_10_marks: Box<[(PropertyKeyIndex, u32)]> = queues.k_2_10.drain(..).collect();
        k_2_10_marks.sort();
        k_2_10_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.k_2_10.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                *length = len as u16;
                if let Some(array) = k2pow10.keys.get(index) {
                    mark_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut k_2_12_marks: Box<[(PropertyKeyIndex, u32)]> = queues.k_2_12.drain(..).collect();
        k_2_12_marks.sort();
        k_2_12_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.k_2_12.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                *length = len as u16;
                if let Some(array) = k2pow12.keys.get(index) {
                    mark_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut k_2_16_marks: Box<[(PropertyKeyIndex, u32)]> = queues.k_2_16.drain(..).collect();
        k_2_16_marks.sort();
        k_2_16_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.k_2_16.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                *length = len as u16;
                if let Some(array) = k2pow16.keys.get(index) {
                    mark_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut k_2_24_marks: Box<[(PropertyKeyIndex, u32)]> = queues.k_2_24.drain(..).collect();
        k_2_24_marks.sort();
        k_2_24_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.k_2_24.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                *length = len;
                if let Some(array) = k2pow24.keys.get(index) {
                    mark_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
        let mut k_2_32_marks: Box<[(PropertyKeyIndex, u32)]> = queues.k_2_32.drain(..).collect();
        k_2_32_marks.sort();
        k_2_32_marks.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.k_2_32.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                *length = len;
                if let Some(array) = k2pow32.keys.get(index) {
                    mark_array_with_u32_length(array, &mut queues, len);
                }
            }
        });
    }

    sweep(agent, &bits, root_realms, gc);
}

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
        #[cfg(feature = "array-buffer")]
        data_views,
        #[cfg(feature = "array-buffer")]
        data_view_byte_lengths,
        #[cfg(feature = "array-buffer")]
        data_view_byte_offsets,
        #[cfg(feature = "date")]
        dates,
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
        proxys,
        realms,
        #[cfg(feature = "regexp")]
        regexps,
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
        private: _private_environments,
    } = environments;
    let ElementArrays {
        e2pow4,
        e2pow6,
        e2pow8,
        e2pow10,
        e2pow12,
        e2pow16,
        e2pow24,
        e2pow32,
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
            caches.sweep_cache(&compactions, &bits.caches);
            caches.sweep_values(&compactions);
        });

        s.spawn(|| {
            for value in globals_iter {
                value.sweep_values(&compactions);
            }
        });
        if !e2pow10.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow10.descriptors,
                    &compactions,
                    &compactions.e_2_10,
                    &bits.e_2_10,
                );
                sweep_heap_u16_elements_vector_values(
                    &mut e2pow10.values,
                    &compactions,
                    &bits.e_2_10,
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
                );
                sweep_heap_u16_elements_vector_values(
                    &mut e2pow12.values,
                    &compactions,
                    &bits.e_2_12,
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
                );
                sweep_heap_u16_elements_vector_values(
                    &mut e2pow16.values,
                    &compactions,
                    &bits.e_2_16,
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
                );
                sweep_heap_u32_elements_vector_values(
                    &mut e2pow24.values,
                    &compactions,
                    &bits.e_2_24,
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
                );
                sweep_heap_u32_elements_vector_values(
                    &mut e2pow32.values,
                    &compactions,
                    &bits.e_2_32,
                );
            });
        }
        if !e2pow4.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow4.descriptors,
                    &compactions,
                    &compactions.e_2_4,
                    &bits.e_2_4,
                );
                sweep_heap_u8_elements_vector_values(&mut e2pow4.values, &compactions, &bits.e_2_4);
            });
        }
        if !e2pow6.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow6.descriptors,
                    &compactions,
                    &compactions.e_2_6,
                    &bits.e_2_6,
                );
                sweep_heap_u8_elements_vector_values(&mut e2pow6.values, &compactions, &bits.e_2_6);
            });
        }
        if !e2pow8.values.is_empty() {
            s.spawn(|| {
                sweep_heap_elements_vector_descriptors(
                    &mut e2pow8.descriptors,
                    &compactions,
                    &compactions.e_2_8,
                    &bits.e_2_8,
                );
                sweep_heap_u8_elements_vector_values(&mut e2pow8.values, &compactions, &bits.e_2_8);
            });
        }
        if !k2pow10.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_u16_property_key_vector(&mut k2pow10.keys, &compactions, &bits.k_2_10);
            });
        }
        if !k2pow12.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_u16_property_key_vector(&mut k2pow12.keys, &compactions, &bits.k_2_12);
            });
        }
        if !k2pow16.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_u16_property_key_vector(&mut k2pow16.keys, &compactions, &bits.k_2_16);
            });
        }
        if !k2pow24.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_u32_property_key_vector(&mut k2pow24.keys, &compactions, &bits.k_2_24);
            });
        }
        if !k2pow32.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_u32_property_key_vector(&mut k2pow32.keys, &compactions, &bits.k_2_32);
            });
        }
        if !k2pow4.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_u8_property_key_vector(&mut k2pow4.keys, &compactions, &bits.k_2_4);
            });
        }
        if !k2pow6.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_u8_property_key_vector(&mut k2pow6.keys, &compactions, &bits.k_2_6);
            });
        }
        if !k2pow8.keys.is_empty() {
            s.spawn(|| {
                sweep_heap_u8_property_key_vector(&mut k2pow8.keys, &compactions, &bits.k_2_8);
            });
        }
        #[cfg(feature = "array-buffer")]
        if !array_buffers.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(array_buffers, &compactions, &bits.array_buffers);
                sweep_side_table_values(array_buffer_detach_keys, &compactions);
            });
        }
        if !arrays.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(arrays, &compactions, &bits.arrays);
            });
        }
        if !array_iterators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(array_iterators, &compactions, &bits.array_iterators);
            });
        }
        if !async_generators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(async_generators, &compactions, &bits.async_generators);
            });
        }
        if !await_reactions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(await_reactions, &compactions, &bits.await_reactions);
            });
        }
        if !bigints.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(bigints, &compactions, &bits.bigints);
            });
        }
        if !bound_functions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(bound_functions, &compactions, &bits.bound_functions);
            });
        }
        if !builtin_constructors.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    builtin_constructors,
                    &compactions,
                    &bits.builtin_constructors,
                );
            });
        }
        if !builtin_functions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(builtin_functions, &compactions, &bits.builtin_functions);
            });
        }
        #[cfg(feature = "array-buffer")]
        if !data_views.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(data_views, &compactions, &bits.data_views);
                sweep_side_table_values(data_view_byte_lengths, &compactions);
                sweep_side_table_values(data_view_byte_offsets, &compactions);
            });
        }
        #[cfg(feature = "date")]
        if !dates.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(dates, &compactions, &bits.dates);
            });
        }
        if !declarative.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(declarative, &compactions, &bits.declarative_environments);
            });
        }
        if !ecmascript_functions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    ecmascript_functions,
                    &compactions,
                    &bits.ecmascript_functions,
                );
            });
        }
        if !embedder_objects.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(embedder_objects, &compactions, &bits.embedder_objects);
            });
        }
        if !errors.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(errors, &compactions, &bits.errors);
            });
        }
        if !executables.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(executables, &compactions, &bits.executables);
            });
        }
        if !finalization_registrys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    finalization_registrys,
                    &compactions,
                    &bits.finalization_registrys,
                );
            });
        }
        if !function.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(function, &compactions, &bits.function_environments);
            });
        }
        if !generators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(generators, &compactions, &bits.generators);
            });
        }
        if !global.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(global, &compactions, &bits.global_environments);
            });
        }
        if !module.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(module, &compactions, &bits.module_environments);
            });
        }
        if !maps.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(maps, &compactions, &bits.maps);
            });
        }
        if !map_iterators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(map_iterators, &compactions, &bits.map_iterators);
            });
        }
        if !modules.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(modules, &compactions, &bits.modules);
            });
        }
        if !module_request_records.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    module_request_records,
                    &compactions,
                    &bits.module_request_records,
                );
            });
        }
        if !numbers.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(numbers, &compactions, &bits.numbers);
            });
        }
        if !object.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(object, &compactions, &bits.object_environments);
            });
        }
        if !object_shapes.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(object_shapes, &compactions, &bits.object_shapes);
            });
            s.spawn(|| {
                sweep_heap_vector_values(
                    object_shape_transitions,
                    &compactions,
                    &bits.object_shapes,
                );
            });
        }
        if !objects.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(objects, &compactions, &bits.objects);
            });
        }
        if !primitive_objects.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(primitive_objects, &compactions, &bits.primitive_objects);
            });
        }
        if !promise_reaction_records.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    promise_reaction_records,
                    &compactions,
                    &bits.promise_reaction_records,
                );
            });
        }
        if !promise_resolving_functions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    promise_resolving_functions,
                    &compactions,
                    &bits.promise_resolving_functions,
                );
            });
        }
        if !promise_finally_functions.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    promise_finally_functions,
                    &compactions,
                    &bits.promise_finally_functions,
                );
            });
        }
        if !promises.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(promises, &compactions, &bits.promises);
            });
        }
        if !proxys.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(proxys, &compactions, &bits.proxys);
            });
        }
        if !realms.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(realms, &compactions, &bits.realms);
            });
        }
        #[cfg(feature = "regexp")]
        if !regexps.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(regexps, &compactions, &bits.regexps);
            });
        }
        if !scripts.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(scripts, &compactions, &bits.scripts);
            });
        }
        #[cfg(feature = "set")]
        if !sets.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(sets, &compactions, &bits.sets);
            });
        }
        #[cfg(feature = "set")]
        if !set_iterators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(set_iterators, &compactions, &bits.set_iterators);
            });
        }
        #[cfg(feature = "shared-array-buffer")]
        if !shared_array_buffers.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    shared_array_buffers,
                    &compactions,
                    &bits.shared_array_buffers,
                );
            });
        }
        if !source_text_module_records.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(
                    source_text_module_records,
                    &compactions,
                    &bits.source_text_module_records,
                );
            });
        }
        if !source_codes.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(source_codes, &compactions, &bits.source_codes);
            });
        }
        if !string_iterators.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(string_iterators, &compactions, &bits.string_iterators);
            });
        }
        if !strings.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(strings, &compactions, &bits.strings);
                sweep_lookup_table(string_lookup_table, &compactions);
            });
        }
        if !symbols.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(symbols, &compactions, &bits.symbols);
            });
        }
        #[cfg(feature = "array-buffer")]
        if !typed_arrays.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(typed_arrays, &compactions, &bits.typed_arrays);
                sweep_side_table_values(typed_array_byte_lengths, &compactions);
                sweep_side_table_values(typed_array_byte_offsets, &compactions);
                sweep_side_table_values(typed_array_array_lengths, &compactions);
            });
        }
        #[cfg(feature = "weak-refs")]
        if !weak_maps.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(weak_maps, &compactions, &bits.weak_maps);
            });
        }
        #[cfg(feature = "weak-refs")]
        if !weak_refs.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(weak_refs, &compactions, &bits.weak_refs);
            });
        }
        #[cfg(feature = "weak-refs")]
        if !weak_sets.is_empty() {
            s.spawn(|| {
                sweep_heap_vector_values(weak_sets, &compactions, &bits.weak_sets);
            });
        }
    });
}

#[test]
fn test_heap_gc() {
    use crate::engine::context::GcScope;
    use crate::{
        ecmascript::execution::{DefaultHostHooks, agent::Options},
        engine::rootable::HeapRootData,
    };

    let mut agent = Agent::new(Options::default(), &DefaultHostHooks);

    let (mut gc, mut scope) = unsafe { GcScope::create_root() };
    let mut gc = GcScope::new(&mut gc, &mut scope);
    assert!(agent.heap.objects.is_empty());
    let obj = HeapRootData::Object(OrdinaryObject::create_object(&mut agent, None, &[]));
    agent.heap.globals.borrow_mut().push(Some(obj));
    heap_gc(&mut agent, &mut [], gc.reborrow());

    assert_eq!(agent.heap.objects.len(), 1);
    assert_eq!(agent.heap.elements.e2pow4.values.len(), 0);
    assert!(agent.heap.globals.borrow().last().is_some());
    println!(
        "Global #1: {:#?}",
        agent.heap.globals.borrow().last().unwrap()
    );
}
