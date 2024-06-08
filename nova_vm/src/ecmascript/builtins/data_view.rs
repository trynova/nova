use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, ObjectHeapData,
            OrdinaryObjectInternalSlots, Value,
        },
    },
    heap::{indexes::DataViewIndex, CreateHeapData, Heap},
};

use self::data::DataViewHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DataView(pub(crate) DataViewIndex);

impl DataView {
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

impl Index<DataView> for Agent {
    type Output = DataViewHeapData;

    fn index(&self, index: DataView) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<DataView> for Agent {
    fn index_mut(&mut self, index: DataView) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<DataView> for Heap {
    type Output = DataViewHeapData;

    fn index(&self, index: DataView) -> &Self::Output {
        self.data_views
            .get(index.0.into_index())
            .expect("DataView out of bounds")
            .as_ref()
            .expect("DataView slot empty")
    }
}

impl IndexMut<DataView> for Heap {
    fn index_mut(&mut self, index: DataView) -> &mut Self::Output {
        self.data_views
            .get_mut(index.0.into_index())
            .expect("DataView out of bounds")
            .as_mut()
            .expect("DataView slot empty")
    }
}

impl OrdinaryObjectInternalSlots for DataView {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::DataView;

    fn get_backing_object(self, agent: &Agent) -> Option<crate::ecmascript::types::OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> crate::ecmascript::types::OrdinaryObject {
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

impl InternalMethods for DataView {}

impl Index<DataViewIndex> for Agent {
    type Output = DataViewHeapData;

    fn index(&self, index: DataViewIndex) -> &Self::Output {
        self.heap
            .data_views
            .get(index.into_index())
            .expect("DataViewIndex out of bounds")
            .as_ref()
            .expect("DataViewIndex slot empty")
    }
}

impl IndexMut<DataViewIndex> for Agent {
    fn index_mut(&mut self, index: DataViewIndex) -> &mut Self::Output {
        self.heap
            .data_views
            .get_mut(index.into_index())
            .expect("DataViewIndex out of bounds")
            .as_mut()
            .expect("DataViewIndex slot empty")
    }
}

impl CreateHeapData<DataViewHeapData, DataView> for Heap {
    fn create(&mut self, data: DataViewHeapData) -> DataView {
        self.data_views.push(Some(data));
        DataView::from(DataViewIndex::last(&self.data_views))
    }
}
