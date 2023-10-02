use super::Environment;
use crate::types::{String, Value};
use std::collections::HashMap;

/// 9.1.1.1 Declarative Environment Records
/// https://tc39.es/ecma262/#sec-declarative-environment-records
#[derive(Debug)]
pub struct DeclarativeEnvironment {
    pub outer_env: Option<Environment>,
    pub bindings: HashMap<String, Binding>,
}

// TODO: Consider splitting binding this into ValueBinding
// and BindingOptions. The options can fit a u8 and are fairly
// often not needed by algorithms.
#[derive(Debug)]
pub struct Binding {
    pub value: Option<Value>,
    pub strict: bool,
    pub mutable: bool,
    pub deletable: bool,
}

impl DeclarativeEnvironment {
    /// 9.1.1.1.1 HasBinding ( N )
    /// https://tc39.es/ecma262/#sec-declarative-environment-records-hasbinding-n
    pub fn has_binding(self, name: String) -> bool {
        // 1. If envRec has a binding for N, return true.
        // 2. Return false.
        return self.bindings.contains_key(&name);
    }
}
