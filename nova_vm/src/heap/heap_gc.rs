use super::{
    heap_bits::{CompactionLists, HeapBits, HeapCompaction, WorkQueues},
    indexes::{
        ArrayIndex, BuiltinFunctionIndex, DateIndex, ElementIndex, ErrorIndex, ObjectIndex,
        RegExpIndex, StringIndex, SymbolIndex,
    },
    Heap,
};
use crate::ecmascript::types::Value;

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
        let mut arrays: Box<[ArrayIndex]> = queues.arrays.drain(..).collect();
        arrays.sort();
        arrays.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.arrays.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                let heap_data = heap.arrays.get(index).unwrap().as_ref().unwrap();
                if let Some(object_index) = heap_data.object_index {
                    queues.push_value(Value::Object(object_index));
                }
                queues.push_elements_vector(&heap_data.elements);
            }
        });
        let mut errors: Box<[ErrorIndex]> = queues.errors.drain(..).collect();
        errors.sort();
        errors.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.errors.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                let data = heap.errors.get(index).unwrap().as_ref().unwrap();
                queues.objects.push(data.object_index);
            }
        });
        let mut builtin_functions: Box<[BuiltinFunctionIndex]> =
            queues.builtin_functions.drain(..).collect();
        builtin_functions.sort();
        builtin_functions.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.builtin_functions.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                let data = heap.builtin_functions.get(index).unwrap().as_ref().unwrap();
                if let Some(object_index) = data.object_index {
                    queues.objects.push(object_index);
                }
            }
        });
        let mut dates: Box<[DateIndex]> = queues.dates.drain(..).collect();
        dates.sort();
        dates.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.dates.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                let data = heap.dates.get(index).unwrap().as_ref().unwrap();
                queues.objects.push(data.object_index);
            }
        });
        let mut objects: Box<[ObjectIndex]> = queues.objects.drain(..).collect();
        objects.sort();
        objects.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.objects.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                let heap_data = heap.objects.get(index).unwrap().as_ref().unwrap();
                queues.push_elements_vector(&heap_data.keys);
                queues.push_elements_vector(&heap_data.values);
            }
        });
        let mut regexps: Box<[RegExpIndex]> = queues.regexps.drain(..).collect();
        regexps.sort();
        regexps.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.regexps.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                let data = heap.regexps.get(index).unwrap().as_ref().unwrap();
                queues.objects.push(data.object_index);
            }
        });
        let mut strings: Box<[StringIndex]> = queues.strings.drain(..).collect();
        strings.sort();
        strings.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.strings.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
            }
        });
        let mut symbols: Box<[SymbolIndex]> = queues.symbols.drain(..).collect();
        symbols.sort();
        symbols.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.symbols.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                let data = heap.symbols.get(index).unwrap().as_ref().unwrap();
                if let Some(string_index) = data.descriptor {
                    queues.push_value(string_index.into());
                }
            }
        });
        let mut e_2_4: Box<[(ElementIndex, u32)]> = queues.e_2_4.drain(..).collect();
        e_2_4.sort();
        e_2_4.iter().for_each(|&(idx, len)| {
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
                collect_values(
                    &mut queues,
                    &heap
                        .elements
                        .e2pow4
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[..(len as usize)],
                )
            }
        });
        let mut e_2_6: Box<[(ElementIndex, u32)]> = queues.e_2_6.drain(..).collect();
        e_2_6.sort();
        e_2_6.iter().for_each(|&(idx, len)| {
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
                collect_values(
                    &mut queues,
                    &heap
                        .elements
                        .e2pow6
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[..(len as usize)],
                );
            }
        });
        let mut e_2_8: Box<[(ElementIndex, u32)]> = queues.e_2_8.drain(..).collect();
        e_2_8.sort();
        e_2_8.iter().for_each(|&(idx, len)| {
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
                collect_values(
                    &mut queues,
                    &heap
                        .elements
                        .e2pow8
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[..len as usize],
                );
            }
        });
        let mut e_2_10: Box<[(ElementIndex, u32)]> = queues.e_2_10.drain(..).collect();
        e_2_10.sort();
        e_2_10.iter().for_each(|&(idx, len)| {
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
                collect_values(
                    &mut queues,
                    &heap
                        .elements
                        .e2pow10
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[..len as usize],
                );
            }
        });
        let mut e_2_12: Box<[(ElementIndex, u32)]> = queues.e_2_12.drain(..).collect();
        e_2_12.sort();
        e_2_12.iter().for_each(|&(idx, len)| {
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
                collect_values(
                    &mut queues,
                    &heap
                        .elements
                        .e2pow12
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[..len as usize],
                );
            }
        });
        let mut e_2_16: Box<[(ElementIndex, u32)]> = queues.e_2_16.drain(..).collect();
        e_2_16.sort();
        e_2_16.iter().for_each(|&(idx, len)| {
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
                collect_values(
                    &mut queues,
                    &heap
                        .elements
                        .e2pow16
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[..len as usize],
                );
            }
        });
        let mut e_2_24: Box<[(ElementIndex, u32)]> = queues.e_2_24.drain(..).collect();
        e_2_24.sort();
        e_2_24.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.e_2_24.get_mut(index) {
                if *marked {
                    // Already marked, panic: Elements are uniquely owned
                    // and any other reference existing to this entry is a sign of
                    // a GC algorithm bug.
                    panic!("ElementsVector was not unique");
                }
                *marked = true;
                *length = len as u32;
                collect_values(
                    &mut queues,
                    &heap
                        .elements
                        .e2pow24
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[..len as usize],
                );
            }
        });
        let mut e_2_32: Box<[(ElementIndex, u32)]> = queues.e_2_32.drain(..).collect();
        e_2_32.sort();
        e_2_32.iter().for_each(|&(idx, len)| {
            let index = idx.into_index();
            if let Some((marked, length)) = bits.e_2_32.get_mut(index) {
                if *marked {
                    // Already marked, panic: Elements are uniquely owned
                    // and any other reference existing to this entry is a sign of
                    // a GC algorithm bug.
                    panic!("ElementsVector was not unique");
                }
                *marked = true;
                *length = len as u32;
                collect_values(
                    &mut queues,
                    &heap
                        .elements
                        .e2pow32
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice()[..len as usize],
                );
            }
        });
    }

    sweep(heap, &bits);
}

