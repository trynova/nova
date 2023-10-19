use super::EnvironmentIndex;
use crate::types::{String, Value};
use std::{collections::HashMap, marker::PhantomData, num::NonZeroU32};

#[derive(Debug, Clone, Copy)]
pub struct DeclarativeEnvironmentIndex(NonZeroU32, PhantomData<DeclarativeEnvironment>);

impl DeclarativeEnvironmentIndex {
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

/// 9.1.1.1 Declarative Environment Records
/// https://tc39.es/ecma262/#sec-declarative-environment-records
#[derive(Debug)]
pub struct DeclarativeEnvironment {
    pub outer_env: Option<EnvironmentIndex>,
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
