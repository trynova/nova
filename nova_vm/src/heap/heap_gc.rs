use super::{
    element_array::ElementArrayKey,
    heap_bits::{HeapBits, WorkQueues},
    indexes::{
        ArrayIndex, DateIndex, ElementIndex, ErrorIndex, FunctionIndex, ObjectIndex, RegExpIndex,
        StringIndex, SymbolIndex,
    },
    ElementsVector, Heap,
};
use crate::ecmascript::types::Value;
use std::sync::atomic::Ordering;

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
                let ElementsVector {
                    elements_index,
                    cap,
                    ..
                } = &heap_data.elements;
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
        let mut functions: Box<[FunctionIndex]> = queues.functions.drain(..).collect();
        functions.sort();
        functions.iter().for_each(|&idx| {
            let index = idx.into_index();
            if let Some(marked) = bits.functions.get(index) {
                if marked.load(Ordering::Acquire) {
                    // Already marked, ignore
                    return;
                }
                marked.store(true, Ordering::Relaxed);
                let data = heap.functions.get(index).unwrap().as_ref().unwrap();
                if let Some(object_index) = data.object_index {
                    queues.objects.push(object_index);
                }
                // if let Some(bound) = &data.bound {
                //     bound.iter().for_each(|&value| {
                //         queues.push_value(value);
                //     })
                // }
                // if let Some(visible) = &data.visible {
                //     visible.iter().for_each(|&value| {
                //         queues.push_value(value);
                //     })
                // }
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
                let ElementsVector {
                    elements_index,
                    cap,
                    ..
                } = &heap_data.keys;
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
                let ElementsVector {
                    elements_index,
                    cap,
                    ..
                } = &heap_data.values;
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
                heap.elements
                    .e2pow4
                    .values
                    .get(index)
                    .unwrap()
                    .unwrap()
                    .iter()
                    .for_each(|&value| {
                        if let Some(value) = value {
                            queues.push_value(value)
                        }
                    });
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
                heap.elements
                    .e2pow6
                    .values
                    .get(index)
                    .unwrap()
                    .unwrap()
                    .iter()
                    .for_each(|&value| {
                        if let Some(value) = value {
                            queues.push_value(value)
                        }
                    });
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
                heap.elements
                    .e2pow8
                    .values
                    .get(index)
                    .unwrap()
                    .unwrap()
                    .iter()
                    .for_each(|&value| {
                        if let Some(value) = value {
                            queues.push_value(value)
                        }
                    });
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
                heap.elements
                    .e2pow10
                    .values
                    .get(index)
                    .unwrap()
                    .unwrap()
                    .iter()
                    .for_each(|&value| {
                        if let Some(value) = value {
                            queues.push_value(value)
                        }
                    });
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
                heap.elements
                    .e2pow12
                    .values
                    .get(index)
                    .unwrap()
                    .unwrap()
                    .iter()
                    .for_each(|&value| {
                        if let Some(value) = value {
                            queues.push_value(value)
                        }
                    });
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
                heap.elements
                    .e2pow16
                    .values
                    .get(index)
                    .unwrap()
                    .unwrap()
                    .iter()
                    .for_each(|&value| {
                        if let Some(value) = value {
                            queues.push_value(value)
                        }
                    });
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
                heap.elements
                    .e2pow24
                    .values
                    .get(index)
                    .unwrap()
                    .unwrap()
                    .iter()
                    .for_each(|&value| {
                        if let Some(value) = value {
                            queues.push_value(value)
                        }
                    });
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
                heap.elements
                    .e2pow32
                    .values
                    .get(index)
                    .unwrap()
                    .unwrap()
                    .iter()
                    .for_each(|&value| {
                        if let Some(value) = value {
                            queues.push_value(value)
                        }
                    });
            }
        });
    }

    sweep(heap, &bits);
}

fn sweep(heap: &mut Heap, bits: &HeapBits) {
    bits.e_2_4.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.elements.e2pow4.values.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.e_2_6.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.elements.e2pow6.values.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.e_2_8.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.elements.e2pow8.values.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.e_2_10.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.elements.e2pow10.values.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.e_2_12.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.elements.e2pow12.values.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.e_2_16.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.elements.e2pow16.values.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.e_2_24.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.elements.e2pow24.values.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.e_2_32.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.elements.e2pow32.values.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.arrays.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.arrays.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.bigints.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.bigints.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.dates.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.dates.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.errors.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.errors.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.functions.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.functions.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.numbers.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.numbers.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.objects.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.objects.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.regexps.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.regexps.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.strings.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.strings.get_mut(index).unwrap();
            reference.take();
        }
    });
    bits.symbols.iter().enumerate().for_each(|(index, bit)| {
        if !bit.load(Ordering::Acquire) {
            let reference = heap.symbols.get_mut(index).unwrap();
            reference.take();
        }
    });
}
