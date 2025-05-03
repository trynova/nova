// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
pub mod into_function;

use super::{
    value::{
        BOUND_FUNCTION_DISCRIMINANT, BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_FUNCTION_DISCRIMINANT, BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
        ECMASCRIPT_FUNCTION_DISCRIMINANT,
    }, InternalMethods, Object, OrdinaryObject, InternalSlots, PropertyKey, Value, String
};
use crate::engine::{context::{ Bindable, GcScope, NoGcScope}, TryResult};
use crate::{
    ecmascript::{
        builtins::{
            bound_function::BoundFunction, control_abstraction_objects::promise_objects::promise_abstract_operations::promise_resolving_functions::BuiltinPromiseResolvingFunction, ArgumentsList, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction
        },
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::PropertyDescriptor,
    }, engine::rootable::{HeapRootData, HeapRootRef, Rootable}, heap::{CompactionLists, HeapMarkAndSweep, WorkQueues}
};

pub(crate) use data::*;
pub use into_function::IntoFunction;
pub(crate) use into_function::{
    FunctionInternalProperties, function_create_backing_object,
    function_internal_define_own_property, function_internal_delete, function_internal_get,
    function_internal_get_own_property, function_internal_has_property,
    function_internal_own_property_keys, function_internal_set, function_try_get,
    function_try_has_property, function_try_set,
};

/// https://tc39.es/ecma262/#function-object
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Function<'a> {
    BoundFunction(BoundFunction<'a>) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction<'a>) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction<'a>) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinGeneratorFunction = BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction(BuiltinConstructorFunction<'a>) =
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction<'a>) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
}

impl core::fmt::Debug for Function<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Function::BoundFunction(d) => write!(f, "BoundFunction({d:?})"),
            Function::BuiltinFunction(d) => write!(f, "BuiltinFunction({d:?})"),
            Function::ECMAScriptFunction(d) => write!(f, "ECMAScriptFunction({d:?})"),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(d) => {
                write!(f, "BuiltinConstructorFunction({d:?})")
            }
            Function::BuiltinPromiseResolvingFunction(d) => {
                write!(f, "BuiltinPromiseResolvingFunction({d:?})")
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }
}

impl<'a> From<BoundFunction<'a>> for Function<'a> {
    fn from(value: BoundFunction<'a>) -> Self {
        Function::BoundFunction(value)
    }
}

impl<'a> TryFrom<Object<'a>> for Function<'a> {
    type Error = ();
    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::BoundFunction(d) => Ok(Function::BoundFunction(d)),
            Object::BuiltinFunction(d) => Ok(Function::BuiltinFunction(d)),
            Object::ECMAScriptFunction(d) => Ok(Function::ECMAScriptFunction(d)),
            Object::BuiltinGeneratorFunction => Ok(Function::BuiltinGeneratorFunction),
            Object::BuiltinConstructorFunction(data) => {
                Ok(Function::BuiltinConstructorFunction(data))
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                Ok(Function::BuiltinPromiseResolvingFunction(data))
            }
            Object::BuiltinPromiseCollectorFunction => {
                Ok(Function::BuiltinPromiseCollectorFunction)
            }
            Object::BuiltinProxyRevokerFunction => Ok(Function::BuiltinProxyRevokerFunction),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for Function<'a> {
    type Error = ();
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::BoundFunction(d) => Ok(Function::BoundFunction(d)),
            Value::BuiltinFunction(d) => Ok(Function::BuiltinFunction(d)),
            Value::ECMAScriptFunction(d) => Ok(Function::ECMAScriptFunction(d)),
            Value::BuiltinGeneratorFunction => Ok(Function::BuiltinGeneratorFunction),
            Value::BuiltinConstructorFunction(data) => {
                Ok(Function::BuiltinConstructorFunction(data))
            }
            Value::BuiltinPromiseResolvingFunction(data) => {
                Ok(Function::BuiltinPromiseResolvingFunction(data))
            }
            Value::BuiltinPromiseCollectorFunction => Ok(Function::BuiltinPromiseCollectorFunction),
            Value::BuiltinProxyRevokerFunction => Ok(Function::BuiltinProxyRevokerFunction),
            _ => Err(()),
        }
    }
}

impl<'a> From<Function<'a>> for Object<'a> {
    fn from(value: Function<'a>) -> Self {
        match value {
            Function::BoundFunction(d) => Object::from(d),
            Function::BuiltinFunction(d) => Object::from(d),
            Function::ECMAScriptFunction(d) => Object::from(d),
            Function::BuiltinGeneratorFunction => Object::BuiltinGeneratorFunction,
            Function::BuiltinConstructorFunction(data) => {
                Object::BuiltinConstructorFunction(data.unbind())
            }
            Function::BuiltinPromiseResolvingFunction(data) => {
                Object::BuiltinPromiseResolvingFunction(data.unbind())
            }
            Function::BuiltinPromiseCollectorFunction => Object::BuiltinPromiseCollectorFunction,
            Function::BuiltinProxyRevokerFunction => Object::BuiltinProxyRevokerFunction,
        }
    }
}

