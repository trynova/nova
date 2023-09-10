mod data;
mod internal_methods;
mod property_key;
mod property_storage;

use super::Value;
use crate::{
    builtins::ordinary,
    execution::{agent::ExceptionType, Agent, Intrinsics, JsResult},
    heap::{
        indexes::{ArrayIndex, FunctionIndex, ObjectIndex},
        GetHeapData,
    },
    types::PropertyDescriptor,
};

pub use data::ObjectData;
pub use internal_methods::InternalMethods;
pub use property_key::PropertyKey;
pub use property_storage::PropertyStorage;

/// 6.1.7 The Object Type
/// https://tc39.es/ecma262/#sec-object-type
#[derive(Debug, Clone, Copy)]
pub enum Object {
    Object(ObjectIndex),
    Array(ArrayIndex),
    Function(FunctionIndex),
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
        match value {
            Object::Object(x) => Value::Object(x),
            Object::Array(x) => Value::Array(x),
            Object::Function(x) => Value::Function(x),
        }
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

    /// [[Extensible]]
    pub fn extensible(self, agent: &mut Agent) -> bool {
        match self {
            Object::Object(object) => agent.current_realm().borrow().heap.get(object).extensible,
            Object::Array(_) => true,
            Object::Function(_) => true,
        }
    }

    /// [[Extensible]]
    pub fn set_extensible(self, agent: &mut Agent, value: bool) {
        match self {
            Object::Object(object) => {
                let realm = agent.current_realm();
                let mut realm = realm.borrow_mut();
                let object = realm.heap.get_mut(object);
                object.extensible = true;
            }
            // TODO: Correct object/function impl
            Object::Array(_) => {}
            Object::Function(_) => {}
            _ => unreachable!(),
        }
    }

    /// [[Prototype]]
    pub fn prototype(self, agent: &mut Agent) -> Option<Value> {
        let realm = agent.current_realm();
        let realm = realm.borrow();

        match self {
            Object::Object(object) => {
                let object = realm.heap.get(object);
                object.prototype.try_into().ok()
            }
            Object::Array(array) => {
                let array = realm.heap.get(array);

                if let Some(object_index) = array.object_index {
                    Some(realm.heap.get(object_index).prototype)
                } else {
                    Some(Intrinsics::array_prototype().into_value())
                }
            }
            Object::Function(_) => Some(Intrinsics::function_prototype().into_value()),
            _ => unreachable!(),
        }
    }

    /// [[Prototype]]
    pub fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        match self {
            Object::Object(object) => {
                let realm = agent.current_realm();
                let mut realm = realm.borrow_mut();
                let object = realm.heap.get_mut(object);
                object.prototype = prototype.map(|object| object.into_value()).unwrap();
            }
            Object::Array(_) => todo!(),
            Object::Function(_) => todo!(),
            _ => unreachable!(),
        }
    }

    pub fn internal_methods<'a>(self, agent: &mut Agent) -> &'a InternalMethods {
        // TODO: Logic for fetching methods for objects/anything else.
        &ordinary::METHODS
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
        let success = (self.internal_methods(agent).define_own_property)(
            agent,
            self,
            property_key,
            property_descriptor,
        )?;

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
        self: Self,
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
        Ok(self.internal_methods(agent).define_own_property)(
            agent,
            self,
            property_key,
            new_descriptor,
        )
    }
}
