use super::{
    element_array::ElementArrayKey,
    heap_bits::{CompactionLists, HeapBits, WorkQueues},
    indexes::{
        ArrayIndex, BuiltinFunctionIndex, DateIndex, ElementIndex, ErrorIndex, ObjectIndex,
        RegExpIndex, StringIndex, SymbolIndex,
    },
    ElementsVector, Heap,
};
use crate::ecmascript::types::Value;
use std::sync::atomic::Ordering;

fn collect_elements(queues: &mut WorkQueues, elements: &ElementsVector) {
    let ElementsVector {
        elements_index,
        cap,
        ..
    } = &elements;
    match cap {
        ElementArrayKey::E4 => queues.e_2_4.push(*elements_index),
        ElementArrayKey::E6 => queues.e_2_6.push(*elements_index),
        ElementArrayKey::E8 => queues.e_2_8.push(*elements_index),
        ElementArrayKey::E10 => queues.e_2_10.push(*elements_index),
        ElementArrayKey::E12 => queues.e_2_12.push(*elements_index),
        ElementArrayKey::E16 => queues.e_2_16.push(*elements_index),
        ElementArrayKey::E24 => queues.e_2_24.push(*elements_index),
        ElementArrayKey::E32 => queues.e_2_32.push(*elements_index),
    }
}

fn collect_values(queues: &mut WorkQueues, values: &[Option<Value>]) {
    values.iter().for_each(|maybe_value| {
        if let Some(value) = maybe_value {
            queues.push_value(*value);
        }
    });
}

pub fn heap_gc(heap: &mut Heap) {
    let bits = HeapBits::new(heap);
    let mut queues = WorkQueues::new(heap);

    heap.globals.iter().for_each(|&value| {
        queues.push_value(value);
    });

    while !queues.is_empty() {
        let mut arrays: Box<[ArrayIndex]> = queues.arrays.drain(..).collect();
        arrays.sort();
        arrays.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.arrays.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                let heap_data = heap.arrays.get(index).unwrap().as_ref().unwrap();
                if let Some(object_index) = heap_data.object_index {
                    queues.push_value(Value::Object(object_index));
                }
                collect_elements(&mut queues, &heap_data.elements);
            }
        });
        let mut errors: Box<[ErrorIndex]> = queues.errors.drain(..).collect();
        errors.sort();
        errors.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.errors.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                let data = heap.errors.get(index).unwrap().as_ref().unwrap();
                queues.objects.push(data.object_index);
            }
        });
        let mut builtin_functions: Box<[BuiltinFunctionIndex]> =
            queues.builtin_functions.drain(..).collect();
        builtin_functions.sort();
        builtin_functions.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.builtin_functions.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
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
            if let Some(marked) = bits.dates.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                let data = heap.dates.get(index).unwrap().as_ref().unwrap();
                queues.objects.push(data.object_index);
            }
        });
        let mut objects: Box<[ObjectIndex]> = queues.objects.drain(..).collect();
        objects.sort();
        objects.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.objects.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                let heap_data = heap.objects.get(index).unwrap().as_ref().unwrap();
                collect_elements(&mut queues, &heap_data.keys);
                collect_elements(&mut queues, &heap_data.values);
            }
        });
        let mut regexps: Box<[RegExpIndex]> = queues.regexps.drain(..).collect();
        regexps.sort();
        regexps.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.regexps.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                let data = heap.regexps.get(index).unwrap().as_ref().unwrap();
                queues.objects.push(data.object_index);
            }
        });
        let mut strings: Box<[StringIndex]> = queues.strings.drain(..).collect();
        strings.sort();
        strings.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.strings.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
            }
        });
        let mut symbols: Box<[SymbolIndex]> = queues.symbols.drain(..).collect();
        symbols.sort();
        symbols.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.symbols.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                let data = heap.symbols.get(index).unwrap().as_ref().unwrap();
                if let Some(string_index) = data.descriptor {
                    queues.push_value(string_index.into());
                }
            }
        });
        let mut e_2_4: Box<[ElementIndex]> = queues.e_2_4.drain(..).collect();
        e_2_4.sort();
        e_2_4.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.e_2_4.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                collect_values(
                    &mut queues,
                    heap.elements
                        .e2pow4
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice(),
                )
            }
        });
        let mut e_2_6: Box<[ElementIndex]> = queues.e_2_6.drain(..).collect();
        e_2_6.sort();
        e_2_6.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.e_2_6.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                collect_values(
                    &mut queues,
                    heap.elements
                        .e2pow6
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice(),
                );
            }
        });
        let mut e_2_8: Box<[ElementIndex]> = queues.e_2_8.drain(..).collect();
        e_2_8.sort();
        e_2_8.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.e_2_8.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                collect_values(
                    &mut queues,
                    heap.elements
                        .e2pow8
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice(),
                );
            }
        });
        let mut e_2_10: Box<[ElementIndex]> = queues.e_2_10.drain(..).collect();
        e_2_10.sort();
        e_2_10.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.e_2_10.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                collect_values(
                    &mut queues,
                    heap.elements
                        .e2pow10
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice(),
                );
            }
        });
        let mut e_2_12: Box<[ElementIndex]> = queues.e_2_12.drain(..).collect();
        e_2_12.sort();
        e_2_12.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.e_2_12.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                collect_values(
                    &mut queues,
                    heap.elements
                        .e2pow12
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice(),
                );
            }
        });
        let mut e_2_16: Box<[ElementIndex]> = queues.e_2_16.drain(..).collect();
        e_2_16.sort();
        e_2_16.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.e_2_16.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                collect_values(
                    &mut queues,
                    heap.elements
                        .e2pow16
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice(),
                );
            }
        });
        let mut e_2_24: Box<[ElementIndex]> = queues.e_2_24.drain(..).collect();
        e_2_24.sort();
        e_2_24.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.e_2_24.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                collect_values(
                    &mut queues,
                    heap.elements
                        .e2pow24
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice(),
                );
            }
        });
        let mut e_2_32: Box<[ElementIndex]> = queues.e_2_32.drain(..).collect();
        e_2_32.sort();
        e_2_32.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.e_2_32.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                collect_values(
                    &mut queues,
                    heap.elements
                        .e2pow32
                        .values
                        .get(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .as_slice(),
                );
            }
        });
    }

    sweep(heap, &bits);
}

