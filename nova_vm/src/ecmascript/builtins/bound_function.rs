use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::Agent,
        types::{BoundFunctionHeapData, InternalMethods, OrdinaryObjectInternalSlots},
    },
    heap::{
        indexes::BoundFunctionIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BoundFunction(BoundFunctionIndex);

impl BoundFunction {
    pub(crate) const fn _def() -> Self {
        BoundFunction(BoundFunctionIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl OrdinaryObjectInternalSlots for BoundFunction {
    fn internal_extensible(self, _agent: &Agent) -> bool {
        todo!()
    }

    fn internal_set_extensible(self, _agent: &mut Agent, _value: bool) {
        todo!()
    }

    fn internal_prototype(self, _agent: &Agent) -> Option<crate::ecmascript::types::Object> {
        todo!()
    }

    fn internal_set_prototype(
        self,
        _agent: &mut Agent,
        _prototype: Option<crate::ecmascript::types::Object>,
    ) {
        todo!()
    }
}

impl InternalMethods for BoundFunction {
    fn internal_get_prototype_of(
        self,
        _agent: &mut Agent,
    ) -> crate::ecmascript::execution::JsResult<Option<crate::ecmascript::types::Object>> {
        todo!()
    }

    fn internal_set_prototype_of(
        self,
        _agent: &mut Agent,
        _prototype: Option<crate::ecmascript::types::Object>,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_is_extensible(
        self,
        _agent: &mut Agent,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_prevent_extensions(
        self,
        _agent: &mut Agent,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<Option<crate::ecmascript::types::PropertyDescriptor>>
    {
        todo!()
    }

    fn internal_define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
        _property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_has_property(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_get(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
        _receiver: crate::ecmascript::types::Value,
    ) -> crate::ecmascript::execution::JsResult<crate::ecmascript::types::Value> {
        todo!()
    }

    fn internal_set(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
        _value: crate::ecmascript::types::Value,
        _receiver: crate::ecmascript::types::Value,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_delete(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_own_property_keys(
        self,
        _agent: &mut Agent,
    ) -> crate::ecmascript::execution::JsResult<Vec<crate::ecmascript::types::PropertyKey>> {
        todo!()
    }
}

impl Index<BoundFunction> for Agent {
    type Output = BoundFunctionHeapData;

    fn index(&self, index: BoundFunction) -> &Self::Output {
        self.heap
            .bound_functions
            .get(index.0.into_index())
            .expect("BoundFunction out of bounds")
            .as_ref()
            .expect("BoundFunction slot empty")
    }
}

impl IndexMut<BoundFunction> for Agent {
    fn index_mut(&mut self, index: BoundFunction) -> &mut Self::Output {
        self.heap
            .bound_functions
            .get_mut(index.0.into_index())
            .expect("BoundFunction out of bounds")
            .as_mut()
            .expect("BoundFunction slot empty")
    }
}

impl CreateHeapData<BoundFunctionHeapData, BoundFunction> for Heap {
    fn create(&mut self, data: BoundFunctionHeapData) -> BoundFunction {
        self.bound_functions.push(Some(data));
        BoundFunction(BoundFunctionIndex::last(&self.bound_functions))
    }
}

impl HeapMarkAndSweep for BoundFunction {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.bound_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.0.into_u32();
        self.0 = BoundFunctionIndex::from_u32(
            self_index - compactions.bound_functions.get_shift_for_index(self_index),
        );
    }
}

impl HeapMarkAndSweep for BoundFunctionHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.name.mark_values(queues);
        self.function.mark_values(queues);
        self.object_index.mark_values(queues);
        self.bound_values.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.name.sweep_values(compactions);
        self.function.sweep_values(compactions);
        self.object_index.sweep_values(compactions);
        self.bound_values.sweep_values(compactions);
    }
}
