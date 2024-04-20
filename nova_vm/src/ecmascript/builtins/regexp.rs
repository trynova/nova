use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone, Copy)]
pub struct RegExpHeapData {
    pub(crate) object_index: ObjectIndex,
    // _regex: RegExp,
}

impl Default for RegExpHeapData {
    fn default() -> Self {
        Self {
            object_index: ObjectIndex::from_u32_index(0),
        }
    }
}
