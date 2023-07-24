use crate::heap::{heap_trace::HeapTrace, Heap, HeapBits};

#[derive(Debug, Clone)]
pub struct ObjectHeapData {
    pub(crate) bits: HeapBits,
    pub(crate) _extensible: bool,
    // TODO: It's probably not necessary to have a whole data descriptor here.
    // A prototype can only be set to be null or an object, meaning that most of the
    // possible Value options are impossible.
    // We could possibly do with just a `Option<ObjectIndex>` but it would cause issues
    // with functions and possible other special object cases we want to track with partially
    // separate heap fields later down the line.
    // pub(crate) prototype: PropertyDescriptor,
    // pub(crate) entries: Vec<ObjectEntry>,
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
        // match &data.prototype {
        //     PropertyDescriptor::Data { value, .. } => value.trace(heap),
        //     PropertyDescriptor::Blocked { .. } => {}
        //     PropertyDescriptor::ReadOnly { get, .. } => {
        //         heap.functions[*get as usize].trace(heap);
        //     }
        //     PropertyDescriptor::WriteOnly { set, .. } => {
        //         heap.functions[*set as usize].trace(heap);
        //     }
        //     PropertyDescriptor::ReadWrite { get, set, .. } => {
        //         heap.functions[*get as usize].trace(heap);
        //         heap.functions[*set as usize].trace(heap);
        //     }
        // }
        // for reference in data.entries.iter() {
        //     match reference.key {
        //         PropertyKey::SmallAsciiString(_) | PropertyKey::Smi(_) => {}
        //         PropertyKey::String(idx) => heap.strings[idx as usize].trace(heap),
        //         PropertyKey::Symbol(idx) => heap.symbols[idx as usize].trace(heap),
        //     }
        //     match &reference.value {
        //         PropertyDescriptor::Data { value, .. } => value.trace(heap),
        //         PropertyDescriptor::Blocked { .. } => {}
        //         PropertyDescriptor::ReadOnly { get, .. } => {
        //             heap.functions[*get as usize].trace(heap);
        //         }
        //         PropertyDescriptor::WriteOnly { set, .. } => {
        //             heap.functions[*set as usize].trace(heap);
        //         }
        //         PropertyDescriptor::ReadWrite { get, set, .. } => {
        //             heap.functions[*get as usize].trace(heap);
        //             heap.functions[*set as usize].trace(heap);
        //         }
        //     }
        // }
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
