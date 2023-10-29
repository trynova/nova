use crate::{ecmascript::types::Value, heap::indexes::ObjectIndex};

#[derive(Debug, Clone)]
pub struct FunctionHeapData {
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
