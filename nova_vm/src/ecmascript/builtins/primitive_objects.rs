// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, BIGINT_DISCRIMINANT, BOOLEAN_DISCRIMINANT, BUILTIN_STRING_MEMORY, BigInt,
        FLOAT_DISCRIMINANT, HeapBigInt, HeapNumber, HeapString, INTEGER_DISCRIMINANT,
        InternalMethods, InternalSlots, JsResult, NUMBER_DISCRIMINANT, Number, Object,
        OrdinaryObject, Primitive, PropertyDescriptor, PropertyKey, ProtoIntrinsics,
        SMALL_BIGINT_DISCRIMINANT, SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT,
        SYMBOL_DISCRIMINANT, SetResult, SmallBigInt, SmallF64, SmallInteger, String, Symbol,
        TryError, TryGetResult, TryHasResult, TryResult, Value, is_compatible_property_descriptor,
        js_result_into_try, object_handle, ordinary_define_own_property, ordinary_delete,
        ordinary_get, ordinary_get_own_property, ordinary_has_property, ordinary_set, unwrap_try,
    },
    engine::context::{Bindable, GcScope, NoGcScope, bindable_handle},
    heap::{
        ArenaAccess, ArenaAccessMut, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        HeapSweepWeakReference, IntrinsicPrimitiveObjectIndexes, WorkQueues, arena_vec_access,
        indexes::BaseIndex,
    },
};
use small_string::SmallString;

use super::{
    ObjectShape, PropertyLookupCache, ordinary_own_property_keys, ordinary_try_get,
    ordinary_try_has_property, ordinary_try_set,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PrimitiveObject<'a>(BaseIndex<'a, PrimitiveObjectRecord<'static>>);
object_handle!(PrimitiveObject);
arena_vec_access!(PrimitiveObject, 'a, PrimitiveObjectRecord, primitive_objects);

impl IntrinsicPrimitiveObjectIndexes {
    pub(crate) const fn get_primitive_object<'a>(
        self,
        base: BaseIndex<'a, PrimitiveObjectRecord<'static>>,
    ) -> PrimitiveObject<'a> {
        PrimitiveObject(BaseIndex::from_index_u32_const(
            self as u32 + base.get_index_u32_const() + Self::PRIMITIVE_OBJECT_INDEX_OFFSET,
        ))
    }
}

impl PrimitiveObject<'_> {
    pub fn is_bigint_object(self, agent: &Agent) -> bool {
        matches!(
            self.get(agent).data,
            PrimitiveObjectData::BigInt(_) | PrimitiveObjectData::SmallBigInt(_)
        )
    }

    pub fn is_number_object(self, agent: &Agent) -> bool {
        matches!(
            self.get(agent).data,
            PrimitiveObjectData::SmallF64(_)
                | PrimitiveObjectData::Integer(_)
                | PrimitiveObjectData::Number(_)
        )
    }

    pub fn is_string_object(self, agent: &Agent) -> bool {
        matches!(
            self.get(agent).data,
            PrimitiveObjectData::String(_) | PrimitiveObjectData::SmallString(_)
        )
    }

    pub fn is_symbol_object(self, agent: &Agent) -> bool {
        matches!(self.get(agent).data, PrimitiveObjectData::Symbol(_))
    }
}

