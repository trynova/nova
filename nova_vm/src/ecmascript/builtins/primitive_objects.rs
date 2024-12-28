// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::engine::context::{GcScope, NoGcScope};
use crate::{
    ecmascript::{
        builtins::ordinary::{
            is_compatible_property_descriptor, ordinary_define_own_property, ordinary_delete,
            ordinary_get, ordinary_get_own_property, ordinary_has_property, ordinary_set,
        },
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            bigint::{HeapBigInt, SmallBigInt},
            BigInt, HeapNumber, HeapString, InternalMethods, InternalSlots, IntoObject, IntoValue,
            Number, Object, OrdinaryObject, PropertyDescriptor, PropertyKey, String, Symbol, Value,
            BIGINT_DISCRIMINANT, BOOLEAN_DISCRIMINANT, BUILTIN_STRING_MEMORY, FLOAT_DISCRIMINANT,
            INTEGER_DISCRIMINANT, NUMBER_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT,
            SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT, SYMBOL_DISCRIMINANT,
        },
    },
    engine::small_f64::SmallF64,
    heap::{
        indexes::PrimitiveObjectIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
    SmallInteger,
};
use small_string::SmallString;

use super::ordinary::{
    ordinary_own_property_keys, ordinary_try_get, ordinary_try_has_property, ordinary_try_set,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PrimitiveObject(PrimitiveObjectIndex);

impl From<PrimitiveObjectIndex> for PrimitiveObject {
    fn from(value: PrimitiveObjectIndex) -> Self {
        Self(value)
    }
}

impl From<PrimitiveObject> for Object {
    fn from(value: PrimitiveObject) -> Self {
        Self::PrimitiveObject(value)
    }
}

impl From<PrimitiveObject> for Value {
    fn from(value: PrimitiveObject) -> Self {
        Self::PrimitiveObject(value)
    }
}

impl IntoObject for PrimitiveObject {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl IntoValue for PrimitiveObject {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl TryFrom<Object> for PrimitiveObject {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::PrimitiveObject(obj) => Ok(obj),
            _ => Err(()),
        }
    }
}

impl TryFrom<Value> for PrimitiveObject {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::PrimitiveObject(obj) => Ok(obj),
            _ => Err(()),
        }
    }
}

impl Index<PrimitiveObject> for Agent {
    type Output = PrimitiveObjectHeapData;

    fn index(&self, index: PrimitiveObject) -> &Self::Output {
        &self.heap.primitive_objects[index]
    }
}

impl IndexMut<PrimitiveObject> for Agent {
    fn index_mut(&mut self, index: PrimitiveObject) -> &mut Self::Output {
        &mut self.heap.primitive_objects[index]
    }
}

impl Index<PrimitiveObject> for Vec<Option<PrimitiveObjectHeapData>> {
    type Output = PrimitiveObjectHeapData;

    fn index(&self, index: PrimitiveObject) -> &Self::Output {
        self.get(index.get_index())
            .expect("PrimitiveObject out of bounds")
            .as_ref()
            .expect("PrimitiveObject slot empty")
    }
}

impl IndexMut<PrimitiveObject> for Vec<Option<PrimitiveObjectHeapData>> {
    fn index_mut(&mut self, index: PrimitiveObject) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("PrimitiveObject out of bounds")
            .as_mut()
            .expect("PrimitiveObject slot empty")
    }
}

