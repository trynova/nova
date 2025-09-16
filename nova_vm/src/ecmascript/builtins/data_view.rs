// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

use self::data::{DataViewRecord, SharedDataViewHeapData};

use super::{
    ArrayBuffer,
    array_buffer::{ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset},
    shared_array_buffer::SharedArrayBuffer,
};

pub(crate) mod abstract_operations;
pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct DataView<'a>(BaseIndex<'a, DataViewRecord<'static>>);
bindable_handle!(DataView);

impl<'gc> DataView<'gc> {
    #[inline]
    pub fn byte_length(self, agent: &Agent) -> Option<usize> {
        let byte_length = self.get(agent).byte_length;
        if byte_length == ViewedArrayBufferByteLength::heap() {
            Some(*agent.heap.data_view_byte_lengths.get(&self).unwrap())
        } else if byte_length == ViewedArrayBufferByteLength::auto() {
            None
        } else {
            Some(byte_length.0 as usize)
        }
    }

    #[inline]
    pub fn byte_offset(self, agent: &Agent) -> usize {
        let byte_offset = self.get(agent).byte_offset;
        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            *agent.heap.data_view_byte_offsets.get(&self).unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    #[inline]
    pub fn get_viewed_array_buffer(
        self,
        agent: &Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> ArrayBuffer<'gc> {
        self.get(agent).viewed_array_buffer.bind(gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    #[inline(always)]
    fn get<'a>(self, agent: &'a Agent) -> &'a DataViewRecord<'gc> {
        self.get_direct(&agent.heap.data_views)
    }

    #[inline(always)]
    fn get_mut<'a>(self, agent: &'a mut Agent) -> &'a mut DataViewRecord<'gc> {
        self.get_direct_mut(&mut agent.heap.data_views)
    }

    #[inline(always)]
    fn get_direct<'a>(self, data_views: &'a [DataViewRecord<'static>]) -> &'a DataViewRecord<'gc> {
        data_views
            .get(self.get_index())
            .expect("Invalid DataView reference")
    }

    #[inline(always)]
    fn get_direct_mut<'a>(
        self,
        data_views: &'a mut [DataViewRecord<'static>],
    ) -> &'a mut DataViewRecord<'gc> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<&'a mut DataViewRecord<'static>, &'a mut DataViewRecord<'gc>>(
                data_views
                    .get_mut(self.get_index())
                    .expect("Invalid DataView reference"),
            )
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SharedDataView<'a>(BaseIndex<'a, SharedDataViewHeapData<'static>>);
bindable_handle!(SharedDataView);

impl<'gc> SharedDataView<'gc> {
    #[inline]
    pub fn byte_length(self, agent: &Agent) -> Option<usize> {
        let byte_length = self.get(agent).byte_length;
        if byte_length == ViewedArrayBufferByteLength::heap() {
            Some(*agent.heap.shared_data_view_byte_lengths.get(&self).unwrap())
        } else if byte_length == ViewedArrayBufferByteLength::auto() {
            None
        } else {
            Some(byte_length.0 as usize)
        }
    }

    #[inline]
    pub fn byte_offset(self, agent: &Agent) -> usize {
        let byte_offset = self.get(agent).byte_offset;
        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            *agent.heap.shared_data_view_byte_offsets.get(&self).unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    #[inline]
    pub fn get_viewed_array_buffer(
        self,
        agent: &Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> SharedArrayBuffer<'gc> {
        self.get(agent).viewed_array_buffer.bind(gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    #[inline(always)]
    fn get<'a>(self, agent: &'a Agent) -> &'a SharedDataViewHeapData<'gc> {
        self.get_direct(&agent.heap.shared_data_views)
    }

    #[inline(always)]
    fn get_mut<'a>(self, agent: &'a mut Agent) -> &'a mut SharedDataViewHeapData<'gc> {
        self.get_direct_mut(&mut agent.heap.shared_data_views)
    }

    #[inline(always)]
    fn get_direct<'a>(
        self,
        shared_data_views: &'a [SharedDataViewHeapData<'static>],
    ) -> &'a SharedDataViewHeapData<'gc> {
        shared_data_views
            .get(self.get_index())
            .expect("Invalid DataView reference")
    }

    #[inline(always)]
    fn get_direct_mut<'a>(
        self,
        shared_data_views: &'a mut [SharedDataViewHeapData<'static>],
    ) -> &'a mut SharedDataViewHeapData<'gc> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<
                &'a mut SharedDataViewHeapData<'static>,
                &'a mut SharedDataViewHeapData<'gc>,
            >(
                shared_data_views
                    .get_mut(self.get_index())
                    .expect("Invalid DataView reference"),
            )
        }
    }
}

impl<'a> From<DataView<'a>> for Value<'a> {
    fn from(value: DataView<'a>) -> Self {
        Value::DataView(value)
    }
}

impl<'a> From<DataView<'a>> for Object<'a> {
    fn from(value: DataView<'a>) -> Self {
        Object::DataView(value)
    }
}

impl<'a> TryFrom<Object<'a>> for DataView<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::DataView(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for DataView<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::DataView;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.unbind()
                .get_mut(agent)
                .object_index
                .replace(backing_object)
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for DataView<'a> {}

impl Rootable for DataView<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::DataView(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::DataView(object) => Some(object),
            _ => None,
        }
    }
}

impl<'a> From<SharedDataView<'a>> for Value<'a> {
    fn from(value: SharedDataView<'a>) -> Self {
        Value::SharedDataView(value)
    }
}

impl<'a> From<SharedDataView<'a>> for Object<'a> {
    fn from(value: SharedDataView<'a>) -> Self {
        Object::SharedDataView(value)
    }
}

impl<'a> TryFrom<Object<'a>> for SharedDataView<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::SharedDataView(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for SharedDataView<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::DataView;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.unbind()
                .get_mut(agent)
                .object_index
                .replace(backing_object)
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for SharedDataView<'a> {}

impl Rootable for SharedDataView<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::SharedDataView(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::SharedDataView(object) => Some(object),
            _ => None,
        }
    }
}

impl<'a> CreateHeapData<DataViewRecord<'a>, DataView<'a>> for Heap {
    fn create(&mut self, data: DataViewRecord<'a>) -> DataView<'a> {
        self.data_views.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<Option<DataViewRecord<'static>>>();
        DataView(BaseIndex::last_t(&self.data_views))
    }
}

impl HeapMarkAndSweep for DataView<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.data_views.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.data_views.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for DataView<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.data_views.shift_weak_index(self.0).map(Self)
    }
}

impl HeapMarkAndSweep for SharedDataView<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.shared_data_views.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.shared_data_views.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for SharedDataView<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .shared_data_views
            .shift_weak_index(self.0)
            .map(Self)
    }
}
