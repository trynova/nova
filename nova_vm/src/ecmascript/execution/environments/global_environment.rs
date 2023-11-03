// use super::declarative_environment::Binding;
use crate::ecmascript::types::{Object, String};
use crate::heap::element_array::ElementsVector;
use std::collections::HashMap;

/// 9.1.1.4 Global Environment Records
/// https://tc39.es/ecma262/#sec-global-environment-records
#[derive(Debug)]
pub struct GlobalEnvironment {
    /// [[ObjectRecord]]
    /// The Object Environment Record is inlined here.
    pub object_record: Object,

    /// [[GlobalThisValue]]
    pub global_this_value: Object,

    /// [[DeclarativeRecord]]
    /// The Declaration Environment Record is inlined here.
    declarative_record: HashMap<String, &'static ()>,

    /// [[VarNames]]
    var_names: ElementsVector,

    /// [[OuterEnv]]
    ///
    /// Per https://tc39.es/ecma262/#sec-the-environment-record-type-hierarchy:
    /// > A _Global Environment Record_ is used for Script global declarations. It does not have an outer environment; its \[\[OuterEnv\]\] is null.
    outer_env: (),
}

impl GlobalEnvironment {
    pub(crate) fn new(global_object: Object, this_value: Object) -> Self {
        Self {
            object_record: global_object,
            global_this_value: this_value,
            declarative_record: Default::default(),
            var_names: todo!(),
            outer_env: (),
        }
    }
}
