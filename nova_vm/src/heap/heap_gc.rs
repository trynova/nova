use std::thread;

use super::{
    element_array::ElementArrays,
    heap_bits::{
        sweep_heap_u16_elements_vector_values, sweep_heap_u32_elements_vector_values,
        sweep_heap_u8_elements_vector_values, sweep_heap_vector_values, CompactionLists, HeapBits,
        HeapMarkAndSweep, WorkQueues,
    },
    indexes::{
        ArrayBufferIndex, ArrayIndex, BigIntIndex, BoundFunctionIndex, BuiltinFunctionIndex,
        DataViewIndex, DateIndex, ECMAScriptFunctionIndex, ElementIndex, EmbedderObjectIndex,
        ErrorIndex, FinalizationRegistryIndex, MapIndex, NumberIndex, ObjectIndex,
        PrimitiveObjectIndex, PromiseIndex, ProxyIndex, RegExpIndex, SetIndex,
        SharedArrayBufferIndex, StringIndex, SymbolIndex, TypedArrayIndex, WeakMapIndex,
        WeakRefIndex, WeakSetIndex,
    },
    Heap,
};
use crate::ecmascript::{
    execution::{
        DeclarativeEnvironmentIndex, Environments, FunctionEnvironmentIndex,
        GlobalEnvironmentIndex, ObjectEnvironmentIndex, RealmIdentifier,
    },
    scripts_and_modules::{module::ModuleIdentifier, script::ScriptIdentifier},
    types::Value,
};

fn collect_values(queues: &mut WorkQueues, values: &[Option<Value>]) {
    values.iter().for_each(|maybe_value| {
        if let Some(value) = maybe_value {
            queues.push_value(*value);
        }
    });
}

