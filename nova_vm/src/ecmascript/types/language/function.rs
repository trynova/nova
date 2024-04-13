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
    InternalMethods, IntoObject, IntoValue, Object, OrdinaryObject, OrdinaryObjectInternalSlots,
    PropertyKey, Value,
};
use crate::{
    ecmascript::{
        builtins::{ArgumentsList, BuiltinFunction, ECMAScriptFunction},
        execution::{Agent, JsResult},
        types::PropertyDescriptor,
    },
    heap::{
        indexes::{BoundFunctionIndex, BuiltinFunctionIndex, ECMAScriptFunctionIndex},
        GetHeapData,
    },
};

pub(crate) use data::*;
pub use into_function::IntoFunction;

/// https://tc39.es/ecma262/#function-object
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Function {
    BoundFunction(BoundFunctionIndex) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunctionIndex) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunctionIndex) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinGeneratorFunction = BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction = BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolveFunction = BUILTIN_PROMISE_RESOLVE_FUNCTION_DISCRIMINANT,
    BuiltinPromiseRejectFunction = BUILTIN_PROMISE_REJECT_FUNCTION_DISCRIMINANT,
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
            Function::BuiltinPromiseRejectFunction => todo!(),
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

impl From<BoundFunctionIndex> for Function {
    fn from(value: BoundFunctionIndex) -> Self {
        Function::BoundFunction(value)
    }
}

impl From<BuiltinFunctionIndex> for Function {
    fn from(value: BuiltinFunctionIndex) -> Self {
        Function::BuiltinFunction(value)
    }
}

impl From<ECMAScriptFunctionIndex> for Function {
    fn from(value: ECMAScriptFunctionIndex) -> Self {
        Function::ECMAScriptFunction(value)
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
            Object::BuiltinPromiseRejectFunction => Ok(Function::BuiltinPromiseResolveFunction),
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
            Value::BuiltinPromiseRejectFunction => Ok(Function::BuiltinPromiseRejectFunction),
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
            Function::BuiltinPromiseRejectFunction => Object::BuiltinPromiseRejectFunction,
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
            Function::BuiltinPromiseRejectFunction => Value::BuiltinPromiseRejectFunction,
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
        Self::BuiltinFunction(idx)
    }
}

impl OrdinaryObjectInternalSlots for Function {
    fn extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = match self {
            Function::BoundFunction(d) => agent.heap.get(d).object_index,
            Function::BuiltinFunction(d) => agent.heap.get(d).object_index,
            Function::ECMAScriptFunction(d) => agent.heap.get(d).object_index,
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction => todo!(),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        } {
            OrdinaryObject::from(object_index).extensible(agent)
        } else {
            true
        }
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = match self {
            Function::BoundFunction(d) => agent.heap.get(d).object_index,
            Function::BuiltinFunction(d) => agent.heap.get(d).object_index,
            Function::ECMAScriptFunction(d) => agent.heap.get(d).object_index,
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction => todo!(),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        } {
            OrdinaryObject::from(object_index).set_extensible(agent, value)
        } else if !value {
            // Create function base object and set inextensible
            todo!()
        }
    }

    fn prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = match self {
            Function::BoundFunction(d) => agent.heap.get(d).object_index,
            Function::BuiltinFunction(d) => agent.heap.get(d).object_index,
            Function::ECMAScriptFunction(d) => agent.heap.get(d).object_index,
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction => todo!(),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        } {
            OrdinaryObject::from(object_index).prototype(agent)
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

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = match self {
            Function::BoundFunction(d) => agent.heap.get(d).object_index,
            Function::BuiltinFunction(d) => agent.heap.get(d).object_index,
            Function::ECMAScriptFunction(d) => agent.heap.get(d).object_index,
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction => todo!(),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        } {
            OrdinaryObject::from(object_index).set_prototype(agent, prototype)
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
    fn get_prototype_of(self, _agent: &mut Agent) -> JsResult<Option<Object>> {
        todo!()
    }

    fn set_prototype_of(self, _agent: &mut Agent, _prototype: Option<Object>) -> JsResult<bool> {
        todo!()
    }

    fn is_extensible(self, _agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn prevent_extensions(self, _agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        todo!()
    }

    fn define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        todo!()
    }

    fn has_property(self, _agent: &mut Agent, _property_key: PropertyKey) -> JsResult<bool> {
        todo!()
    }

    fn get(self, agent: &mut Agent, property_key: PropertyKey, receiver: Value) -> JsResult<Value> {
        match self {
            Function::BoundFunction(_) => todo!(),
            Function::BuiltinFunction(x) => {
                BuiltinFunction::from(x).get(agent, property_key, receiver)
            }
            Function::ECMAScriptFunction(x) => {
                ECMAScriptFunction::from(x).get(agent, property_key, receiver)
            }
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction => todo!(),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn set(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> JsResult<bool> {
        todo!()
    }

    fn delete(self, _agent: &mut Agent, _property_key: PropertyKey) -> JsResult<bool> {
        todo!()
    }

    fn own_property_keys(self, _agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        todo!()
    }

    fn call(
        self,
        agent: &mut Agent,
        this_argument: Value,
        arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        match self {
            Function::BoundFunction(_idx) => todo!(),
            Function::BuiltinFunction(idx) => {
                BuiltinFunction::from(idx).call(agent, this_argument, arguments_list)
            }
            Function::ECMAScriptFunction(idx) => {
                ECMAScriptFunction::from(idx).call(agent, this_argument, arguments_list)
            }
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction => todo!(),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }

    fn construct(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
    ) -> JsResult<Object> {
        match self {
            Function::BoundFunction(_) => todo!(),
            Function::BuiltinFunction(idx) => {
                BuiltinFunction::from(idx).construct(agent, arguments_list, new_target)
            }
            Function::ECMAScriptFunction(idx) => {
                ECMAScriptFunction::from(idx).construct(agent, arguments_list, new_target)
            }
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction => todo!(),
            Function::BuiltinPromiseResolveFunction => todo!(),
            Function::BuiltinPromiseRejectFunction => todo!(),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
            Function::ECMAScriptAsyncFunction => todo!(),
            Function::ECMAScriptAsyncGeneratorFunction => todo!(),
            Function::ECMAScriptConstructorFunction => todo!(),
            Function::ECMAScriptGeneratorFunction => todo!(),
        }
    }
}
