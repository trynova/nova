use crate::{
    ecmascript::{
        execution::agent::ExceptionType,
        types::{String, Value},
    },
    heap::indexes::ObjectIndex,
};

#[derive(Debug, Clone, Copy)]
pub struct ErrorHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) kind: ExceptionType,
    pub(crate) message: Option<String>,
    pub(crate) cause: Option<Value>,
    // TODO: stack? name?
}

impl ErrorHeapData {
    pub(crate) fn new(kind: ExceptionType, message: Option<String>, cause: Option<Value>) -> Self {
        Self {
            object_index: None,
            kind,
            message,
            cause,
        }
    }
}