pub fn heap_gc(heap: &mut Heap) {
    let mut bits = HeapBits::new(heap);
    let mut queues = WorkQueues::new(heap);

    heap.globals.iter().for_each(|&value| {
        queues.push_value(value);
    });

    while !queues.is_empty() {
        let Heap {
            abstract_closures,
            array_buffers,
            arrays,
            bigints,
            bound_functions,
            builtin_functions,
            data_views,
            dates,
            ecmascript_functions,
            elements,
            embedder_objects,
            environments,
            errors,
            finalization_registrys,
            globals: _,
            maps,
            modules,
            numbers,
            objects,
            primitive_objects,
            promise_capability_records,
            promise_reaction_records,
            promise_reject_functions,
            promises,
            proxys,
            realms,
            regexps,
            scripts,
            sets,
            shared_array_buffers,
            strings,
            symbols,
            typed_arrays,
            weak_maps,
            weak_refs,
            weak_sets,
        } = heap;
        let Environments {
            declarative: declarative_environments,
            function: function_environments,
            global: global_environments,
            object: object_environments,
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
        } = elements;
        let mut module_marks: Box<[ModuleIdentifier]> = queues.modules.drain(..).collect();
        module_marks.sort();
        module_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.modules.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                modules.get(index).mark_values(&mut queues, ());
            }
        });
        let mut script_marks: Box<[ScriptIdentifier]> = queues.scripts.drain(..).collect();
        script_marks.sort();
        script_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.scripts.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                scripts.get(index).mark_values(&mut queues, ());
            }
        });
        let mut realm_marks: Box<[RealmIdentifier]> = queues.realms.drain(..).collect();
        realm_marks.sort();
        realm_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.realms.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                realms.get(index).mark_values(&mut queues, ());
            }
        });

        let mut declarative_environment_marks: Box<[DeclarativeEnvironmentIndex]> =
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
                declarative_environments
                    .get(index)
                    .mark_values(&mut queues, ());
            }
        });
        let mut function_environment_marks: Box<[FunctionEnvironmentIndex]> =
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
                function_environments
                    .get(index)
                    .mark_values(&mut queues, ());
            }
        });
        let mut global_environment_marks: Box<[GlobalEnvironmentIndex]> =
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
                global_environments.get(index).mark_values(&mut queues, ());
            }
        });
        let mut object_environment_marks: Box<[ObjectEnvironmentIndex]> =
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
                object_environments.get(index).mark_values(&mut queues, ());
            }
        });

        let mut array_marks: Box<[ArrayIndex]> = queues.arrays.drain(..).collect();
        array_marks.sort();
        array_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.arrays.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                arrays.get(index).mark_values(&mut queues, ());
            }
        });
        let mut array_buffer_marks: Box<[ArrayBufferIndex]> =
            queues.array_buffers.drain(..).collect();
        array_buffer_marks.sort();
        array_buffer_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.array_buffers.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                array_buffers.get(index).mark_values(&mut queues, ());
            }
        });
        let mut bigint_marks: Box<[BigIntIndex]> = queues.bigints.drain(..).collect();
        bigint_marks.sort();
        bigint_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.bigints.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                bigints.get(index).mark_values(&mut queues, ());
            }
        });
        let mut bound_function_marks: Box<[BoundFunctionIndex]> =
            queues.bound_functions.drain(..).collect();
        bound_function_marks.sort();
        bound_function_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.bound_functions.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                bound_functions.get(index).mark_values(&mut queues, ());
            }
        });
        let mut error_marks: Box<[ErrorIndex]> = queues.errors.drain(..).collect();
        let mut ecmascript_function_marks: Box<[ECMAScriptFunctionIndex]> =
            queues.ecmascript_functions.drain(..).collect();
        ecmascript_function_marks.sort();
        ecmascript_function_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.ecmascript_functions.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                ecmascript_functions.get(index).mark_values(&mut queues, ());
            }
        });
        error_marks.sort();
        error_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.errors.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                errors.get(index).mark_values(&mut queues, ());
            }
        });
        let mut builtin_functions_marks: Box<[BuiltinFunctionIndex]> =
            queues.builtin_functions.drain(..).collect();
        builtin_functions_marks.sort();
        builtin_functions_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.builtin_functions.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                builtin_functions.get(index).mark_values(&mut queues, ());
            }
        });
        let mut data_view_marks: Box<[DataViewIndex]> = queues.data_views.drain(..).collect();
        data_view_marks.sort();
        data_view_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.data_views.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                data_views.get(index).mark_values(&mut queues, ());
            }
        });
        let mut date_marks: Box<[DateIndex]> = queues.dates.drain(..).collect();
        date_marks.sort();
        date_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.dates.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                dates.get(index).mark_values(&mut queues, ());
            }
        });
        let mut embedder_object_marks: Box<[EmbedderObjectIndex]> =
            queues.embedder_objects.drain(..).collect();
        embedder_object_marks.sort();
        embedder_object_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.embedder_objects.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                embedder_objects.get(index).mark_values(&mut queues, ());
            }
        });
        let mut finalization_registry_marks: Box<[FinalizationRegistryIndex]> =
            queues.finalization_registrys.drain(..).collect();
        finalization_registry_marks.sort();
        finalization_registry_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.finalization_registrys.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                finalization_registrys
                    .get(index)
                    .mark_values(&mut queues, ());
            }
        });
        let mut object_marks: Box<[ObjectIndex]> = queues.objects.drain(..).collect();
        object_marks.sort();
        object_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.objects.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                objects.get(index).mark_values(&mut queues, ());
            }
        });
        let mut promise_marks: Box<[PromiseIndex]> = queues.promises.drain(..).collect();
        promise_marks.sort();
        promise_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.promises.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                promises.get(index).mark_values(&mut queues, ());
            }
        });
        let mut proxy_marks: Box<[ProxyIndex]> = queues.proxys.drain(..).collect();
        proxy_marks.sort();
        proxy_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.proxys.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                proxys.get(index).mark_values(&mut queues, ());
            }
        });
        let mut map_marks: Box<[MapIndex]> = queues.maps.drain(..).collect();
        map_marks.sort();
        map_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.maps.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                maps.get(index).mark_values(&mut queues, ());
            }
        });
        let mut number_marks: Box<[NumberIndex]> = queues.numbers.drain(..).collect();
        number_marks.sort();
        number_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.numbers.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                numbers.get(index).mark_values(&mut queues, ());
            }
        });
        let mut primitive_object_marks: Box<[PrimitiveObjectIndex]> =
            queues.primitive_objects.drain(..).collect();
        primitive_object_marks.sort();
        primitive_object_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.primitive_objects.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                primitive_objects.get(index).mark_values(&mut queues, ());
            }
        });
        let mut regexp_marks: Box<[RegExpIndex]> = queues.regexps.drain(..).collect();
        regexp_marks.sort();
        regexp_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.regexps.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                regexps.get(index).mark_values(&mut queues, ());
            }
        });
        let mut set_marks: Box<[SetIndex]> = queues.sets.drain(..).collect();
        set_marks.sort();
        set_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.sets.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                sets.get(index).mark_values(&mut queues, ());
            }
        });
        let mut shared_array_buffer_marks: Box<[SharedArrayBufferIndex]> =
            queues.shared_array_buffers.drain(..).collect();
        shared_array_buffer_marks.sort();
        shared_array_buffer_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.shared_array_buffers.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                shared_array_buffers.get(index).mark_values(&mut queues, ());
            }
        });
        let mut string_marks: Box<[StringIndex]> = queues.strings.drain(..).collect();
        string_marks.sort();
        string_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.strings.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                strings.get(index).mark_values(&mut queues, ());
            }
        });
        let mut symbol_marks: Box<[SymbolIndex]> = queues.symbols.drain(..).collect();
        symbol_marks.sort();
        symbol_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.symbols.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                symbols.get(index).mark_values(&mut queues, ());
            }
        });
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
                typed_arrays.get(index).mark_values(&mut queues, ());
            }
        });
        let mut weak_map_marks: Box<[WeakMapIndex]> = queues.weak_maps.drain(..).collect();
        weak_map_marks.sort();
        weak_map_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.weak_maps.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                weak_maps.get(index).mark_values(&mut queues, ());
            }
        });
        let mut weak_ref_marks: Box<[WeakRefIndex]> = queues.weak_refs.drain(..).collect();
        weak_ref_marks.sort();
        weak_ref_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.weak_refs.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                weak_refs.get(index).mark_values(&mut queues, ());
            }
        });
        let mut weak_set_marks: Box<[WeakSetIndex]> = queues.weak_sets.drain(..).collect();
        weak_set_marks.sort();
        weak_set_marks.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.weak_sets.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                weak_sets.get(index).mark_values(&mut queues, ());
            }
        });
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
                e2pow4.values.get(index).mark_values(&mut queues, len);
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
                e2pow6.values.get(index).mark_values(&mut queues, len);
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
                e2pow8.values.get(index).mark_values(&mut queues, len);
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
                e2pow10.values.get(index).mark_values(&mut queues, len);
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
                e2pow12.values.get(index).mark_values(&mut queues, len);
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
                e2pow16.values.get(index).mark_values(&mut queues, len);
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
                e2pow24.values.get(index).mark_values(&mut queues, len);
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
                e2pow32.values.get(index).mark_values(&mut queues, len);
            }
        });
    }

    sweep(heap, &bits);
}

