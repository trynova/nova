use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// 9.2 PrivateEnvironment Records
/// https://tc39.es/ecma262/#sec-privateenvironment-records
#[derive(Debug)]
pub struct PrivateEnvironment {
    /// [[OuterPrivateEnvironment]]
    outer_private_environment: Option<Rc<RefCell<PrivateEnvironment>>>,

    /// [[Names]]
    names: HashMap<&'static str, ()>, // TODO: Implement private names
}
