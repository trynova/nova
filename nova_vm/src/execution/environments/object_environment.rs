use std::{marker::PhantomData, num::NonZeroU32};

use super::Environment;
use crate::types::Object;

#[derive(Debug, Clone, Copy)]
pub struct ObjectEnvironmentIndex(NonZeroU32, PhantomData<ObjectEnvironment>);

impl ObjectEnvironmentIndex {
    pub const fn from_u32_index(value: u32) -> Self {
        assert!(value != u32::MAX);
        // SAFETY: Number is not max value and will not overflow to zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value + 1) }, PhantomData)
    }

    pub const fn into_index(self) -> usize {
        self.0.get() as usize - 1
    }
}

/// 9.1.1.2 Object Environment Records
/// https://tc39.es/ecma262/#sec-object-environment-records
#[derive(Debug)]
pub struct ObjectEnvironment {
    /// [[BindingObject]]
    binding_object: Object,

    /// [[IsWithEnvironment]]
    is_with_environment: bool,

    /// [[OuterEnv]]
    outer_env: Option<Environment>,
}
