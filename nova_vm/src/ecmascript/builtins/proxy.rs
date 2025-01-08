// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use abstract_operations::{validate_non_revoked_proxy, NonRevokedProxy};
use data::ProxyHeapData;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                call, call_function, create_array_from_list, get_object_method,
                try_get_object_method,
            },
            testing_and_comparison::{is_extensible, same_value},
            type_conversion::to_boolean,
        },
        builtins::ArgumentsList,
        execution::{agent::ExceptionType, Agent, JsResult},
        types::{
            Function, InternalMethods, InternalSlots, IntoObject, IntoValue, Object,
            OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value, BUILTIN_STRING_MEMORY,
        },
    },
    engine::{
        context::{GcScope, NoGcScope},
        TryResult,
    },
    heap::{
        indexes::{BaseIndex, ProxyIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

use super::ordinary::is_compatible_property_descriptor;

pub(crate) mod abstract_operations;
pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Proxy(pub(crate) ProxyIndex);

impl Proxy {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<Proxy> for ProxyIndex {
    fn from(val: Proxy) -> Self {
        val.0
    }
}

impl From<ProxyIndex> for Proxy {
    fn from(value: ProxyIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for Proxy {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Proxy {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Proxy> for Value {
    fn from(val: Proxy) -> Self {
        Value::Proxy(val)
    }
}

impl From<Proxy> for Object {
    fn from(val: Proxy) -> Self {
        Object::Proxy(val)
    }
}

impl InternalSlots for Proxy {
    #[inline(always)]
    fn get_backing_object(self, _agent: &Agent) -> Option<OrdinaryObject<'static>> {
        todo!()
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        todo!()
    }

    fn create_backing_object(self, _agent: &mut Agent) -> OrdinaryObject<'static> {
        todo!()
    }

    fn internal_extensible(self, _agent: &Agent) -> bool {
        todo!();
    }

    fn internal_set_extensible(self, _agent: &mut Agent, _value: bool) {
        todo!();
    }

    fn internal_prototype(self, _agent: &Agent) -> Option<Object> {
        todo!();
    }

    fn internal_set_prototype(self, _agent: &mut Agent, _prototype: Option<Object>) {
        todo!();
    }
}

impl InternalMethods for Proxy {
    fn try_get_prototype_of(
        self,
        _: &mut Agent,
        _: NoGcScope<'_, '_>,
    ) -> TryResult<Option<Object>> {
        TryResult::Break(())
    }

    fn internal_get_prototype_of(
        self,
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<Option<Object>> {
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;

        // 5. Let trap be ? GetMethod(handler, "getPrototypeOf").
        let target = target.scope(agent, gc.nogc());
        let trap = if let TryResult::Continue(trap) = try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.getPrototypeOf.into(),
            gc.nogc(),
        ) {
            trap?
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.getPrototypeOf.into(),
                gc.reborrow(),
            )?
            .map(Function::unbind);
            let trap = trap.map(|t| t.bind(gc.nogc()));
            handler = scoped_handler.get(agent).bind(gc.nogc());
            trap
        };

        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[GetPrototypeOf]]().
            return target.get(agent).internal_get_prototype_of(agent, gc);
        };

        // 7. Let handlerProto be ? Call(trap, handler, « target »).
        let handler_proto = call_function(
            agent,
            trap.unbind(),
            handler.into(),
            Some(ArgumentsList(&[target.get(agent).into()])),
            gc.reborrow(),
        )?;

        // 8. If handlerProto is not an Object and handlerProto is not null, throw a TypeError exception.
        let handler_proto = if handler_proto.is_null() {
            None
        } else if let Ok(handler_proto) = Object::try_from(handler_proto) {
            Some(handler_proto)
        } else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Handler prototype must be an object or null",
                gc.nogc(),
            ));
        };

        // 9. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target = is_extensible(agent, target.get(agent), gc.reborrow())?;

        // 10. If extensibleTarget is true, return handlerProto.
        if extensible_target {
            return Ok(handler_proto);
        }

        // 11. Let targetProto be ? target.[[GetPrototypeOf]]().
        let target_proto = target
            .get(agent)
            .internal_get_prototype_of(agent, gc.reborrow())?;

        // 12. If SameValue(handlerProto, targetProto) is false, throw a TypeError exception.
        if handler_proto != target_proto {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "handlerProto and targetProto are not the same value",
                gc.nogc(),
            ));
        }

        // 13. Return handlerProto.
        Ok(handler_proto)
    }

    fn try_set_prototype_of(
        self,
        _: &mut Agent,
        _: Option<Object>,
        _: NoGcScope,
    ) -> TryResult<bool> {
        TryResult::Break(())
    }

    /// ### 0.5.2 [[[SetPrototypeOf]] ( V )](https://tc39.es/ecma262/#sec-proxy-object-internal-methods-and-internal-slots-setprototypeof-v)
    ///
    /// The [[SetPrototypeOf]] internal method of a Proxy exotic object O takes
    /// argument V (an Object or null) and returns either a normal completion
    /// containing a Boolean or a throw completion.
    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;
        // 5. Let trap be ? GetMethod(handler, "setPrototypeOf").
        let trap = if let TryResult::Continue(trap) = try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.setPrototypeOf.into(),
            gc.nogc(),
        ) {
            trap?
        } else {
            let scoped_target = target.scope(agent, gc.nogc());
            let scoped_handler = handler.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.setPrototypeOf.into(),
                gc.reborrow(),
            )?
            .map(Function::unbind);
            let gc = gc.nogc();
            let trap = trap.map(|t| t.bind(gc));
            target = scoped_target.get(agent).bind(gc);
            handler = scoped_handler.get(agent).bind(gc);
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[SetPrototypeOf]](V).
            return target.internal_set_prototype_of(agent, prototype, gc);
        };
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, V »)).
        let argument = call_function(
            agent,
            trap.unbind(),
            handler.into_value(),
            Some(ArgumentsList(&[target.into(), prototype.into()])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);
        // 8. If booleanTrapResult is false, return false.
        if !boolean_trap_result {
            return Ok(false);
        }
        // 9. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target = is_extensible(agent, target, gc.reborrow())?;
        // 10. If extensibleTarget is true, return true.
        if extensible_target {
            return Ok(true);
        }
        // 11. Let targetProto be ? target.[[GetPrototypeOf]]().
        let target_proto = target.internal_get_prototype_of(agent, gc.reborrow())?;
        // 12. If SameValue(V, targetProto) is false, throw a TypeError exception.
        if prototype != target_proto {
            return  Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'setPrototypeOf' on proxy: trap returned truish for setting a new prototype on the non-extensible proxy target",
                gc.nogc(),
            ));
        }
        // 13. Return true.
        Ok(true)
    }

    fn try_is_extensible(self, _: &mut Agent, _: NoGcScope) -> TryResult<bool> {
        TryResult::Break(())
    }

    fn internal_is_extensible(self, agent: &mut Agent, mut gc: GcScope<'_, '_>) -> JsResult<bool> {
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;

        // 5. Let trap be ? GetMethod(handler, "isExtensible").
        let target = target.scope(agent, gc.nogc());
        let trap = if let TryResult::Continue(trap) = try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.isExtensible.into(),
            gc.nogc(),
        ) {
            trap?
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.isExtensible.into(),
                gc.reborrow(),
            )?
            .map(Function::unbind);
            let trap = trap.map(|t| t.bind(gc.nogc()));
            handler = scoped_handler.get(agent).bind(gc.nogc());
            trap
        };

        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? IsExtensible(target).
            return is_extensible(agent, target.get(agent), gc.reborrow());
        };

        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target »)).
        let argument = call_function(
            agent,
            trap.unbind(),
            handler.into_value(),
            Some(ArgumentsList(&[target.get(agent).into()])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);

        // 8. Let targetResult be ? IsExtensible(target).
        let target_result = is_extensible(agent, target.get(agent), gc.reborrow())?;

        // 9. If booleanTrapResult is not targetResult, throw a TypeError exception.
        if boolean_trap_result != target_result {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "proxy must report same extensiblitity as target",
                gc.nogc(),
            ));
        };

        // 10. Return booleanTrapResult.
        Ok(boolean_trap_result)
    }

    fn try_prevent_extensions(self, _: &mut Agent, _: NoGcScope) -> TryResult<bool> {
        TryResult::Break(())
    }

    fn internal_prevent_extensions(
        self,
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;

        // 5. Let trap be ? GetMethod(handler, "preventExtensions").
        let target = target.scope(agent, gc.nogc());
        let trap = if let TryResult::Continue(trap) = try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.preventExtensions.into(),
            gc.nogc(),
        ) {
            trap?
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.preventExtensions.into(),
                gc.reborrow(),
            )?
            .map(Function::unbind);
            let trap = trap.map(|f| f.bind(gc.nogc()));
            handler = scoped_handler.get(agent).bind(gc.nogc());
            trap
        };

        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[PreventExtensions]]().
            return target.get(agent).internal_prevent_extensions(agent, gc);
        };

        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target »)).
        let argument = call_function(
            agent,
            trap.unbind(),
            handler.into_value(),
            Some(ArgumentsList(&[target.get(agent).into()])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);

        // 8. If booleanTrapResult is true, then

        if boolean_trap_result {
            // a. Let extensibleTarget be ? IsExtensible(target).
            let extensible_target = is_extensible(agent, target.get(agent), gc.reborrow())?;

            // b. If extensibleTarget is true, throw a TypeError exception.
            if extensible_target {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "proxy can't report an extensible object as non-extensible",
                    gc.nogc(),
                ));
            }
        };

        // 9. Return booleanTrapResult.
        Ok(boolean_trap_result)
    }

    fn try_get_own_property(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: NoGcScope,
    ) -> TryResult<Option<PropertyDescriptor>> {
        TryResult::Break(())
    }

    /// ### 10.5.5 [[[GetOwnProperty]] ( P )](https://tc39.es/ecma262/#sec-proxy-object-internal-methods-and-internal-slots-getownproperty-p)
    ///
    /// The [[GetOwnProperty]] internal method of a Proxy exotic object O takes
    /// argument P (a property key) and returns either a normal completion
    /// containing either a Property Descriptor or undefined, or a throw completion.
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<Option<PropertyDescriptor>> {
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;
        // 5. Let trap be ? GetMethod(handler, "getOwnPropertyDescriptor").
        let trap = if let TryResult::Continue(trap) = try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.getOwnPropertyDescriptor.into(),
            gc.nogc(),
        ) {
            trap?
        } else {
            let scoped_target = target.scope(agent, gc.nogc());
            let scoped_handler = handler.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.getOwnPropertyDescriptor.into(),
                gc.reborrow(),
            )?
            .map(Function::unbind);
            let gc = gc.nogc();
            let trap = trap.map(|f| f.bind(gc));
            handler = scoped_handler.get(agent).bind(gc);
            target = scoped_target.get(agent).bind(gc);
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[GetOwnProperty]](P).
            return target.internal_get_own_property(agent, property_key, gc);
        };
        // 7. Let trapResultObj be ? Call(trap, handler, « target, P »).
        let trap_result_obj = call_function(
            agent,
            trap.unbind(),
            handler.into_value(),
            Some(ArgumentsList(&[target.into(), property_key.into()])),
            gc.reborrow(),
        )?;
        // 8. If trapResultObj is not an Object and trapResultObj is not undefined, throw a TypeError exception.
        if !trap_result_obj.is_object() && !trap_result_obj.is_undefined() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "proxy [[GetOwnProperty]] must return an object or undefined",
                gc.nogc(),
            ));
        };
        // 9. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let target_desc = target.internal_get_own_property(agent, property_key, gc.reborrow())?;
        // 10. If trapResultObj is undefined, then
        if trap_result_obj.is_undefined() {
            if let Some(target_desc) = target_desc {
                // b. If targetDesc.[[Configurable]] is false, throw a TypeError exception.
                if target_desc.configurable == Some(false) {
                    return Err(agent.throw_exception(
                        ExceptionType::TypeError,
                        format!(
                            "proxy can't report a non-configurable own property '{}' as non-existent.",
                            property_key.as_display(agent)
                        ),
                        gc.nogc(),
                    ));
                }
            } else {
                // a. If targetDesc is undefined, return undefined.
                return Ok(None);
            }
            // c. Let extensibleTarget be ? IsExtensible(target).
            let extensible_target = is_extensible(agent, target, gc.reborrow())?;
            // d. If extensibleTarget is false, throw a TypeError exception.
            if !extensible_target {
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    format!(
                        "proxy can't report a extensibleTarget own property '{}' as non-existent.",
                        property_key.as_display(agent)
                    ),
                    gc.nogc(),
                ));
            };
            // e. Return undefined.
            return Ok(None);
        };
        // 11. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target = is_extensible(agent, target, gc.reborrow())?;
        // 12. Let resultDesc be ? ToPropertyDescriptor(trapResultObj).
        let result_desc = if let TryResult::Continue(result_desc) =
            PropertyDescriptor::try_to_property_descriptor(agent, trap_result_obj, gc.nogc())
        {
            result_desc?
        } else {
            PropertyDescriptor::to_property_descriptor(agent, trap_result_obj, gc.reborrow())?
        };
        // 13. Perform CompletePropertyDescriptor(resultDesc).
        PropertyDescriptor::complete_property_descriptor(result_desc.clone(), agent, gc.nogc())?;
        // 14. Let valid be IsCompatiblePropertyDescriptor(extensibleTarget, resultDesc, targetDesc).
        let valid = is_compatible_property_descriptor(
            agent,
            extensible_target,
            result_desc.clone(),
            target_desc.clone(),
            gc.nogc(),
        );
        // 15. If valid is false, throw a TypeError exception.
        if !valid {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "valid",
                gc.nogc(),
            ));
        };
        // 16. If resultDesc.[[Configurable]] is false, then
        if result_desc.configurable == Some(false) {
            // b. If resultDesc has a [[Writable]] field and resultDesc.[[Writable]] is false, then
            if let Some(false) = result_desc.writable {
                // i. Assert: targetDesc has a [[Writable]] field.
                // ii. If targetDesc.[[Writable]] is true, throw a TypeError exception.
                if let Some(target_desc) = target_desc {
                    if target_desc.writable == Some(true) {
                        return  Err(agent.throw_exception(
                                ExceptionType::TypeError,
                                format!(
                                    "proxy can't report existing writable property '{}' as non-writable",
                                    property_key.as_display(agent)
                                ),
                                gc.nogc()
                        ));
                    }
                }
            }
            // a. If targetDesc is undefined or targetDesc.[[Configurable]] is true, then
            //   i. Throw a TypeError exception.
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                format!(
                    "proxy can't report a non-existent property '{}' as non-configurable",
                    property_key.as_display(agent)
                ),
                gc.nogc(),
            ));
        };
        // 17. Return resultDesc.
        Ok(Some(result_desc))
    }

    fn try_define_own_property(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: PropertyDescriptor,
        _: NoGcScope,
    ) -> TryResult<bool> {
        TryResult::Break(())
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;
        // 5. Let trap be ? GetMethod(handler, "defineProperty").
        let trap = if let TryResult::Continue(trap) = try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.defineProperty.into(),
            gc.nogc(),
        ) {
            trap?
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let scoped_target = target.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.defineProperty.into(),
                gc.reborrow(),
            )?
            .map(Function::unbind);
            let gc = gc.nogc();
            let trap = trap.map(|f| f.bind(gc));
            handler = scoped_handler.get(agent).bind(gc);
            target = scoped_target.get(agent).bind(gc);
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[DefineOwnProperty]](P, Desc).
            return target.internal_define_own_property(
                agent,
                property_key.unbind(),
                property_descriptor,
                gc,
            );
        };
        // 7. Let descObj be FromPropertyDescriptor(Desc).
        let desc_obj = PropertyDescriptor::from_property_descriptor(
            property_descriptor.clone().into(),
            agent,
            gc.nogc(),
        );
        // 8. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, P, descObj »)).
        let p = property_key.convert_to_value(agent, gc.nogc());
        let argument = call_function(
            agent,
            trap.unbind(),
            handler.into_value(),
            Some(ArgumentsList(&[target.into_value(), p, desc_obj.into()])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);
        // 9. If booleanTrapResult is false, return false.
        if !boolean_trap_result {
            return Ok(false);
        };
        // 10. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let target_desc = target.internal_get_own_property(agent, property_key, gc.reborrow())?;
        // 11. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target = is_extensible(agent, target, gc.reborrow())?;
        // 12. If Desc has a [[Configurable]] field and Desc.[[Configurable]] is false, then
        let setting_config_false = property_descriptor.configurable == Some(false);
        // 14. If targetDesc is undefined, then
        let gc = gc.into_nogc();
        if target_desc.is_none() {
            // a. If extensibleTarget is false, throw a TypeError exception.
            if !extensible_target {
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    format!(
                        "proxy can't define a new property '{}' on a non-extensible object",
                        property_key.as_display(agent)
                    ),
                    gc,
                ));
            }
            // b. If settingConfigFalse is true, throw a TypeError exception.
            if setting_config_false {
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    format!(
                        "proxy can't define a non-existent '{}' property as non-configurable",
                        property_key.as_display(agent)
                    ),
                    gc,
                ));
            }
        } else {
            // 15. Else,
            // a. If IsCompatiblePropertyDescriptor(extensibleTarget, Desc, targetDesc) is false, throw a TypeError exception.
            if !is_compatible_property_descriptor(
                agent,
                extensible_target,
                property_descriptor.clone(),
                target_desc.clone(),
                gc,
            ) {
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    format!(
                        "proxy can't define an incompatible property descriptor ('{}', proxy can't report an existing non-configurable property as configurable)",
                        property_key.as_display(agent)
                    ),
                    gc,
                ));
            };
            // b. If settingConfigFalse is true and targetDesc.[[Configurable]] is true, throw a TypeError exception.
            if setting_config_false {
                if let Some(ref target_desc) = target_desc {
                    if target_desc.configurable == Some(true) {
                        return Err(agent.throw_exception(
                            ExceptionType::TypeError,
                            format!(
                                "proxy can't define an incompatible property descriptor ('{}', proxy can't define an existing configurable property as non-configurable)",
                                property_key.as_display(agent)
                            ),
                            gc,
                        ));
                    }
                }
            }
            // c. If IsDataDescriptor(targetDesc) is true, targetDesc.[[Configurable]] is false, and targetDesc.[[Writable]] is true, then
            if let Some(target_desc) = target_desc {
                if property_descriptor.is_data_descriptor()
                    && target_desc.configurable == Some(false)
                    && target_desc.writable == Some(true)
                {
                    //  i. If Desc has a [[Writable]] field and Desc.[[Writable]] is false, throw a TypeError exception.
                    if let Some(writable) = property_descriptor.writable {
                        if !writable {
                            return Err(agent.throw_exception(
                                ExceptionType::TypeError,
                                format!(
                                    "proxy can't define an incompatible property descriptor ('{}', proxy can't define an existing non-configurable writable property as non-writable)",
                                    property_key.as_display(agent)
                                ),
                                gc,
                            ));
                        }
                    }
                }
            }
        };
        // 16. Return true.
        Ok(true)
    }

    fn try_has_property(self, _: &mut Agent, _: PropertyKey, _: NoGcScope) -> TryResult<bool> {
        TryResult::Break(())
    }

    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        let mut property_key = property_key.bind(gc.nogc());

        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;

        // 5. Let trap be ? GetMethod(handler, "has").
        let scoped_target = target.scope(agent, gc.nogc());
        let scoped_property_key = property_key.scope(agent, gc.nogc());
        let trap = if let TryResult::Continue(trap) =
            try_get_object_method(agent, handler, BUILTIN_STRING_MEMORY.has.into(), gc.nogc())
        {
            trap?
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.has.into(),
                gc.reborrow(),
            )?
            .map(Function::unbind);
            let gc = gc.nogc();
            let trap = trap.map(|f| f.bind(gc));
            handler = scoped_handler.get(agent).bind(gc);
            target = scoped_target.get(agent).bind(gc);
            property_key = scoped_property_key.get(agent).bind(gc);
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // Return ? target.[[HasProperty]](P).
            return target.internal_has_property(agent, property_key.unbind(), gc);
        };

        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, P »)).
        let p = property_key.convert_to_value(agent, gc.nogc());
        let argument = call_function(
            agent,
            trap.unbind(),
            handler.into_value(),
            Some(ArgumentsList(&[target.into_value(), p])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);

        // 8. If booleanTrapResult is false, then
        if !boolean_trap_result {
            // a. Let targetDesc be ? target.[[GetOwnProperty]](P).
            let target = scoped_target.get(agent).bind(gc.nogc());
            let property_key = scoped_property_key.get(agent).bind(gc.nogc());
            let target_desc =
                target.internal_get_own_property(agent, property_key.unbind(), gc.reborrow())?;
            //    b. If targetDesc is not undefined, then
            if let Some(target_desc) = target_desc {
                //  i. If targetDesc.[[Configurable]] is false, throw a TypeError exception.
                if target_desc.configurable == Some(false) {
                    let property_key = scoped_property_key.get(agent).bind(gc.nogc());
                    let message = String::from_string(
                        agent,
                        format!("proxy can't report a non-configurable own property '{}' as non-existent", property_key.as_display(agent)),
                        gc.into_nogc(),
                    );
                    return Err(
                        agent.throw_exception_with_message(ExceptionType::TypeError, message)
                    );
                }
                // ii. Let extensibleTarget be ? IsExtensible(target).
                // iii. If extensibleTarget is false, throw a TypeError exception.
                if !is_extensible(agent, scoped_target.get(agent), gc.reborrow())? {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "proxy can't report an extensible object as non-extensible",
                        gc.into_nogc(),
                    ));
                }
            }
        };

        // 9. Return booleanTrapResult.
        Ok(boolean_trap_result)
    }

    fn try_get(self, _: &mut Agent, _: PropertyKey, _: Value, _: NoGcScope) -> TryResult<Value> {
        TryResult::Break(())
    }

    /// ### [10.5.8 [[Get]] ( P, Receiver )](https://tc39.es/ecma262/#sec-proxy-object-internal-methods-and-internal-slots-get-p-receiver)
    ///
    /// The \[\[Get]] internal method of a Proxy exotic object O takes
    /// arguments P (a property key) and Receiver (an ECMAScript language
    /// value) and returns either a normal completion containing an ECMAScript
    /// language value or a throw completion.
    ///
    /// > #### Note
    /// > \[\[Get]] for Proxy objects enforces the following invariants:
    /// >
    /// > The value reported for a property must be the same as the value of
    /// > the corresponding target object property if the target object
    /// > property is a non-writable, non-configurable own data property.
    /// > The value reported for a property must be undefined if the
    /// > corresponding target object property is a non-configurable own
    /// > accessor property that has undefined as its \[\[Get]] attribute.
    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        mut receiver: Value,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        let mut property_key = property_key.bind(gc.nogc());
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;
        // 5. Let trap be ? GetMethod(handler, "get").
        let scoped_target = target.scope(agent, gc.nogc());
        let scoped_property_key = property_key.scope(agent, gc.nogc());
        let trap = if let TryResult::Continue(trap) =
            try_get_object_method(agent, handler, BUILTIN_STRING_MEMORY.get.into(), gc.nogc())
        {
            trap?
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let scoped_receiver = receiver.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.get.into(),
                gc.reborrow(),
            )?
            .map(Function::unbind);
            let gc = gc.nogc();
            let trap = trap.map(|f| f.bind(gc));
            handler = scoped_handler.get(agent).bind(gc);
            receiver = scoped_receiver.get(agent).bind(gc);
            target = scoped_target.get(agent).bind(gc);
            property_key = scoped_property_key.get(agent).bind(gc);
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[Get]](P, Receiver).
            return target.internal_get(agent, property_key.unbind(), receiver, gc);
        };
        // 7. Let trapResult be ? Call(trap, handler, « target, P, Receiver »).
        let p = property_key.convert_to_value(agent, gc.nogc());
        let trap_result = call_function(
            agent,
            trap.unbind(),
            handler.into_value(),
            Some(ArgumentsList(&[target.into_value(), p, receiver])),
            gc.reborrow(),
        )?;
        // 8. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let target = scoped_target.get(agent).bind(gc.nogc());
        let property_key = scoped_property_key.get(agent).bind(gc.nogc());
        let target_desc =
            target.internal_get_own_property(agent, property_key.unbind(), gc.reborrow())?;
        // 9. If targetDesc is not undefined and targetDesc.[[Configurable]] is false, then
        if let Some(target_desc) = target_desc {
            if target_desc.configurable == Some(false) {
                // a. If IsDataDescriptor(targetDesc) is true and
                //    targetDesc.[[Writable]] is false, then
                // i. If SameValue(trapResult, targetDesc.[[Value]]) is false,
                //    throw a TypeError exception.
                // b. If IsAccessorDescriptor(targetDesc) is true and
                //    targetDesc.[[Get]] is undefined, then
                // i. If trapResult is not undefined, throw a TypeError exception.
                if target_desc.is_data_descriptor()
                    && target_desc.writable == Some(false)
                    && !same_value(
                        agent,
                        trap_result,
                        target_desc.value.unwrap_or(Value::Undefined),
                    )
                    || target_desc.is_accessor_descriptor()
                        && target_desc.get.is_none()
                        && trap_result.is_undefined()
                {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "Invalid Proxy [[Get]] method",
                        gc.into_nogc(),
                    ));
                }
            }
        }
        // 10. Return trapResult.
        Ok(trap_result)
    }

    fn try_set(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: Value,
        _: Value,
        _: NoGcScope,
    ) -> TryResult<bool> {
        TryResult::Break(())
    }

    /// ### [10.5.9 [[Set]] ( P, V, Receiver )](https://tc39.es/ecma262/#sec-proxy-object-internal-methods-and-internal-slots-set-p-v-receiver)
    ///
    /// The [[Set]] internal method of a Proxy exotic object O takes
    /// arguments P (a property key), V (an ECMAScript language
    /// value), and Receiver (an ECMAScript language value) and returns either a normal completion containing
    /// a Boolean or a throw completion.
    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        mut receiver: Value,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        let mut property_key = property_key.bind(gc.nogc());
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;
        // 5. Let trap be ? GetMethod(handler, "set").
        let scoped_target = target.scope(agent, gc.nogc());
        let scoped_property_key = property_key.scope(agent, gc.nogc());
        let trap = if let TryResult::Continue(trap) =
            try_get_object_method(agent, handler, BUILTIN_STRING_MEMORY.set.into(), gc.nogc())
        {
            trap?
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let scoped_receiver = receiver.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.set.into(),
                gc.reborrow(),
            )?
            .map(Function::unbind);
            let gc = gc.nogc();
            let trap = trap.map(|f| f.bind(gc));
            handler = scoped_handler.get(agent).bind(gc);
            receiver = scoped_receiver.get(agent).bind(gc);
            target = scoped_target.get(agent).bind(gc);
            property_key = scoped_property_key.get(agent).bind(gc);
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[Set]](P, V, Receiver).
            return target.internal_set(agent, property_key.unbind(), value, receiver, gc);
        };
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, P, V, Receiver »)).
        let p = property_key.convert_to_value(agent, gc.nogc());
        let argument = call_function(
            agent,
            trap.unbind(),
            handler.into_value(),
            Some(ArgumentsList(&[target.into_value(), p, value, receiver])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);

        // 8. If booleanTrapResult is false, return false.
        if !boolean_trap_result {
            return Ok(false);
        };

        // 9. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let target = scoped_target.get(agent).bind(gc.nogc());
        let property_key = scoped_property_key.get(agent).bind(gc.nogc());
        let target_desc =
            target.internal_get_own_property(agent, property_key.unbind(), gc.reborrow())?;
        // 10. If targetDesc is not undefined and targetDesc.[[Configurable]] is false, then
        if let Some(target_desc) = target_desc {
            if target_desc.configurable == Some(false) {
                // a. If IsDataDescriptor(targetDesc) is true and
                //    targetDesc.[[Writable]] is false, then
                // i. If SameValue(V, targetDesc.[[Value]]) is false,
                //    throw a TypeError exception.
                // b. If IsAccessorDescriptor(targetDesc) is true and
                //    targetDesc.[[Value]] is undefined, then
                // i. If targetDesc.[[Set]] is undefined,
                //    throw a TypeError exception.
                if target_desc.is_data_descriptor()
                    && target_desc.writable == Some(false)
                    && !same_value(agent, value, target_desc.value.unwrap_or(Value::Undefined))
                    || target_desc.is_accessor_descriptor() && target_desc.set.is_none()
                {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "Invalid Proxy [[Set]] method",
                        gc.into_nogc(),
                    ));
                }
            }
        }
        // 10. Return trapResult.
        Ok(true)
    }

    fn try_delete(self, _: &mut Agent, _: PropertyKey, _: NoGcScope) -> TryResult<bool> {
        TryResult::Break(())
    }

    fn internal_delete(
        self,
        agent: &mut Agent,
        mut property_key: PropertyKey,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;
        // 5. Let trap be ? GetMethod(handler, "deleteProperty").
        let trap = if let TryResult::Continue(trap) = try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.deleteProperty.into(),
            gc.nogc(),
        ) {
            trap?
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let scoped_target = target.scope(agent, gc.nogc());
            let scoped_property_key = property_key.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.deleteProperty.into(),
                gc.reborrow(),
            )?
            .map(Function::unbind);
            let gc = gc.nogc();
            let trap = trap.map(|f| f.bind(gc));
            handler = scoped_handler.get(agent).bind(gc);
            target = scoped_target.get(agent).bind(gc);
            property_key = scoped_property_key.get(agent);
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[Delete]](P).
            return target.internal_delete(agent, property_key, gc);
        };
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, P »)).
        let p = property_key.convert_to_value(agent, gc.nogc());
        let argument = call_function(
            agent,
            trap.unbind(),
            handler.into_value(),
            Some(ArgumentsList(&[target.into_value(), p])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);
        // 8. If booleanTrapResult is false, return false.
        if !boolean_trap_result {
            return Ok(false);
        };
        // 9. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let target_desc = target.internal_get_own_property(agent, property_key, gc.reborrow())?;
        // 10. If targetDesc is undefined, return true.
        let Some(target_desc) = target_desc else {
            return Ok(true);
        };
        // 11. If targetDesc.[[Configurable]] is false, throw a TypeError exception.
        if target_desc.configurable == Some(false) {
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                format!(
                    "property '{}' is non-configurable and can't be deleted",
                    property_key.as_display(agent)
                ),
                gc.into_nogc(),
            ));
        };
        // 12. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target = is_extensible(agent, target, gc.reborrow())?;
        // 13. If extensibleTarget is false, throw a TypeError exception.
        if !extensible_target {
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                format!(
                    "proxy can't delete property '{}' on a non-extensible object",
                    property_key.as_display(agent)
                ),
                gc.into_nogc(),
            ));
        };
        // 14. Return true.
        Ok(true)
    }

    fn try_own_property_keys<'a>(
        self,
        _: &mut Agent,
        _: NoGcScope<'a, '_>,
    ) -> TryResult<Vec<PropertyKey<'a>>> {
        TryResult::Break(())
    }

    fn internal_own_property_keys<'a>(
        self,
        _agent: &mut Agent,
        _gc: GcScope<'a, '_>,
    ) -> JsResult<Vec<PropertyKey<'a>>> {
        todo!();
    }

    /// ### [10.5.12 [[Call]] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-proxy-object-internal-methods-and-internal-slots-call-thisargument-argumentslist)
    ///
    /// The [[Call]] internal method of a Proxy exotic object O takes
    /// arguments thisArgument (an ECMAScript language value)
    /// and argumentsList (a List of ECMAScript language values)
    /// and returns either a normal completion containing an ECMAScript
    /// language value or a throw completion.
    fn internal_call(
        self,
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        let this_argument = arguments.get(1);
        let arguments_list = arguments.get(2);
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;
        // 5. Let trap be ? GetMethod(handler, "apply").
        let trap = if let TryResult::Continue(trap) = try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.apply.into(),
            gc.nogc(),
        ) {
            trap?
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let scoped_target = target.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.apply.into(),
                gc.reborrow(),
            )?
            .map(Function::unbind);
            let gc = gc.nogc();
            let trap = trap.map(|f| f.bind(gc));
            handler = scoped_handler.get(agent).bind(gc);
            target = scoped_target.get(agent).bind(gc);
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? Call(target, thisArgument, argumentsList).
            return call(
                agent,
                target.into_value(),
                this_argument,
                Some(ArgumentsList(&[arguments_list])),
                gc,
            );
        };
        // 7. Let argArray be CreateArrayFromList(argumentsList).
        let arg_array = create_array_from_list(agent, &[arguments_list], gc.nogc());
        // 8. Return ? Call(trap, handler, « target, thisArgument, argArray »).
        return call_function(
            agent,
            trap.unbind(),
            handler.into_value(),
            Some(ArgumentsList(&[
                target.into_value(),
                this_argument,
                arg_array.into_value(),
            ])),
            gc,
        );
    }

    fn internal_construct(
        self,
        _agent: &mut Agent,
        _arguments_list: ArgumentsList,
        _new_target: Function,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Object> {
        todo!()
    }
}

