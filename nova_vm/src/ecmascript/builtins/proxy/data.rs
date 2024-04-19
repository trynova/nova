use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone)]
pub struct ProxyHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
