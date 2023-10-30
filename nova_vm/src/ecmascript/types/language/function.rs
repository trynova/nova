mod data;

use std::ops::Deref;

use super::{InternalMethods, Object, OrdinaryObjectInternalSlots, Value};
use crate::{
    ecmascript::execution::{Agent, JsResult},
    heap::{indexes::FunctionIndex, GetHeapData},
};

pub use data::FunctionHeapData;

/// https://tc39.es/ecma262/#function-object
#[derive(Clone, Copy, PartialEq)]
pub struct Function(FunctionIndex);

impl Deref for Function {
    type Target = FunctionIndex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl From<FunctionIndex> for Function {
    fn from(value: FunctionIndex) -> Self {
        Function(value)
    }
}

impl TryFrom<Object> for Function {
    type Error = ();
    fn try_from(value: Object) -> Result<Self, Self::Error> {
        if let Object::Function(value) = value {
            Ok(Function(value))
        } else {
            Err(())
        }
    }
}

impl TryFrom<Value> for Function {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Function(value) = value {
            Ok(Function(value))
        } else {
            Err(())
        }
    }
}

impl From<Function> for Object {
    fn from(value: Function) -> Self {
        Object::Function(value.0)
    }
}

impl From<Function> for Value {
    fn from(value: Function) -> Self {
        Value::Function(value.0)
    }
}

impl Function {
    pub(crate) const fn new(idx: FunctionIndex) -> Self {
        Self(idx)
    }

    pub fn into_value(self) -> Value {
        self.into()
    }

    pub fn into_object(self) -> Object {
        Object::Function(self.0)
    }
}

impl OrdinaryObjectInternalSlots for Function {
    fn extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            Object::from(object_index).extensible(agent)
        } else {
            true
        }
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            Object::from(object_index).set_extensible(agent, value)
        } else if !value {
            // Create function base object and set inextensible
            todo!()
        }
    }

    fn prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            Object::from(object_index).prototype(agent)
        } else {
            Some(agent.current_realm().intrinsics().function_prototype())
        }
    }

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            Object::from(object_index).set_prototype(agent, prototype)
        } else if prototype
            != Some(
                agent
                    .current_realm()
                    .intrinsics()
                    .function_prototype(),
            )
        {
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

    fn prevent_extensions(self, _agent: &mut Agent) -> crate::ecmascript::execution::JsResult<bool> {
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

    fn construct(self, _agent: &mut Agent, _arguments_list: &[Value]) -> JsResult<Object> {
        todo!()
    }
}
