mod data;
pub mod into_function;

use super::{
    value::{
        BOUND_FUNCTION_DISCRIMINANT, BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_FUNCTION_DISCRIMINANT, BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_REJECT_FUNCTION_DISCRIMINANT,
        BUILTIN_PROMISE_RESOLVE_FUNCTION_DISCRIMINANT, BUILTIN_PROXY_REVOKER_FUNCTION,
        ECMASCRIPT_ASYNC_FUNCTION_DISCRIMINANT, ECMASCRIPT_ASYNC_GENERATOR_FUNCTION_DISCRIMINANT,
        ECMASCRIPT_CONSTRUCTOR_FUNCTION_DISCRIMINANT, ECMASCRIPT_FUNCTION_DISCRIMINANT,
        ECMASCRIPT_GENERATOR_FUNCTION_DISCRIMINANT,
    },
    InternalMethods, IntoObject, IntoValue, Object, OrdinaryObjectInternalSlots, PropertyKey,
    Value,
};
use crate::{
    ecmascript::{
        builtins::{
            bound_function::BoundFunction, control_abstraction_objects::promise_objects::promise_abstract_operations::promise_reject_function::BuiltinPromiseRejectFunction, ArgumentsList, BuiltinFunction, ECMAScriptFunction
        },
        execution::{Agent, JsResult},
        types::PropertyDescriptor,
    },
    heap::{indexes::BuiltinFunctionIndex, CompactionLists, HeapMarkAndSweep, WorkQueues},
};

pub(crate) use data::*;
pub use into_function::IntoFunction;

/// https://tc39.es/ecma262/#function-object
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Function {
    BoundFunction(BoundFunction) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinGeneratorFunction = BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction = BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolveFunction = BUILTIN_PROMISE_RESOLVE_FUNCTION_DISCRIMINANT,
    BuiltinPromiseRejectFunction(BuiltinPromiseRejectFunction) =
        BUILTIN_PROMISE_REJECT_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
    ECMAScriptAsyncFunction = ECMASCRIPT_ASYNC_FUNCTION_DISCRIMINANT,
    ECMAScriptAsyncGeneratorFunction = ECMASCRIPT_ASYNC_GENERATOR_FUNCTION_DISCRIMINANT,
    ECMAScriptConstructorFunction = ECMASCRIPT_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    ECMAScriptGeneratorFunction = ECMASCRIPT_GENERATOR_FUNCTION_DISCRIMINANT,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::BoundFunction(d) => write!(f, "BoundFunction({:?})", d),
            Function::BuiltinFunction(d) => write!(f, "BuiltinFunction({:?})", d),
            Function::ECMAScriptFunction(d) => write!(f, "ECMAScriptFunction({:?})", d),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(d) => {
                write!(f, "BuiltinPromiseRejectFunction({:?})", d)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }
}

impl IntoValue for Function {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Function {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<BoundFunction> for Function {
    fn from(value: BoundFunction) -> Self {
        Function::BoundFunction(value)
    }
}

impl TryFrom<Object> for Function {
    type Error = ();
    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::BoundFunction(d) => Ok(Function::from(d)),
            Object::BuiltinFunction(d) => Ok(Function::from(d)),
            Object::ECMAScriptFunction(d) => Ok(Function::from(d)),
            Object::BuiltinGeneratorFunction => Ok(Function::BuiltinGeneratorFunction),
            Object::BuiltinConstructorFunction => Ok(Function::BuiltinConstructorFunction),
            Object::BuiltinPromiseResolveFunction => Ok(Function::BuiltinPromiseResolveFunction),
            Object::BuiltinPromiseRejectFunction(data) => {
                Ok(Function::BuiltinPromiseRejectFunction(data))
            }
            Object::BuiltinPromiseCollectorFunction => {
                Ok(Function::BuiltinPromiseCollectorFunction)
            }
            Object::BuiltinProxyRevokerFunction => Ok(Function::BuiltinProxyRevokerFunction),
            Object::ECMAScriptAsyncFunction => Ok(Function::ECMAScriptAsyncFunction),
            Object::ECMAScriptAsyncGeneratorFunction => {
                Ok(Function::ECMAScriptAsyncGeneratorFunction)
            }
            Object::ECMAScriptConstructorFunction => Ok(Function::ECMAScriptConstructorFunction),
            Object::ECMAScriptGeneratorFunction => Ok(Function::ECMAScriptGeneratorFunction),
            _ => Err(()),
        }
    }
}

