use crate::{
    execution::{Agent, JsResult},
    types::{Object, Value},
};

/// 6.2.6 The Property Descriptor Specification Type
/// https://tc39.es/ecma262/#sec-property-descriptor-specification-type
#[derive(Debug, Clone, Default)]
pub struct PropertyDescriptor {
    /// [[Value]]
    pub value: Option<Value>,

    /// [[Writable]]
    pub writable: Option<bool>,

    /// [[Get]]
    pub get: Option<Object>,

    /// [[Set]]
    pub set: Option<Object>,

    /// [[Enumerable]]
    pub enumerable: Option<bool>,

    /// [[Configurable]]
    pub configurable: Option<bool>,
}

impl PropertyDescriptor {
    /// 6.2.6.1 IsAccessorDescriptor ( Desc )
    /// https://tc39.es/ecma262/#sec-isaccessordescriptor
    pub fn is_accessor_descriptor(&self) -> bool {
        // 1. If Desc is undefined, return false.
        match (self.get, self.set) {
            // 2. If Desc has a [[Get]] field, return true.
            (Some(_), _) => true,
            // 3. If Desc has a [[Set]] field, return true.
            (_, Some(_)) => true,
            // 4. Return false.
            _ => false,
        }
    }

    /// 6.2.6.2 IsDataDescriptor ( Desc )
    /// https://tc39.es/ecma262/#sec-isdatadescriptor
    pub fn is_data_descriptor(&self) -> bool {
        // 1. If Desc is undefined, return false.
        match (self.value, self.writable) {
            // 2. If Desc has a [[Value]] field, return true.
            (Some(_), _) => true,
            // 3. If Desc has a [[Writable]] field, return true.
            (_, Some(_)) => true,
            // 4. Return false.
            _ => false,
        }
    }

    /// 6.2.6.3 IsGenericDescriptor ( Desc )
    /// https://tc39.es/ecma262/#sec-isgenericdescriptor
    pub fn is_generic_descriptor(&self) -> bool {
        // 1. If Desc is undefined, return false.
        // 2. If IsAccessorDescriptor(Desc) is true, return false.
        // 3. If IsDataDescriptor(Desc) is true, return false.
        // 4. Return true.
        !self.is_accessor_descriptor() && !self.is_data_descriptor()
    }

    /// 6.2.6.4 FromPropertyDescriptor ( Desc )
    /// https://tc39.es/ecma262/#sec-frompropertydescriptor
    pub fn from_property_descriptor(&self, agent: &mut Agent) -> JsResult<Object> {
        let realm = agent.current_realm();
        let realm = realm.borrow_mut();

        // 1. If Desc is undefined, return undefined.

        // 2. Let obj be OrdinaryObjectCreate(%Object.prototype%).
        // 3. Assert: obj is an extensible ordinary object with no own properties.

        // 4. If Desc has a [[Value]] field, then
        // a. Perform ! CreateDataPropertyOrThrow(obj, "value", Desc.[[Value]]).

        // 5. If Desc has a [[Writable]] field, then

        // 6. If Desc has a [[Get]] field, then
        // a. Perform ! CreateDataPropertyOrThrow(obj, "get", Desc.[[Get]]).
        // 7. If Desc has a [[Set]] field, then
        // a. Perform ! CreateDataPropertyOrThrow(obj, "set", Desc.[[Set]]).
        // 8. If Desc has an [[Enumerable]] field, then
        // a. Perform ! CreateDataPropertyOrThrow(obj, "enumerable", Desc.[[Enumerable]]).

        // 9. If Desc has a [[Configurable]] field, then
        // a. Perform ! CreateDataPropertyOrThrow(obj, "configurable", Desc.[[Configurable]]).
        // 10. Return obj.
        todo!()
    }
}
