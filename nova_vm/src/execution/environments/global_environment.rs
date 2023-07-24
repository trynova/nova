use super::{DeclarativeEnvironment, Environment, ObjectEnvironment};
use crate::types::Object;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// 9.1.1.4 Global Environment Records
/// https://tc39.es/ecma262/#sec-global-environment-records
#[derive(Debug)]
pub struct GlobalEnvironment {
    // [[ObjectRecord]]
    object_record: Rc<RefCell<ObjectEnvironment>>,

    /// [[GlobalThisValue]]
    global_this_value: Object,

    /// [[DeclarativeRecord]]
    declarative_record: Rc<RefCell<DeclarativeEnvironment>>,

    /// [[VarNames]]
    var_names: HashMap<&'static str, ()>,

    /// [[OuterEnv]]
    outer_env: Option<Environment>,
}
