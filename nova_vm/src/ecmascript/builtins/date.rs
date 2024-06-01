pub(crate) mod data;

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObjectInternalSlots,
            PropertyKey, Value,
        },
    },
    heap::{
        indexes::DateIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

use self::data::DateHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Date(pub(crate) DateIndex);

impl Date {
    pub(crate) const fn _def() -> Self {
        Self(DateIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl IntoValue for Date {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl From<Date> for Value {
    fn from(value: Date) -> Self {
        Value::Date(value)
    }
}

impl IntoObject for Date {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Date> for Object {
    fn from(value: Date) -> Self {
        Object::Date(value)
    }
}

impl TryFrom<Value> for Date {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, ()> {
        match value {
            Value::Date(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for Date {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, ()> {
        match value {
            Object::Date(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl OrdinaryObjectInternalSlots for Date {
    fn internal_extensible(self, _agent: &Agent) -> bool {
        false
    }

    fn internal_set_extensible(self, _agent: &mut Agent, _value: bool) {
        todo!()
    }

    fn internal_prototype(self, _agent: &Agent) -> Option<Object> {
        todo!()
    }

    fn internal_set_prototype(self, _agent: &mut Agent, _prototype: Option<Object>) {
        todo!()
    }
}

impl InternalMethods for Date {
    fn internal_get_prototype_of(self, _agent: &mut Agent) -> JsResult<Option<Object>> {
        todo!()
    }

    fn internal_set_prototype_of(
        self,
        _agent: &mut Agent,
        _prototype: Option<Object>,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_is_extensible(self, _agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn internal_prevent_extensions(self, _agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn internal_get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
    ) -> JsResult<Option<crate::ecmascript::types::PropertyDescriptor>> {
        todo!()
    }

    fn internal_define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_has_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_get(agent, property_key, receiver)
        } else {
            agent
                .current_realm()
                .intrinsics()
                .date_prototype()
                .internal_get(agent, property_key, receiver)
        }
    }

    fn internal_set(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_delete(self, _agent: &mut Agent, _property_key: PropertyKey) -> JsResult<bool> {
        todo!()
    }

    fn internal_own_property_keys(self, _agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        todo!()
    }
}

impl Index<Date> for Agent {
    type Output = DateHeapData;

    fn index(&self, index: Date) -> &Self::Output {
        self.heap
            .dates
            .get(index.get_index())
            .expect("Date out of bounds")
            .as_ref()
            .expect("Date slot empty")
    }
}

impl IndexMut<Date> for Agent {
    fn index_mut(&mut self, index: Date) -> &mut Self::Output {
        self.heap
            .dates
            .get_mut(index.get_index())
            .expect("Date out of bounds")
            .as_mut()
            .expect("Date slot empty")
    }
}

impl HeapMarkAndSweep for Date {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.dates.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.0.into_u32();
        self.0 =
            DateIndex::from_u32(self_index - compactions.dates.get_shift_for_index(self_index));
    }
}

impl CreateHeapData<DateHeapData, Date> for Heap {
    fn create(&mut self, data: DateHeapData) -> Date {
        self.dates.push(Some(data));
        Date(DateIndex::last(&self.dates))
    }
}
