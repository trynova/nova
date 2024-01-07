use crate::{
    ecmascript::{
        builtins::{Behaviour, ECMAScriptFunction},
        types::Value,
    },
    heap::{element_array::ElementsVector, indexes::ObjectIndex},
};

use super::Function;

#[derive(Debug, Clone)]
pub struct BoundFunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) function: Function,
    pub(crate) length: u8,
    pub(crate) bound_values: ElementsVector,
}

#[derive(Debug, Clone)]
pub struct BuiltinFunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    pub(crate) initial_name: Value,
    pub(crate) behaviour: Behaviour,
    // TODO: Should name be here as an "internal slot" of sorts?
}

#[derive(Debug)]
pub struct ECMAScriptFunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    pub(crate) initial_name: Value,
    pub(crate) ecmascript_function: ECMAScriptFunction,
}

unsafe impl Send for ECMAScriptFunctionHeapData {}
