use crate::{ecmascript::types::String, heap::indexes::ObjectIndex};

use super::{
    abstract_module_records::ModuleRecord, cyclic_module_records::CyclicModuleRecord,
    source_text_module_records::SourceTextModuleRecord,
};

#[derive(Debug)]
pub struct ModuleHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) r#abstract: ModuleRecord,
    pub(crate) cyclic: CyclicModuleRecord,
    pub(crate) source_text: SourceTextModuleRecord,
    pub(crate) exports: Box<[String]>,
}
