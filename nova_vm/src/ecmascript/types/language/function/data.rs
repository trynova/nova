use crate::{
    ecmascript::{
        builtins::{Behaviour, ECMAScriptFunctionObjectHeapData},
        execution::RealmIdentifier,
        types::String,
    },
    heap::{element_array::ElementsVector, indexes::ObjectIndex},
};

use super::Function;

#[derive(Debug, Clone)]
pub struct BoundFunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    pub(crate) function: Function,
    pub(crate) bound_values: ElementsVector,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BuiltinFunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    /// #### \[\[Realm]]
    /// A Realm Record that represents the realm in which the function was
    /// created.
    pub(crate) realm: RealmIdentifier,
    /// #### \[\[InitialName]]
    /// A String that is the initial name of the function. It is used by
    /// 20.2.3.5 (`Function.prototype.toString()`).
    pub(crate) initial_name: Option<String>,
    pub(crate) behaviour: Behaviour,
}

#[derive(Debug)]
pub struct ECMAScriptFunctionHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) length: u8,
    pub(crate) ecmascript_function: ECMAScriptFunctionObjectHeapData,
    pub(crate) name: Option<String>,
}

unsafe impl Send for ECMAScriptFunctionHeapData {}