fn sweep(heap: &mut Heap, bits: &HeapBits) {
    let compactions = CompactionLists::create_from_bits(bits);

    heap.elements
        .e2pow4
        .values
        .compact_u8_vec_values(&bits.e_2_4, &compactions);
    heap.elements
        .e2pow6
        .values
        .compact_u8_vec_values(&bits.e_2_6, &compactions);
    heap.elements
        .e2pow8
        .values
        .compact_u8_vec_values(&bits.e_2_8, &compactions);
    heap.elements
        .e2pow10
        .values
        .compact_u16_vec_values(&bits.e_2_10, &compactions);
    heap.elements
        .e2pow12
        .values
        .compact_u16_vec_values(&bits.e_2_12, &compactions);
    heap.elements
        .e2pow16
        .values
        .compact_u16_vec_values(&bits.e_2_16, &compactions);
    heap.elements
        .e2pow24
        .values
        .compact_u32_vec_values(&bits.e_2_24, &compactions);
    heap.elements
        .e2pow32
        .values
        .compact_u32_vec_values(&bits.e_2_32, &compactions);
    // heap.modules.compact_bool_vec_values(&bits.modules, &compactions);
    // heap.realms.compact_bool_vec_values(&bits.realms, &compactions);
    // heap.scripts.compact_bool_vec_values(&bits.scripts, &compactions);
    // heap.environments.declarative.compact_bool_vec_values(&bits.declarative_environments, &compactions);
    // heap.environments.function.compact_bool_vec_values(&bits.function_environments, &compactions);
    // heap.environments.global.compact_bool_vec_values(&bits.global_environments, &compactions);
    // heap.environments.object.compact_bool_vec_values(&bits.object_environments, &compactions);
    heap.arrays
        .compact_bool_vec_values(&bits.arrays, &compactions);
    // heap.array_buffers.compact_bool_vec_values(&bits.array_buffers);
    // heap.bigints.compact_bool_vec_values(&bits.bigints, &compactions);
    // heap.errors.compact_bool_vec_values(&bits.errors, &compactions);
    // heap.bound_functions.compact_bool_vec_values(&bits.bound_functions, &compactions);
    // heap.builtin_functions.compact_bool_vec_values(&bits.builtin_functions, &compactions);
    // heap.ecmascript_functions.compact_bool_vec_values(&bits.ecmascript_functions, &compactions);
    // heap.dates.compact_bool_vec_values(&bits.dates, &compactions);
    heap.globals.iter_mut().for_each(|value| {
        value.compact_self_values(&compactions);
    });
    heap.numbers
        .compact_bool_vec_values(&bits.numbers, &compactions);
    heap.objects
        .compact_bool_vec_values(&bits.objects, &compactions);
    // heap.regexps.compact_bool_vec_values(&bits.regexps, &compactions);
    heap.strings
        .compact_bool_vec_values(&bits.strings, &compactions);
    heap.symbols
        .compact_bool_vec_values(&bits.symbols, &compactions);
}

#[test]
fn test_heap_gc() {
    let mut heap: Heap = Default::default();
    assert!(!heap.objects.is_empty());
    let obj = Value::Object(heap.create_null_object(vec![]));
    println!("Object: {:#?}", obj);
    heap.globals.push(obj);
    heap_gc(&mut heap);
    println!("Objects: {:#?}", heap.objects);
    assert_eq!(heap.objects.len(), 1);
    assert_eq!(heap.elements.e2pow4.values.len(), 2);
    assert!(heap.globals.last().is_some());
    println!("Global #1: {:#?}", heap.globals.last().unwrap());
}
