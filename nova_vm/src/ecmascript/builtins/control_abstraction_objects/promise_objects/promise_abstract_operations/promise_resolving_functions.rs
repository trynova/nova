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
pub struct PromiseResolvingFunctionHeapData<'gen> {
    pub(crate) object_index: Option<OrdinaryObject<'gen>>,
    pub(crate) promise_capability: PromiseCapability<'gen>,
    pub(crate) resolve_type: PromiseResolvingFunctionType,
}

pub(crate) type BuiltinPromiseResolvingFunctionIndex<'gen> = BaseIndex<PromiseResolvingFunctionHeapData<'gen>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuiltinPromiseResolvingFunction<'gen>(pub(crate) BuiltinPromiseResolvingFunctionIndex<'gen>);

impl BuiltinPromiseResolvingFunction<'_>{
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<BuiltinPromiseResolvingFunction<'gen>> for Function<'gen> {
    fn from(value: BuiltinPromiseResolvingFunction<'gen>) -> Self {
        Self::BuiltinPromiseResolvingFunction(value)
    }
}

impl<'gen> IntoFunction<'gen> for BuiltinPromiseResolvingFunction<'gen> {
    fn into_function(self) -> Function<'gen> {
        self.into()
    }
}

impl<'gen> From<BuiltinPromiseResolvingFunction<'gen>> for Object<'gen> {
    fn from(value: BuiltinPromiseResolvingFunction<'gen>) -> Self {
        Self::BuiltinPromiseResolvingFunction(value)
    }
}

impl<'gen> IntoObject<'gen> for BuiltinPromiseResolvingFunction<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<BuiltinPromiseResolvingFunction<'gen>> for Value<'gen> {
    fn from(value: BuiltinPromiseResolvingFunction<'gen>) -> Self {
        Self::BuiltinPromiseResolvingFunction(value)
    }
}

impl<'gen> IntoValue<'gen> for BuiltinPromiseResolvingFunction<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> InternalSlots<'gen> for BuiltinPromiseResolvingFunction<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> {
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

impl<'gen> InternalMethods<'gen> for BuiltinPromiseResolvingFunction<'gen> {
    fn internal_get_own_property(
        self,
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
    ) -> JsResult<'gen, Option<PropertyDescriptor<'gen>>> {
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
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
        property_descriptor: PropertyDescriptor<'gen>,
    ) -> JsResult<'gen, bool> {
        let object_index = agent[self]
            .object_index
            .unwrap_or_else(|| self.create_backing_object(agent));
        object_index.internal_define_own_property(agent, property_key, property_descriptor)
    }

    fn internal_has_property(self, agent: &mut Agent<'gen>, property_key: PropertyKey<'gen>) -> JsResult<'gen, bool> {
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
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
        receiver: Value<'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
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
        agent: &mut Agent<'gen>,
        property_key: PropertyKey<'gen>,
        value: Value<'gen>,
        receiver: Value<'gen>,
    ) -> JsResult<'gen, bool> {
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

    fn internal_delete(self, agent: &mut Agent<'gen>, property_key: PropertyKey<'gen>) -> JsResult<'gen, bool> {
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

    fn internal_own_property_keys(self, agent: &mut Agent<'gen>) -> JsResult<'gen, Vec<PropertyKey<'gen>>> {
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
        agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        args: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        let arg = args.get(0);
        let promise_capability = agent[self].promise_capability;
        match agent[self].resolve_type {
            PromiseResolvingFunctionType::Resolve => promise_capability.resolve(agent, arg),
            PromiseResolvingFunctionType::Reject => promise_capability.reject(agent, arg),
        };
        Ok(Value::Undefined)
    }
}

impl<'gen> Index<BuiltinPromiseResolvingFunction<'gen>> for Agent<'gen> {
    type Output = PromiseResolvingFunctionHeapData<'gen>;

    fn index(&self, index: BuiltinPromiseResolvingFunction<'gen>) -> &Self::Output {
        &self.heap.promise_resolving_functions[index]
    }
}

impl<'gen> IndexMut<BuiltinPromiseResolvingFunction<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: BuiltinPromiseResolvingFunction<'gen>) -> &mut Self::Output {
        &mut self.heap.promise_resolving_functions[index]
    }
}

impl<'gen> Index<BuiltinPromiseResolvingFunction<'gen>> for Vec<Option<PromiseResolvingFunctionHeapData<'gen>>> {
    type Output = PromiseResolvingFunctionHeapData<'gen>;

    fn index(&self, index: BuiltinPromiseResolvingFunction<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_ref()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl<'gen> IndexMut<BuiltinPromiseResolvingFunction<'gen>> for Vec<Option<PromiseResolvingFunctionHeapData<'gen>>> {
    fn index_mut(&mut self, index: BuiltinPromiseResolvingFunction<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_mut()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl<'gen> CreateHeapData<PromiseResolvingFunctionHeapData<'gen>, BuiltinPromiseResolvingFunction<'gen>> for Heap<'gen> {
    fn create(
        &mut self,
        data: PromiseResolvingFunctionHeapData<'gen>,
    ) -> BuiltinPromiseResolvingFunction<'gen> {
        self.promise_resolving_functions.push(Some(data));
        BuiltinPromiseResolvingFunction(BaseIndex::last(&self.promise_resolving_functions))
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for BuiltinPromiseResolvingFunction<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        queues.promise_resolving_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions
            .promise_resolving_functions
            .shift_index(&mut self.0);
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for PromiseResolvingFunctionHeapData<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        self.object_index.mark_values(queues);
        self.promise_capability.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.promise_capability.sweep_values(compactions);
    }
}
