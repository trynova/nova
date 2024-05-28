use crate::ecmascript::types::OrdinaryObject;

#[derive(Debug, Clone)]
pub struct ProxyHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
}
