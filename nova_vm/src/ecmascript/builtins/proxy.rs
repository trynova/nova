// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod abstract_operations;
mod data;

pub(crate) use abstract_operations::*;
pub(crate) use data::*;

use std::collections::VecDeque;

use ahash::AHashSet;

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, ExceptionType, Function, InternalMethods,
        InternalSlots, JsResult, Object, ObjectShape, OrdinaryObject, PropertyDescriptor,
        PropertyKey, PropertyLookupCache, PropertyOffset, SetAtOffsetProps, SetResult, String,
        TryError, TryGetResult, TryHasResult, TryResult, Value, call, call_function, construct,
        create_array_from_list, create_property_key_list_from_array_like, get_object_method,
        is_callable, is_compatible_property_descriptor, is_constructor, is_extensible,
        object_handle, same_value, to_boolean, try_get_object_method, try_result_into_js,
    },
    engine::{Bindable, GcScope, NoGcScope, Scopable, ScopableCollection},
    heap::{
        ArenaAccess, BaseIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        HeapSweepWeakReference, WorkQueues, arena_vec_access,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Proxy<'a>(BaseIndex<'a, ProxyHeapData<'static>>);
object_handle!(Proxy);
arena_vec_access!(Proxy, 'a, ProxyHeapData, proxies);

impl Proxy<'_> {
    pub(crate) fn is_callable(self, agent: &Agent, gc: NoGcScope) -> bool {
        match self.get(agent) {
            ProxyHeapData::NonRevoked { proxy_target, .. } => {
                if let Object::Proxy(proxy_target) = proxy_target {
                    // TODO: Remove this once is_callable handles callable
                    // Proxies.
                    proxy_target.is_callable(agent, gc)
                } else {
                    is_callable(*proxy_target, gc).is_some()
                }
            }
            ProxyHeapData::RevokedCallable => true,
            ProxyHeapData::Revoked => false,
        }
    }
}

impl<'a> InternalSlots<'a> for Proxy<'a> {
    #[inline(always)]
    fn get_backing_object(self, _: &Agent) -> Option<OrdinaryObject<'static>> {
        unreachable!()
    }

    #[inline(always)]
    fn set_backing_object(self, _: &mut Agent, _: OrdinaryObject<'static>) {
        unreachable!()
    }

    #[inline(always)]
    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
        unreachable!()
    }

    #[inline(always)]
    fn internal_extensible(self, _: &Agent) -> bool {
        unreachable!();
    }

    #[inline(always)]
    fn internal_set_extensible(self, _: &mut Agent, _: bool) {
        unreachable!();
    }

    #[inline(always)]
    fn internal_prototype(self, _: &Agent) -> Option<Object<'static>> {
        unreachable!();
    }

    #[inline(always)]
    fn internal_set_prototype(self, _: &mut Agent, _prototype: Option<Object>) {
        unreachable!();
    }

    #[inline(always)]
    fn object_shape(self, _: &mut Agent) -> ObjectShape<'static> {
        unreachable!()
    }
}

