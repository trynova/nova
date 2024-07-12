// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        builtins::{control_abstraction_objects::promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability, ArgumentsList},
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            Function, InternalMethods, InternalSlots, IntoFunction, IntoObject, IntoValue, Object,
            ObjectHeapData, OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{
        indexes::BaseIndex, CreateHeapData, Heap, HeapMarkAndSweep, ObjectEntry,
        ObjectEntryPropertyDescriptor,
    },
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum PromiseResolvingFunctionType {
    Resolve,
    Reject,
}

/// ### [27.2.1.3.1 Promise Reject Functions]()
///
/// A promise reject function is an anonymous built-in function that has
/// \[\[Promise\]\] and \[\[AlreadyResolved\]\] internal slots.
///
/// The "length" property of a promise reject function is 1𝔽.
#[derive(Debug, Clone, Copy)]
pub struct PromiseResolvingFunctionHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) promise_capability: PromiseCapability,
    pub(crate) resolve_type: PromiseResolvingFunctionType,
}

pub(crate) type BuiltinPromiseResolvingFunctionIndex = BaseIndex<PromiseResolvingFunctionHeapData>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuiltinPromiseResolvingFunction(pub(crate) BuiltinPromiseResolvingFunctionIndex);

impl BuiltinPromiseResolvingFunction {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<BuiltinPromiseResolvingFunction> for Function {
    fn from(value: BuiltinPromiseResolvingFunction) -> Self {
        Self::BuiltinPromiseResolvingFunction(value)
    }
}

impl IntoFunction for BuiltinPromiseResolvingFunction {
    fn into_function(self) -> Function {
        self.into()
    }
}

impl From<BuiltinPromiseResolvingFunction> for Object {
    fn from(value: BuiltinPromiseResolvingFunction) -> Self {
        Self::BuiltinPromiseResolvingFunction(value)
    }
}

impl IntoObject for BuiltinPromiseResolvingFunction {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<BuiltinPromiseResolvingFunction> for Value {
    fn from(value: BuiltinPromiseResolvingFunction) -> Self {
        Self::BuiltinPromiseResolvingFunction(value)
    }
}

impl IntoValue for BuiltinPromiseResolvingFunction {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl InternalSlots for BuiltinPromiseResolvingFunction {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<crate::ecmascript::types::OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> crate::ecmascript::types::OrdinaryObject {
        debug_assert!(self.get_backing_object(agent).is_none());
        let prototype = self.internal_prototype(agent);
        let length_entry = ObjectEntry {
            key: PropertyKey::from(BUILTIN_STRING_MEMORY.length),
            value: ObjectEntryPropertyDescriptor::Data {
                value: 1.into(),
                writable: false,
                enumerable: false,
                configurable: true,
            },
        };
        let name_entry = ObjectEntry {
            key: PropertyKey::from(BUILTIN_STRING_MEMORY.name),
            value: ObjectEntryPropertyDescriptor::Data {
                value: String::EMPTY_STRING.into_value(),
                writable: false,
                enumerable: false,
                configurable: true,
            },
        };
        let (keys, values) = agent
            .heap
            .elements
            .create_object_entries(&[length_entry, name_entry]);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype,
            keys,
            values,
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }
}

impl InternalMethods for BuiltinPromiseResolvingFunction {
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_get_own_property(agent, property_key)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            Ok(Some(PropertyDescriptor {
                value: Some(1.into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            }))
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
            Ok(Some(PropertyDescriptor {
                value: Some(String::EMPTY_STRING.into_value()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            }))
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
        let object_index = agent[self]
            .object_index
            .unwrap_or_else(|| self.create_backing_object(agent));
        object_index.internal_define_own_property(agent, property_key, property_descriptor)
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_has_property(agent, property_key)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
            || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
        {
            Ok(true)
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
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            Ok(1.into())
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
            Ok(String::EMPTY_STRING.into_value())
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
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_set(agent, property_key, value, receiver)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
            || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
        {
            // length and name are not writable
            Ok(false)
        } else {
            self.create_backing_object(agent)
                .internal_set(agent, property_key, value, receiver)
        }
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_delete(agent, property_key)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
            || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
        {
            let object_index = self.create_backing_object(agent);
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
            Ok(vec![
                PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                PropertyKey::from(BUILTIN_STRING_MEMORY.name),
            ])
        }
    }

    fn internal_call(
        self,
        agent: &mut Agent,
        _this_value: Value,
        args: ArgumentsList,
    ) -> JsResult<Value> {
        let arg = args.get(0);
        let promise_capability = agent[self].promise_capability;
        match agent[self].resolve_type {
            PromiseResolvingFunctionType::Resolve => promise_capability.resolve(agent, arg),
            PromiseResolvingFunctionType::Reject => promise_capability.reject(agent, arg),
        };
        Ok(Value::Undefined)
    }
}

impl Index<BuiltinPromiseResolvingFunction> for Agent {
    type Output = PromiseResolvingFunctionHeapData;

    fn index(&self, index: BuiltinPromiseResolvingFunction) -> &Self::Output {
        &self.heap.promise_resolving_functions[index]
    }
}

impl IndexMut<BuiltinPromiseResolvingFunction> for Agent {
    fn index_mut(&mut self, index: BuiltinPromiseResolvingFunction) -> &mut Self::Output {
        &mut self.heap.promise_resolving_functions[index]
    }
}

impl Index<BuiltinPromiseResolvingFunction> for Vec<Option<PromiseResolvingFunctionHeapData>> {
    type Output = PromiseResolvingFunctionHeapData;

    fn index(&self, index: BuiltinPromiseResolvingFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_ref()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl IndexMut<BuiltinPromiseResolvingFunction> for Vec<Option<PromiseResolvingFunctionHeapData>> {
    fn index_mut(&mut self, index: BuiltinPromiseResolvingFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_mut()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl CreateHeapData<PromiseResolvingFunctionHeapData, BuiltinPromiseResolvingFunction> for Heap {
    fn create(
        &mut self,
        data: PromiseResolvingFunctionHeapData,
    ) -> BuiltinPromiseResolvingFunction {
        self.promise_resolving_functions.push(Some(data));
        BuiltinPromiseResolvingFunction(BaseIndex::last(&self.promise_resolving_functions))
    }
}

impl HeapMarkAndSweep for BuiltinPromiseResolvingFunction {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.promise_resolving_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions
            .promise_resolving_functions
            .shift_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for PromiseResolvingFunctionHeapData {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        self.object_index.mark_values(queues);
        self.promise_capability.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.promise_capability.sweep_values(compactions);
    }
}
