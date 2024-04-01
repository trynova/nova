mod data;
mod internal_methods;
mod internal_slots;
mod into_object;
mod property_key;
mod property_storage;
use std::ops::Deref;

use super::{
    value::{
        ARRAY_BUFFER_DISCRIMINANT, ARRAY_DISCRIMINANT, BOUND_FUNCTION_DISCRIMINANT,
        BUILTIN_FUNCTION_DISCRIMINANT, ECMASCRIPT_FUNCTION_DISCRIMINANT, ERROR_DISCRIMINANT,
        OBJECT_DISCRIMINANT,
    },
    Function, IntoValue, Value,
};
use crate::{
    ecmascript::{
        builtins::{error::Error, ArgumentsList, Array, ArrayBuffer},
        execution::{Agent, JsResult},
        types::PropertyDescriptor,
    },
    heap::{
        indexes::{
            ArrayBufferIndex, ArrayIndex, BoundFunctionIndex, BuiltinFunctionIndex,
            ECMAScriptFunctionIndex, ErrorIndex, ObjectIndex,
        },
        GetHeapData,
    },
};

pub use data::ObjectHeapData;
pub use internal_methods::InternalMethods;
pub use internal_slots::OrdinaryObjectInternalSlots;
pub use into_object::IntoObject;
pub use property_key::PropertyKey;
pub use property_storage::PropertyStorage;

/// ### [6.1.7 The Object Type](https://tc39.es/ecma262/#sec-object-type)
///
/// In Nova
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Object {
    Object(ObjectIndex) = OBJECT_DISCRIMINANT,
    // Date(DateIndex) = DATE_DISCRIMINANT,
    Array(ArrayIndex) = ARRAY_DISCRIMINANT,
    ArrayBuffer(ArrayBufferIndex) = ARRAY_BUFFER_DISCRIMINANT,
    Error(ErrorIndex) = ERROR_DISCRIMINANT,
    BoundFunction(BoundFunctionIndex) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunctionIndex) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunctionIndex) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    //RegExp(RegExpIndex) = REGEXP_DISCRIMINANT,
}

#[derive(Debug, Clone, Copy)]
pub struct OrdinaryObject(pub(crate) ObjectIndex);

impl IntoObject for OrdinaryObject {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl IntoValue for OrdinaryObject {
    fn into_value(self) -> Value {
        self.into()
    }
}

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

impl From<OrdinaryObject> for Value {
    fn from(value: OrdinaryObject) -> Self {
        Self::Object(value.0)
    }
}

impl TryFrom<Value> for OrdinaryObject {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Object(data) => Ok(OrdinaryObject(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for OrdinaryObject {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::Object(data) => Ok(OrdinaryObject(data)),
            _ => Err(()),
        }
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

impl From<BoundFunctionIndex> for Object {
    fn from(value: BoundFunctionIndex) -> Self {
        Object::BoundFunction(value)
    }
}

impl From<BuiltinFunctionIndex> for Object {
    fn from(value: BuiltinFunctionIndex) -> Self {
        Object::BuiltinFunction(value)
    }
}

impl From<ECMAScriptFunctionIndex> for Object {
    fn from(value: ECMAScriptFunctionIndex) -> Self {
        Object::ECMAScriptFunction(value)
    }
}

impl From<ErrorIndex> for Object {
    fn from(value: ErrorIndex) -> Self {
        Object::Error(value)
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
    fn try_from(value: Value) -> Result<Self, ()> {
        match value {
            Value::Object(x) => Ok(Object::from(x)),
            Value::Array(x) => Ok(Object::from(x)),
            Value::Error(x) => Ok(Object::from(x)),
            Value::BoundFunction(x) => Ok(Object::from(x)),
            Value::BuiltinFunction(x) => Ok(Object::from(x)),
            Value::ECMAScriptFunction(x) => Ok(Object::from(x)),
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
}

impl OrdinaryObjectInternalSlots for Object {
    fn extensible(self, agent: &Agent) -> bool {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).extensible(agent),
            Object::Array(idx) => Array::from(idx).extensible(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).extensible(agent),
            Object::Error(idx) => Error::from(idx).extensible(agent),
            Object::BoundFunction(idx) => Function::from(idx).extensible(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).extensible(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).extensible(agent),
        }
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).set_extensible(agent, value),
            Object::Array(idx) => Array::from(idx).set_extensible(agent, value),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).set_extensible(agent, value),
            Object::Error(idx) => Error::from(idx).set_extensible(agent, value),
            Object::BoundFunction(idx) => Function::from(idx).set_extensible(agent, value),
            Object::BuiltinFunction(idx) => Function::from(idx).set_extensible(agent, value),
            Object::ECMAScriptFunction(idx) => Function::from(idx).set_extensible(agent, value),
        }
    }

    fn prototype(self, agent: &Agent) -> Option<Object> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).prototype(agent),
            Object::Array(idx) => Array::from(idx).prototype(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).prototype(agent),
            Object::Error(idx) => Error::from(idx).prototype(agent),
            Object::BoundFunction(idx) => Function::from(idx).prototype(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).prototype(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).prototype(agent),
        }
    }

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).set_prototype(agent, prototype),
            Object::Array(idx) => Array::from(idx).set_prototype(agent, prototype),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).set_prototype(agent, prototype),
            Object::Error(idx) => Error::from(idx).set_prototype(agent, prototype),
            Object::BoundFunction(idx) => Function::from(idx).set_prototype(agent, prototype),
            Object::BuiltinFunction(idx) => Function::from(idx).set_prototype(agent, prototype),
            Object::ECMAScriptFunction(idx) => Function::from(idx).set_prototype(agent, prototype),
        }
    }
}

