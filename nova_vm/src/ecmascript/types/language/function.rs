mod data;

use super::{
    value::{
        BOUND_FUNCTION_DISCRIMINANT, BUILTIN_FUNCTION_DISCRIMINANT,
        ECMASCRIPT_FUNCTION_DISCRIMINANT,
    },
    InternalMethods, Object, OrdinaryObjectInternalSlots, Value,
};
use crate::{
    ecmascript::execution::{Agent, JsResult},
    heap::{
        indexes::{BoundFunctionIndex, BuiltinFunctionIndex, ECMAScriptFunctionIndex},
        GetHeapData,
    },
};

pub(crate) use data::*;

/// https://tc39.es/ecma262/#function-object
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Function {
    BoundFunction(BoundFunctionIndex) = BOUND_FUNCTION_DISCRIMINANT,
    BuiltinFunction(BuiltinFunctionIndex) = BUILTIN_FUNCTION_DISCRIMINANT,
    ECMAScriptFunction(ECMAScriptFunctionIndex) = ECMASCRIPT_FUNCTION_DISCRIMINANT,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::BoundFunction(d) => write!(f, "BoundFunction({:?})", d),
            Function::BuiltinFunction(d) => write!(f, "BuiltinFunction({:?})", d),
            Function::ECMAScriptFunction(d) => write!(f, "ECMAScriptFunction({:?})", d),
        }
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
        }
    }
}

impl From<Function> for Value {
    fn from(value: Function) -> Self {
        match value {
            Function::BoundFunction(d) => Value::BoundFunction(d),
            Function::BuiltinFunction(d) => Value::BuiltinFunction(d),
            Function::ECMAScriptFunction(d) => Value::ECMAScriptFunction(d),
        }
    }
}

impl Function {
    pub(crate) const fn new_builtin_function(idx: BuiltinFunctionIndex) -> Self {
        Self::BuiltinFunction(idx)
    }

    pub fn into_value(self) -> Value {
        self.into()
    }

    pub fn into_object(self) -> Object {
        self.into()
    }
}

impl OrdinaryObjectInternalSlots for Function {
    fn extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = match self {
            Function::BoundFunction(d) => agent.heap.get(d).object_index,
            Function::BuiltinFunction(d) => agent.heap.get(d).object_index,
            Function::ECMAScriptFunction(d) => agent.heap.get(d).object_index,
        } {
            Object::from(object_index).extensible(agent)
        } else {
            true
        }
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = match self {
            Function::BoundFunction(d) => agent.heap.get(d).object_index,
            Function::BuiltinFunction(d) => agent.heap.get(d).object_index,
            Function::ECMAScriptFunction(d) => agent.heap.get(d).object_index,
        } {
            Object::from(object_index).set_extensible(agent, value)
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
        } {
            Object::from(object_index).prototype(agent)
        } else {
            Some(agent.current_realm().intrinsics().function_prototype())
        }
    }

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = match self {
            Function::BoundFunction(d) => agent.heap.get(d).object_index,
            Function::BuiltinFunction(d) => agent.heap.get(d).object_index,
            Function::ECMAScriptFunction(d) => agent.heap.get(d).object_index,
        } {
            Object::from(object_index).set_prototype(agent, prototype)
        } else if prototype != Some(agent.current_realm().intrinsics().function_prototype()) {
            // Create function base object with custom prototype
            todo!()
        }
    }
}

impl InternalMethods for Function {
    fn get_prototype_of(
        self,
        _agent: &mut Agent,
    ) -> crate::ecmascript::execution::JsResult<Option<Object>> {
        todo!()
    }

    fn set_prototype_of(
        self,
        _agent: &mut Agent,
        _prototype: Option<Object>,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn is_extensible(self, _agent: &mut Agent) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn prevent_extensions(
        self,
        _agent: &mut Agent,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: super::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<Option<crate::ecmascript::types::PropertyDescriptor>>
    {
        todo!()
    }

    fn define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: super::PropertyKey,
        _property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn has_property(
        self,
        _agent: &mut Agent,
        _property_key: super::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn get(
        self,
        _agent: &mut Agent,
        _property_key: super::PropertyKey,
        _receiver: Value,
    ) -> crate::ecmascript::execution::JsResult<Value> {
        todo!()
    }

    fn set(
        self,
        _agent: &mut Agent,
        _property_key: super::PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn delete(
        self,
        _agent: &mut Agent,
        _property_key: super::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn own_property_keys(
        self,
        _agent: &mut Agent,
    ) -> crate::ecmascript::execution::JsResult<Vec<super::PropertyKey>> {
        todo!()
    }

    fn call(
        self,
        _agent: &mut Agent,
        _this_value: Value,
        _arguments_list: &[Value],
    ) -> JsResult<Value> {
        todo!()
    }

    fn construct(
        self,
        _agent: &mut Agent,
        _arguments_list: &[Value],
        _new_target: Function,
    ) -> JsResult<Object> {
        todo!()
    }
}
