use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::Agent,
        types::{
            bigint::{HeapBigInt, SmallBigInt},
            BigInt, HeapNumber, HeapString, InternalMethods, IntoObject, IntoValue, Number, Object,
            OrdinaryObject, OrdinaryObjectInternalSlots, String, Symbol, Value,
            BIGINT_DISCRIMINANT, BOOLEAN_DISCRIMINANT, FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT,
            NUMBER_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT, SMALL_STRING_DISCRIMINANT,
            STRING_DISCRIMINANT, SYMBOL_DISCRIMINANT,
        },
    },
    heap::{
        indexes::{PrimitiveObjectIndex, SymbolIndex},
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
    SmallInteger,
};
use small_string::SmallString;

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PrimitiveObject(PrimitiveObjectIndex);

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

impl Index<PrimitiveObject> for Agent {
    type Output = PrimitiveObjectHeapData;

    fn index(&self, index: PrimitiveObject) -> &Self::Output {
        self.heap
            .primitive_objects
            .get(index.0.into_index())
            .expect("PrimitiveObject out of bounds")
            .as_ref()
            .expect("PrimitiveObject slot empty")
    }
}

impl IndexMut<PrimitiveObject> for Agent {
    fn index_mut(&mut self, index: PrimitiveObject) -> &mut Self::Output {
        self.heap
            .primitive_objects
            .get_mut(index.0.into_index())
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
}

impl OrdinaryObjectInternalSlots for PrimitiveObject {
    fn internal_extensible(self, agent: &crate::ecmascript::execution::Agent) -> bool {
        todo!()
    }

    fn internal_set_extensible(self, agent: &mut crate::ecmascript::execution::Agent, value: bool) {
        todo!()
    }

    fn internal_prototype(
        self,
        agent: &crate::ecmascript::execution::Agent,
    ) -> Option<crate::ecmascript::types::Object> {
        todo!()
    }

    fn internal_set_prototype(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
        prototype: Option<crate::ecmascript::types::Object>,
    ) {
        todo!()
    }
}

impl InternalMethods for PrimitiveObject {
    fn internal_get_prototype_of(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
    ) -> crate::ecmascript::execution::JsResult<Option<crate::ecmascript::types::Object>> {
        todo!()
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
        prototype: Option<crate::ecmascript::types::Object>,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_is_extensible(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_prevent_extensions(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_get_own_property(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
        property_key: crate::ecmascript::types::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<Option<crate::ecmascript::types::PropertyDescriptor>>
    {
        todo!()
    }

    fn internal_define_own_property(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
        property_key: crate::ecmascript::types::PropertyKey,
        property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_has_property(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
        property_key: crate::ecmascript::types::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_get(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
        property_key: crate::ecmascript::types::PropertyKey,
        receiver: crate::ecmascript::types::Value,
    ) -> crate::ecmascript::execution::JsResult<crate::ecmascript::types::Value> {
        todo!()
    }

    fn internal_set(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
        property_key: crate::ecmascript::types::PropertyKey,
        value: crate::ecmascript::types::Value,
        receiver: crate::ecmascript::types::Value,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_delete(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
        property_key: crate::ecmascript::types::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_own_property_keys(
        self,
        agent: &mut crate::ecmascript::execution::Agent,
    ) -> crate::ecmascript::execution::JsResult<Vec<crate::ecmascript::types::PropertyKey>> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum PrimitiveObjectData {
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    String(HeapString) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Symbol(SymbolIndex) = SYMBOL_DISCRIMINANT,
    Number(HeapNumber) = NUMBER_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    Float(f32) = FLOAT_DISCRIMINANT,
    BigInt(HeapBigInt) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
pub struct PrimitiveObjectHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) data: PrimitiveObjectData,
}

impl PrimitiveObjectHeapData {
    pub(crate) fn new_big_int_object(big_int: BigInt) -> Self {
        let data = match big_int {
            BigInt::BigInt(data) => PrimitiveObjectData::BigInt(data),
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

    pub(crate) fn new_number_object(number: Number) -> Self {
        let data = match number {
            Number::Number(data) => PrimitiveObjectData::Number(data),
            Number::Integer(data) => PrimitiveObjectData::Integer(data),
            Number::Float(data) => PrimitiveObjectData::Float(data),
        };
        Self {
            object_index: None,
            data,
        }
    }

    pub(crate) fn new_string_object(string: String) -> Self {
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
            data: PrimitiveObjectData::Symbol(symbol.0),
        }
    }
}

impl HeapMarkAndSweep for PrimitiveObjectHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
        match self.data {
            PrimitiveObjectData::String(data) => data.mark_values(queues),
            PrimitiveObjectData::Symbol(data) => data.mark_values(queues),
            PrimitiveObjectData::Number(data) => data.mark_values(queues),
            PrimitiveObjectData::BigInt(data) => data.mark_values(queues),
            _ => {}
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
        match &mut self.data {
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
        let self_index = self.0.into_u32();
        self.0 = PrimitiveObjectIndex::from_u32(
            self_index
                - compactions
                    .primitive_objects
                    .get_shift_for_index(self_index),
        );
    }
}

impl CreateHeapData<PrimitiveObjectHeapData, PrimitiveObject> for Heap {
    fn create(&mut self, data: PrimitiveObjectHeapData) -> PrimitiveObject {
        self.primitive_objects.push(Some(data));
        PrimitiveObject(PrimitiveObjectIndex::last(&self.primitive_objects))
    }
}
