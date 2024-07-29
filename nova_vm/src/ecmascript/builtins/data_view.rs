// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData, Value,
        },
    },
    heap::{indexes::DataViewIndex, CreateHeapData, Heap, HeapMarkAndSweep},
};

use self::data::DataViewHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DataView<'gen>(pub(crate) DataViewIndex<'gen>);

impl<'gen> DataView<'gen> {
    pub(crate) const fn _def() -> Self {
        Self(DataViewIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<DataViewIndex<'gen>> for DataView<'gen> {
    fn from(value: DataViewIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for DataView<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for DataView<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<DataView<'gen>> for Value<'gen> {
    fn from(val: DataView<'gen>) -> Self {
        Value::DataView(val)
    }
}

impl<'gen> From<DataView<'gen>> for Object<'gen> {
    fn from(val: DataView<'gen>) -> Self {
        Object::DataView(val)
    }
}

impl<'gen> Index<DataView<'gen>> for Agent<'gen> {
    type Output = DataViewHeapData<'gen>;

    fn index(&self, index: DataView<'gen>) -> &Self::Output {
        &self.heap.data_views[index]
    }
}

impl<'gen> IndexMut<DataView<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: DataView<'gen>) -> &mut Self::Output {
        &mut self.heap.data_views[index]
    }
}

impl<'gen> Index<DataView<'gen>> for Vec<Option<DataViewHeapData<'gen>>> {
    type Output = DataViewHeapData<'gen>;

    fn index(&self, index: DataView<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("DataView out of bounds")
            .as_ref()
            .expect("DataView slot empty")
    }
}

impl<'gen> IndexMut<DataView<'gen>> for Vec<Option<DataViewHeapData<'gen>>> {
    fn index_mut(&mut self, index: DataView<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("DataView out of bounds")
            .as_mut()
            .expect("DataView slot empty")
    }
}

impl<'gen> InternalSlots<'gen> for DataView<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::DataView;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> {
        let prototype = agent
            .current_realm()
            .intrinsics()
            .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype),
            keys: Default::default(),
            values: Default::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }
}

impl<'gen> InternalMethods<'gen> for DataView<'gen> {}

impl<'gen> CreateHeapData<DataViewHeapData<'gen>, DataView<'gen>> for Heap<'gen> {
    fn create(&mut self, data: DataViewHeapData) -> DataView {
        self.data_views.push(Some(data));
        DataView::from(DataViewIndex::last(&self.data_views))
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for DataView<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        queues.data_views.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.data_views.shift_index(&mut self.0);
    }
}
