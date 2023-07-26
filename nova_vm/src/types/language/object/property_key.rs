use crate::{
    types::{String, Value},
    SmallString,
};

#[derive(Debug, Clone, Copy)]
pub struct PropertyKey(Value);

impl Default for PropertyKey {
    fn default() -> Self {
        Self(Value::SmallString(SmallString::from_str_unchecked(
            "unknown",
        )))
    }
}

impl PropertyKey {
    pub(crate) fn new(value: Value) -> Self {
        debug_assert!(matches!(
            value,
            Value::Integer(_) | Value::String(_) | Value::SmallString(_)
        ));
        Self(value)
    }

    pub fn into_value(self) -> Value {
        self.0
    }
}

impl From<String> for PropertyKey {
    fn from(value: String) -> Self {
        Self(value.into_value())
    }
}

impl TryFrom<Value> for PropertyKey {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if value.is_string() || value.is_symbol() || value.is_number() {
            Ok(Self(value))
        } else {
            Err(())
        }
    }
}
