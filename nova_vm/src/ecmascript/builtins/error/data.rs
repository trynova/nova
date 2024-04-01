use crate::{
    ecmascript::{execution::agent::ExceptionType, types::String},
    heap::indexes::ObjectIndex,
};

#[derive(Debug, Clone, Copy)]
pub struct ErrorHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    kind: ExceptionType,
    message: String,
    // TODO: stack? name?
}

impl ErrorHeapData {
    pub(crate) fn new(kind: ExceptionType, message: String) -> Self {
        Self {
            object_index: None,
            kind,
            message,
        }
    }
}