impl TryFrom<Value> for Function {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::BoundFunction(d) => Ok(Function::from(d)),
            Value::BuiltinFunction(d) => Ok(Function::from(d)),
            Value::ECMAScriptFunction(d) => Ok(Function::from(d)),
            Value::BuiltinGeneratorFunction => Ok(Function::BuiltinGeneratorFunction),
            Value::BuiltinConstructorFunction => Ok(Function::BuiltinConstructorFunction),
            Value::BuiltinPromiseResolveFunction => Ok(Function::BuiltinPromiseResolveFunction),
            Value::BuiltinPromiseRejectFunction(data) => {
                Ok(Function::BuiltinPromiseRejectFunction(data))
            }
            Value::BuiltinPromiseCollectorFunction => Ok(Function::BuiltinPromiseCollectorFunction),
            Value::BuiltinProxyRevokerFunction => Ok(Function::BuiltinProxyRevokerFunction),
            Value::ECMAScriptAsyncFunction => Ok(Function::ECMAScriptAsyncFunction),
            Value::ECMAScriptAsyncGeneratorFunction => {
                Ok(Function::ECMAScriptAsyncGeneratorFunction)
            }
            Value::ECMAScriptConstructorFunction => Ok(Function::ECMAScriptConstructorFunction),
            Value::ECMAScriptGeneratorFunction => Ok(Function::ECMAScriptGeneratorFunction),
            _ => Err(()),
        }
    }
}

impl From<Function> for Object {
    fn from(value: Function) -> Self {
        match value {
            Function::BoundFunction(d) => Object::from(d),
            Function::BuiltinFunction(d) => Object::from(d),
            Function::ECMAScriptFunction(d) => Object::from(d),
            Function::BuiltinGeneratorFunction => Object::BuiltinGeneratorFunction,
            Function::BuiltinConstructorFunction => Object::BuiltinConstructorFunction,
            Function::BuiltinPromiseResolveFunction => Object::BuiltinPromiseResolveFunction,
            Function::BuiltinPromiseRejectFunction(data) => {
                Object::BuiltinPromiseRejectFunction(data)
            }
            Function::BuiltinPromiseCollectorFunction => Object::BuiltinPromiseCollectorFunction,
            Function::BuiltinProxyRevokerFunction => Object::BuiltinProxyRevokerFunction,
            Function::ECMAScriptAsyncFunction => Object::ECMAScriptAsyncFunction,
            Function::ECMAScriptAsyncGeneratorFunction => Object::ECMAScriptAsyncGeneratorFunction,
            Function::ECMAScriptConstructorFunction => Object::ECMAScriptConstructorFunction,
            Function::ECMAScriptGeneratorFunction => Object::ECMAScriptGeneratorFunction,
        }
    }
}

impl From<Function> for Value {
    fn from(value: Function) -> Self {
        match value {
            Function::BoundFunction(d) => Value::BoundFunction(d),
            Function::BuiltinFunction(d) => Value::BuiltinFunction(d),
            Function::ECMAScriptFunction(d) => Value::ECMAScriptFunction(d),
            Function::BuiltinGeneratorFunction => Value::BuiltinGeneratorFunction,
            Function::BuiltinConstructorFunction => Value::BuiltinConstructorFunction,
            Function::BuiltinPromiseResolveFunction => Value::BuiltinPromiseResolveFunction,
            Function::BuiltinPromiseRejectFunction(data) => {
                Value::BuiltinPromiseRejectFunction(data)
            }
            Function::BuiltinPromiseCollectorFunction => Value::BuiltinPromiseCollectorFunction,
            Function::BuiltinProxyRevokerFunction => Value::BuiltinProxyRevokerFunction,
            Function::ECMAScriptAsyncFunction => Value::ECMAScriptAsyncFunction,
            Function::ECMAScriptAsyncGeneratorFunction => Value::ECMAScriptAsyncGeneratorFunction,
            Function::ECMAScriptConstructorFunction => Value::ECMAScriptConstructorFunction,
            Function::ECMAScriptGeneratorFunction => Value::ECMAScriptGeneratorFunction,
        }
    }
}

impl Function {
    pub(crate) const fn new_builtin_function(idx: BuiltinFunctionIndex) -> Self {
        Self::BuiltinFunction(BuiltinFunction(idx))
    }
}

