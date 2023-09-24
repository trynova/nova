use super::{DeclarativeEnvironment, Environment};
use crate::{
    heap::indexes::FunctionIndex,
    types::{Object, Value},
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
    new_target: Option<Object>,

    /// [[OuterEnv]]
    outer_env: Option<Environment>,

    declarative_environment: DeclarativeEnvironment,
}
