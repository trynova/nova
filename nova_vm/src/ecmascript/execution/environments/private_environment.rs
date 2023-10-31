use crate::ecmascript::types::{Function, String, Value};
use std::{collections::HashMap, marker::PhantomData, num::NonZeroU32};

#[derive(Debug, Clone, Copy)]
pub struct PrivateEnvironmentIndex(NonZeroU32, PhantomData<PrivateEnvironment>);

impl PrivateEnvironmentIndex {
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
