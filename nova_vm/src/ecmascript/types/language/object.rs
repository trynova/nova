mod data;
mod internal_methods;
mod internal_slots;
mod property_key;
mod property_storage;
use std::ops::Deref;

use super::{
    value::{
        ARRAY_BUFFER_DISCRIMINANT, ARRAY_DISCRIMINANT, FUNCTION_DISCRIMINANT, OBJECT_DISCRIMINANT,
    },
    Function, Value,
};
use crate::{
    ecmascript::{
        builtins::{Array, ArrayBuffer},
        execution::{agent::ExceptionType, Agent, JsResult},
        types::PropertyDescriptor,
    },
    heap::{
        indexes::{ArrayBufferIndex, ArrayIndex, FunctionIndex, ObjectIndex},
        GetHeapData,
    },
};

pub use data::ObjectHeapData;
pub use internal_methods::InternalMethods;
pub use internal_slots::OrdinaryObjectInternalSlots;
pub use property_key::PropertyKey;
pub use property_storage::PropertyStorage;

/// 6.1.7 The Object Type
/// https://tc39.es/ecma262/#sec-object-type
///
/// In Nova
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Object {
    Object(ObjectIndex) = OBJECT_DISCRIMINANT,
    // Date(DateIndex) = DATE_DISCRIMINANT,
    // Error(ErrorIndex) = ERROR_DISCRIMINANT,
    Array(ArrayIndex) = ARRAY_DISCRIMINANT,
    ArrayBuffer(ArrayBufferIndex) = ARRAY_BUFFER_DISCRIMINANT,
    Function(FunctionIndex) = FUNCTION_DISCRIMINANT,
    //RegExp(RegExpIndex) = REGEXP_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
pub struct OrdinaryObject(ObjectIndex);

impl From<OrdinaryObject> for Object {
    fn from(value: OrdinaryObject) -> Self {
        Self::Object(value.0)
    }
}

impl From<ObjectIndex> for OrdinaryObject {
    fn from(value: ObjectIndex) -> Self {
        OrdinaryObject(value)
    }
}

impl Deref for OrdinaryObject {
    type Target = ObjectIndex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OrdinaryObjectInternalSlots for OrdinaryObject {
    fn extensible(self, agent: &Agent) -> bool {
        agent.heap.get(*self).extensible
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        agent.heap.get_mut(*self).extensible = value;
    }

    fn prototype(self, agent: &Agent) -> Option<Object> {
        agent.heap.get(*self).prototype
    }

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        agent.heap.get_mut(*self).prototype = prototype;
    }
}

impl OrdinaryObject {
    pub(crate) const fn new(value: ObjectIndex) -> Self {
        Self(value)
    }
}

impl From<ObjectIndex> for Object {
    fn from(value: ObjectIndex) -> Self {
        Object::Object(value)
    }
}

impl From<ArrayIndex> for Object {
    fn from(value: ArrayIndex) -> Self {
        Object::Array(value)
    }
}

impl From<FunctionIndex> for Object {
    fn from(value: FunctionIndex) -> Self {
        Object::Function(value)
    }
}

impl From<Object> for Value {
    fn from(value: Object) -> Self {
        // SAFETY: Sub-enum.
        unsafe { std::mem::transmute::<Object, Value>(value) }
    }
}

impl TryFrom<Value> for Object {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Object(x) => Ok(Object::Object(x)),
            Value::Array(x) => Ok(Object::Array(x)),
            Value::Function(x) => Ok(Object::Function(x)),
            _ => Err(()),
        }
    }
}

impl Object {
    pub fn into_value(self) -> Value {
        self.into()
    }

    pub fn property_storage(self) -> PropertyStorage {
        PropertyStorage::new(self)
    }

    /// /// 7.3.9 DefinePropertyOrThrow ( O, P, desc )
    /// https://tc39.es/ecma262/#sec-definepropertyorthrow
    pub fn define_property_or_throw(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<()> {
        // 1. Let success be ? O.[[DefineOwnProperty]](P, desc).
        let success = self.define_own_property(agent, property_key, property_descriptor)?;

        // 2. If success is false, throw a TypeError exception.
        if !success {
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                "Cannot assign to property on object.",
            ));
        }

        // 3. Return unused.
        Ok(())
    }

    /// 7.3.5 CreateDataProperty ( O, P, V )
    /// https://tc39.es/ecma262/#sec-createdataproperty
    pub fn create_data_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
    ) -> JsResult<bool> {
        // 1. Let newDesc be the PropertyDescriptor {
        //      [[Value]]: V, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: true
        //    }.
        let new_descriptor = PropertyDescriptor {
            value: Some(value),
            writable: Some(true),
            enumerable: Some(true),
            configurable: Some(true),
            ..Default::default()
        };

        // 2. Return ? O.[[DefineOwnProperty]](P, newDesc).
        self.define_own_property(agent, property_key, new_descriptor)
    }
}

