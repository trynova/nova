// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
mod function_internal_properties;

pub(crate) use data::*;
pub(crate) use function_internal_properties::*;

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BOUND_FUNCTION_DISCRIMINANT,
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT, BUILTIN_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_FINALLY_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
        BoundFunction, BuiltinConstructorFunction, BuiltinFunction, BuiltinPromiseFinallyFunction,
        BuiltinPromiseResolvingFunction, ECMASCRIPT_FUNCTION_DISCRIMINANT, ECMAScriptFunction,
        InternalMethods, InternalSlots, JsResult, Object, OrdinaryObject, PropertyDescriptor,
        PropertyKey, PropertyLookupCache, PropertyOffset, ProtoIntrinsics, SetAtOffsetProps,
        SetResult, String, TryGetResult, TryHasResult, TryResult, Value,
    },
    engine::{Bindable, GcScope, HeapRootData, NoGcScope, bindable_handle},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

/// ### [20.2.4 Function Instances](https://tc39.es/ecma262/#sec-function-instances)
///
/// Every Function instance is an ECMAScript [function object](https://tc39.es/ecma262/#function-object)
/// and has the internal slots listed in [Table 28](https://tc39.es/ecma262/#table-internal-slots-of-ecmascript-function-objects).
/// Function objects created using the `Function.prototype.bind` method ([20.2.3.2](https://tc39.es/ecma262/#sec-function.prototype.bind))
/// have the internal slots listed in [Table 29](https://tc39.es/ecma262/#table-internal-slots-of-bound-function-exotic-objects).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Function<'a> {
    BoundFunction(BoundFunction<'a>) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction<'a>) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction<'a>) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction(BuiltinConstructorFunction<'a>) =
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction<'a>) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseFinallyFunction(BuiltinPromiseFinallyFunction<'a>) =
        BUILTIN_PROMISE_FINALLY_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
}
bindable_handle!(Function);

impl Function<'_> {
    /// Call the function with the given arguments.
    pub fn call<'gc>(
        self,
        agent: &mut Agent,
        this_argument: Value,
        args: &mut [Value],
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        self.internal_call(
            agent,
            this_argument,
            ArgumentsList::from_mut_slice(args),
            gc,
        )
    }

    /// Returns `true` if the function can be called as a constructor.
    pub fn is_constructor(self, agent: &Agent) -> bool {
        match self {
            Function::BoundFunction(f) => f.is_constructor(agent),
            Function::BuiltinFunction(f) => f.is_constructor(agent),
            Function::ECMAScriptFunction(f) => f.is_constructor(agent),
            Function::BuiltinPromiseResolvingFunction(_) => false,
            Function::BuiltinPromiseFinallyFunction(_) => false,
            Function::BuiltinConstructorFunction(_) => true,
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    /// Returns the name of the function.
    pub fn name<'a>(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> String<'a> {
        match self {
            Function::BoundFunction(f) => f.get_name(agent).bind(gc),
            Function::BuiltinFunction(f) => f.get_name(agent).bind(gc),
            Function::ECMAScriptFunction(f) => f.get_name(agent).bind(gc),
            Function::BuiltinConstructorFunction(f) => f.get_name(agent).bind(gc),
            Function::BuiltinPromiseResolvingFunction(f) => f.get_name(agent).bind(gc),
            Function::BuiltinPromiseFinallyFunction(f) => f.get_name(agent).bind(gc),
            _ => todo!(),
        }
    }
}

impl core::fmt::Debug for Function<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BoundFunction(d) => write!(f, "BoundFunction({d:?})"),
            Self::BuiltinFunction(d) => write!(f, "BuiltinFunction({d:?})"),
            Self::ECMAScriptFunction(d) => write!(f, "ECMAScriptFunction({d:?})"),
            Self::BuiltinConstructorFunction(d) => {
                write!(f, "BuiltinConstructorFunction({d:?})")
            }
            Self::BuiltinPromiseResolvingFunction(d) => {
                write!(f, "BuiltinPromiseResolvingFunction({d:?})")
            }
            Self::BuiltinPromiseFinallyFunction(d) => {
                write!(f, "BuiltinPromiseFinallyFunction({d:?})")
            }
            Self::BuiltinPromiseCollectorFunction => todo!(),
            Self::BuiltinProxyRevokerFunction => todo!(),
        }
    }
}

