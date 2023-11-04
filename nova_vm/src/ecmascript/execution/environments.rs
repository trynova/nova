//! ### [9.1 Environment Records](https://tc39.es/ecma262/#sec-environment-records)
//!
//! Environment Record is a specification type used to define the association of
//! Identifiers to specific variables and functions, based upon the lexical
//! nesting structure of ECMAScript code. Usually an Environment Record is
//! associated with some specific syntactic structure of ECMAScript code such as
//! a FunctionDeclaration, a BlockStatement, or a Catch clause of a
//! TryStatement. Each time such code is evaluated, a new Environment Record is
//! created to record the identifier bindings that are created by that code.
//!
//! Every Environment Record has an \[\[OuterEnv\]\] field, which is either null or
//! a reference to an outer Environment Record. This is used to model the
//! logical nesting of Environment Record values. The outer reference of an
//! (inner) Environment Record is a reference to the Environment Record that
//! logically surrounds the inner Environment Record. An outer Environment
//! Record may, of course, have its own outer Environment Record. An Environment
//! Record may serve as the outer environment for multiple inner Environment
//! Records. For example, if a FunctionDeclaration contains two nested
//! FunctionDeclarations then the Environment Records of each of the nested
//! functions will have as their outer Environment Record the Environment Record
//! of the current evaluation of the surrounding function.

use std::{marker::PhantomData, num::NonZeroU32};

mod declarative_environment;
mod function_environment;
mod global_environment;
mod object_environment;
mod private_environment;

pub use declarative_environment::DeclarativeEnvironment;
pub use function_environment::FunctionEnvironment;
pub use global_environment::GlobalEnvironment;
pub use object_environment::ObjectEnvironment;
pub use private_environment::PrivateEnvironment;

/// ### [\[\[OuterEnv\]\]](https://tc39.es/ecma262/#sec-environment-records)
///
/// Every Environment Record has an \[\[OuterEnv\]\] field, which is either null
/// or a reference to an outer Environment Record. This is used to model the
/// logical nesting of Environment Record values. The outer reference of an
/// (inner) Environment Record is a reference to the Environment Record that
/// logically surrounds the inner Environment Record. An outer Environment
/// Record may, of course, have its own outer Environment Record. An Environment
/// Record may serve as the outer environment for multiple inner Environment
/// Records. For example, if a FunctionDeclaration contains two nested
/// FunctionDeclarations then the Environment Records of each of the nested
/// functions will have as their outer Environment Record the Environment Record
/// of the current evaluation of the surrounding function.
pub(super) type OuterEnv = Option<EnvironmentIndex>;

macro_rules! create_environment_index {
    ($name: ident, $index: ident) => {
        #[derive(Debug, Clone, Copy)]
        pub(crate) struct $index(NonZeroU32, PhantomData<$name>);

        impl $index {
            pub(crate) const fn from_u32_index(value: u32) -> Self {
                assert!(value != u32::MAX);
                // SAFETY: Number is not max value and will not overflow to zero.
                // This check is done manually to allow const context.
                Self(unsafe { NonZeroU32::new_unchecked(value + 1) }, PhantomData)
            }

            pub(crate) const fn from_usize_index(value: usize) -> Self {
                debug_assert!(value < u32::MAX as usize);
                Self(
                    unsafe { NonZeroU32::new_unchecked(value as u32 + 1) },
                    PhantomData,
                )
            }

            pub(crate) const fn into_index(self) -> usize {
                self.0.get() as usize - 1
            }
        }
    };
}

create_environment_index!(DeclarativeEnvironment, DeclarativeEnvironmentIndex);
create_environment_index!(FunctionEnvironment, FunctionEnvironmentIndex);
create_environment_index!(GlobalEnvironment, GlobalEnvironmentIndex);
create_environment_index!(ObjectEnvironment, ObjectEnvironmentIndex);
create_environment_index!(PrivateEnvironment, PrivateEnvironmentIndex);

/// ### [9.1.1 The Environment Record Type Hierarchy](https://tc39.es/ecma262/#sec-the-environment-record-type-hierarchy)
///
/// Environment Records can be thought of as existing in a simple
/// object-oriented hierarchy where Environment Record is an abstract class with
/// three concrete subclasses: Declarative Environment Record, Object
/// Environment Record, and Global Environment Record. Function Environment
/// Records and Module Environment Records are subclasses of Declarative
/// Environment Record.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum EnvironmentIndex {
    // Leave 0 for None option
    DeclarativeEnvironment(DeclarativeEnvironmentIndex) = 1,
    FunctionEnvironment(FunctionEnvironmentIndex),
    GlobalEnvironment(GlobalEnvironmentIndex),
    ObjectEnvironment(ObjectEnvironmentIndex),
}

#[derive(Debug)]
pub struct Environments {
    declarative: Vec<Option<DeclarativeEnvironment>>,
    function: Vec<Option<FunctionEnvironment>>,
    global: Vec<Option<GlobalEnvironment>>,
    object: Vec<Option<ObjectEnvironment>>,
}

impl Default for Environments {
    fn default() -> Self {
        Self {
            declarative: Vec::with_capacity(256),
            function: Vec::with_capacity(1024),
            global: Vec::with_capacity(1),
            object: Vec::with_capacity(1024),
        }
    }
}

impl Environments {
    pub fn push_declarative_environment(
        &mut self,
        env: DeclarativeEnvironment,
    ) -> DeclarativeEnvironmentIndex {
        self.declarative.push(Some(env));
        DeclarativeEnvironmentIndex::from_u32_index(self.declarative.len() as u32)
    }

    pub fn push_function_environment(
        &mut self,
        env: FunctionEnvironment,
    ) -> FunctionEnvironmentIndex {
        self.function.push(Some(env));
        FunctionEnvironmentIndex::from_u32_index(self.function.len() as u32)
    }

    pub fn push_global_environment(&mut self, env: GlobalEnvironment) -> GlobalEnvironmentIndex {
        self.global.push(Some(env));
        GlobalEnvironmentIndex::from_u32_index(self.global.len() as u32)
    }

    pub fn push_object_environment(&mut self, env: ObjectEnvironment) -> ObjectEnvironmentIndex {
        self.object.push(Some(env));
        ObjectEnvironmentIndex::from_u32_index(self.object.len() as u32)
    }

    pub fn get_declarative_environment(
        &self,
        index: DeclarativeEnvironmentIndex,
    ) -> &DeclarativeEnvironment {
        self.declarative
            .get(index.into_index())
            .expect("DeclarativeEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("DeclarativeEnvironmentIndex pointed to a None")
    }

    pub fn get_function_environment(
        &self,
        index: FunctionEnvironmentIndex,
    ) -> &FunctionEnvironment {
        self.function
            .get(index.into_index())
            .expect("FunctionEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("FunctionEnvironmentIndex pointed to a None")
    }

    pub fn get_global_environment(&self, index: GlobalEnvironmentIndex) -> &GlobalEnvironment {
        self.global
            .get(index.into_index())
            .expect("GlobalEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("GlobalEnvironmentIndex pointed to a None")
    }

    pub fn get_object_environment(&self, index: ObjectEnvironmentIndex) -> &ObjectEnvironment {
        self.object
            .get(index.into_index())
            .expect("ObjectEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("ObjectEnvironmentIndex pointed to a None")
    }
}
