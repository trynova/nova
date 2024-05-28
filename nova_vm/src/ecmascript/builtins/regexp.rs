use crate::ecmascript::types::OrdinaryObject;

#[derive(Debug, Clone, Copy, Default)]
pub struct RegExpHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    // _regex: RegExp,
}