impl<'a> From<Function<'a>> for Value<'a> {
    fn from(value: Function<'a>) -> Self {
        match value {
            Function::BoundFunction(d) => Value::BoundFunction(d.unbind()),
            Function::BuiltinFunction(d) => Value::BuiltinFunction(d.unbind()),
            Function::ECMAScriptFunction(d) => Value::ECMAScriptFunction(d.unbind()),
            Function::BuiltinGeneratorFunction => Value::BuiltinGeneratorFunction,
            Function::BuiltinConstructorFunction(data) => {
                Value::BuiltinConstructorFunction(data.unbind())
            }
            Function::BuiltinPromiseResolvingFunction(data) => {
                Value::BuiltinPromiseResolvingFunction(data.unbind())
            }
            Function::BuiltinPromiseCollectorFunction => Value::BuiltinPromiseCollectorFunction,
            Function::BuiltinProxyRevokerFunction => Value::BuiltinProxyRevokerFunction,
        }
    }
}

impl Function<'_> {
    pub fn is_constructor(self, agent: &Agent) -> bool {
        match self {
            Function::BoundFunction(f) => f.is_constructor(agent),
            Function::BuiltinFunction(f) => f.is_constructor(agent),
            Function::ECMAScriptFunction(f) => f.is_constructor(agent),
            Function::BuiltinPromiseResolvingFunction(_) => false,
            Function::BuiltinGeneratorFunction => todo!(),
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
            _ => todo!(),
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for Function<'_> {
    type Of<'a> = Function<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
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
            Function::BoundFunction(d) => agent[d].object_index,
            Function::BuiltinFunction(d) => agent[d].object_index,
            Function::ECMAScriptFunction(d) => agent[d].object_index,
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(d) => agent[d].object_index,
            Function::BuiltinPromiseResolvingFunction(d) => agent[d].object_index,
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_set_extensible(agent, value)
        } else if !value {
            // Create function base object and set inextensible
            todo!()
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_set_prototype(agent, prototype)
        } else if prototype
            != Some(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .function_prototype()
                    .into(),
            )
        {
            // Create function base object with custom prototype
            todo!()
        }
    }
}

