mod data;

use super::{InternalMethods, Object, Value};
use crate::{
    ecmascript::execution::{Agent, JsResult},
    heap::indexes::FunctionIndex,
};

pub use data::FunctionHeapData;

/// https://tc39.es/ecma262/#function-object
#[derive(Clone, Copy)]
pub struct Function(pub FunctionIndex);

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
    pub fn into_value(self) -> Value {
        self.into()
    }

    pub fn into_object(self) -> Object {
        Object::Function(self.0)
    }
}

impl InternalMethods for Function {
    fn get_prototype_of(
        agent: &mut Agent,
        object: Self,
    ) -> crate::ecmascript::execution::JsResult<Option<Object>> {
        todo!()
    }

    fn set_prototype_of(
        agent: &mut Agent,
        object: Self,
        prototype: Option<Object>,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn is_extensible(
        agent: &mut Agent,
        object: Self,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn prevent_extensions(
        agent: &mut Agent,
        object: Self,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn get_own_property(
        agent: &mut Agent,
        object: Self,
        property_key: super::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<Option<crate::ecmascript::types::PropertyDescriptor>>
    {
        todo!()
    }

    fn define_own_property(
        agent: &mut Agent,
        object: Self,
        property_key: super::PropertyKey,
        property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn has_property(
        agent: &mut Agent,
        object: Self,
        property_key: super::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn get(
        agent: &mut Agent,
        object: Self,
        property_key: super::PropertyKey,
        receiver: Value,
    ) -> crate::ecmascript::execution::JsResult<Value> {
        todo!()
    }

    fn set(
        agent: &mut Agent,
        object: Self,
        property_key: super::PropertyKey,
        value: Value,
        receiver: Value,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn delete(
        agent: &mut Agent,
        object: Self,
        property_key: super::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn own_property_keys(
        agent: &mut Agent,
        object: Self,
    ) -> crate::ecmascript::execution::JsResult<Vec<super::PropertyKey>> {
        todo!()
    }

    fn call(
        agent: &mut Agent,
        object: Self,
        this_value: Value,
        arguments_list: &[Value],
    ) -> JsResult<Value> {
        todo!()
    }

    fn construct(agent: &mut Agent, object: Self, arguments_list: &[Value]) -> JsResult<Object> {
        todo!()
    }
}
