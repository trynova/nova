// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use data::{DataViewByteLength, DataViewByteOffset};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    heap::{indexes::DataViewIndex, CreateHeapData, Heap, HeapMarkAndSweep},
};

use self::data::DataViewHeapData;

use super::ArrayBuffer;

pub(crate) mod abstract_operations;
pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct DataView(pub(crate) DataViewIndex);

impl DataView {
    #[inline]
    pub fn byte_length(self, agent: &Agent) -> Option<usize> {
        let byte_length = agent[self].byte_length;
        if byte_length == DataViewByteLength::heap() {
            Some(*agent.heap.data_view_byte_lengths.get(&self).unwrap())
        } else if byte_length == DataViewByteLength::auto() {
            None
        } else {
            Some(byte_length.0 as usize)
        }
    }

    #[inline]
    pub fn byte_offset(self, agent: &Agent) -> usize {
        let byte_offset = agent[self].byte_offset;
        if byte_offset == DataViewByteOffset::heap() {
            *agent.heap.data_view_byte_offsets.get(&self).unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    #[inline]
    pub fn get_viewed_array_buffer(self, agent: &Agent) -> ArrayBuffer {
        agent[self].viewed_array_buffer
    }

    pub(crate) const fn _def() -> Self {
        Self(DataViewIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<DataViewIndex> for DataView {
    fn from(value: DataViewIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for DataView {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for DataView {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<DataView> for Value {
    fn from(val: DataView) -> Self {
        Value::DataView(val)
    }
}

impl From<DataView> for Object {
    fn from(val: DataView) -> Self {
        Object::DataView(val)
    }
}

impl TryFrom<Object> for DataView {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::DataView(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl Index<DataView> for Agent {
    type Output = DataViewHeapData;

    fn index(&self, index: DataView) -> &Self::Output {
        &self.heap.data_views[index]
    }
}

impl IndexMut<DataView> for Agent {
    fn index_mut(&mut self, index: DataView) -> &mut Self::Output {
        &mut self.heap.data_views[index]
    }
}

impl Index<DataView> for Vec<Option<DataViewHeapData>> {
    type Output = DataViewHeapData;

    fn index(&self, index: DataView) -> &Self::Output {
        self.get(index.get_index())
            .expect("DataView out of bounds")
            .as_ref()
            .expect("DataView slot empty")
    }
}

impl IndexMut<DataView> for Vec<Option<DataViewHeapData>> {
    fn index_mut(&mut self, index: DataView) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("DataView out of bounds")
            .as_mut()
            .expect("DataView slot empty")
    }
}

impl InternalSlots for DataView {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::DataView;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl InternalMethods for DataView {}

impl CreateHeapData<DataViewHeapData, DataView> for Heap {
    fn create(&mut self, data: DataViewHeapData) -> DataView {
        self.data_views.push(Some(data));
        DataView::from(DataViewIndex::last(&self.data_views))
    }
}

impl HeapMarkAndSweep for DataView {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.data_views.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.data_views.shift_index(&mut self.0);
    }
}
