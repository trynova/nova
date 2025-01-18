// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{
    function, value::{
        BOUND_FUNCTION_DISCRIMINANT, BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_FUNCTION_DISCRIMINANT, BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
        ECMASCRIPT_FUNCTION_DISCRIMINANT,
    }, Function, InternalMethods, InternalSlots, IntoFunction, IntoObject, IntoValue, Object, OrdinaryObject, PropertyKey, Value, PROXY_DISCRIMINANT
};
use crate::{ecmascript::builtins::proxy::Proxy, engine::{context::{GcScope, NoGcScope}, Scoped, TryResult}};
use crate::{
    ecmascript::{
        builtins::{
            bound_function::BoundFunction, control_abstraction_objects::promise_objects::promise_abstract_operations::promise_resolving_functions::BuiltinPromiseResolvingFunction, ArgumentsList, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction
        },
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::PropertyDescriptor,
    }, engine::rootable::{HeapRootData, HeapRootRef, Rootable}, heap::{CompactionLists, HeapMarkAndSweep, WorkQueues}
};

/// https://tc39.es/ecma262/#function-object
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Callable<'a> {
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
    Proxy(Proxy<'a>) = PROXY_DISCRIMINANT,
}

impl std::fmt::Debug for Callable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Callable::BoundFunction(d) => write!(f, "BoundFunction({:?})", d),
            Callable::BuiltinFunction(d) => write!(f, "BuiltinFunction({:?})", d),
            Callable::ECMAScriptFunction(d) => write!(f, "ECMAScriptFunction({:?})", d),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(d) => {
                write!(f, "BuiltinConstructorFunction({:?})", d)
            }
            Callable::BuiltinPromiseResolvingFunction(d) => {
                write!(f, "BuiltinPromiseResolvingFunction({:?})", d)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(d) => write!(f, "Proxy({:?})", d),
        }
    }
}

impl IntoValue for Callable<'_> {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl<'a> IntoObject<'a> for Callable<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> IntoFunction<'a> for Callable<'a> {
    fn into_function(self) -> Function<'a> {
        match self {
            Callable::BoundFunction(bound_function) => Function::BoundFunction(bound_function),
            Callable::BuiltinFunction(builtin_function) => {
                Function::BuiltinFunction(builtin_function)
            }
            Callable::ECMAScriptFunction(ecmascript_function) => {
                Function::ECMAScriptFunction(ecmascript_function)
            }
            Callable::BuiltinGeneratorFunction => Function::BuiltinGeneratorFunction,
            Callable::BuiltinConstructorFunction(constructor_function) => {
                Function::BuiltinConstructorFunction(constructor_function)
            }
            Callable::BuiltinPromiseResolvingFunction(promise_resolving_function) => {
                Function::BuiltinPromiseResolvingFunction(promise_resolving_function)
            }
            Callable::BuiltinPromiseCollectorFunction => Function::BuiltinPromiseCollectorFunction,
            Callable::BuiltinProxyRevokerFunction => Function::BuiltinProxyRevokerFunction,
            Callable::Proxy(_) => todo!(),
        }
    }
}

impl<'a> From<BoundFunction<'a>> for Callable<'a> {
    fn from(value: BoundFunction<'a>) -> Self {
        Callable::BoundFunction(value)
    }
}

impl<'a> TryFrom<Object<'a>> for Callable<'a> {
    type Error = ();
    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::BoundFunction(d) => Ok(Callable::BoundFunction(d)),
            Object::BuiltinFunction(d) => Ok(Callable::BuiltinFunction(d)),
            Object::ECMAScriptFunction(d) => Ok(Callable::ECMAScriptFunction(d)),
            Object::BuiltinGeneratorFunction => Ok(Callable::BuiltinGeneratorFunction),
            Object::BuiltinConstructorFunction(data) => {
                Ok(Callable::BuiltinConstructorFunction(data))
            }
            Object::BuiltinPromiseResolvingFunction(data) => {
                Ok(Callable::BuiltinPromiseResolvingFunction(data))
            }
            Object::BuiltinPromiseCollectorFunction => {
                Ok(Callable::BuiltinPromiseCollectorFunction)
            }
            Object::BuiltinProxyRevokerFunction => Ok(Callable::BuiltinProxyRevokerFunction),
            Object::Proxy(d) => Ok(Callable::Proxy(d)),
            _ => Err(()),
        }
    }
}

