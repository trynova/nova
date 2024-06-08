use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            bigint::{HeapBigInt, SmallBigInt},
            BigInt, HeapNumber, HeapString, InternalMethods, InternalSlots, IntoObject, IntoValue,
            Number, Object, ObjectHeapData, OrdinaryObject, String, Symbol, Value,
            BIGINT_DISCRIMINANT, BOOLEAN_DISCRIMINANT, FLOAT_DISCRIMINANT, INTEGER_DISCRIMINANT,
            NUMBER_DISCRIMINANT, SMALL_BIGINT_DISCRIMINANT, SMALL_STRING_DISCRIMINANT,
            STRING_DISCRIMINANT, SYMBOL_DISCRIMINANT,
        },
    },
    heap::{
        indexes::PrimitiveObjectIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
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

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject {
        debug_assert!(self.get_backing_object(agent).is_none());
        let prototype = self.internal_prototype(agent).unwrap();
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype),
            keys: Default::default(),
            values: Default::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }
}

impl InternalMethods for PrimitiveObject {}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum PrimitiveObjectData {
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    String(HeapString) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Symbol(Symbol) = SYMBOL_DISCRIMINANT,
    Number(HeapNumber) = NUMBER_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    Float(f32) = FLOAT_DISCRIMINANT,
    BigInt(HeapBigInt) = BIGINT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
}

impl TryFrom<PrimitiveObjectData> for BigInt {
    type Error = ();

    fn try_from(value: PrimitiveObjectData) -> Result<Self, Self::Error> {
        match value {
            PrimitiveObjectData::BigInt(data) => Ok(BigInt::BigInt(data)),
            PrimitiveObjectData::SmallBigInt(data) => Ok(BigInt::SmallBigInt(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<PrimitiveObjectData> for Number {
    type Error = ();

    fn try_from(value: PrimitiveObjectData) -> Result<Self, Self::Error> {
        match value {
            PrimitiveObjectData::Number(data) => Ok(Number::Number(data)),
            PrimitiveObjectData::Integer(data) => Ok(Number::Integer(data)),
            PrimitiveObjectData::Float(data) => Ok(Number::Float(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<PrimitiveObjectData> for String {
    type Error = ();

    fn try_from(value: PrimitiveObjectData) -> Result<Self, Self::Error> {
        match value {
            PrimitiveObjectData::String(data) => Ok(String::String(data)),
            PrimitiveObjectData::SmallString(data) => Ok(String::SmallString(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<PrimitiveObjectData> for Symbol {
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
            data: PrimitiveObjectData::Symbol(symbol),
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
