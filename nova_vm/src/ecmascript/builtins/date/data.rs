use crate::{ecmascript::types::OrdinaryObject};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy)]
pub struct DateHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) date: Option<SystemTime>,
}

impl DateHeapData {
    pub(crate) fn new_invalid() -> Self {
        Self {
            object_index: None,
            date: None,
        }
    }
}