impl PrimitiveObject {
    pub(crate) const fn _def() -> Self {
        PrimitiveObject(PrimitiveObjectIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn is_bigint_object(self, agent: &Agent) -> bool {
        matches!(
            agent[self].data,
            PrimitiveObjectData::BigInt(_) | PrimitiveObjectData::SmallBigInt(_)
        )
    }

    pub fn is_number_object(self, agent: &Agent) -> bool {
        matches!(
            agent[self].data,
            PrimitiveObjectData::Float(_)
                | PrimitiveObjectData::Integer(_)
                | PrimitiveObjectData::Number(_)
        )
    }

    pub fn is_string_object(self, agent: &Agent) -> bool {
        matches!(
            agent[self].data,
            PrimitiveObjectData::String(_) | PrimitiveObjectData::SmallString(_)
        )
    }

    pub fn is_symbol_object(self, agent: &Agent) -> bool {
        matches!(agent[self].data, PrimitiveObjectData::Symbol(_))
    }
}

impl InternalSlots for PrimitiveObject {
    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }

    fn internal_prototype(
        self,
        agent: &crate::ecmascript::execution::Agent,
    ) -> Option<crate::ecmascript::types::Object> {
        match self.get_backing_object(agent) {
            Some(obj) => obj.internal_prototype(agent),
            None => {
                let intrinsic_default_proto = match agent[self].data {
                    PrimitiveObjectData::Boolean(_) => ProtoIntrinsics::Boolean,
                    PrimitiveObjectData::String(_) | PrimitiveObjectData::SmallString(_) => {
                        ProtoIntrinsics::String
                    }
                    PrimitiveObjectData::Symbol(_) => ProtoIntrinsics::Symbol,
                    PrimitiveObjectData::Number(_)
                    | PrimitiveObjectData::Integer(_)
                    | PrimitiveObjectData::Float(_) => ProtoIntrinsics::Number,
                    PrimitiveObjectData::BigInt(_) | PrimitiveObjectData::SmallBigInt(_) => {
                        ProtoIntrinsics::BigInt
                    }
                };
                // TODO: Should take realm from "backing object"'s Realm/None
                // variant
                Some(
                    agent
                        .current_realm()
                        .intrinsics()
                        .get_intrinsic_default_proto(intrinsic_default_proto),
                )
            }
        }
    }
}

impl InternalMethods for PrimitiveObject {
    fn try_get_own_property(
        self,
        agent: &mut Agent,
        _: NoGcScope<'_, '_>,
        property_key: PropertyKey,
    ) -> Option<Option<PropertyDescriptor>> {
        // For non-string primitive objects:
        // 1. Return OrdinaryGetOwnProperty(O, P).
        // For string exotic objects:
        // 1. Let desc be OrdinaryGetOwnProperty(S, P).
        // 2. If desc is not undefined, return desc.
        if let Some(backing_object) = self.get_backing_object(agent) {
            if let Some(property_descriptor) =
                ordinary_get_own_property(agent, backing_object, property_key)
            {
                return Some(Some(property_descriptor));
            }
        }

        if let Ok(string) = String::try_from(agent[self].data) {
            // 3. Return StringGetOwnProperty(S, P).
            Some(string.get_property_descriptor(agent, property_key))
        } else {
            Some(None)
        }
    }

