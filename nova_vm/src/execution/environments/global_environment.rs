use std::collections::HashMap;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use super::declarative_environment::Binding;
use crate::heap::element_array::ElementsVector;
use crate::types::{Object, String};

#[derive(Debug, Clone, Copy)]
pub struct GlobalEnvironmentIndex(NonZeroU32, PhantomData<GlobalEnvironment>);

impl GlobalEnvironmentIndex {
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

/// 9.1.1.4 Global Environment Records
/// https://tc39.es/ecma262/#sec-global-environment-records
#[derive(Debug)]
pub struct GlobalEnvironment {
    /// [[ObjectRecord]]
    /// The Object Environment Record is inlined here.
    object_record: Object,

    /// [[GlobalThisValue]]
    global_this_value: Object,

    /// [[DeclarativeRecord]]
    /// The Declaration Environment Record is inlined here.
    declarative_record: HashMap<String, Binding>,

    /// [[VarNames]]
    var_names: ElementsVector,

    /// [[OuterEnv]]
    ///
    /// Per https://tc39.es/ecma262/#sec-the-environment-record-type-hierarchy:
    /// > A _Global Environment Record_ is used for Script global declarations. It does not have an outer environment; its \[\[OuterEnv\]\] is null.
    outer_env: (),
}

impl GlobalEnvironment {
    pub(crate) fn new(global_object: Object, this_value: Object) -> Self {
        Self {
            object_record: global_object,
            global_this_value: this_value,
            declarative_record: Default::default(),
            var_names: todo!(),
            outer_env: (),
        }
    }
}
