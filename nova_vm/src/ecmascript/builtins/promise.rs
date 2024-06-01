use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObjectInternalSlots,
            PropertyDescriptor, PropertyKey, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, PromiseIndex},
        CreateHeapData, Heap, ObjectEntry, ObjectEntryPropertyDescriptor,
    },
};

use self::data::PromiseHeapData;

use super::ordinary::ordinary_set_prototype_of_check_loop;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Promise(pub(crate) PromiseIndex);

impl Promise {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<Promise> for PromiseIndex {
    fn from(val: Promise) -> Self {
        val.0
    }
}

impl From<PromiseIndex> for Promise {
    fn from(value: PromiseIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for Promise {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Promise {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Promise> for Value {
    fn from(val: Promise) -> Self {
        Value::Promise(val)
    }
}

impl From<Promise> for Object {
    fn from(val: Promise) -> Self {
        Object::Promise(val)
    }
}

impl OrdinaryObjectInternalSlots for Promise {
    fn internal_extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_extensible(agent)
        } else {
            true
        }
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set_extensible(agent, value)
        } else {
            // Create base object and set inextensible
            todo!()
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_prototype(agent)
        } else {
            Some(
                agent
                    .current_realm()
                    .intrinsics()
                    .promise_prototype()
                    .into_object(),
            )
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set_prototype(agent, prototype)
        } else {
            // Create base object and set inextensible
            todo!()
        }
    }
}

impl InternalMethods for Promise {
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(self.internal_prototype(agent))
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set_prototype_of(agent, prototype)
        } else {
            // If we're setting %Promise.prototype% then we can still avoid creating the ObjectHeapData.
            let current = agent.current_realm().intrinsics().promise_prototype();
            if prototype == Some(current.into_object()) {
                return Ok(true);
            }
            if ordinary_set_prototype_of_check_loop(agent, current.into_object(), prototype) {
                // OrdinarySetPrototypeOf 7.b.i: Setting prototype would cause a loop to occur.
                return Ok(false);
            }
            self.internal_set_prototype(agent, prototype);
            Ok(true)
        }
    }

    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        Ok(self.internal_extensible(agent))
    }

    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        self.internal_set_extensible(agent, false);
        Ok(true)
    }

    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_get_own_property(agent, property_key)
        } else {
            Ok(None)
        }
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_has_property(agent, property_key)
        } else {
            let prototype = agent.current_realm().intrinsics().promise_prototype();
            let new_entry = ObjectEntry {
                key: property_key,
                value: ObjectEntryPropertyDescriptor::from(property_descriptor),
            };
            let object_index = agent
                .heap
                .create_object_with_prototype(prototype.into_object(), &[new_entry]);
            agent[self].object_index = Some(object_index);
            Ok(true)
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_has_property(agent, property_key)
        } else {
            let parent = self.internal_get_prototype_of(agent)?;
            parent.map_or(Ok(false), |parent| {
                parent.internal_has_property(agent, property_key)
            })
        }
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
            let parent = self.internal_get_prototype_of(agent)?;
            parent.map_or(Ok(Value::Undefined), |parent| {
                parent.internal_get(agent, property_key, receiver)
            })
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set(agent, property_key, value, receiver)
        } else {
            let prototype = agent.current_realm().intrinsics().promise_prototype();
            prototype.internal_set(agent, property_key, value, receiver)
        }
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_delete(agent, property_key)
        } else {
            // Non-existing property
            Ok(true)
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_own_property_keys(agent)
        } else {
            Ok(vec![])
        }
    }
}

impl CreateHeapData<PromiseHeapData, Promise> for Heap {
    fn create(&mut self, data: PromiseHeapData) -> Promise {
        self.promises.push(Some(data));
        Promise(PromiseIndex::last(&self.promises))
    }
}

impl Index<Promise> for Agent {
    type Output = PromiseHeapData;

    fn index(&self, index: Promise) -> &Self::Output {
        self.heap
            .promises
            .get(index.get_index())
            .expect("Promise out of bounds")
            .as_ref()
            .expect("Promise slot empty")
    }
}

impl IndexMut<Promise> for Agent {
    fn index_mut(&mut self, index: Promise) -> &mut Self::Output {
        self.heap
            .promises
            .get_mut(index.get_index())
            .expect("Promise out of bounds")
            .as_mut()
            .expect("Promise slot empty")
    }
}