impl<'a> InternalMethods<'a> for Proxy<'a> {
    fn try_get_prototype_of<'gc>(
        self,
        _: &mut Agent,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<Object<'gc>>> {
        TryError::GcError.into()
    }

    fn internal_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Object<'gc>>> {
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
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.getPrototypeOf.into(),
            gc.nogc(),
        ))? {
            trap
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.getPrototypeOf.into(),
                gc.reborrow(),
            )?;
            handler = scoped_handler.get(agent).local();
            trap
        };

        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[GetPrototypeOf]]().
            return target
                .get(agent)
                .local()
                .internal_get_prototype_of(agent, gc);
        };

        // 7. Let handlerProto be ? Call(trap, handler, « target »).
        let handler_proto = call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [target
                .get(agent)
                .local()
                .into()])),
            gc.reborrow(),
        )?;

        // 8. If handlerProto is not an Object and handlerProto is not null, throw a TypeError exception.
        let handler_proto = if handler_proto.is_null() {
            None
        } else if let Ok(handler_proto) = Object::try_from(handler_proto) {
            Some(handler_proto.scope(agent, gc.nogc()))
        } else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Handler prototype must be an object or null",
                gc.into_nogc(),
            ));
        };

        // 9. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target = is_extensible(agent, target.get(agent).local(), gc.reborrow())?;

        // 10. If extensibleTarget is true, return handlerProto.
        if extensible_target {
            return Ok(handler_proto.map(|p| p.get(agent).local()));
        }

        // 11. Let targetProto be ? target.[[GetPrototypeOf]]().
        let target_proto = target
            .get(agent)
            .local()
            .internal_get_prototype_of(agent, gc.reborrow())?;

        let handler_proto = handler_proto.map(|p| p.get(agent).local());

        // 12. If SameValue(handlerProto, targetProto) is false, throw a TypeError exception.
        if handler_proto != target_proto {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "handlerProto and targetProto are not the same value",
                gc.into_nogc(),
            ));
        }

        // 13. Return handlerProto.
        Ok(handler_proto)
    }

    fn try_set_prototype_of<'gc>(
        self,
        _: &mut Agent,
        _: Option<Object>,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        TryError::GcError.into()
    }

    /// ### [10.5.2 \[\[SetPrototypeOf\]\] ( V )](https://tc39.es/ecma262/#sec-proxy-object-internal-methods-and-internal-slots-setprototypeof-v)
    ///
    /// The \[\[SetPrototypeOf]] internal method of a Proxy exotic object O takes
    /// argument V (an Object or null) and returns either a normal completion
    /// containing a Boolean or a throw completion.
    fn internal_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let nogc = gc.nogc();
        let mut prototype = prototype;
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, nogc)?;
        let scoped_target = target.scope(agent, nogc);
        let mut scoped_prototype = None;
        // 5. Let trap be ? GetMethod(handler, "setPrototypeOf").
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.setPrototypeOf.into(),
            nogc,
        ))? {
            trap
        } else {
            scoped_prototype = prototype.map(|p| p.scope(agent, nogc));
            let scoped_handler = handler.scope(agent, nogc);
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.setPrototypeOf.into(),
                gc.reborrow(),
            )?;
            let gc = gc.nogc();
            target = scoped_target.get(agent).local();
            handler = scoped_handler.get(agent).local();
            prototype = scoped_prototype.as_ref().map(|p| p.get(agent).local());
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[SetPrototypeOf]](V).
            return target.internal_set_prototype_of(agent, prototype, gc);
        };
        let scoped_prototype = if scoped_prototype.is_none() && prototype.is_some() {
            prototype.map(|p| p.scope(agent, gc.nogc()))
        } else {
            scoped_prototype
        };
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, V »)).
        let argument = call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                target.into(),
                prototype.map_or(Value::Null, |p| p.into()),
            ])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);
        // 8. If booleanTrapResult is false, return false.
        if !boolean_trap_result {
            return Ok(false);
        }
        // 9. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target =
            is_extensible(agent, scoped_target.get(agent).local(), gc.reborrow())?;
        // 10. If extensibleTarget is true, return true.
        if extensible_target {
            return Ok(true);
        }
        // 11. Let targetProto be ? target.[[GetPrototypeOf]]().
        let target_proto = scoped_target
            .get(agent)
            .local()
            .internal_get_prototype_of(agent, gc.reborrow())?;
        // 12. If SameValue(V, targetProto) is false, throw a TypeError exception.
        if scoped_prototype.map(|p| p.get(agent).local()) != target_proto {
            return  Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'setPrototypeOf' on proxy: trap returned truish for setting a new prototype on the non-extensible proxy target",
                gc.into_nogc(),
            ));
        }
        // 13. Return true.
        Ok(true)
    }

    fn try_is_extensible<'gc>(self, _: &mut Agent, _: NoGcScope<'gc, '_>) -> TryResult<'gc, bool> {
        TryError::GcError.into()
    }

    fn internal_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
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
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.isExtensible.into(),
            gc.nogc(),
        ))? {
            trap
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.isExtensible.into(),
                gc.reborrow(),
            )?;
            handler = scoped_handler.get(agent).local();
            trap
        };

        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? IsExtensible(target).
            return is_extensible(agent, target.get(agent).local(), gc);
        };

        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target »)).
        let argument = call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [target
                .get(agent)
                .local()
                .into()])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);

        // 8. Let targetResult be ? IsExtensible(target).
        let target_result = is_extensible(agent, target.get(agent).local(), gc.reborrow())?;

        // 9. If booleanTrapResult is not targetResult, throw a TypeError exception.
        if boolean_trap_result != target_result {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "proxy must report same extensiblitity as target",
                gc.into_nogc(),
            ));
        };

        // 10. Return booleanTrapResult.
        Ok(boolean_trap_result)
    }

    fn try_prevent_extensions<'gc>(
        self,
        _: &mut Agent,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        TryError::GcError.into()
    }

    fn internal_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
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
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.preventExtensions.into(),
            gc.nogc(),
        ))? {
            trap
        } else {
            let scoped_handler = handler.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.preventExtensions.into(),
                gc.reborrow(),
            )?;
            handler = scoped_handler.get(agent).local();
            trap
        };

        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[PreventExtensions]]().
            return target
                .get(agent)
                .local()
                .internal_prevent_extensions(agent, gc);
        };

        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target »)).
        let argument = call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [target
                .get(agent)
                .local()
                .into()])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);

        // 8. If booleanTrapResult is true, then

        if boolean_trap_result {
            // a. Let extensibleTarget be ? IsExtensible(target).
            let extensible_target = is_extensible(agent, target.get(agent).local(), gc.reborrow())?;

            // b. If extensibleTarget is true, throw a TypeError exception.
            if extensible_target {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "proxy can't report an extensible object as non-extensible",
                    gc.into_nogc(),
                ));
            }
        };

        // 9. Return booleanTrapResult.
        Ok(boolean_trap_result)
    }

    fn try_get_own_property<'gc>(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: Option<PropertyLookupCache>,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        TryError::GcError.into()
    }

    /// ### [10.5.5 \[\[GetOwnProperty\]\] ( P )](https://tc39.es/ecma262/#sec-proxy-object-internal-methods-and-internal-slots-getownproperty-p)
    ///
    /// The \[\[GetOwnProperty]] internal method of a Proxy exotic object O takes
    /// argument P (a property key) and returns either a normal completion
    /// containing either a Property Descriptor or undefined, or a throw completion.
    fn internal_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<PropertyDescriptor<'gc>>> {
        let nogc = gc.nogc();
        let mut property_key = property_key;
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, nogc)?;
        let scoped_target = target.scope(agent, nogc);
        let mut scoped_property_key = None;
        // 5. Let trap be ? GetMethod(handler, "getOwnPropertyDescriptor").
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.getOwnPropertyDescriptor.into(),
            nogc,
        ))? {
            trap
        } else {
            let scoped_handler = handler.scope(agent, nogc);
            scoped_property_key = Some(property_key.scope(agent, nogc));
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.getOwnPropertyDescriptor.into(),
                gc.reborrow(),
            )?;
            let gc = gc.nogc();
            crate::engine::bind!(let trap = trap, gc);
            handler = scoped_handler.get(agent).local();
            target = scoped_target.get(agent).local();
            property_key = scoped_property_key.as_ref().unwrap().get(agent).local();
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[GetOwnProperty]](P).
            return target.internal_get_own_property(agent, property_key, gc);
        };
        // 7. Let trapResultObj be ? Call(trap, handler, « target, P »).
        let scoped_property_key =
            scoped_property_key.unwrap_or_else(|| property_key.scope(agent, gc.nogc()));
        let p = property_key.convert_to_value(agent, gc.nogc());
        let trap_result_obj = call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                target.into(),
                p.into(),
            ])),
            gc.reborrow(),
        )?;
        // 8. If trapResultObj is not an Object and trapResultObj is not undefined, throw a TypeError exception.
        let trap_result_obj_is_undefined = trap_result_obj.is_undefined();
        if !trap_result_obj.is_object() && !trap_result_obj_is_undefined {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "proxy [[GetOwnProperty]] must return an object or undefined",
                gc.into_nogc(),
            ));
        };
        let trap_result_obj = trap_result_obj.scope(agent, gc.nogc());
        // 9. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let target_desc = scoped_target.get(agent).local().internal_get_own_property(
            agent,
            scoped_property_key.get(agent).local(),
            gc.reborrow(),
        )?;
        // 10. If trapResultObj is undefined, then
        if trap_result_obj_is_undefined {
            // a. If targetDesc is undefined,
            let Some(target_desc) = target_desc else {
                // return undefined.
                return Ok(None);
            };
            // b. If targetDesc.[[Configurable]] is false,
            if target_desc.configurable == Some(false) {
                // throw a TypeError exception.
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    format!(
                        "proxy can't report a non-configurable own property '{}' as non-existent.",
                        scoped_property_key.get(agent).local().as_display(agent)
                    ),
                    gc.into_nogc(),
                ));
            }
            // c. Let extensibleTarget be ? IsExtensible(target).
            let extensible_target =
                is_extensible(agent, scoped_target.get(agent).local(), gc.reborrow())?;
            // d. If extensibleTarget is false, throw a TypeError exception.
            if !extensible_target {
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    format!(
                        "proxy can't report a extensibleTarget own property '{}' as non-existent.",
                        scoped_property_key.get(agent).local().as_display(agent)
                    ),
                    gc.into_nogc(),
                ));
            };
            // e. Return undefined.
            return Ok(None);
        };
        let target_desc = target_desc.map(|desc| desc.scope(agent, gc.nogc()));
        // 11. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target =
            is_extensible(agent, scoped_target.get(agent).local(), gc.reborrow())?;
        // 12. Let resultDesc be ? ToPropertyDescriptor(trapResultObj).
        let mut result_desc = PropertyDescriptor::to_property_descriptor(
            agent,
            trap_result_obj.get(agent).local(),
            gc.reborrow(),
        )?;
        // 13. Perform CompletePropertyDescriptor(resultDesc).
        result_desc.complete_property_descriptor()?;
        // 14. Let valid be IsCompatiblePropertyDescriptor(extensibleTarget, resultDesc, targetDesc).
        let target_desc = target_desc.map(|desc| desc.take(agent, gc.nogc()));
        let valid = match is_compatible_property_descriptor(
            agent,
            extensible_target,
            result_desc.clone(),
            target_desc.clone(),
            gc.nogc(),
        ) {
            Ok(b) => b,
            Err(err) => {
                return Err(agent.throw_allocation_exception(err, gc.into_nogc()));
            }
        };
        // 15. If valid is false, throw a TypeError exception.
        if !valid {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "valid",
                gc.into_nogc(),
            ));
        };
        // 16. If resultDesc.[[Configurable]] is false, then
        if result_desc.configurable == Some(false) {
            // a. If targetDesc is undefined or targetDesc.[[Configurable]] is true, then
            if target_desc
                .as_ref()
                .is_none_or(|d| d.configurable == Some(true))
            {
                // i. Throw a TypeError exception.
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    format!(
                        "proxy can't report a non-existent property '{}' as non-configurable",
                        scoped_property_key.get(agent).local().as_display(agent)
                    ),
                    gc.into_nogc(),
                ));
            }
            let target_desc = target_desc.unwrap();
            // b. If resultDesc has a [[Writable]] field and resultDesc.[[Writable]] is false, then
            if result_desc.writable == Some(false) {
                // i. Assert: targetDesc has a [[Writable]] field.
                assert!(target_desc.writable.is_some());
                // ii. If targetDesc.[[Writable]] is true, throw a TypeError exception.
                if target_desc.writable == Some(true) {
                    return Err(agent.throw_exception(
                        ExceptionType::TypeError,
                        format!(
                            "proxy can't report existing writable property '{}' as non-writable",
                            scoped_property_key.get(agent).local().as_display(agent)
                        ),
                        gc.into_nogc(),
                    ));
                }
            }
        };
        // 17. Return resultDesc.
        Ok(Some(result_desc))
    }

    fn try_define_own_property<'gc>(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: PropertyDescriptor,
        _: Option<PropertyLookupCache>,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        TryError::GcError.into()
    }

    fn internal_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let nogc = gc.nogc();
        crate::engine::bind!(let o = self, gc);
        let property_key = property_key.scope(agent, nogc);
        crate::engine::bind!(let property_descriptor = property_descriptor, gc);
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy { target, handler } = validate_non_revoked_proxy(agent, o, nogc)?;
        let scoped_target = target.scope(agent, nogc);
        let scoped_handler = handler.scope(agent, nogc);
        let property_descriptor = property_descriptor.scope(agent, nogc);
        // 5. Let trap be ? GetMethod(handler, "defineProperty").
        let trap = get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.defineProperty.into(),
            gc.reborrow(),
        )?;
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[DefineOwnProperty]](P, Desc).
            return scoped_target
                .get(agent)
                .local()
                .internal_define_own_property(
                    agent,
                    property_key.get(agent).local(),
                    property_descriptor.take(agent, gc.nogc()),
                    gc,
                );
        };
        let trap = trap;
        // 7. Let descObj be FromPropertyDescriptor(Desc).
        let desc_obj = PropertyDescriptor::from_property_descriptor(
            Some(property_descriptor.get(agent, gc.nogc())),
            agent,
            gc.nogc(),
        );
        // 8. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, P, descObj »)).
        let p = property_key
            .get(agent)
            .local()
            .convert_to_value(agent, gc.nogc());
        let argument = call_function(
            agent,
            trap,
            scoped_handler.get(agent).local().into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                scoped_target.get(agent).local().into(),
                p.into(),
                desc_obj.map_or(Value::Null, |d| d.into()),
            ])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);
        // 9. If booleanTrapResult is false, return false.
        if !boolean_trap_result {
            return Ok(false);
        };
        // 10. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let target_desc = scoped_target
            .get(agent)
            .local()
            .internal_get_own_property(agent, property_key.get(agent).local(), gc.reborrow())?
            .map(|desc| desc.scope(agent, gc.nogc()));
        // 11. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target =
            is_extensible(agent, scoped_target.get(agent).local(), gc.reborrow())?;
        // 12. If Desc has a [[Configurable]] field and Desc.[[Configurable]] is false, then
        let setting_config_false = property_descriptor.configurable == Some(false);
        // 14. If targetDesc is undefined, then
        let gc = gc.into_nogc();
        let target_desc = target_desc.map(|desc| desc.get(agent, gc));
        let property_descriptor = property_descriptor.take(agent, gc);
        let Some(target_desc) = target_desc else {
            // a. If extensibleTarget is false, throw a TypeError exception.
            if !extensible_target {
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    format!(
                        "proxy can't define a new property '{}' on a non-extensible object",
                        property_key.get(agent).local().as_display(agent)
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
                        property_key.get(agent).local().as_display(agent)
                    ),
                    gc,
                ));
            }
            // 16. Return true.
            return Ok(true);
        };
        // 15. Else,
        let is_compatible = is_compatible_property_descriptor(
            agent,
            extensible_target,
            property_descriptor.clone(),
            Some(target_desc.clone()),
            gc,
        )
        .map_err(|err| agent.throw_allocation_exception(err, gc))?;
        // a. If IsCompatiblePropertyDescriptor(extensibleTarget, Desc, targetDesc) is false, throw a TypeError exception.
        if !is_compatible {
            let message = format!(
                "proxy can't define an incompatible property descriptor ('{}', proxy can't report an existing non-configurable property as configurable)",
                property_key.get(agent).local().as_display(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, message, gc));
        }
        // b. If settingConfigFalse is true and targetDesc.[[Configurable]] is true, throw a TypeError exception.
        if setting_config_false && target_desc.configurable == Some(true) {
            let message = format!(
                "proxy can't define an incompatible property descriptor ('{}', proxy can't define an existing configurable property as non-configurable)",
                property_key.get(agent).local().as_display(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, message, gc));
        }
        // c. If IsDataDescriptor(targetDesc) is true,
        if target_desc.is_data_descriptor()
                    // targetDesc.[[Configurable]] is false,
                    && target_desc.configurable == Some(false)
                    // and targetDesc.[[Writable]] is true, then
                    && target_desc.writable == Some(true)
                    //  i. If Desc has a [[Writable]] field and Desc.[[Writable]] is false, throw a TypeError exception.
                    && property_descriptor.writable == Some(false)
        {
            let message = format!(
                "proxy can't define an incompatible property descriptor ('{}', proxy can't define an existing non-configurable writable property as non-writable)",
                property_key.get(agent).local().as_display(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, message, gc));
        }
        // 16. Return true.
        Ok(true)
    }

    fn try_has_property<'gc>(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryHasResult<'gc>> {
        TryHasResult::Proxy(self).into()
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let nogc = gc.nogc();
        let mut property_key = property_key;

        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, nogc)?;

        // 5. Let trap be ? GetMethod(handler, "has").
        let scoped_target = target.scope(agent, nogc);
        let scoped_property_key = property_key.scope(agent, nogc);
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.has.into(),
            nogc,
        ))? {
            trap
        } else {
            let scoped_handler = handler.scope(agent, nogc);
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.has.into(),
                gc.reborrow(),
            )?;
            let gc = gc.nogc();
            handler = scoped_handler.get(agent).local();
            target = scoped_target.get(agent).local();
            property_key = scoped_property_key.get(agent).local();
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // Return ? target.[[HasProperty]](P).
            return target.internal_has_property(agent, property_key, gc);
        };

        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, P »)).
        let p = property_key.convert_to_value(agent, gc.nogc());
        let argument = call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                target.into(),
                p.into(),
            ])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);

        // 8. If booleanTrapResult is false, then
        if !boolean_trap_result {
            // a. Let targetDesc be ? target.[[GetOwnProperty]](P).
            let target_desc = scoped_target.get(agent).local().internal_get_own_property(
                agent,
                scoped_property_key.get(agent).local(),
                gc.reborrow(),
            )?;
            //    b. If targetDesc is not undefined, then
            if let Some(target_desc) = target_desc {
                //  i. If targetDesc.[[Configurable]] is false, throw a TypeError exception.
                if target_desc.configurable == Some(false) {
                    let gc = gc.into_nogc();
                    crate::engine::bind!(let property_key = scoped_property_key.get(agent).local(), gc);
                    let message = String::from_string(
                        agent,
                        format!(
                            "proxy can't report a non-configurable own property '{}' as non-existent",
                            property_key.as_display(agent)
                        ),
                        gc,
                    );
                    return Err(agent.throw_exception_with_message(
                        ExceptionType::TypeError,
                        message,
                        gc,
                    ));
                }
                // ii. Let extensibleTarget be ? IsExtensible(target).
                // iii. If extensibleTarget is false, throw a TypeError exception.
                if !is_extensible(agent, scoped_target.get(agent).local(), gc.reborrow())? {
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

    fn try_get<'gc>(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: Value<'static>,
        _: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        TryGetResult::Proxy(self).into()
    }

    /// ### [10.5.8 \[\[Get\]\] ( P, Receiver )](https://tc39.es/ecma262/#sec-proxy-object-internal-methods-and-internal-slots-get-p-receiver)
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
    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        let nogc = gc.nogc();
        let mut property_key = property_key;
        let mut receiver = receiver;
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, nogc)?;
        // 5. Let trap be ? GetMethod(handler, "get").
        let scoped_target = target.scope(agent, nogc);
        let scoped_property_key = property_key.scope(agent, nogc);
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.get.into(),
            nogc,
        ))? {
            trap
        } else {
            let scoped_handler = handler.scope(agent, nogc);
            let scoped_receiver = receiver.scope(agent, nogc);
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.get.into(),
                gc.reborrow(),
            )?;
            let gc = gc.nogc();
            handler = scoped_handler.get(agent).local();
            receiver = scoped_receiver.get(agent).local();
            target = scoped_target.get(agent).local();
            property_key = scoped_property_key.get(agent).local();
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[Get]](P, Receiver).
            return target.internal_get(agent, property_key, receiver, gc);
        };
        // 7. Let trapResult be ? Call(trap, handler, « target, P, Receiver »).
        let p = property_key.convert_to_value(agent, gc.nogc());
        let trap_result = call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                target.into(),
                p.into(),
                receiver,
            ])),
            gc.reborrow(),
        )?
        .scope(agent, gc.nogc());
        // 8. Let targetDesc be ? target.[[GetOwnProperty]](P).
        crate::engine::bind!(let target = scoped_target.get(agent).local(), gc);
        crate::engine::bind!(let property_key = scoped_property_key.get(agent).local(), gc);
        let target_desc = target.internal_get_own_property(agent, property_key, gc.reborrow())?;
        crate::engine::bind!(let trap_result = trap_result.get(agent).local(), gc);
        // 9. If targetDesc is not undefined and
        if let Some(target_desc) = target_desc
            // targetDesc.[[Configurable]] is false, then
            && target_desc.configurable == Some(false)
            // a. If IsDataDescriptor(targetDesc) is true and
            && (target_desc.is_data_descriptor()
                // targetDesc.[[Writable]] is false, then
                && target_desc.writable == Some(false)
                // i. If SameValue(trapResult, targetDesc.[[Value]]) is false, ...
                && !same_value(
                    agent,
                    trap_result,
                    target_desc.value.unwrap_or(Value::Undefined),
                )
                // b. If IsAccessorDescriptor(targetDesc) is true and
                || target_desc.is_accessor_descriptor()
                    // targetDesc.[[Get]] is undefined, then
                    && target_desc.get.is_none_or(|g| g.is_none())
                    // i. If trapResult is not undefined, ...
                    && !trap_result.is_undefined())
        {
            // ... throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Invalid Proxy [[Get]] method",
                gc.into_nogc(),
            ));
        }
        // 10. Return trapResult.
        Ok(trap_result)
    }

    fn try_set<'gc>(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: Value<'static>,
        _: Value<'static>,
        _: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        SetResult::Proxy(self).into()
    }

    /// ### [10.5.9 \[\[Set\]\] ( P, V, Receiver )](https://tc39.es/ecma262/#sec-proxy-object-internal-methods-and-internal-slots-set-p-v-receiver)
    ///
    /// The \[\[Set]] internal method of a Proxy exotic object O takes
    /// arguments P (a property key), V (an ECMAScript language
    /// value), and Receiver (an ECMAScript language value) and returns either a normal completion containing
    /// a Boolean or a throw completion.
    fn internal_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let nogc = gc.nogc();
        let mut property_key = property_key;
        let mut value = value;
        let mut receiver = receiver;
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, nogc)?;
        // 5. Let trap be ? GetMethod(handler, "set").
        let scoped_target = target.scope(agent, nogc);
        let scoped_property_key = property_key.scope(agent, nogc);
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.set.into(),
            nogc,
        ))? {
            trap
        } else {
            let scoped_value = value.scope(agent, nogc);
            let scoped_handler = handler.scope(agent, nogc);
            let scoped_receiver = receiver.scope(agent, nogc);
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.set.into(),
                gc.reborrow(),
            )?;
            let gc = gc.nogc();
            value = scoped_value.get(agent).local();
            handler = scoped_handler.get(agent).local();
            receiver = scoped_receiver.get(agent).local();
            target = scoped_target.get(agent).local();
            property_key = scoped_property_key.get(agent).local();
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[Set]](P, V, Receiver).
            return target.internal_set(agent, property_key, value, receiver, gc);
        };
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, P, V, Receiver »)).
        let scoped_value = value.scope(agent, gc.nogc());
        let p = property_key.convert_to_value(agent, gc.nogc());
        let argument = call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                target.into(),
                p.into(),
                value,
                receiver,
            ])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);

        // 8. If booleanTrapResult is false, return false.
        if !boolean_trap_result {
            return Ok(false);
        };

        // 9. Let targetDesc be ? target.[[GetOwnProperty]](P).
        crate::engine::bind!(let target = scoped_target.get(agent).local(), gc);
        crate::engine::bind!(let property_key = scoped_property_key.get(agent).local(), gc);
        let target_desc = target.internal_get_own_property(agent, property_key, gc.reborrow())?;
        // 10. If targetDesc is not undefined and
        if let Some(target_desc) = target_desc
            // targetDesc.[[Configurable]] is false, then
            && target_desc.configurable == Some(false)
            // a. If IsDataDescriptor(targetDesc) is true and
            && (target_desc.is_data_descriptor()
                // targetDesc.[[Writable]] is false, then
                && target_desc.writable == Some(false)
                // i. If SameValue(V, targetDesc.[[Value]]) is false, ...
                && !same_value(
                    agent,
                    scoped_value.get(agent).local(),
                    target_desc.value.unwrap_or(Value::Undefined),
                )
                // b. If IsAccessorDescriptor(targetDesc) is true, then
                || target_desc.is_accessor_descriptor()
                    // i. If targetDesc.[[Set]] is undefined, ...
                    && target_desc.set.is_none_or(|s| s.is_none()))
        {
            // ... throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Invalid Proxy [[Set]] method",
                gc.into_nogc(),
            ));
        }
        // 10. Return trapResult.
        Ok(true)
    }

    fn try_delete<'gc>(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        TryError::GcError.into()
    }

    fn internal_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let nogc = gc.nogc();
        let mut property_key = property_key;
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, nogc)?;
        let mut scoped_target = None;
        let mut scoped_property_key = None;
        // 5. Let trap be ? GetMethod(handler, "deleteProperty").
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.deleteProperty.into(),
            nogc,
        ))? {
            trap
        } else {
            let scoped_handler = handler.scope(agent, nogc);
            scoped_target = Some(target.scope(agent, nogc));
            scoped_property_key = Some(property_key.scope(agent, nogc));
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.deleteProperty.into(),
                gc.reborrow(),
            )?;
            let gc = gc.nogc();
            handler = scoped_handler.get(agent).local();
            target = scoped_target.as_ref().unwrap().get(agent).local();
            property_key = scoped_property_key.as_ref().unwrap().get(agent).local();
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[Delete]](P).
            return target.internal_delete(agent, property_key, gc);
        };
        let scoped_target = scoped_target.unwrap_or_else(|| target.scope(agent, gc.nogc()));
        // 7. Let booleanTrapResult be ToBoolean(? Call(trap, handler, « target, P »)).
        let scoped_property_key =
            scoped_property_key.unwrap_or_else(|| property_key.scope(agent, gc.nogc()));
        let p = property_key.convert_to_value(agent, gc.nogc());
        let argument = call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                target.into(),
                p.into(),
            ])),
            gc.reborrow(),
        )?;
        let boolean_trap_result = to_boolean(agent, argument);
        // 8. If booleanTrapResult is false, return false.
        if !boolean_trap_result {
            return Ok(false);
        };
        // 9. Let targetDesc be ? target.[[GetOwnProperty]](P).
        let target_desc = scoped_target.get(agent).local().internal_get_own_property(
            agent,
            scoped_property_key.get(agent).local(),
            gc.reborrow(),
        )?;
        // 10. If targetDesc is undefined, return true.
        let Some(target_desc) = target_desc else {
            return Ok(true);
        };
        // 11. If targetDesc.[[Configurable]] is false,
        if target_desc.configurable == Some(false) {
            // throw a TypeError exception.
            let message = format!(
                "property '{}' is non-configurable and can't be deleted",
                scoped_property_key.get(agent).local().as_display(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, message, gc.into_nogc()));
        };
        // 12. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target =
            is_extensible(agent, scoped_target.get(agent).local(), gc.reborrow())?;
        // 13. If extensibleTarget is false,
        if !extensible_target {
            // throw a TypeError exception.
            let message = format!(
                "proxy can't delete property '{}' on a non-extensible object",
                scoped_property_key.get(agent).local().as_display(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, message, gc.into_nogc()));
        };
        // 14. Return true.
        Ok(true)
    }

    fn try_own_property_keys<'gc>(
        self,
        _: &mut Agent,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        TryError::GcError.into()
    }

    fn internal_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Vec<PropertyKey<'gc>>> {
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, gc.nogc())?;
        // 5. Let trap be ? GetMethod(handler, "ownKeys").
        let mut scoped_target = None;
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.ownKeys.into(),
            gc.nogc(),
        ))? {
            trap
        } else {
            scoped_target = Some(target.scope(agent, gc.nogc()));
            let scoped_handler = handler.scope(agent, gc.nogc());
            let trap = get_object_method(
                agent,
                handler,
                BUILTIN_STRING_MEMORY.ownKeys.into(),
                gc.reborrow(),
            )?;
            let gc = gc.nogc();
            handler = scoped_handler.get(agent).local();
            target = scoped_target.as_ref().unwrap().get(agent).local();
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? target.[[OwnPropertyKeys]]().
            return target.internal_own_property_keys(agent, gc);
        };
        let scoped_target = scoped_target.unwrap_or_else(|| target.scope(agent, gc.nogc()));
        // 7. Let trapResultArray be ? Call(trap, handler, « target »).
        let trap_result_array = call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [target.into()])),
            gc.reborrow(),
        )?;
        // 8. Let trapResult be ? CreateListFromArrayLike(trapResultArray, property-key).
        let trap_result =
            create_property_key_list_from_array_like(agent, trap_result_array, gc.reborrow())?;
        // 9. If trapResult contains any duplicate entries, throw a TypeError exception.
        let mut unique_trap_results = Vec::with_capacity(trap_result.len(agent));
        for value in trap_result.iter(agent) {
            let p = value.get(gc.nogc());
            if unique_trap_results.contains(&p) {
                let message = format!(
                    "proxy [[OwnPropertyKeys]] can't report property '{}' more than once",
                    p.as_display(agent),
                );
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    message,
                    gc.into_nogc(),
                ));
            }
            unique_trap_results.push(p);
        }
        // 10. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target =
            is_extensible(agent, scoped_target.get(agent).local(), gc.reborrow())?;
        // 11. Let targetKeys be ? target.[[OwnPropertyKeys]]().
        let target_keys = scoped_target
            .get(agent)
            .local()
            .internal_own_property_keys(agent, gc.reborrow())?;
        // 13. Assert: targetKeys contains no duplicate entries.
        debug_assert!({
            let mut seen = AHashSet::with_capacity(target_keys.len());
            let mut all_unique = true;
            for key in target_keys.iter() {
                if seen.contains(key) {
                    all_unique = false;
                    break;
                }
                seen.insert(*key);
            }
            all_unique
        });
        let target_keys_len = target_keys.len();
        let target_keys = target_keys.scope(agent, gc.nogc());
        // 14. Let targetConfigurableKeys be a new empty List.
        let mut target_configurable_keys =
            Vec::<PropertyKey>::with_capacity(target_keys_len).scope(agent, gc.nogc());

        // 15. Let targetNonconfigurableKeys be a new empty List.
        let mut target_nonconfigurable_keys = Vec::<PropertyKey>::new().scope(agent, gc.nogc());
        // 16. For each element key of targetKeys, do
        for key in target_keys.iter(agent) {
            // a. Let desc be ? target.[[GetOwnProperty]](key).
            let desc = scoped_target.get(agent).local().internal_get_own_property(
                agent,
                key.get(gc.nogc()),
                gc.reborrow(),
            )?;
            //  b. If desc is not undefined and desc.[[Configurable]] is false, then
            if desc.is_some_and(|d| d.configurable == Some(false)) {
                // i. Append key to targetNonconfigurableKeys.
                target_nonconfigurable_keys.push(agent, key.get(gc.nogc()));
            } else {
                // c. Else,
                // i. Append key to targetConfigurableKeys.
                target_configurable_keys.push(agent, key.get(gc.nogc()));
            }
        }
        let gc = gc.into_nogc();
        let trap_result = trap_result
            .iter(agent)
            .map(|p| p.get(gc))
            .collect::<Vec<PropertyKey>>();
        // 17. If extensibleTarget is true and targetNonconfigurableKeys is empty, then
        if extensible_target && target_nonconfigurable_keys.is_empty(agent) {
            // a. Return trapResult.
            return Ok(trap_result);
        }
        let target_configurable_keys = target_configurable_keys.take(agent).local();
        let target_nonconfigurable_keys = target_nonconfigurable_keys.take(agent).local();
        // 18. Let uncheckedResultKeys be a List whose elements are the elements of trapResult.
        let mut unchecked_result_keys = VecDeque::from(trap_result.clone());
        // 19. For each element key of targetNonconfigurableKeys, do
        for key in target_nonconfigurable_keys {
            // a. If uncheckedResultKeys does not contain key, throw a TypeError exception.
            if !unchecked_result_keys.contains(&key) {
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    format!(
                        "proxy can't skip a non-configurable property '{}'",
                        key.as_display(agent)
                    ),
                    gc,
                ));
            }
            if let Some(pos) = unchecked_result_keys
                .iter()
                .position(|unchecked_key| *unchecked_key == key)
            {
                // b. Remove the key from uncheckedResultKeys
                unchecked_result_keys.remove(pos);
            }
        }
        // 20. If extensibleTarget is true, return trapResult.
        if extensible_target {
            return Ok(trap_result);
        };
        // 21. For each element key of targetConfigurableKeys, do
        for key in target_configurable_keys {
            // a. If uncheckedResultKeys does not contain key, throw a TypeError exception.
            if !unchecked_result_keys.contains(&key) {
                return Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    format!("proxy can't report an existing own property '{}' as non-existent on a non-extensible object", key.as_display(agent)),
                    gc,
                ));
            }
            if let Some(pos) = unchecked_result_keys
                .iter()
                .position(|unchecked_key| *unchecked_key == key)
            {
                // b. Remove the key from uncheckedResultKeys
                unchecked_result_keys.remove(pos);
            }
        }
        // 22. If uncheckedResultKeys is not empty, throw a TypeError exception.
        if !unchecked_result_keys.is_empty() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "trap returned extra keys but proxy target is non-extensible",
                gc,
            ));
        }
        // 23. Return trapResult.
        Ok(trap_result)
    }

    #[inline(always)]
    fn get_own_property_at_offset<'gc>(
        self,
        _: &Agent,
        _: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryGetResult<'gc> {
        // TODO: Check if self is non-revoked, try to go through the Proxy
        // without trigger the trap.
        TryGetResult::Proxy(self)
    }

    #[inline(always)]
    fn set_at_offset<'gc>(
        self,
        _: &mut Agent,
        _: &SetAtOffsetProps,
        _: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        // TODO: Check if self is non-revoked, try to go through the Proxy
        // without trigger the trap.
        SetResult::Proxy(self).into()
    }

    /// ### [10.5.12 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-proxy-object-internal-methods-and-internal-slots-call-thisargument-argumentslist)
    ///
    /// The \[\[Call]] internal method of a Proxy exotic object O takes
    /// arguments thisArgument (an ECMAScript language value)
    /// and argumentsList (a List of ECMAScript language values)
    /// and returns either a normal completion containing an ECMAScript
    /// language value or a throw completion.
    fn internal_call<'gc>(
        self,
        agent: &mut Agent,
        this_argument: Value,
        arguments_list: ArgumentsList<'_, 'static>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        let nogc = gc.nogc();
        let mut this_argument = this_argument;
        let mut arguments_list = arguments_list;
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 3. Let handler be O.[[ProxyHandler]].
        // 4. Assert: handler is an Object.
        let NonRevokedProxy {
            mut target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, nogc)?;
        // 5. Let trap be ? GetMethod(handler, "apply").
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.apply.into(),
            nogc,
        ))? {
            trap
        } else {
            let scoped_handler = handler.scope(agent, nogc);
            let scoped_target = target.scope(agent, nogc);
            let scoped_this_argument = this_argument.scope(agent, nogc);
            let mut args = arguments_list;
            let trap = {
                let handler = handler;
                args.with_scoped(
                    agent,
                    |agent, _, gc| {
                        get_object_method(agent, handler, BUILTIN_STRING_MEMORY.apply.into(), gc)
                    },
                    gc.reborrow(),
                )?
            };
            let gc = gc.nogc();
            handler = scoped_handler.get(agent).local();
            target = scoped_target.get(agent).local();
            this_argument = scoped_this_argument.get(agent).local();
            arguments_list = args;
            trap
        };
        // 6. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? Call(target, thisArgument, argumentsList).
            return call(
                agent,
                target.into(),
                this_argument,
                Some(arguments_list),
                gc,
            );
        };
        // 7. Let argArray be CreateArrayFromList(argumentsList).
        let arg_array = create_array_from_list(agent, arguments_list.as_slice(), gc.nogc());
        // 8. Return ? Call(trap, handler, « target, thisArgument, argArray »).
        call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                target.into(),
                this_argument,
                arg_array.into(),
            ])),
            gc,
        )
    }

    fn internal_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList<'_, 'static>,
        new_target: Function,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        let nogc = gc.nogc();
        crate::engine::bind!(let mut arguments_list = arguments_list, gc);
        let mut new_target = new_target;
        // 1. Perform ? ValidateNonRevokedProxy(O).
        // 2. Let target be O.[[ProxyTarget]].
        // 4. Let handler be O.[[ProxyHandler]].
        // 5. Assert: handler is an Object.
        let NonRevokedProxy {
            target,
            mut handler,
        } = validate_non_revoked_proxy(agent, self, nogc)?;
        // 3. Assert: IsConstructor(target) is true.
        let mut target = is_constructor(agent, target).unwrap();
        // 6. Let trap be ? GetMethod(handler, "construct").
        let trap = if let Some(trap) = try_result_into_js(try_get_object_method(
            agent,
            handler,
            BUILTIN_STRING_MEMORY.construct.into(),
            nogc,
        ))? {
            trap
        } else {
            let scoped_new_target = new_target.scope(agent, nogc);
            let scoped_target = target.scope(agent, nogc);
            let scoped_handler = handler.scope(agent, nogc);
            let mut args = arguments_list;
            let trap = {
                let handler = handler;
                args.with_scoped(
                    agent,
                    |agent, _, gc| {
                        get_object_method(
                            agent,
                            handler,
                            BUILTIN_STRING_MEMORY.construct.into(),
                            gc,
                        )
                    },
                    gc.reborrow(),
                )
            }?;
            let gc = gc.nogc();
            new_target = scoped_new_target.get(agent).local();
            target = scoped_target.get(agent).local();
            handler = scoped_handler.get(agent).local();
            arguments_list = args;
            trap
        };
        // 7. If trap is undefined, then
        let Some(trap) = trap else {
            // a. Return ? Construct(target, argumentsList, newTarget).
            return construct(agent, target, Some(arguments_list), Some(new_target), gc);
        };
        // 8. Let argArray be CreateArrayFromList(argumentsList).
        let arg_array = create_array_from_list(agent, arguments_list.as_slice(), gc.nogc());
        // 9. Let newObj be ? Call(trap, handler, « target, argArray, newTarget »).
        let new_obj = call_function(
            agent,
            trap,
            handler.into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                target.into(),
                arg_array.into(),
                new_target.into(),
            ])),
            gc.reborrow(),
        )?;
        if let Ok(new_obj) = Object::try_from(new_obj) {
            // 11. Return newObj.
            Ok(new_obj)
        } else {
            // 10. If newObj is not an Object, throw a TypeError exception.
            Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "proxy [[Construct]] must return an object",
                gc.into_nogc(),
            ))
        }
    }
}

/// ### [10.5.15 ProxyCreate ( target, handler )](https://tc39.es/ecma262/#sec-proxycreate)
///
/// The abstract operation ProxyCreate takes arguments target (an ECMAScript
/// language value) and handler (an ECMAScript language value) and returns
/// either a normal completion containing a Proxy exotic object or a throw
/// completion. It is used to specify the creation of new Proxy objects.
pub(crate) fn proxy_create<'a>(
    agent: &mut Agent,
    target: Value,
    handler: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, Proxy<'a>> {
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
    let p = agent.heap.create(ProxyHeapData::NonRevoked {
        proxy_target: target,
        proxy_handler: handler,
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

impl<'a> CreateHeapData<ProxyHeapData<'a>, Proxy<'a>> for Heap {
    fn create(&mut self, data: ProxyHeapData<'a>) -> Proxy<'a> {
        self.proxies.push(data);
        self.alloc_counter += core::mem::size_of::<ProxyHeapData<'static>>();
        Proxy(BaseIndex::last(&self.proxies))
    }
}

impl HeapMarkAndSweep for Proxy<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.proxies.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.proxies.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for Proxy<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.proxies.shift_weak_index(self.0).map(Self)
    }
}
