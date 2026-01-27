// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod abstract_operations;
mod data;

pub(crate) use abstract_operations::*;
pub(crate) use data::*;

use crate::{
    ecmascript::{
        Agent, JsResult, ProtoIntrinsics, TryResult,
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, InternalSlots, OrdinaryObject,
            PropertyDescriptor, PropertyKey, SetResult, String, TryGetResult, TryHasResult, Value,
            object_handle,
        },
        unwrap_try,
    },
    engine::context::{Bindable, GcScope, NoGcScope},
    heap::{
        ArenaAccess, ArenaAccessMut, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        HeapSweepWeakReference, ObjectEntry, ObjectEntryPropertyDescriptor, WorkQueues,
        arena_vec_access, indexes::BaseIndex,
    },
};
use oxc_ast::ast::RegExpFlags;
use wtf8::Wtf8Buf;

use super::ordinary::{
    PropertyLookupCache, ordinary_get_own_property, ordinary_has_property, ordinary_set,
    ordinary_try_get, ordinary_try_has_property, ordinary_try_set,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RegExp<'a>(BaseIndex<'a, RegExpHeapData<'static>>);
object_handle!(RegExp);
arena_vec_access!(RegExp, 'a, RegExpHeapData, regexps);

impl<'a> RegExp<'a> {
    /// Fast-path for RegExp object debug stringifying; this does not take into
    /// account any prototype-modifications.
    #[inline(always)]
    pub(crate) fn create_regexp_string(self, agent: &Agent) -> Wtf8Buf {
        self.get(agent).create_regexp_string(agent)
    }

    /// ### \[\[OriginalSource]]
    pub(crate) fn original_source(self, agent: &Agent) -> String<'a> {
        self.get(agent).original_source
    }

    /// ### \[\[OriginalFlags]]
    pub(crate) fn original_flags(self, agent: &Agent) -> RegExpFlags {
        self.get(agent).original_flags
    }

    pub(crate) fn set_last_index(
        self,
        agent: &mut Agent,
        last_index: RegExpLastIndex,
        gc: NoGcScope,
    ) -> bool {
        debug_assert!(last_index.is_valid());
        // If we're setting the last index and we have a backing object,
        // then we set the value there first and observe the result.
        if self.get_backing_object(agent).is_some() {
            // Note: The lastIndex is an unconfigurable data property: It
            // cannot be turned into a getter or setter and will thus never
            // call into JavaScript.
            let success = unwrap_try(ordinary_try_set(
                agent,
                self,
                BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                last_index.get_value().unwrap().into(),
                self.into(),
                None,
                gc,
            ))
            .into_boolean()
            .unwrap();
            if success {
                // We successfully set the value, so set it in our direct
                // data as well.
                self.get_mut(agent).last_index = last_index;
            }
            success
        } else {
            // Note: lastIndex property is writable, so setting its value
            // always succeeds. We can just set this directly here.
            self.get_mut(agent).last_index = last_index;
            true
        }
    }

    /// ### \[\[LastIndex]]
    ///
    /// This is a custom internal slot that stores the "lastIndex" property of
    /// a RegExp assuming it has an expected value (32-bit unsigned integer or
    /// undefined). The method returns `None` if the property has an unexpected
    /// value, otherwise it returns the length value (0 if value is undefined).
    pub(crate) fn try_get_last_index(self, agent: &Agent) -> Option<u32> {
        let last_index = self.get(agent).last_index.get_value();
        if last_index.is_some() || self.get_backing_object(agent).is_none() {
            Some(last_index.unwrap_or(0))
        } else {
            None
        }
    }
}

impl<'a> InternalSlots<'a> for RegExp<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::RegExp;

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        assert!(self.get_backing_object(agent).is_none());
        let prototype = self.internal_prototype(agent).unwrap();
        let last_index = self.get(agent).last_index;
        let backing_object = OrdinaryObject::create_object(
            agent,
            Some(prototype),
            &[ObjectEntry {
                key: BUILTIN_STRING_MEMORY.lastIndex.into(),
                value: ObjectEntryPropertyDescriptor::Data {
                    value: last_index
                        .get_value()
                        .map_or(Value::Undefined, |i| i.into()),
                    writable: true,
                    enumerable: false,
                    configurable: false,
                },
            }],
        )
        .expect("Should perform GC here");
        self.set_backing_object(agent, backing_object);
        backing_object
    }

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_mut(agent)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for RegExp<'a> {
    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        if let Some(backing_object) = self.get_backing_object(agent) {
            // If a backing object exists, it's the only one with correct
            // knowledge of all our properties, including lastIndex.
            TryResult::Continue(ordinary_get_own_property(
                agent,
                self.into(),
                backing_object,
                property_key,
                cache,
                gc,
            ))
        } else if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // If no backing object exists, we can turn lastIndex into a
            // PropertyDescriptor statically.
            TryResult::Continue(Some(self.get(agent).last_index.into_property_descriptor()))
        } else {
            TryResult::Continue(None)
        }
    }

    fn try_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryHasResult<'gc>> {
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // lastIndex always exists
            TryHasResult::Custom(0, self.bind(gc).into()).into()
        } else {
            ordinary_try_has_property(
                agent,
                self.into(),
                self.get_backing_object(agent),
                property_key,
                cache,
                gc,
            )
        }
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // lastIndex always exists
            Ok(true)
        } else if let Some(backing_object) = self.get_backing_object(agent) {
            ordinary_has_property(agent, self.into(), backing_object, property_key, gc)
        } else {
            // a. Let parent be ? O.[[GetPrototypeOf]]().
            // Note: We know statically what this ends up doing.
            let parent = agent
                .current_realm_record()
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
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        // Regardless of the backing object, we might have a valid value
        // for lastIndex.
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into()
            && let Some(last_index) = self.get(agent).last_index.get_value()
        {
            return TryGetResult::Value(last_index.into()).into();
        }
        ordinary_try_get(
            agent,
            self.into(),
            self.get_backing_object(agent),
            property_key,
            receiver,
            cache,
            gc,
        )
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let property_key = property_key.bind(gc.nogc());
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // Regardless of the backing object, we might have a valid value
            // for lastIndex.
            if let Some(last_index) = self.get(agent).last_index.get_value() {
                return Ok(last_index.into());
            }
        }
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_get(agent, property_key.unbind(), receiver, gc)
        } else {
            // a. Let parent be ? O.[[GetPrototypeOf]]().
            // Note: We know statically what this ends up doing.
            let parent = agent
                .current_realm_record()
                .intrinsics()
                .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);

            // c. Return ? parent.[[Get]](P, Receiver).
            parent.internal_get(agent, property_key.unbind(), receiver, gc)
        }
    }

    fn try_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // If we're setting the last index and we have a backing object,
            // then we set the value there first and observe the result.
            let new_last_index = RegExpLastIndex::from_value(value);
            if self.get_backing_object(agent).is_some() {
                // Note: The lastIndex is an unconfigurable data property: It
                // cannot be turned into a getter or setter and will thus never
                // call into JavaScript.
                let success = unwrap_try(ordinary_try_set(
                    agent,
                    self,
                    property_key,
                    value,
                    receiver,
                    cache,
                    gc,
                ))
                .into_boolean()
                .unwrap();
                if success {
                    // We successfully set the value, so set it in our direct
                    // data as well.
                    self.get_mut(agent).last_index = new_last_index;
                    SetResult::Done.into()
                } else {
                    SetResult::Unwritable.into()
                }
            } else {
                // Note: lastIndex property is writable, so setting its value
                // always succeeds. We can just set this directly here.
                self.get_mut(agent).last_index = new_last_index;
                // If we we set a value that is not a valid index or undefined,
                // we need to create the backing object and set the actual
                // value there.
                if !new_last_index.is_valid() && value.is_undefined() {
                    unwrap_try(self.create_backing_object(agent).try_set(
                        agent,
                        property_key,
                        value,
                        receiver,
                        cache,
                        gc,
                    ));
                }
                SetResult::Done.into()
            }
        } else {
            // If something else is being set, fall back onto the ordinary
            // abstract operation.
            ordinary_try_set(agent, self, property_key, value, receiver, cache, gc)
        }
    }

    fn internal_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        if property_key == BUILTIN_STRING_MEMORY.lastIndex.into() {
            // Note: lastIndex is an unconfigurable data property: It cannot
            // become a getter or setter and will thus never call into
            // JavaScript.
            Ok(
                unwrap_try(self.try_set(agent, property_key, value, receiver, None, gc.nogc()))
                    .into_boolean()
                    .unwrap(),
            )
        } else {
            // If something else is being set, fall back onto the ordinary
            // abstract operation.
            ordinary_set(agent, self.into(), property_key, value, receiver, gc)
        }
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
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

impl HeapSweepWeakReference for RegExp<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.regexps.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<RegExpHeapData<'a>, RegExp<'a>> for Heap {
    fn create(&mut self, data: RegExpHeapData<'a>) -> RegExp<'a> {
        self.regexps.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<RegExpHeapData<'static>>();
        RegExp(BaseIndex::last(&self.regexps))
    }
}
