use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        builtins::temporal::plain_time::data::PlainTimeHeapData,
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

pub(crate) mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TemporalPlainTime<'a>(BaseIndex<'a, PlainTimeHeapData<'static>>);

impl TemporalPlainTime<'_> {
    //TODO
    pub(crate) const fn _def() -> Self {
        TemporalPlainTime(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

bindable_handle!(TemporalPlainTime);

impl<'a> From<TemporalPlainTime<'a>> for Value<'a> {
    fn from(value: TemporalPlainTime<'a>) -> Self {
        Value::PlainTime(value)
    }
}
impl<'a> From<TemporalPlainTime<'a>> for Object<'a> {
    fn from(value: TemporalPlainTime<'a>) -> Self {
        Object::PlainTime(value)
    }
}
impl<'a> TryFrom<Value<'a>> for TemporalPlainTime<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, ()> {
        match value {
            Value::PlainTime(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}
impl<'a> TryFrom<Object<'a>> for TemporalPlainTime<'a> {
    type Error = ();

    fn try_from(object: Object<'a>) -> Result<Self, ()> {
        match object {
            Object::PlainTime(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for TemporalPlainTime<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::TemporalPlainTime;
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }
    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl<'a> InternalMethods<'a> for TemporalPlainTime<'a> {}

// TODO: get rid of Index impls, replace with get/get_mut/get_direct/get_direct_mut functions
impl Index<TemporalPlainTime<'_>> for Agent {
    type Output = PlainTimeHeapData<'static>;

    fn index(&self, index: TemporalPlainTime<'_>) -> &Self::Output {
        &self.heap.plain_times[index]
    }
}

impl IndexMut<TemporalPlainTime<'_>> for Agent {
    fn index_mut(&mut self, index: TemporalPlainTime<'_>) -> &mut Self::Output {
        &mut self.heap.plain_times[index]
    }
}

impl Index<TemporalPlainTime<'_>> for Vec<PlainTimeHeapData<'static>> {
    type Output = PlainTimeHeapData<'static>;

    fn index(&self, index: TemporalPlainTime<'_>) -> &Self::Output {
        self.get(index.get_index())
            .expect("heap acess out of bounds")
    }
}

impl IndexMut<TemporalPlainTime<'_>> for Vec<PlainTimeHeapData<'static>> {
    fn index_mut(&mut self, index: TemporalPlainTime<'_>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("heap access out of bounds")
    }
}

impl Rootable for TemporalPlainTime<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::PlainTime(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::PlainTime(object) => Some(object),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for TemporalPlainTime<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.plain_times.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.plain_times.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for TemporalPlainTime<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.plain_times.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<PlainTimeHeapData<'a>, TemporalPlainTime<'a>> for Heap {
    fn create(&mut self, data: PlainTimeHeapData<'a>) -> TemporalPlainTime<'a> {
        self.plain_times.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<PlainTimeHeapData<'static>>();
        TemporalPlainTime(BaseIndex::last(&self.plain_times))
    }
}
