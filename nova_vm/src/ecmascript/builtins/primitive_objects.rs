// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};
use std::ops::ControlFlow;

use crate::{
    SmallInteger,
    ecmascript::{
        builtins::ordinary::{
            is_compatible_property_descriptor, ordinary_define_own_property, ordinary_delete,
            ordinary_get, ordinary_get_own_property, ordinary_has_property, ordinary_set,
        },
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            BIGINT_DISCRIMINANT, BOOLEAN_DISCRIMINANT, BUILTIN_STRING_MEMORY, BigInt,
            FLOAT_DISCRIMINANT, HeapNumber, HeapString, INTEGER_DISCRIMINANT, InternalMethods,
            InternalSlots, IntoObject, IntoPrimitive, IntoValue, NUMBER_DISCRIMINANT, NoCache,
            Number, Object, OrdinaryObject, Primitive, PropertyDescriptor, PropertyKey,
            SMALL_BIGINT_DISCRIMINANT, SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT,
            SYMBOL_DISCRIMINANT, SetCachedProps, SetCachedResult, String, Symbol, TryGetContinue,
            TryGetResult, Value, bigint::HeapBigInt,
        },
    },
    engine::{
        TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
        small_bigint::SmallBigInt,
        small_f64::SmallF64,
        unwrap_try,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::PrimitiveObjectIndex,
    },
};
use small_string::SmallString;

use super::ordinary::{
    caches::PropertyLookupCache, ordinary_own_property_keys, ordinary_try_get,
    ordinary_try_has_property_entry, ordinary_try_set, shape::ObjectShape,
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
    type Output = PrimitiveObjectHeapData<'static>;

    fn index(&self, index: PrimitiveObject) -> &Self::Output {
        &self.heap.primitive_objects[index]
    }
}

impl IndexMut<PrimitiveObject<'_>> for Agent {
    fn index_mut(&mut self, index: PrimitiveObject) -> &mut Self::Output {
        &mut self.heap.primitive_objects[index]
    }
}

impl Index<PrimitiveObject<'_>> for Vec<Option<PrimitiveObjectHeapData<'static>>> {
    type Output = PrimitiveObjectHeapData<'static>;

    fn index(&self, index: PrimitiveObject) -> &Self::Output {
        self.get(index.get_index())
            .expect("PrimitiveObject out of bounds")
            .as_ref()
            .expect("PrimitiveObject slot empty")
    }
}

impl IndexMut<PrimitiveObject<'_>> for Vec<Option<PrimitiveObjectHeapData<'static>>> {
    fn index_mut(&mut self, index: PrimitiveObject) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("PrimitiveObject out of bounds")
            .as_mut()
            .expect("PrimitiveObject slot empty")
    }
}

impl PrimitiveObject<'_> {
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
            PrimitiveObjectData::SmallF64(_)
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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for PrimitiveObject<'_> {
    type Of<'a> = PrimitiveObject<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> InternalSlots<'a> for PrimitiveObject<'a> {
    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            agent[self]
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }

    fn object_shape(self, agent: &mut Agent) -> ObjectShape<'static> {
        if let Some(bo) = self.get_backing_object(agent) {
            bo.object_shape(agent)
        } else {
            agent[self]
                .data
                .into_primitive()
                .object_shape(agent)
                .unwrap()
        }
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
                    | PrimitiveObjectData::SmallF64(_) => ProtoIntrinsics::Number,
                    PrimitiveObjectData::BigInt(_) | PrimitiveObjectData::SmallBigInt(_) => {
                        ProtoIntrinsics::BigInt
                    }
                };
                // TODO: Should take realm from "backing object"'s Realm/None
                // variant
                Some(
                    agent
                        .current_realm_record()
                        .intrinsics()
                        .get_intrinsic_default_proto(intrinsic_default_proto),
                )
            }
        }
    }
}

