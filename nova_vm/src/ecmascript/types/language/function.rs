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
    }, InternalMethods, IntoObject, IntoValue, Object, OrdinaryObject, InternalSlots, PropertyKey, Value
};
use crate::{
    ecmascript::{
        builtins::{
            bound_function::BoundFunction, control_abstraction_objects::promise_objects::promise_abstract_operations::promise_resolving_functions::BuiltinPromiseResolvingFunction, ArgumentsList, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction
        },
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::PropertyDescriptor,
    }, engine::rootable::{HeapRootData, HeapRootRef, Rootable}, heap::{indexes::BuiltinFunctionIndex, CompactionLists, HeapMarkAndSweep, WorkQueues}
};

pub(crate) use data::*;
pub use into_function::IntoFunction;
pub(crate) use into_function::{
    function_create_backing_object, function_internal_define_own_property,
    function_internal_delete, function_internal_get, function_internal_get_own_property,
    function_internal_has_property, function_internal_own_property_keys, function_internal_set,
    FunctionInternalProperties,
};

/// https://tc39.es/ecma262/#function-object
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Function {
    BoundFunction(BoundFunction) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunction) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunction) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
    BuiltinGeneratorFunction = BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT,
    BuiltinConstructorFunction(BuiltinConstructorFunction) =
        BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT,
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction) =
        BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT,
    BuiltinPromiseCollectorFunction = BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT,
    BuiltinProxyRevokerFunction = BUILTIN_PROXY_REVOKER_FUNCTION,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::BoundFunction(d) => write!(f, "BoundFunction({:?})", d),
            Function::BuiltinFunction(d) => write!(f, "BuiltinFunction({:?})", d),
            Function::ECMAScriptFunction(d) => write!(f, "ECMAScriptFunction({:?})", d),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(d) => {
                write!(f, "BuiltinConstructorFunction({:?})", d)
            }
            Function::BuiltinPromiseResolvingFunction(d) => {
                write!(f, "BuiltinPromiseResolvingFunction({:?})", d)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
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

impl IntoFunction for Function {
    #[inline(always)]
    fn into_function(self) -> Function {
        self
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

impl TryFrom<Value> for Function {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
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

impl From<Function> for Object {
    fn from(value: Function) -> Self {
        match value {
            Function::BoundFunction(d) => Object::from(d),
            Function::BuiltinFunction(d) => Object::from(d),
            Function::ECMAScriptFunction(d) => Object::from(d),
            Function::BuiltinGeneratorFunction => Object::BuiltinGeneratorFunction,
            Function::BuiltinConstructorFunction(data) => Object::BuiltinConstructorFunction(data),
            Function::BuiltinPromiseResolvingFunction(data) => {
                Object::BuiltinPromiseResolvingFunction(data)
            }
            Function::BuiltinPromiseCollectorFunction => Object::BuiltinPromiseCollectorFunction,
            Function::BuiltinProxyRevokerFunction => Object::BuiltinProxyRevokerFunction,
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
            Function::BuiltinConstructorFunction(data) => Value::BuiltinConstructorFunction(data),
            Function::BuiltinPromiseResolvingFunction(data) => {
                Value::BuiltinPromiseResolvingFunction(data)
            }
            Function::BuiltinPromiseCollectorFunction => Value::BuiltinPromiseCollectorFunction,
            Function::BuiltinProxyRevokerFunction => Value::BuiltinProxyRevokerFunction,
        }
    }
}

impl Function {
    pub(crate) const fn new_builtin_function(idx: BuiltinFunctionIndex) -> Self {
        Self::BuiltinFunction(BuiltinFunction(idx))
    }

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
}

impl InternalSlots for Function {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject {
        unreachable!("Function should not try to create backing object");
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject) {
        unreachable!("Function should not try to set backing object");
    }

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
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
            Function::BuiltinConstructorFunction(x) => x.internal_get_prototype_of(agent),
            Function::BuiltinPromiseResolvingFunction(x) => x.internal_get_prototype_of(agent),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
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
            Function::BuiltinConstructorFunction(x) => {
                x.internal_set_prototype_of(agent, prototype)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_set_prototype_of(agent, prototype)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Function::BoundFunction(x) => x.internal_is_extensible(agent),
            Function::BuiltinFunction(x) => x.internal_is_extensible(agent),
            Function::ECMAScriptFunction(x) => x.internal_is_extensible(agent),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.internal_is_extensible(agent),
            Function::BuiltinPromiseResolvingFunction(x) => x.internal_is_extensible(agent),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        match self {
            Function::BoundFunction(x) => x.internal_prevent_extensions(agent),
            Function::BuiltinFunction(x) => x.internal_prevent_extensions(agent),
            Function::ECMAScriptFunction(x) => x.internal_prevent_extensions(agent),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.internal_prevent_extensions(agent),
            Function::BuiltinPromiseResolvingFunction(x) => x.internal_prevent_extensions(agent),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
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
            Function::BuiltinConstructorFunction(x) => {
                x.internal_get_own_property(agent, property_key)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_get_own_property(agent, property_key)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
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
            Function::BuiltinConstructorFunction(x) => {
                x.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_define_own_property(agent, property_key, property_descriptor)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Function::BoundFunction(x) => x.internal_has_property(agent, property_key),
            Function::BuiltinFunction(x) => x.internal_has_property(agent, property_key),
            Function::ECMAScriptFunction(x) => x.internal_has_property(agent, property_key),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.internal_has_property(agent, property_key),
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_has_property(agent, property_key)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
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
            Function::BuiltinConstructorFunction(x) => {
                x.internal_get(agent, property_key, receiver)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_get(agent, property_key, receiver)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
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
            Function::BuiltinConstructorFunction(x) => {
                x.internal_set(agent, property_key, value, receiver)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_set(agent, property_key, value, receiver)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self {
            Function::BoundFunction(x) => x.internal_delete(agent, property_key),
            Function::BuiltinFunction(x) => x.internal_delete(agent, property_key),
            Function::ECMAScriptFunction(x) => x.internal_delete(agent, property_key),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.internal_delete(agent, property_key),
            Function::BuiltinPromiseResolvingFunction(x) => x.internal_delete(agent, property_key),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        match self {
            Function::BoundFunction(x) => x.internal_own_property_keys(agent),
            Function::BuiltinFunction(x) => x.internal_own_property_keys(agent),
            Function::ECMAScriptFunction(x) => x.internal_own_property_keys(agent),
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(x) => x.internal_own_property_keys(agent),
            Function::BuiltinPromiseResolvingFunction(x) => x.internal_own_property_keys(agent),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
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
            Function::BuiltinConstructorFunction(x) => {
                x.internal_call(agent, this_argument, arguments_list)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_call(agent, this_argument, arguments_list)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
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
            Function::BuiltinConstructorFunction(x) => {
                x.internal_construct(agent, arguments_list, new_target)
            }
            Function::BuiltinPromiseResolvingFunction(x) => {
                x.internal_construct(agent, arguments_list, new_target)
            }
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
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

impl Function {
    pub fn call(self, agent: &mut Agent, this_argument: Value, args: &[Value]) -> JsResult<Value> {
        self.internal_call(agent, this_argument, ArgumentsList(args))
    }
}

impl Rootable for Function {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::BoundFunction(bound_function) => Err(HeapRootData::BoundFunction(bound_function)),
            Self::BuiltinFunction(builtin_function) => {
                Err(HeapRootData::BuiltinFunction(builtin_function))
            }
            Self::ECMAScriptFunction(ecmascript_function) => {
                Err(HeapRootData::ECMAScriptFunction(ecmascript_function))
            }
            Self::BuiltinGeneratorFunction => Err(HeapRootData::BuiltinGeneratorFunction),
            Self::BuiltinConstructorFunction(builtin_constructor_function) => Err(
                HeapRootData::BuiltinConstructorFunction(builtin_constructor_function),
            ),
            Self::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => Err(
                HeapRootData::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function),
            ),
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