/// ### [10.5.15 ProxyCreate ( target, handler )](https://tc39.es/ecma262/#sec-proxycreate)
///
/// The abstract operation ProxyCreate takes arguments target (an ECMAScript
/// language value) and handler (an ECMAScript language value) and returns
/// either a normal completion containing a Proxy exotic object or a throw
/// completion. It is used to specify the creation of new Proxy objects.
pub(crate) fn proxy_create(
    agent: &mut Agent,
    target: Value,
    handler: Value,
    gc: NoGcScope,
) -> JsResult<Proxy> {
    // 1. If target is not an Object, throw a TypeError exception.
    let Ok(target) = Object::try_from(target) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Proxy target must be an object",
            gc,
        ));
    };
    // 2. If handler is not an Object, throw a TypeError exception.
    let Ok(handler) = Object::try_from(handler) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Proxy handler must be an object",
            gc,
        ));
    };
    // 3. Let P be MakeBasicObject(« [[ProxyHandler]], [[ProxyTarget]] »).
    let p = agent.heap.create(ProxyHeapData {
        proxy_target: Some(target),
        proxy_handler: Some(handler),
    });
    // 4. Set P's essential internal methods, except for [[Call]] and
    // [[Construct]], to the definitions specified in 10.5.
    // 5. If IsCallable(target) is true, then
    // a. Set P.[[Call]] as specified in 10.5.12.
    // b. If IsConstructor(target) is true, then
    // i. Set P.[[Construct]] as specified in 10.5.13.
    // 6. Set P.[[ProxyTarget]] to target.
    // 7. Set P.[[ProxyHandler]] to handler.
    // 8. Return P.
    Ok(p)
}

