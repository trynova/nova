use crate::ecmascript::types::{Function, Value};
use std::collections::HashMap;

use super::PrivateEnvironmentIndex;

#[derive(Debug)]
pub enum PrivateName {
    Field(Option<Value>),
    Method(Option<Function>),
    /// Accessor(get, set)
    Accessor(Option<Function>, Option<Function>),
}

impl PrivateName {
    pub fn description(&self) -> &'static str {
        "identifier"
    }
}

/// ### [9.2 PrivateEnvironment Records](https://tc39.es/ecma262/#sec-privateenvironment-records)
///
/// A PrivateEnvironment Record is a specification mechanism used to track
/// Private Names based upon the lexical nesting structure of ClassDeclarations
/// and ClassExpressions in ECMAScript code. They are similar to, but distinct
/// from, Environment Records. Each PrivateEnvironment Record is associated
/// with a ClassDeclaration or ClassExpression. Each time such a class is
/// evaluated, a new PrivateEnvironment Record is created to record the Private
/// Names declared by that class.
#[derive(Debug)]
pub struct PrivateEnvironment {
    /// ### \[\[OuterPrivateEnvironment\]\]
    ///
    /// The PrivateEnvironment Record of the nearest containing class. null if
    /// the class with which this PrivateEnvironment Record is associated is
    /// not contained in any other class.
    outer_private_environment: Option<PrivateEnvironmentIndex>,

    /// ### \[\[Names\]\]
    ///
    /// The Private Names declared by this class.
    names: HashMap<String, PrivateName>,
}

/// ### [9.2.1.1 NewPrivateEnvironment ( outerPrivEnv )](https://tc39.es/ecma262/#sec-newprivateenvironment)
///
/// The abstract operation NewPrivateEnvironment takes argument outerPrivEnv (a
/// PrivateEnvironment Record or null) and returns a PrivateEnvironment Record.
pub(crate) fn new_private_environment(
    outer_private_environment: Option<PrivateEnvironmentIndex>,
) -> PrivateEnvironment {
    // 1. Let names be a new empty List.
    // 2. Return the PrivateEnvironment Record {
    PrivateEnvironment {
        // [[OuterPrivateEnvironment]]: outerPrivEnv,
        outer_private_environment,
        // [[Names]]: names
        names: Default::default(),
    }
    // }.
}
