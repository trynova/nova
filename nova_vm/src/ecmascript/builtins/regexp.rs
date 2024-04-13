use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone, Copy)]
pub struct RegExpHeapData {
    pub(crate) object_index: ObjectIndex,
    // _regex: RegExp,
}
