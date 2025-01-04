// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub(crate) mod abstract_operations;
pub(crate) mod data;

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData,
            OrdinaryObject, PropertyDescriptor, PropertyKey, Value, BUILTIN_STRING_MEMORY,
        },
    },
    engine::{
        context::{GcScope, NoGcScope},
        unwrap_try, TryResult,
    },
    heap::{
        indexes::{BaseIndex, RegExpIndex},
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, ObjectEntry,
        ObjectEntryPropertyDescriptor, WorkQueues,
    },
};
pub(crate) use abstract_operations::*;
pub(crate) use data::RegExpHeapData;
use data::RegExpLastIndex;

use super::ordinary::{ordinary_set, ordinary_try_set};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RegExp(RegExpIndex);

impl RegExp {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<RegExp> for Value {
    fn from(value: RegExp) -> Self {
        Self::RegExp(value)
    }
}

impl From<RegExp> for Object {
    fn from(value: RegExp) -> Self {
        Self::RegExp(value)
    }
}

impl TryFrom<Object> for RegExp {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::RegExp(regexp) => Ok(regexp),
            _ => Err(()),
        }
    }
}

impl TryFrom<Value> for RegExp {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::RegExp(regexp) => Ok(regexp),
            _ => Err(()),
        }
    }
}

impl IntoValue for RegExp {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for RegExp {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl InternalSlots for RegExp {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::RegExp;

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        assert!(self.get_backing_object(agent).is_none());
        let prototype = self.internal_prototype(agent);
        let last_index = agent[self].last_index;
        let (keys, values) = agent.heap.elements.create_object_entries(&[ObjectEntry {
            key: BUILTIN_STRING_MEMORY.lastIndex.into(),
            value: ObjectEntryPropertyDescriptor::Data {
                value: last_index
                    .get_value()
                    .map_or(Value::Undefined, |i| i.into()),
                writable: true,
                enumerable: false,
                configurable: false,
            },
        }]);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype,
            keys,
            values,
        });
        self.set_backing_object(agent, backing_object);
        backing_object
    }

    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }
}