impl OrdinaryObjectInternalSlots for Object {
    fn extensible(self, agent: &Agent) -> bool {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).extensible(agent),
            Object::Array(idx) => Array::from(idx).extensible(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).extensible(agent),
            Object::Function(idx) => Function::from(idx).extensible(agent),
        }
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).set_extensible(agent, value),
            Object::Array(idx) => Array::from(idx).set_extensible(agent, value),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).set_extensible(agent, value),
            Object::Function(idx) => Function::from(idx).set_extensible(agent, value),
        }
    }

    fn prototype(self, agent: &Agent) -> Option<Object> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).prototype(agent),
            Object::Array(idx) => Array::from(idx).prototype(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).prototype(agent),
            Object::Function(idx) => Function::from(idx).prototype(agent),
        }
    }

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).set_prototype(agent, prototype),
            Object::Array(idx) => Array::from(idx).set_prototype(agent, prototype),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).set_prototype(agent, prototype),
            Object::Function(idx) => Function::from(idx).set_prototype(agent, prototype),
        }
    }
}

impl InternalMethods for Object {
    fn get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).get_prototype_of(agent),
            Object::Array(idx) => Array::from(idx).get_prototype_of(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).get_prototype_of(agent),
            Object::Function(idx) => Function::from(idx).get_prototype_of(agent),
        }
    }

    fn set_prototype_of(self, agent: &mut Agent, prototype: Option<Object>) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).set_prototype_of(agent, prototype),
            Object::Array(idx) => Array::from(idx).set_prototype_of(agent, prototype),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).set_prototype_of(agent, prototype),
            Object::Function(idx) => Function::from(idx).set_prototype_of(agent, prototype),
        }
    }

    fn is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).is_extensible(agent),
            Object::Array(idx) => Array::from(idx).is_extensible(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).is_extensible(agent),
            Object::Function(idx) => Function::from(idx).is_extensible(agent),
        }
    }

    fn prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).prevent_extensions(agent),
            Object::Array(idx) => Array::from(idx).prevent_extensions(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).prevent_extensions(agent),
            Object::Function(idx) => Function::from(idx).prevent_extensions(agent),
        }
    }

    fn get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).get_own_property(agent, property_key),
            Object::Array(idx) => Array::from(idx).get_own_property(agent, property_key),
            Object::ArrayBuffer(idx) => {
                ArrayBuffer::from(idx).get_own_property(agent, property_key)
            }
            Object::Function(idx) => Function::from(idx).get_own_property(agent, property_key),
        }
    }

    fn define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).define_own_property(
                agent,
                property_key,
                property_descriptor,
            ),
            Object::Array(idx) => {
                Array::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::ArrayBuffer(idx) => {
                ArrayBuffer::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::Function(idx) => {
                Function::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
        }
    }

    fn has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).has_property(agent, property_key),
            Object::Array(idx) => Array::from(idx).has_property(agent, property_key),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).has_property(agent, property_key),
            Object::Function(idx) => Function::from(idx).has_property(agent, property_key),
        }
    }

    fn get(self, agent: &mut Agent, property_key: PropertyKey, receiver: Value) -> JsResult<Value> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).get(agent, property_key, receiver),
            Object::Array(idx) => Array::from(idx).get(agent, property_key, receiver),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).get(agent, property_key, receiver),
            Object::Function(idx) => Function::from(idx).get(agent, property_key, receiver),
        }
    }

    fn set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        match self {
            Object::Object(idx) => {
                OrdinaryObject::from(idx).set(agent, property_key, value, receiver)
            }
            Object::Array(idx) => Array::from(idx).set(agent, property_key, value, receiver),
            Object::ArrayBuffer(idx) => {
                ArrayBuffer::from(idx).set(agent, property_key, value, receiver)
            }
            Object::Function(idx) => Function::from(idx).set(agent, property_key, value, receiver),
        }
    }

    fn delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).delete(agent, property_key),
            Object::Array(idx) => Array::from(idx).delete(agent, property_key),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).delete(agent, property_key),
            Object::Function(idx) => Function::from(idx).delete(agent, property_key),
        }
    }

    fn own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).own_property_keys(agent),
            Object::Array(idx) => Array::from(idx).own_property_keys(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).own_property_keys(agent),
            Object::Function(idx) => Function::from(idx).own_property_keys(agent),
        }
    }

    fn call(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments_list: &[Value],
    ) -> JsResult<Value> {
        match self {
            Object::Function(idx) => Function::from(idx).call(agent, this_value, arguments_list),
            _ => unreachable!(),
        }
    }

    fn construct(
        self,
        agent: &mut Agent,
        arguments_list: &[Value],
        new_target: Function,
    ) -> JsResult<Object> {
        match self {
            Object::Function(idx) => {
                Function::from(idx).construct(agent, arguments_list, new_target)
            }
            _ => unreachable!(),
        }
    }
}
