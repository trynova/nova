use crate::{
    ecmascript::{
        builtins::{Behaviour, ECMAScriptFunction},
        types::String,
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
    pub(crate) name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BuiltinFunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    pub(crate) initial_name: Option<String>,
    pub(crate) behaviour: Behaviour,
    pub(crate) name: Option<String>,
}

#[derive(Debug)]
pub struct ECMAScriptFunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    pub(crate) ecmascript_function: ECMAScriptFunction,
    pub(crate) name: Option<String>,
}

unsafe impl Send for ECMAScriptFunctionHeapData {}