impl<'a> InternalMethods<'a> for PrimitiveObject<'a> {
    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<PropertyDescriptor<'gc>>> {
        let o = self.bind(gc);
        // For non-string primitive objects:
        // 1. Return OrdinaryGetOwnProperty(O, P).
        // For string exotic objects:
        // 1. Let desc be OrdinaryGetOwnProperty(S, P).
        // 2. If desc is not undefined, return desc.
        if let Some(backing_object) = o.get_backing_object(agent)
            && let Some(property_descriptor) =
                ordinary_get_own_property(agent, o.into_object(), backing_object, property_key, gc)
        {
            return TryResult::Continue(Some(property_descriptor));
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
                return match is_compatible_property_descriptor(
                    agent,
                    self.internal_extensible(agent),
                    property_descriptor,
                    Some(string_desc),
                    gc,
                ) {
                    Ok(b) => TryResult::Continue(b),
                    Err(_) => TryResult::Break(()),
                };
            }
            // 3. Return ! OrdinaryDefineOwnProperty(S, P, Desc).
        }

        let backing_object = self
            .get_backing_object(agent)
            .unwrap_or_else(|| self.create_backing_object(agent));
        match ordinary_define_own_property(
            agent,
            self.into_object(),
            backing_object,
            property_key,
            property_descriptor,
            gc,
        ) {
            Ok(b) => TryResult::Continue(b),
            Err(_) => TryResult::Break(()),
        }
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        if let Ok(string) = String::try_from(agent[self].data)
            && string.get_property_value(agent, property_key).is_some()
        {
            return TryResult::Continue(true);
        }

        // 1. Return ? OrdinaryHasProperty(O, P).
        ordinary_try_has_property_entry(agent, self, property_key, gc)
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let property_key = property_key.bind(gc.nogc());
        if let Ok(string) = String::try_from(agent[self].data)
            && string.get_property_value(agent, property_key).is_some()
        {
            return Ok(true);
        }

        // 1. Return ? OrdinaryHasProperty(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_has_property(
                agent,
                self.into_object(),
                backing_object,
                property_key.unbind(),
                gc,
            ),
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
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryGetResult<'gc> {
        if let Ok(string) = String::try_from(agent[self].data)
            && let Some(value) = string.get_property_value(agent, property_key)
        {
            return TryGetContinue::Value(value.bind(gc)).into();
        }

        // 1. Return ? OrdinaryGet(O, P, Receiver).
        match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_try_get(
                agent,
                self.into_object(),
                backing_object,
                property_key,
                receiver,
                gc,
            ),
            None => {
                // a. Let parent be ? O.[[GetPrototypeOf]]().
                let Some(parent) = unwrap_try(self.try_get_prototype_of(agent, gc)) else {
                    // b. If parent is null, return undefined.
                    return TryGetContinue::Unset.into();
                };

                // c. Return ? parent.[[Get]](P, Receiver).
                parent.try_get(agent, property_key, receiver, None, gc)
            }
        }
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let property_key = property_key.bind(gc.nogc());
        if let Ok(string) = String::try_from(agent[self].data)
            && let Some(value) = string.get_property_value(agent, property_key)
        {
            return Ok(value.bind(gc.into_nogc()));
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
        if let Ok(string) = String::try_from(agent[self].data)
            && string.get_property_value(agent, property_key).is_some()
        {
            return TryResult::Continue(false);
        }

        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_try_set(agent, self.into_object(), property_key, value, receiver, gc)
    }

    fn internal_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let property_key = property_key.bind(gc.nogc());
        if let Ok(string) = String::try_from(agent[self].data)
            && string.get_property_value(agent, property_key).is_some()
        {
            return Ok(false);
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
            Some(backing_object) => TryResult::Continue(ordinary_delete(
                agent,
                self.into_object(),
                backing_object,
                property_key,
                gc,
            )),
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

    fn get_cached<'gc>(
        self,
        agent: &mut Agent,
        p: PropertyKey,
        cache: PropertyLookupCache,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<TryGetContinue<'gc>, NoCache> {
        if let Ok(string) = String::try_from(agent[self].data)
            && let Some(value) = string.get_property_value(agent, p)
        {
            value.into()
        } else {
            let shape = self.object_shape(agent);
            shape.get_cached(agent, p, self.into_value(), cache, gc)
        }
    }

    fn set_cached<'gc>(
        self,
        agent: &mut Agent,
        props: &SetCachedProps,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<SetCachedResult<'gc>, NoCache> {
        if String::try_from(agent[self].data)
            .is_ok_and(|s| s.get_property_value(agent, props.p).is_some())
        {
            SetCachedResult::Unwritable.into()
        } else {
            let shape = self.object_shape(agent);
            shape.set_cached(agent, self.into_object(), props, gc)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum PrimitiveObjectData<'a> {
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    String(HeapString<'a>) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Symbol(Symbol<'a>) = SYMBOL_DISCRIMINANT,
    Number(HeapNumber<'a>) = NUMBER_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
    BigInt(HeapBigInt<'a>) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

impl<'a> TryFrom<PrimitiveObjectData<'a>> for BigInt<'a> {
    type Error = ();

    fn try_from(value: PrimitiveObjectData<'a>) -> Result<Self, Self::Error> {
        match value {
            PrimitiveObjectData::BigInt(data) => Ok(BigInt::BigInt(data)),
            PrimitiveObjectData::SmallBigInt(data) => Ok(BigInt::SmallBigInt(data)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<PrimitiveObjectData<'a>> for Number<'a> {
    type Error = ();

    fn try_from(value: PrimitiveObjectData<'a>) -> Result<Self, Self::Error> {
        match value {
            PrimitiveObjectData::Number(data) => Ok(Number::Number(data)),
            PrimitiveObjectData::Integer(data) => Ok(Number::Integer(data)),
            PrimitiveObjectData::SmallF64(data) => Ok(Number::SmallF64(data)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<PrimitiveObjectData<'a>> for String<'a> {
    type Error = ();

    fn try_from(value: PrimitiveObjectData<'a>) -> Result<Self, Self::Error> {
        match value {
            PrimitiveObjectData::String(data) => Ok(String::String(data)),
            PrimitiveObjectData::SmallString(data) => Ok(String::SmallString(data)),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<PrimitiveObjectData<'a>> for Symbol<'a> {
    type Error = ();

    fn try_from(value: PrimitiveObjectData<'a>) -> Result<Self, Self::Error> {
        match value {
            PrimitiveObjectData::Symbol(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> From<PrimitiveObjectData<'a>> for Value<'a> {
    fn from(value: PrimitiveObjectData<'a>) -> Self {
        value.into_primitive().into_value()
    }
}

impl<'a> IntoPrimitive<'a> for PrimitiveObjectData<'a> {
    fn into_primitive(self) -> Primitive<'a> {
        match self {
            PrimitiveObjectData::Boolean(d) => Primitive::Boolean(d),
            PrimitiveObjectData::String(d) => Primitive::String(d),
            PrimitiveObjectData::SmallString(d) => Primitive::SmallString(d),
            PrimitiveObjectData::Symbol(d) => Primitive::Symbol(d),
            PrimitiveObjectData::Number(d) => Primitive::Number(d),
            PrimitiveObjectData::Integer(d) => Primitive::Integer(d),
            PrimitiveObjectData::SmallF64(d) => Primitive::SmallF64(d),
            PrimitiveObjectData::BigInt(d) => Primitive::BigInt(d),
            PrimitiveObjectData::SmallBigInt(d) => Primitive::SmallBigInt(d),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PrimitiveObjectHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) data: PrimitiveObjectData<'a>,
}

impl<'a> PrimitiveObjectHeapData<'a> {
    pub(crate) fn new_big_int_object(big_int: BigInt<'a>) -> Self {
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

    pub(crate) fn new_number_object(number: Number<'a>) -> Self {
        let data = match number {
            Number::Number(data) => PrimitiveObjectData::Number(data.unbind()),
            Number::Integer(data) => PrimitiveObjectData::Integer(data),
            Number::SmallF64(data) => PrimitiveObjectData::SmallF64(data),
        };
        Self {
            object_index: None,
            data,
        }
    }

    pub(crate) fn new_string_object(string: String<'a>) -> Self {
        let data = match string {
            String::String(data) => PrimitiveObjectData::String(data),
            String::SmallString(data) => PrimitiveObjectData::SmallString(data),
        };
        Self {
            object_index: None,
            data,
        }
    }

    pub(crate) fn new_symbol_object(symbol: Symbol<'a>) -> Self {
        Self {
            object_index: None,
            data: PrimitiveObjectData::Symbol(symbol.unbind()),
        }
    }
}

impl Rootable for PrimitiveObject<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::PrimitiveObject(value.unbind()))
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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for PrimitiveObjectHeapData<'_> {
    type Of<'a> = PrimitiveObjectHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for PrimitiveObjectHeapData<'static> {
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

impl HeapSweepWeakReference for PrimitiveObject<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .primitive_objects
            .shift_weak_index(self.0)
            .map(Self)
    }
}

impl<'a> CreateHeapData<PrimitiveObjectHeapData<'a>, PrimitiveObject<'a>> for Heap {
    fn create(&mut self, data: PrimitiveObjectHeapData<'a>) -> PrimitiveObject<'a> {
        self.primitive_objects.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<PrimitiveObjectHeapData<'static>>>();
        PrimitiveObject(PrimitiveObjectIndex::last(&self.primitive_objects))
    }
}