impl Index<Proxy> for Agent {
    type Output = ProxyHeapData;

    fn index(&self, index: Proxy) -> &Self::Output {
        &self.heap.proxys[index]
    }
}

impl IndexMut<Proxy> for Agent {
    fn index_mut(&mut self, index: Proxy) -> &mut Self::Output {
        &mut self.heap.proxys[index]
    }
}

impl Index<Proxy> for Vec<Option<ProxyHeapData>> {
    type Output = ProxyHeapData;

    fn index(&self, index: Proxy) -> &Self::Output {
        self.get(index.get_index())
            .expect("Proxy out of bounds")
            .as_ref()
            .expect("Proxy slot empty")
    }
}

impl IndexMut<Proxy> for Vec<Option<ProxyHeapData>> {
    fn index_mut(&mut self, index: Proxy) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Proxy out of bounds")
            .as_mut()
            .expect("Proxy slot empty")
    }
}

impl CreateHeapData<ProxyHeapData, Proxy> for Heap {
    fn create(&mut self, data: ProxyHeapData) -> Proxy {
        self.proxys.push(Some(data));
        Proxy(ProxyIndex::last(&self.proxys))
    }
}

impl HeapMarkAndSweep for Proxy {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.proxys.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.proxys.shift_index(&mut self.0);
    }
}
