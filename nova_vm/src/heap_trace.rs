use crate::heap::Heap;

pub trait HeapTrace {
    fn trace(&self, heap: &Heap);

    fn root(&self, heap: &Heap);

    fn unroot(&self, heap: &Heap);

    fn finalize(&mut self, heap: &Heap);
}
