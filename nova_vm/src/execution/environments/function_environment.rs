use super::{DeclarativeEnvironment, Environment};
use crate::types::Value;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub enum ThisBindingStatus {
    Lexical,
    Initialized,
    Uninitialized,
}

#[derive(Debug)]
struct ECMAScriptFunction;

/// 9.1.1.3 Function Environment Records
/// https://tc39.es/ecma262/#sec-function-environment-records
#[derive(Debug)]
pub struct FunctionEnvironment {
    /// [[ThisValue]]
    this_value: Value,

    /// [[ThisBindingStatus]]
    this_binding_status: ThisBindingStatus,

    /// [[FunctionObject]]
    function_object: ECMAScriptFunction,

    /// [[NewTarget]]
    new_target: Option<Value>,

    /// [[OuterEnv]]
    outer_env: Option<Environment>,

    // NOTE: This is how we implement the spec's inheritance of function
    //       environments.
    declarative_environment: Rc<RefCell<DeclarativeEnvironment>>,
}
