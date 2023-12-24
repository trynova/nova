use crate::{ecmascript::types::Value, heap::indexes::ObjectIndex};

use super::Function;

#[derive(Debug, Clone)]
pub struct BuiltinFunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    pub initial_name: Value,
    // pub behaviour: Behaviour,
    // TODO: Should we create a `BoundFunctionHeapData` for an exotic object
    //       that allows setting fields and other deoptimizations?
    // pub(super) uses_arguments: bool,
    // pub(super) bound: Option<Box<[Value]>>,
    // pub(super) visible: Option<Vec<Value>>,
    // TODO: Should name be here as an "internal slot" of sorts?
}

#[derive(Debug, Clone)]
pub struct ECMAScriptFunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    pub(crate) initial_name: Value,
}

#[derive(Debug, Clone)]
pub struct BoundFunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) function: Function,
    pub(crate) length: u8,
}