impl TryFrom<Value> for Callable<'_> {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::BoundFunction(d) => Ok(Callable::BoundFunction(d)),
            Value::BuiltinFunction(d) => Ok(Callable::BuiltinFunction(d)),
            Value::ECMAScriptFunction(d) => Ok(Callable::ECMAScriptFunction(d)),
            Value::BuiltinGeneratorFunction => Ok(Callable::BuiltinGeneratorFunction),
            Value::BuiltinConstructorFunction(data) => {
                Ok(Callable::BuiltinConstructorFunction(data))
            }
            Value::BuiltinPromiseResolvingFunction(data) => {
                Ok(Callable::BuiltinPromiseResolvingFunction(data))
            }
            Value::BuiltinPromiseCollectorFunction => Ok(Callable::BuiltinPromiseCollectorFunction),
            Value::BuiltinProxyRevokerFunction => Ok(Callable::BuiltinProxyRevokerFunction),
            Value::Proxy(d) => Ok(Callable::Proxy(d)),
            _ => Err(()),
        }
    }
}

impl<'a> From<Callable<'a>> for Object<'a> {
    fn from(value: Callable<'a>) -> Self {
        match value {
            Callable::BoundFunction(d) => Object::from(d),
            Callable::BuiltinFunction(d) => Object::from(d),
            Callable::ECMAScriptFunction(d) => Object::from(d),
            Callable::BuiltinGeneratorFunction => Object::BuiltinGeneratorFunction,
            Callable::BuiltinConstructorFunction(data) => {
                Object::BuiltinConstructorFunction(data.unbind())
            }
            Callable::BuiltinPromiseResolvingFunction(data) => {
                Object::BuiltinPromiseResolvingFunction(data.unbind())
            }
            Callable::BuiltinPromiseCollectorFunction => Object::BuiltinPromiseCollectorFunction,
            Callable::BuiltinProxyRevokerFunction => Object::BuiltinProxyRevokerFunction,
            Callable::Proxy(d) => Object::from(d),
        }
    }
}

impl From<Callable<'_>> for Value {
    fn from(value: Callable) -> Self {
        match value {
            Callable::BoundFunction(d) => Value::BoundFunction(d.unbind()),
            Callable::BuiltinFunction(d) => Value::BuiltinFunction(d.unbind()),
            Callable::ECMAScriptFunction(d) => Value::ECMAScriptFunction(d.unbind()),
            Callable::BuiltinGeneratorFunction => Value::BuiltinGeneratorFunction,
            Callable::BuiltinConstructorFunction(data) => {
                Value::BuiltinConstructorFunction(data.unbind())
            }
            Callable::BuiltinPromiseResolvingFunction(data) => {
                Value::BuiltinPromiseResolvingFunction(data.unbind())
            }
            Callable::BuiltinPromiseCollectorFunction => Value::BuiltinPromiseCollectorFunction,
            Callable::BuiltinProxyRevokerFunction => Value::BuiltinProxyRevokerFunction,
            Callable::Proxy(d) => Value::Proxy(d.unbind()),
        }
    }
}

