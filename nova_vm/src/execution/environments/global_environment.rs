use std::collections::HashMap;

use super::declarative_environment::Binding;
use super::{DeclarativeEnvironment, Environment, ObjectEnvironment};
use crate::heap::element_array::ElementsVector;
use crate::types::{Object, String};

/// 9.1.1.4 Global Environment Records
/// https://tc39.es/ecma262/#sec-global-environment-records
#[derive(Debug)]
pub struct GlobalEnvironment {
    /// [[ObjectRecord]]
    /// The Object Environment Record is inlined here.
    object_record: Object,

    /// [[GlobalThisValue]]
    global_this_value: Object,

    /// [[DeclarativeRecord]]
    /// The Declaration Environment Record is inlined here.
    declarative_record: HashMap<String, Binding>,

    /// [[VarNames]]
    var_names: ElementsVector,

    /// [[OuterEnv]]
    ///
    /// Per https://tc39.es/ecma262/#sec-the-environment-record-type-hierarchy:
    /// > A _Global Environment Record_ is used for Script global declarations. It does not have an outer environment; its \[\[OuterEnv\]\] is null.
    outer_env: (),
}