impl<'a> InternalSlots<'a> for PrimitiveObject<'a> {
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

    fn object_shape(self, agent: &mut Agent) -> ObjectShape<'static> {
        if let Some(bo) = self.get_backing_object(agent) {
            bo.object_shape(agent)
        } else {
            let primitive: Primitive = self.get(agent).data.into();
            primitive.object_shape(agent).unwrap()
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        match self.get_backing_object(agent) {
            Some(obj) => obj.internal_prototype(agent),
            None => {
                let intrinsic_default_proto = match self.get(agent).data {
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
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        let o = self.bind(gc);
        // For non-string primitive objects:
        // 1. Return OrdinaryGetOwnProperty(O, P).
        // For string exotic objects:
        // 1. Let desc be OrdinaryGetOwnProperty(S, P).
        // 2. If desc is not undefined, return desc.
        if let Some(backing_object) = o.get_backing_object(agent)
            && let Some(property_descriptor) =
                ordinary_get_own_property(agent, o.into(), backing_object, property_key, cache, gc)
        {
            return TryResult::Continue(Some(property_descriptor));
        }

        if let Ok(string) = String::try_from(self.get(agent).data) {
            // 3. Return StringGetOwnProperty(S, P).
            TryResult::Continue(string.get_property_descriptor(agent, property_key))
        } else {
            TryResult::Continue(None)
        }
    }

    fn try_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        if let Ok(string) = String::try_from(self.get(agent).data) {
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
                    Err(_) => TryError::GcError.into(),
                };
            }
            // 3. Return ! OrdinaryDefineOwnProperty(S, P, Desc).
        }

        let backing_object = self
            .get_backing_object(agent)
            .unwrap_or_else(|| self.create_backing_object(agent));
        js_result_into_try(ordinary_define_own_property(
            agent,
            self.into(),
            backing_object,
            property_key,
            property_descriptor,
            cache,
            gc,
        ))
    }

    fn try_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryHasResult<'gc>> {
        if let Ok(string) = String::try_from(self.get(agent).data)
            && string.get_property_value(agent, property_key).is_some()
        {
            return TryHasResult::Custom(0, self.bind(gc).into()).into();
        }

        // 1. Return ? OrdinaryHasProperty(O, P).
        ordinary_try_has_property(
            agent,
            self.into(),
            self.get_backing_object(agent),
            property_key,
            cache,
            gc,
        )
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let property_key = property_key.bind(gc.nogc());
        if let Ok(string) = String::try_from(self.get(agent).data)
            && string.get_property_value(agent, property_key).is_some()
        {
            return Ok(true);
        }

        // 1. Return ? OrdinaryHasProperty(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_has_property(
                agent,
                self.into(),
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
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        if let Ok(string) = String::try_from(self.get(agent).data)
            && let Some(value) = string.get_property_value(agent, property_key)
        {
            return TryGetResult::Value(value.bind(gc)).into();
        }
        // 1. Return ? OrdinaryGet(O, P, Receiver).
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
        if let Ok(string) = String::try_from(self.get(agent).data)
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

    fn try_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        if let Ok(string) = String::try_from(self.get(agent).data)
            && string.get_property_value(agent, property_key).is_some()
        {
            return SetResult::Unwritable.into();
        }

        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_try_set(agent, self, property_key, value, receiver, cache, gc)
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
        if let Ok(string) = String::try_from(self.get(agent).data)
            && string.get_property_value(agent, property_key).is_some()
        {
            return Ok(false);
        }

        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_set(
            agent,
            self.into(),
            property_key.unbind(),
            value,
            receiver,
            gc,
        )
    }

    fn try_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        if let Ok(string) = String::try_from(self.get(agent).data) {
            // A String will return unconfigurable descriptors for length and
            // all valid string indexes, making delete return false.
            if property_key == BUILTIN_STRING_MEMORY.length.into() {
                return TryResult::Continue(false);
            } else if let PropertyKey::Integer(index) = property_key {
                let index = index.into_i64();
                if index >= 0 && (index as usize) < string.utf16_len_(agent) {
                    return TryResult::Continue(false);
                }
            }
        }

        // 1. Return ! OrdinaryDelete(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => TryResult::Continue(ordinary_delete(
                agent,
                self.into(),
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
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        if let Ok(string) = String::try_from(self.get(agent).data) {
            let len = string.utf16_len_(agent);
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
bindable_handle!(PrimitiveObjectData);

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

impl<'a> From<PrimitiveObjectData<'a>> for Primitive<'a> {
    fn from(value: PrimitiveObjectData<'a>) -> Self {
        match value {
            PrimitiveObjectData::Boolean(d) => Self::Boolean(d),
            PrimitiveObjectData::String(d) => Self::String(d),
            PrimitiveObjectData::SmallString(d) => Self::SmallString(d),
            PrimitiveObjectData::Symbol(d) => Self::Symbol(d),
            PrimitiveObjectData::Number(d) => Self::Number(d),
            PrimitiveObjectData::Integer(d) => Self::Integer(d),
            PrimitiveObjectData::SmallF64(d) => Self::SmallF64(d),
            PrimitiveObjectData::BigInt(d) => Self::BigInt(d),
            PrimitiveObjectData::SmallBigInt(d) => Self::SmallBigInt(d),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PrimitiveObjectRecord<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) data: PrimitiveObjectData<'a>,
}
bindable_handle!(PrimitiveObjectRecord);

impl<'a> PrimitiveObjectRecord<'a> {
    pub(crate) const BLANK: Self = Self {
        object_index: None,
        data: PrimitiveObjectData::Boolean(false),
    };

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

impl HeapMarkAndSweep for PrimitiveObjectRecord<'static> {
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

impl<'a> CreateHeapData<PrimitiveObjectRecord<'a>, PrimitiveObject<'a>> for Heap {
    fn create(&mut self, data: PrimitiveObjectRecord<'a>) -> PrimitiveObject<'a> {
        self.primitive_objects.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<PrimitiveObjectRecord<'static>>();
        PrimitiveObject(BaseIndex::last(&self.primitive_objects))
    }
}