impl<'a> Callable<'a> {
    /// Unbind this Callable from its current lifetime. This is necessary to
    /// use the Callable as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> Callable<'static> {
        unsafe { std::mem::transmute::<Callable<'a>, Callable<'static>>(self) }
    }

    // Bind this Callable to the garbage collection lifetime. This enables
    // Rust's borrow checker to verify that your Functions cannot not be
    // invalidated by garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let function = function.bind(&gc);
    // ```
    // to make sure that the unbound Callable cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> Callable<'gc> {
        unsafe { std::mem::transmute::<Callable<'a>, Callable<'gc>>(self) }
    }

    pub fn scope<'b>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'b>,
    ) -> Scoped<'b, Callable<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub fn is_constructor(self, agent: &Agent) -> bool {
        match self {
            Callable::BoundFunction(f) => f.is_constructor(agent),
            Callable::BuiltinFunction(f) => f.is_constructor(agent),
            Callable::ECMAScriptFunction(f) => f.is_constructor(agent),
            Callable::BuiltinPromiseResolvingFunction(_) => false,
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(_) => true,
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(_) => true,
        }
    }
}

impl<'a> InternalSlots<'a> for Callable<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
        unreachable!("Callable should not try to create backing object");
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        unreachable!("Callable should not try to set backing object");
    }

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        match self {
            Callable::BoundFunction(d) => agent[d].object_index,
            Callable::BuiltinFunction(d) => agent[d].object_index,
            Callable::ECMAScriptFunction(d) => agent[d].object_index,
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(d) => agent[d].object_index,
            Callable::BuiltinPromiseResolvingFunction(d) => agent[d].object_index,
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(_) => unreachable!(),
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
                    .current_realm()
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

impl<'a> InternalMethods<'a> for Callable<'a> {
    fn try_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<Object<'gc>>> {
        match self {
            Callable::BoundFunction(x) => x.try_get_prototype_of(agent, gc),
            Callable::BuiltinFunction(x) => x.try_get_prototype_of(agent, gc),
            Callable::ECMAScriptFunction(x) => x.try_get_prototype_of(agent, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => x.try_get_prototype_of(agent, gc),
            Callable::BuiltinPromiseResolvingFunction(x) => x.try_get_prototype_of(agent, gc),
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.try_get_prototype_of(agent, gc),
        }
    }

    fn try_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Callable::BoundFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Callable::BuiltinFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Callable::ECMAScriptFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => x.try_set_prototype_of(agent, prototype, gc),
            Callable::BuiltinPromiseResolvingFunction(x) => {
                x.try_set_prototype_of(agent, prototype, gc)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.try_set_prototype_of(agent, prototype, gc),
        }
    }

    fn try_is_extensible(self, agent: &mut Agent, gc: NoGcScope) -> TryResult<bool> {
        match self {
            Callable::BoundFunction(x) => x.try_is_extensible(agent, gc),
            Callable::BuiltinFunction(x) => x.try_is_extensible(agent, gc),
            Callable::ECMAScriptFunction(x) => x.try_is_extensible(agent, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => x.try_is_extensible(agent, gc),
            Callable::BuiltinPromiseResolvingFunction(x) => x.try_is_extensible(agent, gc),
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.try_is_extensible(agent, gc),
        }
    }

    fn try_prevent_extensions(self, agent: &mut Agent, gc: NoGcScope) -> TryResult<bool> {
        match self {
            Callable::BoundFunction(x) => x.try_prevent_extensions(agent, gc),
            Callable::BuiltinFunction(x) => x.try_prevent_extensions(agent, gc),
            Callable::ECMAScriptFunction(x) => x.try_prevent_extensions(agent, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => x.try_prevent_extensions(agent, gc),
            Callable::BuiltinPromiseResolvingFunction(x) => x.try_prevent_extensions(agent, gc),
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.try_prevent_extensions(agent, gc),
        }
    }

    fn try_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<Option<PropertyDescriptor>> {
        match self {
            Callable::BoundFunction(x) => x.try_get_own_property(agent, property_key, gc),
            Callable::BuiltinFunction(x) => x.try_get_own_property(agent, property_key, gc),
            Callable::ECMAScriptFunction(x) => x.try_get_own_property(agent, property_key, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => {
                x.try_get_own_property(agent, property_key, gc)
            }
            Callable::BuiltinPromiseResolvingFunction(x) => {
                x.try_get_own_property(agent, property_key, gc)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.try_get_own_property(agent, property_key, gc),
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
            Callable::BoundFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Callable::BuiltinFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Callable::ECMAScriptFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Callable::BuiltinPromiseResolvingFunction(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => {
                x.try_define_own_property(agent, property_key, property_descriptor, gc)
            }
        }
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Callable::BoundFunction(x) => x.try_has_property(agent, property_key, gc),
            Callable::BuiltinFunction(x) => x.try_has_property(agent, property_key, gc),
            Callable::ECMAScriptFunction(x) => x.try_has_property(agent, property_key, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => x.try_has_property(agent, property_key, gc),
            Callable::BuiltinPromiseResolvingFunction(x) => {
                x.try_has_property(agent, property_key, gc)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.try_has_property(agent, property_key, gc),
        }
    }

    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope,
    ) -> JsResult<bool> {
        match self {
            Callable::BoundFunction(x) => x.internal_has_property(agent, property_key, gc),
            Callable::BuiltinFunction(x) => x.internal_has_property(agent, property_key, gc),
            Callable::ECMAScriptFunction(x) => x.internal_has_property(agent, property_key, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => {
                x.internal_has_property(agent, property_key, gc)
            }
            Callable::BuiltinPromiseResolvingFunction(x) => {
                x.internal_has_property(agent, property_key, gc)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.internal_has_property(agent, property_key, gc),
        }
    }

    fn try_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<Value> {
        match self {
            Callable::BoundFunction(x) => x.try_get(agent, property_key, receiver, gc),
            Callable::BuiltinFunction(x) => x.try_get(agent, property_key, receiver, gc),
            Callable::ECMAScriptFunction(x) => x.try_get(agent, property_key, receiver, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => x.try_get(agent, property_key, receiver, gc),
            Callable::BuiltinPromiseResolvingFunction(x) => {
                x.try_get(agent, property_key, receiver, gc)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.try_get(agent, property_key, receiver, gc),
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope,
    ) -> JsResult<Value> {
        match self {
            Callable::BoundFunction(x) => x.internal_get(agent, property_key, receiver, gc),
            Callable::BuiltinFunction(x) => x.internal_get(agent, property_key, receiver, gc),
            Callable::ECMAScriptFunction(x) => x.internal_get(agent, property_key, receiver, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => {
                x.internal_get(agent, property_key, receiver, gc)
            }
            Callable::BuiltinPromiseResolvingFunction(x) => {
                x.internal_get(agent, property_key, receiver, gc)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.internal_get(agent, property_key, receiver, gc),
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
            Callable::BoundFunction(x) => x.try_set(agent, property_key, value, receiver, gc),
            Callable::BuiltinFunction(x) => x.try_set(agent, property_key, value, receiver, gc),
            Callable::ECMAScriptFunction(x) => x.try_set(agent, property_key, value, receiver, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => {
                x.try_set(agent, property_key, value, receiver, gc)
            }
            Callable::BuiltinPromiseResolvingFunction(x) => {
                x.try_set(agent, property_key, value, receiver, gc)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.try_set(agent, property_key, value, receiver, gc),
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope,
    ) -> JsResult<bool> {
        match self {
            Callable::BoundFunction(x) => x.internal_set(agent, property_key, value, receiver, gc),
            Callable::BuiltinFunction(x) => {
                x.internal_set(agent, property_key, value, receiver, gc)
            }
            Callable::ECMAScriptFunction(x) => {
                x.internal_set(agent, property_key, value, receiver, gc)
            }
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => {
                x.internal_set(agent, property_key, value, receiver, gc)
            }
            Callable::BuiltinPromiseResolvingFunction(x) => {
                x.internal_set(agent, property_key, value, receiver, gc)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.internal_set(agent, property_key, value, receiver, gc),
        }
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Callable::BoundFunction(x) => x.try_delete(agent, property_key, gc),
            Callable::BuiltinFunction(x) => x.try_delete(agent, property_key, gc),
            Callable::ECMAScriptFunction(x) => x.try_delete(agent, property_key, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => x.try_delete(agent, property_key, gc),
            Callable::BuiltinPromiseResolvingFunction(x) => x.try_delete(agent, property_key, gc),
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.try_delete(agent, property_key, gc),
        }
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
        match self {
            Callable::BoundFunction(x) => x.try_own_property_keys(agent, gc),
            Callable::BuiltinFunction(x) => x.try_own_property_keys(agent, gc),
            Callable::ECMAScriptFunction(x) => x.try_own_property_keys(agent, gc),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => x.try_own_property_keys(agent, gc),
            Callable::BuiltinPromiseResolvingFunction(x) => x.try_own_property_keys(agent, gc),
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.try_own_property_keys(agent, gc),
        }
    }

    fn internal_call(
        self,
        agent: &mut Agent,
        this_argument: Value,
        arguments_list: ArgumentsList,
        gc: GcScope,
    ) -> JsResult<Value> {
        match self {
            Callable::BoundFunction(x) => x.internal_call(agent, this_argument, arguments_list, gc),
            Callable::BuiltinFunction(x) => {
                x.internal_call(agent, this_argument, arguments_list, gc)
            }
            Callable::ECMAScriptFunction(x) => {
                x.internal_call(agent, this_argument, arguments_list, gc)
            }
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => {
                x.internal_call(agent, this_argument, arguments_list, gc)
            }
            Callable::BuiltinPromiseResolvingFunction(x) => {
                x.internal_call(agent, this_argument, arguments_list, gc)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.internal_call(agent, this_argument, arguments_list, gc),
        }
    }

    fn internal_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: function::Function<'_>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Object<'gc>> {
        match self {
            Callable::BoundFunction(x) => {
                x.internal_construct(agent, arguments_list, new_target, gc)
            }
            Callable::BuiltinFunction(x) => {
                x.internal_construct(agent, arguments_list, new_target, gc)
            }
            Callable::ECMAScriptFunction(x) => {
                x.internal_construct(agent, arguments_list, new_target, gc)
            }
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => {
                x.internal_construct(agent, arguments_list, new_target, gc)
            }
            Callable::BuiltinPromiseResolvingFunction(x) => {
                x.internal_construct(agent, arguments_list, new_target, gc)
            }
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.internal_construct(agent, arguments_list, new_target, gc),
        }
    }
}

impl HeapMarkAndSweep for Callable<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Callable::BoundFunction(x) => x.mark_values(queues),
            Callable::BuiltinFunction(x) => x.mark_values(queues),
            Callable::ECMAScriptFunction(x) => x.mark_values(queues),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => x.mark_values(queues),
            Callable::BuiltinPromiseResolvingFunction(x) => x.mark_values(queues),
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Callable::BoundFunction(x) => x.sweep_values(compactions),
            Callable::BuiltinFunction(x) => x.sweep_values(compactions),
            Callable::ECMAScriptFunction(x) => x.sweep_values(compactions),
            Callable::BuiltinGeneratorFunction => todo!(),
            Callable::BuiltinConstructorFunction(x) => x.sweep_values(compactions),
            Callable::BuiltinPromiseResolvingFunction(x) => x.sweep_values(compactions),
            Callable::BuiltinPromiseCollectorFunction => todo!(),
            Callable::BuiltinProxyRevokerFunction => todo!(),
            Callable::Proxy(x) => x.sweep_values(compactions),
        }
    }
}

impl Callable<'_> {
    pub fn call(
        self,
        agent: &mut Agent,
        this_argument: Value,
        args: &[Value],
        gc: GcScope,
    ) -> JsResult<Value> {
        self.internal_call(agent, this_argument, ArgumentsList(args), gc)
    }
}

impl Rootable for Callable<'_> {
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
            Self::Proxy(d) => Err(HeapRootData::Proxy(d.unbind())),
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
            HeapRootData::Proxy(proxy) => Some(Self::Proxy(proxy)),
            // Note: We use a catch-all here as we expect function variant
            // additions to be rare.
            _ => None,
        }
    }
}