impl InternalMethods for RegExp {
    fn try_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<Option<PropertyDescriptor>> {
        if let Some(backing_object) = self.get_backing_object(agent) {
            // If a backing object exists, it's the only one with correct
            // knowledge of all our properties, including lastIndex.
            backing_object.try_get_own_property(agent, property_key, gc)
        } else if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // If no backing object exists, we can turn lastIndex into a
            // PropertyDescriptor statically.
            TryResult::Continue(Some(agent[self].last_index.into_property_descriptor()))
        } else {
            TryResult::Continue(None)
        }
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // lastIndex always exists
            TryResult::Continue(true)
        } else if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.try_has_property(agent, property_key, gc)
        } else {
            // a. Let parent be ? O.[[GetPrototypeOf]]().
            // Note: We know statically what this ends up doing.
            let parent = agent
                .current_realm()
                .intrinsics()
                .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);

            // a. Return ? parent.[[HasProperty]](P).
            parent.try_has_property(agent, property_key, gc)
        }
    }

    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope,
    ) -> JsResult<bool> {
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // lastIndex always exists
            Ok(true)
        } else if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_has_property(agent, property_key, gc)
        } else {
            // a. Let parent be ? O.[[GetPrototypeOf]]().
            // Note: We know statically what this ends up doing.
            let parent = agent
                .current_realm()
                .intrinsics()
                .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);

            // a. Return ? parent.[[HasProperty]](P).
            parent.internal_has_property(agent, property_key, gc)
        }
    }

    fn try_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<Value> {
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // Regardless of the backing object, we might have a valid value
            // for lastIndex.
            if let Some(last_index) = agent[self].last_index.get_value() {
                return TryResult::Continue(last_index.into());
            }
        }
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.try_get(agent, property_key, receiver, gc)
        } else {
            // a. Let parent be ? O.[[GetPrototypeOf]]().
            // Note: We know statically what this ends up doing.
            let parent = agent
                .current_realm()
                .intrinsics()
                .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);

            // c. Return ? parent.[[Get]](P, Receiver).
            parent.try_get(agent, property_key, receiver, gc)
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope,
    ) -> JsResult<Value> {
        let property_key = property_key.bind(gc.nogc());
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // Regardless of the backing object, we might have a valid value
            // for lastIndex.
            if let Some(last_index) = agent[self].last_index.get_value() {
                return Ok(last_index.into());
            }
        }
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_get(agent, property_key.unbind(), receiver, gc)
        } else {
            // a. Let parent be ? O.[[GetPrototypeOf]]().
            // Note: We know statically what this ends up doing.
            let parent = agent
                .current_realm()
                .intrinsics()
                .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);

            // c. Return ? parent.[[Get]](P, Receiver).
            parent.internal_get(agent, property_key.unbind(), receiver, gc)
        }
    }

    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // If we're setting the last index and we have a backing object,
            // then we set the value there first and observe the result.
            let new_last_index = RegExpLastIndex::from_value(value);
            if let Some(backing_object) = self.get_backing_object(agent) {
                // Note: The lastIndex is an unconfigurable data property: It
                // cannot be turned into a getter or setter and will thus never
                // call into JavaScript.
                let success =
                    unwrap_try(backing_object.try_set(agent, property_key, value, receiver, gc));
                if success {
                    // We successfully set the value, so set it in our direct
                    // data as well.
                    agent[self].last_index = new_last_index;
                }
                TryResult::Continue(success)
            } else {
                // Note: lastIndex property is writable, so setting its value
                // always succeeds. We can just set this directly here.
                agent[self].last_index = new_last_index;
                // If we we set a value that is not a valid index or undefined,
                // we need to create the backing object and set the actual
                // value there.
                if !new_last_index.is_valid() && value.is_undefined() {
                    unwrap_try(self.create_backing_object(agent).try_set(
                        agent,
                        property_key,
                        value,
                        receiver,
                        gc,
                    ));
                }
                TryResult::Continue(true)
            }
        } else {
            // If something else is being set, fall back onto the ordinary
            // abstract operation.
            ordinary_try_set(agent, self.into_object(), property_key, value, receiver, gc)
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope,
    ) -> JsResult<bool> {
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // Note: lastIndex is an unconfigurable data property: It cannot
            // become a getter or setter and will thus never call into
            // JavaScript.
            Ok(unwrap_try(self.try_set(
                agent,
                property_key,
                value,
                receiver,
                gc.nogc(),
            )))
        } else {
            // If something else is being set, fall back onto the ordinary
            // abstract operation.
            ordinary_set(agent, self.into_object(), property_key, value, receiver, gc)
        }
    }

    fn try_own_property_keys<'a>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<Vec<PropertyKey<'a>>> {
        TryResult::Continue(
            if let Some(backing_object) = self.get_backing_object(agent) {
                // Note: If backing object exists, it also contains the
                // "lastIndex" key so we do not need to add it ourselves.
                unwrap_try(backing_object.try_own_property_keys(agent, gc))
            } else {
                vec![BUILTIN_STRING_MEMORY.lastIndex.into()]
            },
        )
    }
}

impl HeapMarkAndSweep for RegExp {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.regexps.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.regexps.shift_index(&mut self.0);
    }
}

impl Index<RegExp> for Agent {
    type Output = RegExpHeapData;

    fn index(&self, index: RegExp) -> &Self::Output {
        &self.heap.regexps[index]
    }
}

impl IndexMut<RegExp> for Agent {
    fn index_mut(&mut self, index: RegExp) -> &mut Self::Output {
        &mut self.heap.regexps[index]
    }
}

impl Index<RegExp> for Vec<Option<RegExpHeapData>> {
    type Output = RegExpHeapData;

    fn index(&self, index: RegExp) -> &Self::Output {
        self.get(index.get_index())
            .expect("RegExp out of bounds")
            .as_ref()
            .expect("RegExp slot empty")
    }
}

impl IndexMut<RegExp> for Vec<Option<RegExpHeapData>> {
    fn index_mut(&mut self, index: RegExp) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("RegExp out of bounds")
            .as_mut()
            .expect("RegExp slot empty")
    }
}

impl CreateHeapData<RegExpHeapData, RegExp> for Heap {
    fn create(&mut self, data: RegExpHeapData) -> RegExp {
        self.regexps.push(Some(data));
        RegExp(RegExpIndex::last(&self.regexps))
    }
}
