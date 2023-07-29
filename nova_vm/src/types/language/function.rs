use crate::heap::{FunctionHeapData, Handle};

use super::{Object, Value};

/// https://tc39.es/ecma262/#function-object
#[derive(Clone, Copy)]
pub struct Function(pub Handle<FunctionHeapData>);

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl From<Handle<FunctionHeapData>> for Function {
    fn from(value: Handle<FunctionHeapData>) -> Self {
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