fn sweep(heap: &mut Heap, bits: &HeapBits) {
    let _compaction_lists = CompactionLists::create_from_bits(bits);

    let mut iter = bits.e_2_4.iter();
    heap.elements.e2pow4.values.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.e_2_4.iter();
    heap.elements.e2pow6.values.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.e_2_6.iter();
    heap.elements.e2pow6.values.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.e_2_8.iter();
    heap.elements.e2pow10.values.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.e_2_10.iter();
    heap.elements.e2pow12.values.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.e_2_16.iter();
    heap.elements.e2pow16.values.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.e_2_24.iter();
    heap.elements.e2pow24.values.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.e_2_32.iter();
    heap.elements.e2pow32.values.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.e_2_32.iter();
    heap.elements.e2pow32.values.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.modules.iter();
    heap.modules.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.realms.iter();
    heap.realms.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.scripts.iter();
    heap.scripts.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.declarative_environments.iter();
    heap.environments.declarative.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.function_environments.iter();
    heap.environments.function.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.global_environments.iter();
    heap.environments.global.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.object_environments.iter();
    heap.environments.object.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.arrays.iter();
    heap.arrays.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.array_buffers.iter();
    heap.array_buffers.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.bigints.iter();
    heap.bigints.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.errors.iter();
    heap.errors.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.bound_functions.iter();
    heap.bound_functions.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.builtin_functions.iter();
    heap.builtin_functions.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.ecmascript_functions.iter();
    heap.ecmascript_functions.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.dates.iter();
    heap.dates.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.numbers.iter();
    heap.numbers.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.objects.iter();
    heap.objects.retain_mut(|_object| {
        iter
            .next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.regexps.iter();
    heap.regexps.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.strings.iter();
    heap.strings.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
    let mut iter = bits.symbols.iter();
    heap.symbols.retain_mut(|_vec| {
        iter.next()
            .map(|bit| bit.load(Ordering::Relaxed))
            .unwrap_or(true)
    });
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
    assert_eq!(heap.objects.len(), 0);
}
