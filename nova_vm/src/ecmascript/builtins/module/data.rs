use small_string::SmallString;

use crate::{
    ecmascript::{
        execution::{ModuleEnvironmentIndex, RealmIdentifier},
        scripts_and_modules::module::ModuleIdentifier,
        types::{PropertyKey, String},
    },
    heap::indexes::{ObjectIndex, StringIndex},
};

use super::{
    abstract_module_records::ModuleRecord, cyclic_module_records::CyclicModuleRecord,
    source_text_module_records::SourceTextModuleRecord, Module,
};

#[derive(Debug)]
pub struct ModuleHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
    pub(crate) r#abstract: ModuleRecord,
    pub(crate) cyclic: CyclicModuleRecord,
    pub(crate) source_text: SourceTextModuleRecord,
    pub(crate) exports: Box<[String]>,
}
