// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub(crate) mod abstract_operations;
pub(crate) mod data;

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData,
            OrdinaryObject, PropertyDescriptor, PropertyKey, Value, BUILTIN_STRING_MEMORY,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::HeapRootData,
        unwrap_try, Scoped, TryResult,
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
pub struct RegExp<'a>(RegExpIndex<'a>);

impl RegExp<'_> {
    /// Unbind this RegExp from its current lifetime. This is necessary to use
    /// the RegExp as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> RegExp<'static> {
        unsafe { core::mem::transmute::<Self, RegExp<'static>>(self) }
    }

    // Bind this RegExp to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your RegExps cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let reg_exp = reg_exp.bind(&gc);
    // ```
    // to make sure that the unbound RegExp cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> RegExp<'gc> {
        unsafe { core::mem::transmute::<Self, RegExp<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, RegExp<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'a> From<RegExp<'a>> for Value<'a> {
    fn from(value: RegExp<'a>) -> Self {
        Self::RegExp(value)
    }
}

impl<'a> From<RegExp<'a>> for Object<'a> {
    fn from(value: RegExp) -> Self {
        Self::RegExp(value.unbind())
    }
}

impl<'a> TryFrom<Object<'a>> for RegExp<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::RegExp(regexp) => Ok(regexp),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for RegExp<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::RegExp(regexp) => Ok(regexp),
            _ => Err(()),
        }
    }
}

impl<'a> IntoValue<'a> for RegExp<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> IntoObject<'a> for RegExp<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> InternalSlots<'a> for RegExp<'a> {
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

impl<'a> InternalMethods<'a> for RegExp<'a> {
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

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Value<'gc>> {
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

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
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

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
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

impl HeapMarkAndSweep for RegExp<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.regexps.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.regexps.shift_index(&mut self.0);
    }
}

impl Index<RegExp<'_>> for Agent {
    type Output = RegExpHeapData;

    fn index(&self, index: RegExp) -> &Self::Output {
        &self.heap.regexps[index]
    }
}

impl IndexMut<RegExp<'_>> for Agent {
    fn index_mut(&mut self, index: RegExp) -> &mut Self::Output {
        &mut self.heap.regexps[index]
    }
}

impl Index<RegExp<'_>> for Vec<Option<RegExpHeapData>> {
    type Output = RegExpHeapData;

    fn index(&self, index: RegExp) -> &Self::Output {
        self.get(index.get_index())
            .expect("RegExp out of bounds")
            .as_ref()
            .expect("RegExp slot empty")
    }
}

impl IndexMut<RegExp<'_>> for Vec<Option<RegExpHeapData>> {
    fn index_mut(&mut self, index: RegExp) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("RegExp out of bounds")
            .as_mut()
            .expect("RegExp slot empty")
    }
}

impl TryFrom<HeapRootData> for RegExp<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::RegExp(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl CreateHeapData<RegExpHeapData, RegExp<'static>> for Heap {
    fn create(&mut self, data: RegExpHeapData) -> RegExp<'static> {
        self.regexps.push(Some(data));
        RegExp(RegExpIndex::last(&self.regexps))
    }
}
