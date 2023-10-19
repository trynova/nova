use std::{collections::HashMap, marker::PhantomData, num::NonZeroU32};

use super::{declarative_environment::Binding, EnvironmentIndex};
use crate::{
    heap::indexes::FunctionIndex,
    types::{Object, String, Value},
};

#[derive(Debug, Clone, Copy)]
pub struct FunctionEnvironmentIndex(NonZeroU32, PhantomData<FunctionEnvironment>);

impl FunctionEnvironmentIndex {
    pub const fn from_u32_index(value: u32) -> Self {
        assert!(value != u32::MAX);
        // SAFETY: Number is not max value and will not overflow to zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value + 1) }, PhantomData)
    }

    pub const fn from_usize_index(value: usize) -> Self {
        debug_assert!(value < u32::MAX as usize);
        Self(
            unsafe { NonZeroU32::new_unchecked(value as u32 + 1) },
            PhantomData,
        )
    }

    pub const fn into_index(self) -> usize {
        self.0.get() as usize - 1
    }
}

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
    outer_env: Option<EnvironmentIndex>,

    /// Per https://tc39.es/ecma262/#sec-the-environment-record-type-hierarchy:
    /// > A _Function Environment Record_ is a _Declarative Environment Record_ [...]
    ///
    /// The Declaration Environment Record is inlined here.
    declarative_environment: HashMap<String, Binding>,
}
