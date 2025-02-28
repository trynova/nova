// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::engine::context::{Bindable, GcScope, NoGcScope};
use crate::engine::rootable::{HeapRootData, HeapRootRef, Rootable};
use crate::engine::{unwrap_try, Scoped, TryResult};
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
    ordinary_own_property_keys, ordinary_try_get, ordinary_try_has_property_entry, ordinary_try_set,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PrimitiveObject<'a>(PrimitiveObjectIndex<'a>);

impl<'a> From<PrimitiveObjectIndex<'a>> for PrimitiveObject<'a> {
    fn from(value: PrimitiveObjectIndex<'a>) -> Self {
        Self(value)
    }
}

impl<'a> From<PrimitiveObject<'a>> for Object<'a> {
    fn from(value: PrimitiveObject) -> Self {
        Self::PrimitiveObject(value.unbind())
    }
}

impl<'a> From<PrimitiveObject<'a>> for Value<'a> {
    fn from(value: PrimitiveObject<'a>) -> Self {
        Self::PrimitiveObject(value)
    }
}

impl<'a> IntoObject<'a> for PrimitiveObject<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> IntoValue<'a> for PrimitiveObject<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> TryFrom<Object<'a>> for PrimitiveObject<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::PrimitiveObject(obj) => Ok(obj),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for PrimitiveObject<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::PrimitiveObject(obj) => Ok(obj),
            _ => Err(()),
        }
    }
}

impl Index<PrimitiveObject<'_>> for Agent {
    type Output = PrimitiveObjectHeapData;

    fn index(&self, index: PrimitiveObject) -> &Self::Output {
        &self.heap.primitive_objects[index]
    }
}

impl IndexMut<PrimitiveObject<'_>> for Agent {
    fn index_mut(&mut self, index: PrimitiveObject) -> &mut Self::Output {
        &mut self.heap.primitive_objects[index]
    }
}

impl Index<PrimitiveObject<'_>> for Vec<Option<PrimitiveObjectHeapData>> {
    type Output = PrimitiveObjectHeapData;

    fn index(&self, index: PrimitiveObject) -> &Self::Output {
        self.get(index.get_index())
            .expect("PrimitiveObject out of bounds")
            .as_ref()
            .expect("PrimitiveObject slot empty")
    }
}

impl IndexMut<PrimitiveObject<'_>> for Vec<Option<PrimitiveObjectHeapData>> {
    fn index_mut(&mut self, index: PrimitiveObject) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("PrimitiveObject out of bounds")
            .as_mut()
            .expect("PrimitiveObject slot empty")
    }
}

impl<'a> PrimitiveObject<'a> {
    /// Unbind this PrimitiveObject from its current lifetime. This is necessary to use
    /// the PrimitiveObject as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> PrimitiveObject<'static> {
        unsafe { core::mem::transmute::<PrimitiveObject, PrimitiveObject<'static>>(self) }
    }

    // Bind this PrimitiveObject to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your PrimitiveObjects cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let number = number.bind(&gc);
    // ```
    // to make sure that the unbound PrimitiveObject cannot be used after binding.
    pub const fn bind(self, _: NoGcScope<'a, '_>) -> PrimitiveObject<'a> {
        unsafe { core::mem::transmute::<PrimitiveObject, PrimitiveObject<'a>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, PrimitiveObject<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

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

impl<'a> InternalSlots<'a> for PrimitiveObject<'a> {
    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
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

impl<'a> InternalMethods<'a> for PrimitiveObject<'a> {
    fn try_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        _: NoGcScope,
    ) -> TryResult<Option<PropertyDescriptor>> {
        // For non-string primitive objects:
        // 1. Return OrdinaryGetOwnProperty(O, P).
        // For string exotic objects:
        // 1. Let desc be OrdinaryGetOwnProperty(S, P).
        // 2. If desc is not undefined, return desc.
        if let Some(backing_object) = self.get_backing_object(agent) {
            if let Some(property_descriptor) =
                ordinary_get_own_property(agent, backing_object, property_key)
            {
                return TryResult::Continue(Some(property_descriptor));
            }
        }

        if let Ok(string) = String::try_from(agent[self].data) {
            // 3. Return StringGetOwnProperty(S, P).
            TryResult::Continue(string.get_property_descriptor(agent, property_key))
        } else {
            TryResult::Continue(None)
        }
    }

    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        if let Ok(string) = String::try_from(agent[self].data) {
            // For string exotic objects:
            // 1. Let stringDesc be StringGetOwnProperty(S, P).
            // 2. If stringDesc is not undefined, then
            if let Some(string_desc) = string.get_property_descriptor(agent, property_key) {
                // a. Let extensible be S.[[Extensible]].
                // b. Return IsCompatiblePropertyDescriptor(extensible, Desc, stringDesc).
                return TryResult::Continue(is_compatible_property_descriptor(
                    agent,
                    self.internal_extensible(agent),
                    property_descriptor,
                    Some(string_desc),
                    gc,
                ));
            }
            // 3. Return ! OrdinaryDefineOwnProperty(S, P, Desc).
        }

