use std::collections::HashMap;

use super::{declarative_environment::Binding, Environment};
use crate::{
    heap::indexes::FunctionIndex,
    types::{Object, String, Value},
};

#[derive(Debug)]
pub enum ThisBindingStatus {
    /// Function is an ArrowFunction and does not have a local `this` value.
    Lexical,
    /// Function is a normal function and does not have a bound `this` value.
    Initialized,
    /// Function is a normal function and has a bound `this` value.
    Uninitialized,
}

/// 9.1.1.3 Function Environment Records
/// https://tc39.es/ecma262/#sec-function-environment-records
#[derive(Debug)]
pub struct FunctionEnvironment {
    /// [[ThisValue]]
    this_value: Value,

    /// [[ThisBindingStatus]]
    this_binding_status: ThisBindingStatus,

    /// [[FunctionObject]]
    function_object: FunctionIndex,

    /// [[NewTarget]]
    ///
    /// If this FunctionEnvironment was created with a [[Construct]] internal method,
    /// this is the value of the _newTarget_ parameter. Otherwise, its value is **undefined**
    /// (implementation wise here None).
    new_target: Option<Object>,

    /// [[OuterEnv]]
    outer_env: Option<Environment>,

    /// Per https://tc39.es/ecma262/#sec-the-environment-record-type-hierarchy:
    /// > A _Function Environment Record_ is a _Declarative Environment Record_ [...]
    ///
    /// The Declaration Environment Record is inlined here.
    declarative_environment: HashMap<String, Binding>,
}
