mod data;
mod internal_methods;
mod property_key;
mod property_storage;

use super::Value;
use crate::{
    builtins::ordinary,
    execution::{agent::ExceptionType, Agent, Intrinsics, JsResult},
    heap::GetHeapData,
    types::PropertyDescriptor,
};

pub use data::ObjectData;
pub use internal_methods::InternalMethods;
pub use property_key::PropertyKey;
pub use property_storage::PropertyStorage;

/// 6.1.7 The Object Type
/// https://tc39.es/ecma262/#sec-object-type
#[derive(Debug, Clone, Copy)]
pub struct Object(Value);

impl TryFrom<Value> for Object {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Object(_) | Value::Array(_) | Value::Function(_) = value {
            Ok(Self(value))
        } else {
            Err(())
        }
    }
}

impl Object {
    pub(crate) fn new(value: Value) -> Self {
        Self(value)
    }

    pub fn into_value(self) -> Value {
        self.0
    }

    /// [[Extensible]]
    pub fn extensible(self, agent: &mut Agent) -> bool {
        let object = self.into_value();

        match object {
            Value::Object(object) => agent.current_realm().borrow().heap.get(object).extensible,
            Value::Array(_) => true,
            Value::Function(_) => true,
            _ => unreachable!(),
        }
    }

    /// [[Extensible]]
    pub fn set_extensible(self, agent: &mut Agent, value: bool) {
        let object = self.into_value();

        match object {
            Value::Object(object) => {
                let realm = agent.current_realm();
                let mut realm = realm.borrow_mut();
                let object = realm.heap.get_mut(object);
                object.extensible = true;
            }
            // TODO: Correct object/function impl
            Value::Array(_) => {}
            Value::Function(_) => {}
            _ => unreachable!(),
        }
    }

    /// [[Prototype]]
    pub fn prototype(self, agent: &mut Agent) -> Option<Value> {
        let object = self.into_value();
        let realm = agent.current_realm();
        let realm = realm.borrow();

        match object {
            Value::Object(object) => {
                let object = realm.heap.get(object);
                object.prototype.try_into().ok()
            }
            Value::Array(array) => {
                let array = realm.heap.get(array);

                if let Some(object_index) = array.object_index {
                    Some(realm.heap.get(object_index).prototype)
                } else {
                    Some(Intrinsics::array_prototype().into_value())
                }
            }
            Value::Function(_) => Some(Intrinsics::function_prototype().into_value()),
            _ => unreachable!(),
        }
    }

    /// [[Prototype]]
    pub fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        let object = self.into_value();

        match object {
            Value::Object(object) => {
                let realm = agent.current_realm();
                let mut realm = realm.borrow_mut();
                let object = realm.heap.get_mut(object);
                object.prototype = prototype.map(|object| object.into_value()).unwrap();
            }
            Value::Array(_) => todo!(),
            Value::Function(_) => todo!(),
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
        );

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
}