        let backing_object = self
            .get_backing_object(agent)
            .unwrap_or_else(|| self.create_backing_object(agent));
        TryResult::Continue(ordinary_define_own_property(
            agent,
            backing_object,
            property_key,
            property_descriptor,
            gc,
        ))
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        if let Ok(string) = String::try_from(agent[self].data) {
            if string
                .get_property_descriptor(agent, property_key)
                .is_some()
            {
                return TryResult::Continue(true);
            }
        }

        // 1. Return ? OrdinaryHasProperty(O, P).
        ordinary_try_has_property_entry(agent, self, property_key, gc)
    }

    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope,
    ) -> JsResult<bool> {
        let property_key = property_key.bind(gc.nogc());
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
            Some(backing_object) => {
                ordinary_has_property(agent, backing_object, property_key.unbind(), gc)
            }
            None => {
                // 3. Let parent be ? O.[[GetPrototypeOf]]().
                // Note: Primitive objects never call into JS from GetPrototypeOf.
                let parent = unwrap_try(self.try_get_prototype_of(agent, gc.nogc()));

                // 4. If parent is not null, then
                if let Some(parent) = parent {
                    // a. Return ? parent.[[HasProperty]](P).
                    parent
                        .unbind()
                        .internal_has_property(agent, property_key.unbind(), gc)
                } else {
                    // 5. Return false.
                    Ok(false)
                }
            }
        }
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Value<'gc>> {
        if let Ok(string) = String::try_from(agent[self].data) {
            if let Some(string_desc) = string.get_property_descriptor(agent, property_key) {
                return TryResult::Continue(string_desc.value.unwrap());
            }
        }

        // 1. Return ? OrdinaryGet(O, P, Receiver).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_try_get(agent, backing_object, property_key, receiver, gc)
            }
            None => {
                // a. Let parent be ? O.[[GetPrototypeOf]]().
                let Some(parent) = unwrap_try(self.try_get_prototype_of(agent, gc)) else {
                    // b. If parent is null, return undefined.
                    return TryResult::Continue(Value::Undefined);
                };

                // c. Return ? parent.[[Get]](P, Receiver).
                parent.try_get(agent, property_key, receiver, gc)
            }
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
        if let Ok(string) = String::try_from(agent[self].data) {
            if let Some(string_desc) = string.get_property_descriptor(agent, property_key) {
                return Ok(string_desc.value.unwrap());
            }
        }

        // 1. Return ? OrdinaryGet(O, P, Receiver).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_get(agent, backing_object, property_key.unbind(), receiver, gc)
            }
            None => {
                // a. Let parent be ? O.[[GetPrototypeOf]]().
                let Some(parent) = unwrap_try(self.try_get_prototype_of(agent, gc.nogc())) else {
                    // b. If parent is null, return undefined.
                    return Ok(Value::Undefined);
                };

                // c. Return ? parent.[[Get]](P, Receiver).
                parent
                    .unbind()
                    .internal_get(agent, property_key.unbind(), receiver, gc)
            }
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
        if let Ok(string) = String::try_from(agent[self].data) {
            if string
                .get_property_descriptor(agent, property_key)
                .is_some()
            {
                return TryResult::Continue(false);
            }
        }

        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_try_set(agent, self.into_object(), property_key, value, receiver, gc)
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope,
    ) -> JsResult<bool> {
        let property_key = property_key.bind(gc.nogc());
        if let Ok(string) = String::try_from(agent[self].data) {
            if string
                .get_property_descriptor(agent, property_key)
                .is_some()
            {
                return Ok(false);
            }
        }

        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_set(
            agent,
            self.into_object(),
            property_key.unbind(),
            value,
            receiver,
            gc,
        )
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        if let Ok(string) = String::try_from(agent[self].data) {
            // A String will return unconfigurable descriptors for length and
            // all valid string indexes, making delete return false.
            if property_key == BUILTIN_STRING_MEMORY.length.into() {
                return TryResult::Continue(false);
            } else if let PropertyKey::Integer(index) = property_key {
                let index = index.into_i64();
                if index >= 0 && (index as usize) < string.utf16_len(agent) {
                    return TryResult::Continue(false);
                }
            }
        }

        // 1. Return ! OrdinaryDelete(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                TryResult::Continue(ordinary_delete(agent, backing_object, property_key, gc))
            }
            None => TryResult::Continue(true),
        }
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
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
                    backing_object_keys = ordinary_own_property_keys(agent, backing_object, gc);
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

            return TryResult::Continue(keys);
        }

        // 1. Return OrdinaryOwnPropertyKeys(O).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                TryResult::Continue(ordinary_own_property_keys(agent, backing_object, gc))
            }
            None => TryResult::Continue(vec![]),
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
    pub(crate) object_index: Option<OrdinaryObject<'static>>,
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

impl Rootable for PrimitiveObject<'static> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::PrimitiveObject(value))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::PrimitiveObject(object) => Some(object),
            _ => None,
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

impl HeapMarkAndSweep for PrimitiveObject<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.primitive_objects.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.primitive_objects.shift_index(&mut self.0);
    }
}

impl CreateHeapData<PrimitiveObjectHeapData, PrimitiveObject<'static>> for Heap {
    fn create(&mut self, data: PrimitiveObjectHeapData) -> PrimitiveObject<'static> {
        self.primitive_objects.push(Some(data));
        PrimitiveObject(PrimitiveObjectIndex::last(&self.primitive_objects))
    }
}