fn sweep(heap: &mut Heap, bits: &HeapBits) {
    let compactions = CompactionLists::create_from_bits(bits);

    let Heap {
        abstract_closures,
        array_buffers,
        arrays,
        bigints,
        bound_functions,
        builtin_functions,
        data_views,
        dates,
        ecmascript_functions,
        elements,
        embedder_objects,
        environments,
        errors,
        finalization_registrys,
        globals,
        maps,
        modules,
        numbers,
        objects,
        primitive_objects,
        promise_capability_records,
        promise_reaction_records,
        promise_reject_functions,
        promises,
        proxys,
        realms,
        regexps,
        scripts,
        sets,
        shared_array_buffers,
        strings,
        symbols,
        typed_arrays,
        weak_maps,
        weak_refs,
        weak_sets,
    } = heap;
    let Environments {
        declarative,
        function,
        global,
        object,
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
    } = elements;

    thread::scope(|s| {
        s.spawn(|| {
            for value in globals {
                value.sweep_values(&compactions, ());
            }
        });
        s.spawn(|| {
            sweep_heap_u16_elements_vector_values(&mut e2pow10.values, &compactions, &bits.e_2_10);
        });
        s.spawn(|| {
            sweep_heap_u16_elements_vector_values(&mut e2pow12.values, &compactions, &bits.e_2_12);
        });
        s.spawn(|| {
            sweep_heap_u16_elements_vector_values(&mut e2pow16.values, &compactions, &bits.e_2_16);
        });
        s.spawn(|| {
            sweep_heap_u32_elements_vector_values(&mut e2pow24.values, &compactions, &bits.e_2_24);
        });
        s.spawn(|| {
            sweep_heap_u32_elements_vector_values(&mut e2pow32.values, &compactions, &bits.e_2_32);
        });
        s.spawn(|| {
            sweep_heap_u8_elements_vector_values(&mut e2pow4.values, &compactions, &bits.e_2_4);
        });
        s.spawn(|| {
            sweep_heap_u8_elements_vector_values(&mut e2pow6.values, &compactions, &bits.e_2_6);
        });
        s.spawn(|| {
            sweep_heap_u8_elements_vector_values(&mut e2pow8.values, &compactions, &bits.e_2_8);
        });
        s.spawn(|| {
            sweep_heap_vector_values(array_buffers, &compactions, &bits.array_buffers);
        });
        s.spawn(|| {
            sweep_heap_vector_values(arrays, &compactions, &bits.arrays);
        });
        s.spawn(|| {
            sweep_heap_vector_values(bigints, &compactions, &bits.bigints);
        });
        s.spawn(|| {
            sweep_heap_vector_values(bound_functions, &compactions, &bits.bound_functions);
        });
        s.spawn(|| {
            sweep_heap_vector_values(builtin_functions, &compactions, &bits.builtin_functions);
        });
        s.spawn(|| {
            sweep_heap_vector_values(data_views, &compactions, &bits.data_views);
        });
        s.spawn(|| {
            sweep_heap_vector_values(dates, &compactions, &bits.dates);
        });
        s.spawn(|| {
            sweep_heap_vector_values(declarative, &compactions, &bits.declarative_environments);
        });
        s.spawn(|| {
            sweep_heap_vector_values(
                ecmascript_functions,
                &compactions,
                &bits.ecmascript_functions,
            );
        });
        s.spawn(|| {
            sweep_heap_vector_values(embedder_objects, &compactions, &bits.embedder_objects);
        });
        s.spawn(|| {
            sweep_heap_vector_values(errors, &compactions, &bits.errors);
        });
        s.spawn(|| {
            sweep_heap_vector_values(
                finalization_registrys,
                &compactions,
                &bits.finalization_registrys,
            );
        });
        s.spawn(|| {
            sweep_heap_vector_values(function, &compactions, &bits.function_environments);
        });
        s.spawn(|| {
            sweep_heap_vector_values(global, &compactions, &bits.global_environments);
        });
        s.spawn(|| {
            sweep_heap_vector_values(maps, &compactions, &bits.maps);
        });
        s.spawn(|| {
            sweep_heap_vector_values(modules, &compactions, &bits.modules);
        });
        s.spawn(|| {
            sweep_heap_vector_values(numbers, &compactions, &bits.numbers);
        });
        s.spawn(|| {
            sweep_heap_vector_values(object, &compactions, &bits.object_environments);
        });
        s.spawn(|| {
            sweep_heap_vector_values(objects, &compactions, &bits.objects);
        });
        s.spawn(|| {
            sweep_heap_vector_values(primitive_objects, &compactions, &bits.primitive_objects);
        });
        s.spawn(|| {
            sweep_heap_vector_values(promises, &compactions, &bits.promises);
        });
        s.spawn(|| {
            sweep_heap_vector_values(proxys, &compactions, &bits.proxys);
        });
        s.spawn(|| {
            sweep_heap_vector_values(realms, &compactions, &bits.realms);
        });
        s.spawn(|| {
            sweep_heap_vector_values(regexps, &compactions, &bits.regexps);
        });
        s.spawn(|| {
            sweep_heap_vector_values(scripts, &compactions, &bits.scripts);
        });
        s.spawn(|| {
            sweep_heap_vector_values(sets, &compactions, &bits.sets);
        });
        s.spawn(|| {
            sweep_heap_vector_values(
                shared_array_buffers,
                &compactions,
                &bits.shared_array_buffers,
            );
        });
        s.spawn(|| {
            sweep_heap_vector_values(strings, &compactions, &bits.strings);
        });
        s.spawn(|| {
            sweep_heap_vector_values(symbols, &compactions, &bits.symbols);
        });
        s.spawn(|| {
            sweep_heap_vector_values(typed_arrays, &compactions, &bits.typed_arrays);
        });
        s.spawn(|| {
            sweep_heap_vector_values(weak_maps, &compactions, &bits.weak_maps);
        });
        s.spawn(|| {
            sweep_heap_vector_values(weak_refs, &compactions, &bits.weak_refs);
        });
        s.spawn(|| {
            sweep_heap_vector_values(weak_sets, &compactions, &bits.weak_sets);
        });
    });
}

#[test]
fn test_heap_gc() {
    let mut heap: Heap = Default::default();
    assert!(heap.objects.is_empty());
    let obj = Value::Object(heap.create_null_object(&[]));
    println!("Object: {:#?}", obj);
    heap.globals.push(obj);
    heap_gc(&mut heap);
    println!("Objects: {:#?}", heap.objects);
    assert_eq!(heap.objects.len(), 1);
    assert_eq!(heap.elements.e2pow4.values.len(), 0);
    assert!(heap.globals.last().is_some());
    println!("Global #1: {:#?}", heap.globals.last().unwrap());
}
