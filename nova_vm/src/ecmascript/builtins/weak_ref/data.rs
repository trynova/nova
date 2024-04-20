use crate::{ecmascript::types::Value, heap::indexes::ObjectIndex};

#[derive(Debug, Clone)]
pub struct WeakRefHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) value: Value,
    pub(crate) is_strong: bool,
}

impl Default for WeakRefHeapData {
    fn default() -> Self {
        Self {
            object_index: None,
            value: Value::Undefined,
            is_strong: false,
        }
    }
}