impl<'a> InternalSlots<'a> for Function<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
        unreachable!("Function should not try to create backing object");
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        unreachable!("Function should not try to set backing object");
    }

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        match self {
            Function::BoundFunction(d) => d.get_backing_object(agent),
            Function::BuiltinFunction(d) => d.get_backing_object(agent),
            Function::ECMAScriptFunction(d) => d.get_backing_object(agent),
            Function::BuiltinConstructorFunction(d) => d.get_backing_object(agent),
            Function::BuiltinPromiseResolvingFunction(d) => d.get_backing_object(agent),
            Function::BuiltinPromiseFinallyFunction(d) => d.get_backing_object(agent),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_set_extensible(self, _: &mut Agent, _: bool) {
        unreachable!()
    }

    fn internal_set_prototype(self, _: &mut Agent, _: Option<Object>) {
        unreachable!()
    }
}

impl<'a> InternalMethods<'a> for Function<'a> {
    fn try_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<Object<'gc>>> {
        match self {
            Function::BoundFunction(x) => x.try_get_prototype_of(agent, gc),
            Function::BuiltinFunction(x) => x.try_get_prototype_of(agent, gc),
            Function::ECMAScriptFunction(x) => x.try_get_prototype_of(agent, gc),
            Function::BuiltinConstructorFunction(x) => x.try_get_prototype_of(agent, gc),
            Function::BuiltinPromiseResolvingFunction(x) => x.try_get_prototype_of(agent, gc),
            Function::BuiltinPromiseFinallyFunction(x) => x.try_get_prototype_of(agent, gc),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        match self {
            Function::BoundFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Function::BuiltinFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Function::ECMAScriptFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Function::BuiltinConstructorFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_set_prototype_of(agent, prototype, gc)
            }
            Function::BuiltinPromiseFinallyFunction(x) => {
                x.try_set_prototype_of(agent, prototype, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        match self {
            Function::BoundFunction(x) => x.try_is_extensible(agent, gc),
            Function::BuiltinFunction(x) => x.try_is_extensible(agent, gc),
            Function::ECMAScriptFunction(x) => x.try_is_extensible(agent, gc),
            Function::BuiltinConstructorFunction(x) => x.try_is_extensible(agent, gc),
            Function::BuiltinPromiseResolvingFunction(x) => x.try_is_extensible(agent, gc),
            Function::BuiltinPromiseFinallyFunction(x) => x.try_is_extensible(agent, gc),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        match self {
            Function::BoundFunction(x) => x.try_prevent_extensions(agent, gc),
            Function::BuiltinFunction(x) => x.try_prevent_extensions(agent, gc),
            Function::ECMAScriptFunction(x) => x.try_prevent_extensions(agent, gc),
            Function::BuiltinConstructorFunction(x) => x.try_prevent_extensions(agent, gc),
            Function::BuiltinPromiseResolvingFunction(x) => x.try_prevent_extensions(agent, gc),
            Function::BuiltinPromiseFinallyFunction(x) => x.try_prevent_extensions(agent, gc),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        match self {
            Function::BoundFunction(x) => x.try_get_own_property(agent, property_key, cache, gc),
            Function::BuiltinFunction(x) => x.try_get_own_property(agent, property_key, cache, gc),
            Function::ECMAScriptFunction(x) => {
                x.try_get_own_property(agent, property_key, cache, gc)
            }
            Function::BuiltinConstructorFunction(x) => {
                x.try_get_own_property(agent, property_key, cache, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_get_own_property(agent, property_key, cache, gc)
            }
            Function::BuiltinPromiseFinallyFunction(x) => {
                x.try_get_own_property(agent, property_key, cache, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        match self {
            Function::BoundFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, cache, gc)
            }
            Function::BuiltinFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, cache, gc)
            }
            Function::ECMAScriptFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, cache, gc)
            }
            Function::BuiltinConstructorFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, cache, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, cache, gc)
            }
            Function::BuiltinPromiseFinallyFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, cache, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryHasResult<'gc>> {
        match self {
            Function::BoundFunction(x) => x.try_has_property(agent, property_key, cache, gc),
            Function::BuiltinFunction(x) => x.try_has_property(agent, property_key, cache, gc),
            Function::ECMAScriptFunction(x) => x.try_has_property(agent, property_key, cache, gc),
            Function::BuiltinConstructorFunction(x) => {
                x.try_has_property(agent, property_key, cache, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_has_property(agent, property_key, cache, gc)
            }
            Function::BuiltinPromiseFinallyFunction(x) => {
                x.try_has_property(agent, property_key, cache, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        match self {
            Function::BoundFunction(x) => x.internal_has_property(agent, property_key, gc),
            Function::BuiltinFunction(x) => x.internal_has_property(agent, property_key, gc),
            Function::ECMAScriptFunction(x) => x.internal_has_property(agent, property_key, gc),
            Function::BuiltinConstructorFunction(x) => {
                x.internal_has_property(agent, property_key, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_has_property(agent, property_key, gc)
            }
            Function::BuiltinPromiseFinallyFunction(x) => {
                x.internal_has_property(agent, property_key, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        match self {
            Function::BoundFunction(x) => x.try_get(agent, property_key, receiver, cache, gc),
            Function::BuiltinFunction(x) => x.try_get(agent, property_key, receiver, cache, gc),
            Function::ECMAScriptFunction(x) => x.try_get(agent, property_key, receiver, cache, gc),
            Function::BuiltinConstructorFunction(x) => {
                x.try_get(agent, property_key, receiver, cache, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_get(agent, property_key, receiver, cache, gc)
            }
            Function::BuiltinPromiseFinallyFunction(x) => {
                x.try_get(agent, property_key, receiver, cache, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        match self {
            Function::BoundFunction(x) => x.internal_get(agent, property_key, receiver, gc),
            Function::BuiltinFunction(x) => x.internal_get(agent, property_key, receiver, gc),
            Function::ECMAScriptFunction(x) => x.internal_get(agent, property_key, receiver, gc),
            Function::BuiltinConstructorFunction(x) => {
                x.internal_get(agent, property_key, receiver, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_get(agent, property_key, receiver, gc)
            }
            Function::BuiltinPromiseFinallyFunction(x) => {
                x.internal_get(agent, property_key, receiver, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        match self {
            Function::BoundFunction(x) => {
                x.try_set(agent, property_key, value, receiver, cache, gc)
            }
            Function::BuiltinFunction(x) => {
                x.try_set(agent, property_key, value, receiver, cache, gc)
            }
            Function::ECMAScriptFunction(x) => {
                x.try_set(agent, property_key, value, receiver, cache, gc)
            }
            Function::BuiltinConstructorFunction(x) => {
                x.try_set(agent, property_key, value, receiver, cache, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_set(agent, property_key, value, receiver, cache, gc)
            }
            Function::BuiltinPromiseFinallyFunction(x) => {
                x.try_set(agent, property_key, value, receiver, cache, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        match self {
            Function::BoundFunction(x) => x.internal_set(agent, property_key, value, receiver, gc),
            Function::BuiltinFunction(x) => {
                x.internal_set(agent, property_key, value, receiver, gc)
            }
            Function::ECMAScriptFunction(x) => {
                x.internal_set(agent, property_key, value, receiver, gc)
            }
            Function::BuiltinConstructorFunction(x) => {
                x.internal_set(agent, property_key, value, receiver, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_set(agent, property_key, value, receiver, gc)
            }
            Function::BuiltinPromiseFinallyFunction(x) => {
                x.internal_set(agent, property_key, value, receiver, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        match self {
            Function::BoundFunction(x) => x.try_delete(agent, property_key, gc),
            Function::BuiltinFunction(x) => x.try_delete(agent, property_key, gc),
            Function::ECMAScriptFunction(x) => x.try_delete(agent, property_key, gc),
            Function::BuiltinConstructorFunction(x) => x.try_delete(agent, property_key, gc),
            Function::BuiltinPromiseResolvingFunction(x) => x.try_delete(agent, property_key, gc),
            Function::BuiltinPromiseFinallyFunction(x) => x.try_delete(agent, property_key, gc),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        match self {
            Function::BoundFunction(x) => x.try_own_property_keys(agent, gc),
            Function::BuiltinFunction(x) => x.try_own_property_keys(agent, gc),
            Function::ECMAScriptFunction(x) => x.try_own_property_keys(agent, gc),
            Function::BuiltinConstructorFunction(x) => x.try_own_property_keys(agent, gc),
            Function::BuiltinPromiseResolvingFunction(x) => x.try_own_property_keys(agent, gc),
            Function::BuiltinPromiseFinallyFunction(x) => x.try_own_property_keys(agent, gc),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn get_own_property_at_offset<'gc>(
        self,
        agent: &Agent,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryGetResult<'gc> {
        match self {
            Function::BoundFunction(f) => f.get_own_property_at_offset(agent, offset, gc),
            Function::BuiltinFunction(f) => f.get_own_property_at_offset(agent, offset, gc),
            Function::ECMAScriptFunction(f) => f.get_own_property_at_offset(agent, offset, gc),
            Function::BuiltinConstructorFunction(f) => {
                f.get_own_property_at_offset(agent, offset, gc)
            }
            Function::BuiltinPromiseResolvingFunction(f) => {
                f.get_own_property_at_offset(agent, offset, gc)
            }
            Function::BuiltinPromiseFinallyFunction(f) => {
                f.get_own_property_at_offset(agent, offset, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn set_at_offset<'gc>(
        self,
        agent: &mut Agent,
        props: &SetAtOffsetProps,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        match self {
            Function::BoundFunction(f) => f.set_at_offset(agent, props, offset, gc),
            Function::BuiltinFunction(f) => f.set_at_offset(agent, props, offset, gc),
            Function::ECMAScriptFunction(f) => f.set_at_offset(agent, props, offset, gc),
            Function::BuiltinConstructorFunction(f) => f.set_at_offset(agent, props, offset, gc),
            Function::BuiltinPromiseResolvingFunction(f) => {
                f.set_at_offset(agent, props, offset, gc)
            }
            Function::BuiltinPromiseFinallyFunction(f) => f.set_at_offset(agent, props, offset, gc),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_call<'gc>(
        self,
        agent: &mut Agent,
        this_argument: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        match self {
            Function::BoundFunction(x) => x.internal_call(agent, this_argument, arguments, gc),
            Function::BuiltinFunction(x) => x.internal_call(agent, this_argument, arguments, gc),
            Function::ECMAScriptFunction(x) => x.internal_call(agent, this_argument, arguments, gc),
            Function::BuiltinConstructorFunction(x) => {
                x.internal_call(agent, this_argument, arguments, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_call(agent, this_argument, arguments, gc)
            }
            Function::BuiltinPromiseFinallyFunction(x) => {
                x.internal_call(agent, this_argument, arguments, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments: ArgumentsList,
        new_target: Function,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        match self {
            Function::BoundFunction(x) => x.internal_construct(agent, arguments, new_target, gc),
            Function::BuiltinFunction(x) => x.internal_construct(agent, arguments, new_target, gc),
            Function::ECMAScriptFunction(x) => {
                x.internal_construct(agent, arguments, new_target, gc)
            }
            Function::BuiltinConstructorFunction(x) => {
                x.internal_construct(agent, arguments, new_target, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_construct(agent, arguments, new_target, gc)
            }
            Function::BuiltinPromiseFinallyFunction(x) => {
                x.internal_construct(agent, arguments, new_target, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }
}

impl HeapMarkAndSweep for Function<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Function::BoundFunction(x) => x.mark_values(queues),
            Function::BuiltinFunction(x) => x.mark_values(queues),
            Function::ECMAScriptFunction(x) => x.mark_values(queues),
            Function::BuiltinConstructorFunction(x) => x.mark_values(queues),
            Function::BuiltinPromiseResolvingFunction(x) => x.mark_values(queues),
            Function::BuiltinPromiseFinallyFunction(x) => x.mark_values(queues),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Function::BoundFunction(x) => x.sweep_values(compactions),
            Function::BuiltinFunction(x) => x.sweep_values(compactions),
            Function::ECMAScriptFunction(x) => x.sweep_values(compactions),
            Function::BuiltinConstructorFunction(x) => x.sweep_values(compactions),
            Function::BuiltinPromiseResolvingFunction(x) => x.sweep_values(compactions),
            Function::BuiltinPromiseFinallyFunction(x) => x.sweep_values(compactions),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }
}

/// === OUTPUT OF object_handle! MACRO ADAPTED FOR Function ===
impl<'a> From<Function<'a>> for Value<'a> {
    #[inline(always)]
    fn from(value: Function<'a>) -> Self {
        match value {
            Function::BoundFunction(d) => Self::BoundFunction(d),
            Function::BuiltinFunction(d) => Self::BuiltinFunction(d),
            Function::ECMAScriptFunction(d) => Self::ECMAScriptFunction(d),
            Function::BuiltinConstructorFunction(d) => Self::BuiltinConstructorFunction(d),
            Function::BuiltinPromiseResolvingFunction(d) => {
                Self::BuiltinPromiseResolvingFunction(d)
            }
            Function::BuiltinPromiseFinallyFunction(d) => Self::BuiltinPromiseFinallyFunction(d),
            Function::BuiltinPromiseCollectorFunction => Self::BuiltinPromiseCollectorFunction,
            Function::BuiltinProxyRevokerFunction => Self::BuiltinProxyRevokerFunction,
        }
    }
}
impl<'a> From<Function<'a>> for HeapRootData {
    #[inline(always)]
    fn from(value: Function<'a>) -> Self {
        match value {
            Function::BoundFunction(d) => Self::from(d),
            Function::BuiltinFunction(d) => Self::from(d),
            Function::ECMAScriptFunction(d) => Self::from(d),
            Function::BuiltinConstructorFunction(d) => Self::from(d),
            Function::BuiltinPromiseResolvingFunction(d) => Self::from(d),
            Function::BuiltinPromiseFinallyFunction(d) => Self::from(d),
            Function::BuiltinPromiseCollectorFunction => Self::BuiltinPromiseCollectorFunction,
            Function::BuiltinProxyRevokerFunction => Self::BuiltinProxyRevokerFunction,
        }
    }
}
impl<'a> TryFrom<Value<'a>> for Function<'a> {
    type Error = ();
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::BoundFunction(d) => Ok(Self::BoundFunction(d)),
            Value::BuiltinFunction(d) => Ok(Self::BuiltinFunction(d)),
            Value::ECMAScriptFunction(d) => Ok(Self::ECMAScriptFunction(d)),
            Value::BuiltinConstructorFunction(data) => Ok(Self::BuiltinConstructorFunction(data)),
            Value::BuiltinPromiseResolvingFunction(data) => {
                Ok(Self::BuiltinPromiseResolvingFunction(data))
            }
            Value::BuiltinPromiseCollectorFunction => Ok(Self::BuiltinPromiseCollectorFunction),
            Value::BuiltinProxyRevokerFunction => Ok(Self::BuiltinProxyRevokerFunction),
            _ => Err(()),
        }
    }
}
impl TryFrom<HeapRootData> for Function<'_> {
    type Error = ();
    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        match value {
            HeapRootData::BoundFunction(d) => Ok(Self::BoundFunction(d)),
            HeapRootData::BuiltinFunction(d) => Ok(Self::BuiltinFunction(d)),
            HeapRootData::ECMAScriptFunction(d) => Ok(Self::ECMAScriptFunction(d)),
            HeapRootData::BuiltinConstructorFunction(data) => {
                Ok(Self::BuiltinConstructorFunction(data))
            }
            HeapRootData::BuiltinPromiseResolvingFunction(data) => {
                Ok(Self::BuiltinPromiseResolvingFunction(data))
            }
            HeapRootData::BuiltinPromiseCollectorFunction => {
                Ok(Self::BuiltinPromiseCollectorFunction)
            }
            HeapRootData::BuiltinProxyRevokerFunction => Ok(Self::BuiltinProxyRevokerFunction),
            _ => Err(()),
        }
    }
}
impl<'a> From<Function<'a>> for Object<'a> {
    fn from(value: Function<'a>) -> Self {
        match value {
            Function::BoundFunction(f) => Self::BoundFunction(f),
            Function::BuiltinFunction(f) => Self::BuiltinFunction(f),
            Function::ECMAScriptFunction(f) => Self::ECMAScriptFunction(f),
            Function::BuiltinConstructorFunction(f) => Self::BuiltinConstructorFunction(f),
            Function::BuiltinPromiseResolvingFunction(f) => {
                Self::BuiltinPromiseResolvingFunction(f)
            }
            Function::BuiltinPromiseFinallyFunction(f) => Self::BuiltinPromiseFinallyFunction(f),
            Function::BuiltinPromiseCollectorFunction => Self::BuiltinPromiseCollectorFunction,
            Function::BuiltinProxyRevokerFunction => Self::BuiltinProxyRevokerFunction,
        }
    }
}
impl<'a> TryFrom<Object<'a>> for Function<'a> {
    type Error = ();
    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::BoundFunction(d) => Ok(Self::BoundFunction(d)),
            Object::BuiltinFunction(d) => Ok(Self::BuiltinFunction(d)),
            Object::ECMAScriptFunction(d) => Ok(Self::ECMAScriptFunction(d)),
            Object::BuiltinConstructorFunction(data) => Ok(Self::BuiltinConstructorFunction(data)),
            Object::BuiltinPromiseResolvingFunction(data) => {
                Ok(Self::BuiltinPromiseResolvingFunction(data))
            }
            Object::BuiltinPromiseCollectorFunction => Ok(Self::BuiltinPromiseCollectorFunction),
            Object::BuiltinProxyRevokerFunction => Ok(Self::BuiltinProxyRevokerFunction),
            _ => Err(()),
        }
    }
}
// === END ===

macro_rules! function_handle {
    ($name: ident) => {
        crate::ecmascript::types::object_handle!($name);

        impl<'a> From<$name<'a>> for crate::ecmascript::types::Function<'a> {
            fn from(value: $name<'a>) -> Self {
                Self::$name(value)
            }
        }

        impl<'a> TryFrom<crate::ecmascript::types::Function<'a>> for $name<'a> {
            type Error = ();

            fn try_from(
                value: crate::ecmascript::types::Function<'a>,
            ) -> Result<Self, Self::Error> {
                match value {
                    crate::ecmascript::types::Function::$name(data) => Ok(data),
                    _ => Err(()),
                }
            }
        }
    };
}
pub(crate) use function_handle;