impl OrdinaryObjectInternalSlots for Function {
    fn internal_extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = match self {
            Function::BoundFunction(d) => agent[d].object_index,
            Function::BuiltinFunction(d) => agent[d].object_index,
            Function::ECMAScriptFunction(d) => agent[d].object_index,
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(d) => agent[d].object_index,
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        } {
            object_index.internal_extensible(agent)
        } else {
            true
        }
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = match self {
            Function::BoundFunction(d) => agent[d].object_index,
            Function::BuiltinFunction(d) => agent[d].object_index,
            Function::ECMAScriptFunction(d) => agent[d].object_index,
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(d) => agent[d].object_index,
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        } {
            object_index.internal_set_extensible(agent, value)
        } else if !value {
            // Create function base object and set inextensible
            todo!()
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = match self {
            Function::BoundFunction(d) => agent[d].object_index,
            Function::BuiltinFunction(d) => agent[d].object_index,
            Function::ECMAScriptFunction(d) => agent[d].object_index,
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(d) => agent[d].object_index,
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        } {
            object_index.internal_prototype(agent)
        } else {
            Some(
                agent
                    .current_realm()
                    .intrinsics()
                    .function_prototype()
                    .into(),
            )
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = match self {
            Function::BoundFunction(d) => agent[d].object_index,
            Function::BuiltinFunction(d) => agent[d].object_index,
            Function::ECMAScriptFunction(d) => agent[d].object_index,
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(d) => agent[d].object_index,
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        } {
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

impl InternalMethods for Function {
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        match self {
            Function::BoundFunction(x) => x.internal_get_prototype_of(agent),
            Function::BuiltinFunction(x) => x.internal_get_prototype_of(agent),
            Function::ECMAScriptFunction(x) => x.internal_get_prototype_of(agent),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => x.internal_get_prototype_of(agent),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        match self {
            Function::BoundFunction(x) => x.internal_set_prototype_of(agent, prototype),
            Function::BuiltinFunction(x) => x.internal_set_prototype_of(agent, prototype),
            Function::ECMAScriptFunction(x) => x.internal_set_prototype_of(agent, prototype),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => {
                x.internal_set_prototype_of(agent, prototype)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Function::BoundFunction(x) => x.internal_is_extensible(agent),
            Function::BuiltinFunction(x) => x.internal_is_extensible(agent),
            Function::ECMAScriptFunction(x) => x.internal_is_extensible(agent),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => x.internal_is_extensible(agent),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Function::BoundFunction(x) => x.internal_prevent_extensions(agent),
            Function::BuiltinFunction(x) => x.internal_prevent_extensions(agent),
            Function::ECMAScriptFunction(x) => x.internal_prevent_extensions(agent),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => x.internal_prevent_extensions(agent),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        match self {
            Function::BoundFunction(x) => x.internal_get_own_property(agent, property_key),
            Function::BuiltinFunction(x) => x.internal_get_own_property(agent, property_key),
            Function::ECMAScriptFunction(x) => x.internal_get_own_property(agent, property_key),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => {
                x.internal_get_own_property(agent, property_key)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        match self {
            Function::BoundFunction(x) => {
                x.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Function::BuiltinFunction(x) => {
                x.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Function::ECMAScriptFunction(x) => {
                x.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => {
                x.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Function::BoundFunction(x) => x.internal_has_property(agent, property_key),
            Function::BuiltinFunction(x) => x.internal_has_property(agent, property_key),
            Function::ECMAScriptFunction(x) => x.internal_has_property(agent, property_key),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => {
                x.internal_has_property(agent, property_key)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        match self {
            Function::BoundFunction(x) => x.internal_get(agent, property_key, receiver),
            Function::BuiltinFunction(x) => x.internal_get(agent, property_key, receiver),
            Function::ECMAScriptFunction(x) => x.internal_get(agent, property_key, receiver),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => {
                x.internal_get(agent, property_key, receiver)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        match self {
            Function::BoundFunction(x) => x.internal_set(agent, property_key, value, receiver),
            Function::BuiltinFunction(x) => x.internal_set(agent, property_key, value, receiver),
            Function::ECMAScriptFunction(x) => x.internal_set(agent, property_key, value, receiver),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => {
                x.internal_set(agent, property_key, value, receiver)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Function::BoundFunction(x) => x.internal_delete(agent, property_key),
            Function::BuiltinFunction(x) => x.internal_delete(agent, property_key),
            Function::ECMAScriptFunction(x) => x.internal_delete(agent, property_key),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => x.internal_delete(agent, property_key),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        match self {
            Function::BoundFunction(x) => x.internal_own_property_keys(agent),
            Function::BuiltinFunction(x) => x.internal_own_property_keys(agent),
            Function::ECMAScriptFunction(x) => x.internal_own_property_keys(agent),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => x.internal_own_property_keys(agent),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_call(
        self,
        agent: &mut Agent,
        this_argument: Value,
        arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        match self {
            Function::BoundFunction(x) => x.internal_call(agent, this_argument, arguments_list),
            Function::BuiltinFunction(x) => x.internal_call(agent, this_argument, arguments_list),
            Function::ECMAScriptFunction(x) => {
                x.internal_call(agent, this_argument, arguments_list)
            }
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => {
                x.internal_call(agent, this_argument, arguments_list)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn internal_construct(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
    ) -> JsResult<Object> {
        match self {
            Function::BoundFunction(x) => x.internal_construct(agent, arguments_list, new_target),
            Function::BuiltinFunction(x) => x.internal_construct(agent, arguments_list, new_target),
            Function::ECMAScriptFunction(x) => {
                x.internal_construct(agent, arguments_list, new_target)
            }
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => {
                x.internal_construct(agent, arguments_list, new_target)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }
}

impl HeapMarkAndSweep for Function {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Function::BoundFunction(x) => x.mark_values(queues),
            Function::BuiltinFunction(x) => x.mark_values(queues),
            Function::ECMAScriptFunction(x) => x.mark_values(queues),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => x.mark_values(queues),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Function::BoundFunction(x) => x.sweep_values(compactions),
            Function::BuiltinFunction(x) => x.sweep_values(compactions),
            Function::ECMAScriptFunction(x) => x.sweep_values(compactions),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction(x) => x.sweep_values(compactions),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }
}
