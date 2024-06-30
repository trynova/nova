use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        builtins::promise::Promise,
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

/// ### [27.2.1.3.1 Promise Reject Functions]()
///
/// A promise reject function is an anonymous built-in function that has
/// \[\[Promise\]\] and \[\[AlreadyResolved\]\] internal slots.
///
/// The "length" property of a promise reject function is 1𝔽.
#[derive(Debug, Clone, Copy)]
pub struct PromiseRejectFunctionHeapData {
    /// \[\[Promise\]\]
    pub(crate) promise: Promise,
    /// \[\[AlreadyResolved\]\]
    pub(crate) already_resolved: bool,
    pub(crate) object_index: Option<OrdinaryObject>,
}

pub(crate) type BuiltinPromiseRejectFunctionIndex = BaseIndex<PromiseRejectFunctionHeapData>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuiltinPromiseRejectFunction(pub(crate) BuiltinPromiseRejectFunctionIndex);

impl BuiltinPromiseRejectFunction {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<BuiltinPromiseRejectFunction> for Function {
    fn from(value: BuiltinPromiseRejectFunction) -> Self {
        Self::BuiltinPromiseRejectFunction(value)
    }
}

impl IntoFunction for BuiltinPromiseRejectFunction {
    fn into_function(self) -> Function {
        self.into()
    }
}

impl From<BuiltinPromiseRejectFunction> for Object {
    fn from(value: BuiltinPromiseRejectFunction) -> Self {
        Self::BuiltinPromiseRejectFunction(value)
    }
}

impl IntoObject for BuiltinPromiseRejectFunction {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<BuiltinPromiseRejectFunction> for Value {
    fn from(value: BuiltinPromiseRejectFunction) -> Self {
        Self::BuiltinPromiseRejectFunction(value)
    }
}

impl IntoValue for BuiltinPromiseRejectFunction {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl PromiseRejectFunctionHeapData {
    /// When a promise reject function is called with argument reason, the
    /// following steps are taken:
    pub(crate) fn call(agent: &mut Agent, _reason: Value) {
        // 1. Let F be the active function object.
        let f = agent.running_execution_context().function.unwrap();
        // 2. Assert: F has a [[Promise]] internal slot whose value is an Object.
        let Function::BuiltinPromiseRejectFunction(f) = f else {
            unreachable!();
        };
        // 3. Let promise be F.[[Promise]].
        // 4. Let alreadyResolved be F.[[AlreadyResolved]].
        let PromiseRejectFunctionHeapData {
            already_resolved, ..
        } = agent[f];
        // 5. If alreadyResolved.[[Value]] is true, return undefined.
        if !already_resolved {
            // 6. Set alreadyResolved.[[Value]] to true.
            agent[f].already_resolved = true;
            // 7. Perform RejectPromise(promise, reason).
            // reject_promise(agent, promise, reason);
            // 8. Return undefined.
        }
    }
}

impl InternalSlots for BuiltinPromiseRejectFunction {
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

impl InternalMethods for BuiltinPromiseRejectFunction {
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
}

impl Index<BuiltinPromiseRejectFunction> for Agent {
    type Output = PromiseRejectFunctionHeapData;

    fn index(&self, index: BuiltinPromiseRejectFunction) -> &Self::Output {
        &self.heap.promise_reject_functions[index]
    }
}

impl IndexMut<BuiltinPromiseRejectFunction> for Agent {
    fn index_mut(&mut self, index: BuiltinPromiseRejectFunction) -> &mut Self::Output {
        &mut self.heap.promise_reject_functions[index]
    }
}

impl Index<BuiltinPromiseRejectFunction> for Vec<Option<PromiseRejectFunctionHeapData>> {
    type Output = PromiseRejectFunctionHeapData;

    fn index(&self, index: BuiltinPromiseRejectFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_ref()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl IndexMut<BuiltinPromiseRejectFunction> for Vec<Option<PromiseRejectFunctionHeapData>> {
    fn index_mut(&mut self, index: BuiltinPromiseRejectFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_mut()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl CreateHeapData<PromiseRejectFunctionHeapData, BuiltinPromiseRejectFunction> for Heap {
    fn create(&mut self, data: PromiseRejectFunctionHeapData) -> BuiltinPromiseRejectFunction {
        self.promise_reject_functions.push(Some(data));
        BuiltinPromiseRejectFunction(BaseIndex::last(&self.promise_reject_functions))
    }
}

impl HeapMarkAndSweep for BuiltinPromiseRejectFunction {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.promise_reject_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions
            .promise_reject_functions
            .shift_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for PromiseRejectFunctionHeapData {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        self.object_index.mark_values(queues);
        self.promise.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.promise.sweep_values(compactions);
    }
}
