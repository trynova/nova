use crate::{ecmascript::types::Value, heap::indexes::ObjectIndex};

#[derive(Debug, Clone)]
pub struct WeakRefHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) value: Value,
    pub(crate) is_strong: bool,
}
