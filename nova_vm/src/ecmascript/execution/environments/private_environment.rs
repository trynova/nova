use crate::ecmascript::types::{Function, String, Value};
use std::collections::HashMap;

use super::PrivateEnvironmentIndex;

#[derive(Debug)]
pub enum PrivateElement {
    Field(Option<Value>),
    Method(Option<Function>),
    /// Accessor(get, set)
    Accessor(Option<Function>, Option<Function>),
}

/// 9.2 PrivateEnvironment Records
/// https://tc39.es/ecma262/#sec-privateenvironment-records
#[derive(Debug)]
pub struct PrivateEnvironment {
    /// [[OuterPrivateEnvironment]]
    outer_private_environment: Option<PrivateEnvironmentIndex>,

    /// [[Names]]
    names: HashMap<String, PrivateElement>, // TODO: Implement private names
}
