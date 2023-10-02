use crate::types::{Function, String, Value};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Debug)]
pub enum PrivateElement {
    Field(Option<Value>),
    Method(Option<Function>),
    /// Accssor(get, set)
    Accessor(Option<Function>, Option<Function>),
}

/// 9.2 PrivateEnvironment Records
/// https://tc39.es/ecma262/#sec-privateenvironment-records
#[derive(Debug)]
pub struct PrivateEnvironment {
    /// [[OuterPrivateEnvironment]]
    outer_private_environment: Option<Rc<RefCell<PrivateEnvironment>>>,

    /// [[Names]]
    names: HashMap<String, PrivateElement>, // TODO: Implement private names
}
