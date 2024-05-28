use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object,
            OrdinaryObjectInternalSlots, PropertyDescriptor, PropertyKey, Value,
        },
    },
    heap::{indexes::DataViewIndex, Heap, ObjectEntry, ObjectEntryPropertyDescriptor},
};

use self::data::DataViewHeapData;

use super::ordinary::ordinary_set_prototype_of_check_loop;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataView(pub(crate) DataViewIndex);

impl From<DataView> for DataViewIndex {
    fn from(val: DataView) -> Self {
        val.0
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
        Value::DataView(val.0)
    }
}

impl From<DataView> for Object {
    fn from(val: DataView) -> Self {
        Object::DataView(val.0)
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
                    .data_view_prototype()
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

impl InternalMethods for DataView {
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
            // If we're setting %DataView.prototype% then we can still avoid creating the ObjectHeapData.
            let current = agent.current_realm().intrinsics().data_view_prototype();
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
            let prototype = agent.current_realm().intrinsics().data_view_prototype();
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
            let prototype = agent.current_realm().intrinsics().data_view_prototype();
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
