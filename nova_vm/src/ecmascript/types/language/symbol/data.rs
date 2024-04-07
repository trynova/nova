use crate::ecmascript::types::String;

#[derive(Debug, Clone, Copy)]
pub struct SymbolHeapData {
    pub(crate) descriptor: Option<String>,
}