    fn try_define_own_property(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, '_>,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> Option<bool> {
        if let Ok(string) = String::try_from(agent[self].data) {
            // For string exotic objects:
            // 1. Let stringDesc be StringGetOwnProperty(S, P).
            // 2. If stringDesc is not undefined, then
            if let Some(string_desc) = string.get_property_descriptor(agent, property_key) {
                // a. Let extensible be S.[[Extensible]].
                // b. Return IsCompatiblePropertyDescriptor(extensible, Desc, stringDesc).
                return Some(is_compatible_property_descriptor(
                    agent,
                    gc,
                    self.internal_extensible(agent),
                    property_descriptor,
                    Some(string_desc),
                ));
            }
            // 3. Return ! OrdinaryDefineOwnProperty(S, P, Desc).
        }

        let backing_object = self
            .get_backing_object(agent)
            .unwrap_or_else(|| self.create_backing_object(agent));
        Some(ordinary_define_own_property(
            agent,
            gc,
            backing_object,
            property_key,
            property_descriptor,
        ))
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, '_>,
        property_key: PropertyKey,
    ) -> Option<bool> {
        if let Ok(string) = String::try_from(agent[self].data) {
            if string
                .get_property_descriptor(agent, property_key)
                .is_some()
            {
                return Some(true);
            }
        }

        // 1. Return ? OrdinaryHasProperty(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_try_has_property(agent, gc, backing_object, property_key)
            }
            None => {
                // 3. Let parent be ? O.[[GetPrototypeOf]]().
                let parent = self.try_get_prototype_of(agent, gc).unwrap();

                // 4. If parent is not null, then
                if let Some(parent) = parent {
                    // a. Return ? parent.[[HasProperty]](P).
                    parent.try_has_property(agent, gc, property_key)
                } else {
                    // 5. Return false.
                    Some(false)
                }
            }
        }
    }

    fn internal_has_property(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        property_key: PropertyKey,
    ) -> JsResult<bool> {
        if let Ok(string) = String::try_from(agent[self].data) {
            if string
                .get_property_descriptor(agent, property_key)
                .is_some()
            {
                return Ok(true);
            }
        }

        // 1. Return ? OrdinaryHasProperty(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_has_property(agent, gc, backing_object, property_key),
            None => {
                // 3. Let parent be ? O.[[GetPrototypeOf]]().
                let parent = self.try_get_prototype_of(agent, gc.nogc()).unwrap();

                // 4. If parent is not null, then
                if let Some(parent) = parent {
                    // a. Return ? parent.[[HasProperty]](P).
                    parent.internal_has_property(agent, gc, property_key)
                } else {
                    // 5. Return false.
                    Ok(false)
                }
            }
        }
    }

    fn try_get(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, '_>,
        property_key: PropertyKey,
        receiver: Value,
    ) -> Option<Value> {
        if let Ok(string) = String::try_from(agent[self].data) {
            if let Some(string_desc) = string.get_property_descriptor(agent, property_key) {
                return Some(string_desc.value.unwrap());
            }
        }

        // 1. Return ? OrdinaryGet(O, P, Receiver).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_try_get(agent, gc, backing_object, property_key, receiver)
            }
            None => {
                // a. Let parent be ? O.[[GetPrototypeOf]]().
                let Some(parent) = self.try_get_prototype_of(agent, gc).unwrap() else {
                    // b. If parent is null, return undefined.
                    return Some(Value::Undefined);
                };

                // c. Return ? parent.[[Get]](P, Receiver).
                parent.try_get(agent, gc, property_key, receiver)
            }
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        if let Ok(string) = String::try_from(agent[self].data) {
            if let Some(string_desc) = string.get_property_descriptor(agent, property_key) {
                return Ok(string_desc.value.unwrap());
            }
        }

        // 1. Return ? OrdinaryGet(O, P, Receiver).
        match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_get(agent, gc, backing_object, property_key, receiver),
            None => {
                // a. Let parent be ? O.[[GetPrototypeOf]]().
                let Some(parent) = self.try_get_prototype_of(agent, gc.nogc()).unwrap() else {
                    // b. If parent is null, return undefined.
                    return Ok(Value::Undefined);
                };

                // c. Return ? parent.[[Get]](P, Receiver).
                parent.internal_get(agent, gc, property_key, receiver)
            }
        }
    }

    fn try_set(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, '_>,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> Option<bool> {
        if let Ok(string) = String::try_from(agent[self].data) {
            if string
                .get_property_descriptor(agent, property_key)
                .is_some()
            {
                return Some(false);
            }
        }

        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_try_set(agent, gc, self.into_object(), property_key, value, receiver)
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        if let Ok(string) = String::try_from(agent[self].data) {
            if string
                .get_property_descriptor(agent, property_key)
                .is_some()
            {
                return Ok(false);
            }
        }

        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_set(agent, gc, self.into_object(), property_key, value, receiver)
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, '_>,
        property_key: PropertyKey,
    ) -> Option<bool> {
        if let Ok(string) = String::try_from(agent[self].data) {
            // A String will return unconfigurable descriptors for length and
            // all valid string indexes, making delete return false.
            if property_key == BUILTIN_STRING_MEMORY.length.into() {
                return Some(false);
            } else if let PropertyKey::Integer(index) = property_key {
                let index = index.into_i64();
                if index >= 0 && (index as usize) < string.utf16_len(agent) {
                    return Some(false);
                }
            }
        }

        // 1. Return ! OrdinaryDelete(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => Some(ordinary_delete(agent, gc, backing_object, property_key)),
            None => Some(true),
        }
    }

    fn try_own_property_keys<'a>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> Option<Vec<PropertyKey<'a>>> {
        if let Ok(string) = String::try_from(agent[self].data) {
            let len = string.utf16_len(agent);
            let mut keys = Vec::with_capacity(len + 1);

            // Insert keys for every index into the string.
            keys.extend(
                (0..len)
                    .map(|idx| PropertyKey::Integer(SmallInteger::try_from(idx as u64).unwrap())),
            );

            let backing_object_keys;
            let (integer_keys, other_keys) = match self.get_backing_object(agent) {
                Some(backing_object) => {
                    backing_object_keys = ordinary_own_property_keys(agent, gc, backing_object);
                    if let Some(PropertyKey::Integer(smi)) = backing_object_keys.first() {
                        debug_assert!(smi.into_i64() >= len as i64);
                    }
                    let split_idx = backing_object_keys
                        .iter()
                        .position(|pk| !pk.is_array_index());
                    if let Some(idx) = split_idx {
                        backing_object_keys.split_at(idx)
                    } else {
                        (&backing_object_keys[..], &[][..])
                    }
                }
                None => (&[][..], &[][..]),
            };

            // Insert the `length` key as the first non-array index key.
            keys.extend(integer_keys);
            keys.push(BUILTIN_STRING_MEMORY.length.into());
            keys.extend(other_keys);

            return Some(keys);
        }

        // 1. Return OrdinaryOwnPropertyKeys(O).
        match self.get_backing_object(agent) {
            Some(backing_object) => Some(ordinary_own_property_keys(agent, gc, backing_object)),
            None => Some(vec![]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum PrimitiveObjectData {
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    String(HeapString<'static>) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Symbol(Symbol<'static>) = SYMBOL_DISCRIMINANT,
    Number(HeapNumber<'static>) = NUMBER_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    Float(SmallF64) = FLOAT_DISCRIMINANT,
    BigInt(HeapBigInt<'static>) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

impl TryFrom<PrimitiveObjectData> for BigInt<'_> {
    type Error = ();

    fn try_from(value: PrimitiveObjectData) -> Result<Self, Self::Error> {
        match value {
            PrimitiveObjectData::BigInt(data) => Ok(BigInt::BigInt(data)),
            PrimitiveObjectData::SmallBigInt(data) => Ok(BigInt::SmallBigInt(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<PrimitiveObjectData> for Number<'static> {
    type Error = ();

    fn try_from(value: PrimitiveObjectData) -> Result<Self, Self::Error> {
        match value {
            PrimitiveObjectData::Number(data) => Ok(Number::Number(data)),
            PrimitiveObjectData::Integer(data) => Ok(Number::Integer(data)),
            PrimitiveObjectData::Float(data) => Ok(Number::SmallF64(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<PrimitiveObjectData> for String<'static> {
    type Error = ();

    fn try_from(value: PrimitiveObjectData) -> Result<Self, Self::Error> {
        match value {
            PrimitiveObjectData::String(data) => Ok(String::String(data)),
            PrimitiveObjectData::SmallString(data) => Ok(String::SmallString(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<PrimitiveObjectData> for Symbol<'_> {
    type Error = ();

    fn try_from(value: PrimitiveObjectData) -> Result<Self, Self::Error> {
        match value {
            PrimitiveObjectData::Symbol(data) => Ok(data),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PrimitiveObjectHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) data: PrimitiveObjectData,
}

impl PrimitiveObjectHeapData {
    pub(crate) fn new_big_int_object(big_int: BigInt) -> Self {
        let data = match big_int {
            BigInt::BigInt(data) => PrimitiveObjectData::BigInt(data.unbind()),
            BigInt::SmallBigInt(data) => PrimitiveObjectData::SmallBigInt(data),
        };
        Self {
            object_index: None,
            data,
        }
    }

    pub(crate) fn new_boolean_object(boolean: bool) -> Self {
        Self {
            object_index: None,
            data: PrimitiveObjectData::Boolean(boolean),
        }
    }

    pub(crate) fn new_number_object(number: Number<'_>) -> Self {
        let data = match number {
            Number::Number(data) => PrimitiveObjectData::Number(data.unbind()),
            Number::Integer(data) => PrimitiveObjectData::Integer(data),
            Number::SmallF64(data) => PrimitiveObjectData::Float(data),
        };
        Self {
            object_index: None,
            data,
        }
    }

    pub(crate) fn new_string_object(string: String<'static>) -> Self {
        let data = match string {
            String::String(data) => PrimitiveObjectData::String(data),
            String::SmallString(data) => PrimitiveObjectData::SmallString(data),
        };
        Self {
            object_index: None,
            data,
        }
    }

    pub(crate) fn new_symbol_object(symbol: Symbol) -> Self {
        Self {
            object_index: None,
            data: PrimitiveObjectData::Symbol(symbol.unbind()),
        }
    }
}

impl HeapMarkAndSweep for PrimitiveObjectHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { object_index, data } = self;
        object_index.mark_values(queues);
        match data {
            PrimitiveObjectData::String(data) => data.mark_values(queues),
            PrimitiveObjectData::Symbol(data) => data.mark_values(queues),
            PrimitiveObjectData::Number(data) => data.mark_values(queues),
            PrimitiveObjectData::BigInt(data) => data.mark_values(queues),
            _ => {}
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { object_index, data } = self;
        object_index.sweep_values(compactions);
        match data {
            PrimitiveObjectData::String(data) => data.sweep_values(compactions),
            PrimitiveObjectData::Symbol(data) => data.sweep_values(compactions),
            PrimitiveObjectData::Number(data) => data.sweep_values(compactions),
            PrimitiveObjectData::BigInt(data) => data.sweep_values(compactions),
            _ => {}
        }
    }
}

impl HeapMarkAndSweep for PrimitiveObject {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.primitive_objects.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.primitive_objects.shift_index(&mut self.0);
    }
}

impl CreateHeapData<PrimitiveObjectHeapData, PrimitiveObject> for Heap {
    fn create(&mut self, data: PrimitiveObjectHeapData) -> PrimitiveObject {
        self.primitive_objects.push(Some(data));
        PrimitiveObject(PrimitiveObjectIndex::last(&self.primitive_objects))
    }
}
