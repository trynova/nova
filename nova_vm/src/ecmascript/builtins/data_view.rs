// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    engine::{
        context::NoGcScope,
        rootable::{HeapRootData, HeapRootRef, Rootable},
        Scoped,
    },
    heap::{
        indexes::{DataViewIndex, IntoBaseIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

use self::data::DataViewHeapData;

use super::{
    array_buffer::{ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset},
    ArrayBuffer,
};

pub(crate) mod abstract_operations;
pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct DataView<'a>(pub(crate) DataViewIndex<'a>);

impl<'a> DataView<'a> {
    /// Unbind this DataView from its current lifetime. This is necessary to use
    /// the DataView as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> DataView<'static> {
        unsafe { core::mem::transmute::<DataView<'a>, DataView<'static>>(self) }
    }

    // Bind this DataView to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your DataViews cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let array_buffer = array_buffer.bind(&gc);
    // ```
    // to make sure that the unbound DataView cannot be used after binding.
    pub const fn bind(self, _: NoGcScope<'a, '_>) -> Self {
        unsafe { core::mem::transmute::<DataView, Self>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, DataView<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    #[inline]
    pub fn byte_length(self, agent: &Agent) -> Option<usize> {
        let byte_length = agent[self].byte_length;
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
        let byte_offset = agent[self].byte_offset;
        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            *agent.heap.data_view_byte_offsets.get(&self).unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    #[inline]
    pub fn get_viewed_array_buffer(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> ArrayBuffer<'a> {
        agent[self].viewed_array_buffer.bind(gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(DataViewIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'a> From<DataViewIndex<'a>> for DataView<'a> {
    fn from(value: DataViewIndex<'a>) -> Self {
        Self(value)
    }
}

impl<'a> IntoBaseIndex<'a, DataViewHeapData> for DataView<'a> {
    fn into_base_index(self) -> DataViewIndex<'a> {
        self.0
    }
}

impl<'a> IntoValue<'a> for DataView<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> IntoObject<'a> for DataView<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
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

impl Index<DataView<'_>> for Agent {
    type Output = DataViewHeapData;

    fn index(&self, index: DataView) -> &Self::Output {
        &self.heap.data_views[index]
    }
}

impl IndexMut<DataView<'_>> for Agent {
    fn index_mut(&mut self, index: DataView) -> &mut Self::Output {
        &mut self.heap.data_views[index]
    }
}

impl Index<DataView<'_>> for Vec<Option<DataViewHeapData>> {
    type Output = DataViewHeapData;

    fn index(&self, index: DataView) -> &Self::Output {
        self.get(index.get_index())
            .expect("DataView out of bounds")
            .as_ref()
            .expect("DataView slot empty")
    }
}

impl IndexMut<DataView<'_>> for Vec<Option<DataViewHeapData>> {
    fn index_mut(&mut self, index: DataView) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("DataView out of bounds")
            .as_mut()
            .expect("DataView slot empty")
    }
}

impl<'a> InternalSlots<'a> for DataView<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::DataView;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
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

impl CreateHeapData<DataViewHeapData, DataView<'static>> for Heap {
    fn create(&mut self, data: DataViewHeapData) -> DataView<'static> {
        self.data_views.push(Some(data));
        DataView::from(DataViewIndex::last(&self.data_views))
    }
}

impl HeapMarkAndSweep for DataView<'static> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.data_views.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.data_views.shift_index(&mut self.0);
    }
}
