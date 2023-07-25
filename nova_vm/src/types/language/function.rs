use super::{Object, Value};

/// https://tc39.es/ecma262/#function-object
#[derive(Clone, Copy)]
pub struct Function(Value);

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Function {
    pub(crate) fn new(value: Value) -> Self {
        Self(value)
    }

    pub fn into_value(self) -> Value {
        self.0
    }

    pub fn into_object(self) -> Object {
        Object::new(self.into_value())
    }
}
