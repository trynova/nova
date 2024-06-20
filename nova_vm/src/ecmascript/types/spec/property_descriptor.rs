use crate::ecmascript::{
    abstract_operations::{
        operations_on_objects::{get, has_property},
        testing_and_comparison::is_callable,
        type_conversion::to_boolean,
    },
    execution::{agent::ExceptionType, Agent, JsResult},
    types::{Function, Object, Value, BUILTIN_STRING_MEMORY},
};

/// ### [6.2.6 The Property Descriptor Specification Type](https://tc39.es/ecma262/#sec-property-descriptor-specification-type)
#[derive(Debug, Clone, Default)]
pub struct PropertyDescriptor {
    /// \[\[Value]]
    pub value: Option<Value>,

    /// \[\[Writable]]
    pub writable: Option<bool>,

    /// \[\[Get]]
    pub get: Option<Function>,

    /// \[\[Set]]
    pub set: Option<Function>,

    /// \[\[Enumerable]]
    pub enumerable: Option<bool>,

    /// \[\[Configurable]]
    pub configurable: Option<bool>,
}

impl PropertyDescriptor {
    /// ### [6.2.6.1 IsAccessorDescriptor ( Desc )](https://tc39.es/ecma262/#sec-isaccessordescriptor)
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

    /// ### [6.2.6.2 IsDataDescriptor ( Desc )](https://tc39.es/ecma262/#sec-isdatadescriptor)
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

    /// ### [6.2.6.3 IsGenericDescriptor ( Desc )](https://tc39.es/ecma262/#sec-isgenericdescriptor)
    pub fn is_generic_descriptor(&self) -> bool {
        // 1. If Desc is undefined, return false.
        // 2. If IsAccessorDescriptor(Desc) is true, return false.
        // 3. If IsDataDescriptor(Desc) is true, return false.
        // 4. Return true.
        !self.is_accessor_descriptor() && !self.is_data_descriptor()
    }

    /// ### [6.2.6.4 FromPropertyDescriptor ( Desc )](https://tc39.es/ecma262/#sec-frompropertydescriptor)
    pub fn from_property_descriptor(&self, agent: &mut Agent) -> JsResult<Object> {
        let _realm = agent.current_realm();

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

    /// ### [6.2.6.5 ToPropertyDescriptor ( Obj )](https://tc39.es/ecma262/#sec-topropertydescriptor)
    ///
    /// The abstract operation ToPropertyDescriptor takes argument Obj (an
    /// ECMAScript language value) and returns either a normal completion
    /// containing a Property Descriptor or a throw completion.
    pub fn to_property_descriptor(agent: &mut Agent, obj: Value) -> JsResult<Self> {
        // 1. If Obj is not an Object, throw a TypeError exception.
        let Ok(obj) = Object::try_from(obj) else {
            return Err(
                agent.throw_exception(ExceptionType::TypeError, "Argument is not an object")
            );
        };
        // 2. Let desc be a new Property Descriptor that initially has no
        // fields.
        let mut desc = PropertyDescriptor::default();
        // 3. Let hasEnumerable be ? HasProperty(Obj, "enumerable").
        let has_enumerable = has_property(agent, obj, BUILTIN_STRING_MEMORY.enumerable.into())?;
        // 4. If hasEnumerable is true, then
        if has_enumerable {
            // a. Let enumerable be ToBoolean(? Get(Obj, "enumerable")).
            let enumerable = get(agent, obj, BUILTIN_STRING_MEMORY.enumerable.into())?;
            let enumerable = to_boolean(agent, enumerable);
            // b. Set desc.[[Enumerable]] to enumerable.
            desc.enumerable = Some(enumerable);
        }
        // 5. Let hasConfigurable be ? HasProperty(Obj, "configurable").
        let has_configurable = has_property(agent, obj, BUILTIN_STRING_MEMORY.configurable.into())?;
        // 6. If hasConfigurable is true, then
        if has_configurable {
            // a. Let configurable be ToBoolean(? Get(Obj, "configurable")).
            let configurable = get(agent, obj, BUILTIN_STRING_MEMORY.configurable.into())?;
            let configurable = to_boolean(agent, configurable);
            // b. Set desc.[[Configurable]] to configurable.
            desc.configurable = Some(configurable);
        }
        // 7. Let hasValue be ? HasProperty(Obj, "value").
        let has_value = has_property(agent, obj, BUILTIN_STRING_MEMORY.value.into())?;
        // 8. If hasValue is true, then
        if has_value {
            // a. Let value be ? Get(Obj, "value").
            let value = get(agent, obj, BUILTIN_STRING_MEMORY.value.into())?;
            // b. Set desc.[[Value]] to value.
            desc.value = Some(value);
        }
        // 9. Let hasWritable be ? HasProperty(Obj, "writable").
        let has_writable = has_property(agent, obj, BUILTIN_STRING_MEMORY.writable.into())?;
        // 10. If hasWritable is true, then
        if has_writable {
            // a. Let writable be ToBoolean(? Get(Obj, "writable")).
            let writable = get(agent, obj, BUILTIN_STRING_MEMORY.writable.into())?;
            let writable = to_boolean(agent, writable);
            // b. Set desc.[[Writable]] to writable.
            desc.writable = Some(writable);
        }
        // 11. Let hasGet be ? HasProperty(Obj, "get").
        let has_get = has_property(agent, obj, BUILTIN_STRING_MEMORY.get.into())?;
        // 12. If hasGet is true, then
        if has_get {
            // a. Let getter be ? Get(Obj, "get").
            let getter = get(agent, obj, BUILTIN_STRING_MEMORY.get.into())?;
            // b. If IsCallable(getter) is false and getter is not undefined,
            // throw a TypeError exception.
            if !is_callable(getter) && !getter.is_undefined() {
                return Err(
                    agent.throw_exception(ExceptionType::TypeError, "getter is not callable")
                );
            }
            // c. Set desc.[[Get]] to getter.
            desc.get = Some(Function::try_from(getter).unwrap());
        }
        // 13. Let hasSet be ? HasProperty(Obj, "set").
        let has_set = has_property(agent, obj, BUILTIN_STRING_MEMORY.set.into())?;
        // 14. If hasSet is true, then
        if has_set {
            // a. Let setter be ? Get(Obj, "set").
            let setter = get(agent, obj, BUILTIN_STRING_MEMORY.set.into())?;
            // b. If IsCallable(setter) is false and setter is not undefined,
            // throw a TypeError exception.
            if !is_callable(setter) && !setter.is_undefined() {
                return Err(
                    agent.throw_exception(ExceptionType::TypeError, "setter is not callable")
                );
            }
            // c. Set desc.[[Set]] to setter.
            desc.set = Some(Function::try_from(setter).unwrap());
        }
        // 15. If desc has a [[Get]] field or desc has a [[Set]] field, then
        if desc.get.is_some() || desc.set.is_some() {
            // a. If desc has a [[Value]] field or desc has a [[Writable]]
            // field, throw a TypeError exception.
            if desc.writable.is_some() || desc.writable.is_some() {
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    "Over-defined property descriptor",
                ));
            }
        }
        // 16. Return desc.
        Ok(desc)
    }

    pub fn is_fully_populated(&self) -> bool {
        ((self.value.is_some() && self.writable.is_some())
            || (self.get.is_some() && self.set.is_some()))
            && self.enumerable.is_some()
            && self.configurable.is_some()
    }

    pub fn has_fields(&self) -> bool {
        self.value.is_some()
            || self.writable.is_some()
            || self.get.is_some()
            || self.set.is_some()
            || self.enumerable.is_some()
            || self.configurable.is_some()
    }
}
