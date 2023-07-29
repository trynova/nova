use std::collections::HashMap;

use crate::{
    heap::{heap_trace::HeapTrace, Heap, HeapBits},
    types::{PropertyDescriptor, PropertyKey},
};

#[derive(Debug, Clone)]
pub struct ObjectHeapData {
    pub(crate) bits: HeapBits,
    pub(crate) extensible: bool,
    // TODO: It's probably not necessary to have a whole data descriptor here.
    // A prototype can only be set to be null or an object, meaning that most of the
    // possible Value options are impossible.
    // We could possibly do with just a `Option<ObjectIndex>` but it would cause issues
    // with functions and possible other special object cases we want to track with partially
    // separate heap fields later down the line.
    pub(crate) prototype: PropertyDescriptor,
    // TODO: Consider using detached vectors for keys/descriptors.
    pub(crate) entries: Vec<ObjectEntry>,
}

#[derive(Debug, Clone)]
pub struct ObjectEntry {
    pub(crate) key: PropertyKey,
    pub(crate) value: PropertyDescriptor,
}

impl ObjectHeapData {
    pub fn dummy() -> Self {
        Self {
            bits: HeapBits::new(),
            extensible: false,
            prototype: PropertyDescriptor::default(),
            entries: Vec::new(),
        }
    }
}

impl HeapTrace for Option<ObjectHeapData> {
    fn trace(&self, heap: &Heap) {
        assert!(self.is_some());
        let data = self.as_ref().unwrap();

        let dirty = data.bits.dirty.replace(false);
        let marked = data.bits.marked.replace(true);

        if marked && !dirty {
            // Do not keep recursing into already-marked heap values.
            return;
        }

        if let Some(value) = data.prototype.value {
            value.trace(heap);
        }

        for entry in data.entries.iter() {
            entry.key.into_value().trace(heap);
            if let Some(value) = entry.value.value {
                value.trace(heap);
            }
        }
    }

    fn root(&self, _heap: &Heap) {
        assert!(self.is_some());
        self.as_ref().unwrap().bits.root();
    }

    fn unroot(&self, _heap: &Heap) {
        assert!(self.is_some());
        self.as_ref().unwrap().bits.unroot();
    }

    fn finalize(&mut self, _heap: &Heap) {
        self.take();
    }
}