impl<'a> InternalMethods<'a> for Function<'a> {
    fn try_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<Object<'gc>>> {
        match self {
            Function::BoundFunction(x) => x.try_get_prototype_of(agent, gc),
            Function::BuiltinFunction(x) => x.try_get_prototype_of(agent, gc),
            Function::ECMAScriptFunction(x) => x.try_get_prototype_of(agent, gc),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.try_get_prototype_of(agent, gc),
            Function::BuiltinPromiseResolvingFunction(x) => x.try_get_prototype_of(agent, gc),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Function::BoundFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Function::BuiltinFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Function::ECMAScriptFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_set_prototype_of(agent, prototype, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_is_extensible(self, agent: &mut Agent, gc: NoGcScope) -> TryResult<bool> {
        match self {
            Function::BoundFunction(x) => x.try_is_extensible(agent, gc),
            Function::BuiltinFunction(x) => x.try_is_extensible(agent, gc),
            Function::ECMAScriptFunction(x) => x.try_is_extensible(agent, gc),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.try_is_extensible(agent, gc),
            Function::BuiltinPromiseResolvingFunction(x) => x.try_is_extensible(agent, gc),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_prevent_extensions(self, agent: &mut Agent, gc: NoGcScope) -> TryResult<bool> {
        match self {
            Function::BoundFunction(x) => x.try_prevent_extensions(agent, gc),
            Function::BuiltinFunction(x) => x.try_prevent_extensions(agent, gc),
            Function::ECMAScriptFunction(x) => x.try_prevent_extensions(agent, gc),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.try_prevent_extensions(agent, gc),
            Function::BuiltinPromiseResolvingFunction(x) => x.try_prevent_extensions(agent, gc),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<PropertyDescriptor<'gc>>> {
        match self {
            Function::BoundFunction(x) => x.try_get_own_property(agent, property_key, gc),
            Function::BuiltinFunction(x) => x.try_get_own_property(agent, property_key, gc),
            Function::ECMAScriptFunction(x) => x.try_get_own_property(agent, property_key, gc),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => {
                x.try_get_own_property(agent, property_key, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_get_own_property(agent, property_key, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Function::BoundFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Function::BuiltinFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Function::ECMAScriptFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Function::BoundFunction(x) => x.try_has_property(agent, property_key, gc),
            Function::BuiltinFunction(x) => x.try_has_property(agent, property_key, gc),
            Function::ECMAScriptFunction(x) => x.try_has_property(agent, property_key, gc),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.try_has_property(agent, property_key, gc),
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_has_property(agent, property_key, gc)
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
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => {
                x.internal_has_property(agent, property_key, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
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
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Value<'gc>> {
        match self {
            Function::BoundFunction(x) => x.try_get(agent, property_key, receiver, gc),
            Function::BuiltinFunction(x) => x.try_get(agent, property_key, receiver, gc),
            Function::ECMAScriptFunction(x) => x.try_get(agent, property_key, receiver, gc),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.try_get(agent, property_key, receiver, gc),
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_get(agent, property_key, receiver, gc)
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
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => {
                x.internal_get(agent, property_key, receiver, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_get(agent, property_key, receiver, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Function::BoundFunction(x) => x.try_set(agent, property_key, value, receiver, gc),
            Function::BuiltinFunction(x) => x.try_set(agent, property_key, value, receiver, gc),
            Function::ECMAScriptFunction(x) => x.try_set(agent, property_key, value, receiver, gc),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => {
                x.try_set(agent, property_key, value, receiver, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.try_set(agent, property_key, value, receiver, gc)
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
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => {
                x.internal_set(agent, property_key, value, receiver, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_set(agent, property_key, value, receiver, gc)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Function::BoundFunction(x) => x.try_delete(agent, property_key, gc),
            Function::BuiltinFunction(x) => x.try_delete(agent, property_key, gc),
            Function::ECMAScriptFunction(x) => x.try_delete(agent, property_key, gc),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.try_delete(agent, property_key, gc),
            Function::BuiltinPromiseResolvingFunction(x) => x.try_delete(agent, property_key, gc),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
        match self {
            Function::BoundFunction(x) => x.try_own_property_keys(agent, gc),
            Function::BuiltinFunction(x) => x.try_own_property_keys(agent, gc),
            Function::ECMAScriptFunction(x) => x.try_own_property_keys(agent, gc),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.try_own_property_keys(agent, gc),
            Function::BuiltinPromiseResolvingFunction(x) => x.try_own_property_keys(agent, gc),
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
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => {
                x.internal_call(agent, this_argument, arguments, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
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
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => {
                x.internal_construct(agent, arguments, new_target, gc)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
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
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.mark_values(queues),
            Function::BuiltinPromiseResolvingFunction(x) => x.mark_values(queues),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Function::BoundFunction(x) => x.sweep_values(compactions),
            Function::BuiltinFunction(x) => x.sweep_values(compactions),
            Function::ECMAScriptFunction(x) => x.sweep_values(compactions),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.sweep_values(compactions),
            Function::BuiltinPromiseResolvingFunction(x) => x.sweep_values(compactions),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }
}

impl Function<'_> {
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
}

impl Rootable for Function<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::BoundFunction(d) => Err(HeapRootData::BoundFunction(d.unbind())),
            Self::BuiltinFunction(d) => Err(HeapRootData::BuiltinFunction(d.unbind())),
            Self::ECMAScriptFunction(d) => Err(HeapRootData::ECMAScriptFunction(d.unbind())),
            Self::BuiltinGeneratorFunction => Err(HeapRootData::BuiltinGeneratorFunction),
            Self::BuiltinConstructorFunction(d) => {
                Err(HeapRootData::BuiltinConstructorFunction(d.unbind()))
            }
            Self::BuiltinPromiseResolvingFunction(d) => {
                Err(HeapRootData::BuiltinPromiseResolvingFunction(d.unbind()))
            }
            Self::BuiltinPromiseCollectorFunction => {
                Err(HeapRootData::BuiltinPromiseCollectorFunction)
            }
            Self::BuiltinProxyRevokerFunction => Err(HeapRootData::BuiltinProxyRevokerFunction),
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::BoundFunction(bound_function) => {
                Some(Self::BoundFunction(bound_function))
            }
            HeapRootData::BuiltinFunction(builtin_function) => {
                Some(Self::BuiltinFunction(builtin_function))
            }
            HeapRootData::ECMAScriptFunction(ecmascript_function) => {
                Some(Self::ECMAScriptFunction(ecmascript_function))
            }
            HeapRootData::BuiltinGeneratorFunction => Some(Self::BuiltinGeneratorFunction),
            HeapRootData::BuiltinConstructorFunction(builtin_constructor_function) => Some(
                Self::BuiltinConstructorFunction(builtin_constructor_function),
            ),
            HeapRootData::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                Some(Self::BuiltinPromiseResolvingFunction(
                    builtin_promise_resolving_function,
                ))
            }
            HeapRootData::BuiltinPromiseCollectorFunction => {
                Some(Self::BuiltinPromiseCollectorFunction)
            }
            HeapRootData::BuiltinProxyRevokerFunction => Some(Self::BuiltinProxyRevokerFunction),
            // Note: We use a catch-all here as we expect function variant
            // additions to be rare.
            _ => None,
        }
    }
}