impl InternalMethods for Object {
    fn get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).get_prototype_of(agent),
            Object::Array(idx) => Array::from(idx).get_prototype_of(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).get_prototype_of(agent),
            Object::Error(idx) => Error::from(idx).get_prototype_of(agent),
            Object::BoundFunction(idx) => Function::from(idx).get_prototype_of(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).get_prototype_of(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).get_prototype_of(agent),
        }
    }

    fn set_prototype_of(self, agent: &mut Agent, prototype: Option<Object>) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).set_prototype_of(agent, prototype),
            Object::Array(idx) => Array::from(idx).set_prototype_of(agent, prototype),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).set_prototype_of(agent, prototype),
            Object::Error(idx) => Error::from(idx).set_prototype_of(agent, prototype),
            Object::BoundFunction(idx) => Function::from(idx).set_prototype_of(agent, prototype),
            Object::BuiltinFunction(idx) => Function::from(idx).set_prototype_of(agent, prototype),
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).set_prototype_of(agent, prototype)
            }
        }
    }

    fn is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).is_extensible(agent),
            Object::Array(idx) => Array::from(idx).is_extensible(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).is_extensible(agent),
            Object::Error(idx) => Error::from(idx).is_extensible(agent),
            Object::BoundFunction(idx) => Function::from(idx).is_extensible(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).is_extensible(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).is_extensible(agent),
        }
    }

    fn prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).prevent_extensions(agent),
            Object::Array(idx) => Array::from(idx).prevent_extensions(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).prevent_extensions(agent),
            Object::Error(idx) => Error::from(idx).prevent_extensions(agent),
            Object::BoundFunction(idx) => Function::from(idx).prevent_extensions(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).prevent_extensions(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).prevent_extensions(agent),
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
            Object::Error(idx) => Error::from(idx).get_own_property(agent, property_key),
            Object::BoundFunction(idx) => Function::from(idx).get_own_property(agent, property_key),
            Object::BuiltinFunction(idx) => {
                Function::from(idx).get_own_property(agent, property_key)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).get_own_property(agent, property_key)
            }
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
            Object::Error(idx) => {
                Error::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::BoundFunction(idx) => {
                Function::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).define_own_property(agent, property_key, property_descriptor)
            }
        }
    }

    fn has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).has_property(agent, property_key),
            Object::Array(idx) => Array::from(idx).has_property(agent, property_key),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).has_property(agent, property_key),
            Object::Error(idx) => Error::from(idx).has_property(agent, property_key),
            Object::BoundFunction(idx) => Function::from(idx).has_property(agent, property_key),
            Object::BuiltinFunction(idx) => Function::from(idx).has_property(agent, property_key),
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).has_property(agent, property_key)
            }
        }
    }

    fn get(self, agent: &mut Agent, property_key: PropertyKey, receiver: Value) -> JsResult<Value> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).get(agent, property_key, receiver),
            Object::Array(idx) => Array::from(idx).get(agent, property_key, receiver),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).get(agent, property_key, receiver),
            Object::Error(idx) => Error::from(idx).get(agent, property_key, receiver),
            Object::BoundFunction(idx) => Function::from(idx).get(agent, property_key, receiver),
            Object::BuiltinFunction(idx) => Function::from(idx).get(agent, property_key, receiver),
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).get(agent, property_key, receiver)
            }
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
            Object::Error(idx) => Error::from(idx).set(agent, property_key, value, receiver),
            Object::BoundFunction(idx) => {
                Function::from(idx).set(agent, property_key, value, receiver)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).set(agent, property_key, value, receiver)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).set(agent, property_key, value, receiver)
            }
        }
    }

    fn delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).delete(agent, property_key),
            Object::Array(idx) => Array::from(idx).delete(agent, property_key),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).delete(agent, property_key),
            Object::Error(idx) => Error::from(idx).delete(agent, property_key),
            Object::BoundFunction(idx) => Function::from(idx).delete(agent, property_key),
            Object::BuiltinFunction(idx) => Function::from(idx).delete(agent, property_key),
            Object::ECMAScriptFunction(idx) => Function::from(idx).delete(agent, property_key),
        }
    }

    fn own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        match self {
            Object::Object(idx) => OrdinaryObject::from(idx).own_property_keys(agent),
            Object::Array(idx) => Array::from(idx).own_property_keys(agent),
            Object::ArrayBuffer(idx) => ArrayBuffer::from(idx).own_property_keys(agent),
            Object::Error(idx) => Error::from(idx).own_property_keys(agent),
            Object::BoundFunction(idx) => Function::from(idx).own_property_keys(agent),
            Object::BuiltinFunction(idx) => Function::from(idx).own_property_keys(agent),
            Object::ECMAScriptFunction(idx) => Function::from(idx).own_property_keys(agent),
        }
    }

    fn call(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        match self {
            Object::BoundFunction(idx) => {
                Function::from(idx).call(agent, this_value, arguments_list)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).call(agent, this_value, arguments_list)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).call(agent, this_value, arguments_list)
            }
            _ => unreachable!(),
        }
    }

    fn construct(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
    ) -> JsResult<Object> {
        match self {
            Object::BoundFunction(idx) => {
                Function::from(idx).construct(agent, arguments_list, new_target)
            }
            Object::BuiltinFunction(idx) => {
                Function::from(idx).construct(agent, arguments_list, new_target)
            }
            Object::ECMAScriptFunction(idx) => {
                Function::from(idx).construct(agent, arguments_list, new_target)
            }
            _ => unreachable!(),
        }
    }
}
