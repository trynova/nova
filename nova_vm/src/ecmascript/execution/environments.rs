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

use crate::ecmascript::types::{Base, Reference, ReferencedName};

use super::{Agent, JsResult};

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
        /// An index used to access an environment from [`Environments`].
        /// Internally, we store the index in a [`NonZeroU32`] with the index
        /// plus one. This allows us to not use an empty value in storage for
        /// the zero index while still saving room for a [`None`] value when
        /// stored in an [`Option`].
        #[derive(Debug, Clone, Copy)]
        pub(crate) struct $index(NonZeroU32, PhantomData<$name>);

        impl $index {
            /// Creates a new index from a u32.
            ///
            /// ## Panics
            /// - If the value is equal to 0.
            pub(crate) const fn from_u32(value: u32) -> Self {
                assert!(value != 0);
                // SAFETY: Number is not 0 and will not overflow to zero.
                // This check is done manually to allow const context.
                Self(unsafe { NonZeroU32::new_unchecked(value) }, PhantomData)
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
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum EnvironmentIndex {
    // Leave 0 for None option
    Declarative(DeclarativeEnvironmentIndex) = 1,
    Function(FunctionEnvironmentIndex),
    Global(GlobalEnvironmentIndex),
    Object(ObjectEnvironmentIndex),
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

/// ### [9.1.2.1 GetIdentifierReference ( env, name, strict )](https://tc39.es/ecma262/#sec-getidentifierreference)
///
/// The abstract operation GetIdentifierReference takes arguments env (an
/// Environment Record or null), name (a String), and strict (a Boolean) and
/// returns either a normal completion containing a Reference Record or a throw
/// completion.
pub(crate) fn get_identifier_reference(
    agent: &mut Agent,
    env: Option<EnvironmentIndex>,
    name: &str,
    strict: bool,
) -> JsResult<Reference> {
    // 1. If env is null, then
    let Some(env) = env else {
        // a. Return the Reference Record {
        return Ok(Reference {
            // [[Base]]: UNRESOLVABLE,
            base: Base::Unresolvable,
            // [[ReferencedName]]: name,
            referenced_name: ReferencedName::String(name.into()),
            // [[Strict]]: strict,
            strict,
            // [[ThisValue]]: EMPTY
            this_value: None,
        });
        // }.
    };

    // 2. Let exists be ? env.HasBinding(name).
    let exists = match env {
        EnvironmentIndex::Declarative(index) => agent
            .heap
            .environments
            .get_declarative_environment(index)
            .has_binding(name),
        EnvironmentIndex::Function(index) => agent
            .heap
            .environments
            .get_function_environment(index)
            .has_binding(name),
        EnvironmentIndex::Global(index) => agent
            .heap
            .environments
            .get_global_environment(index)
            .has_binding(name),
        EnvironmentIndex::Object(_index) => todo!(),
    };

    // 3. If exists is true, then
    if exists {
        // a. Return the Reference Record {
        Ok(Reference {
            // [[Base]]: env,
            base: Base::Environment(env),
            // [[ReferencedName]]: name,
            referenced_name: ReferencedName::String(name.into()),
            // [[Strict]]: strict,
            strict,
            // [[ThisValue]]: EMPTY
            this_value: None,
        })
        // }.
    }
    // 4. Else,
    else {
        // a. Let outer be env.[[OuterEnv]].
        let outer = match env {
            EnvironmentIndex::Declarative(index) => {
                agent
                    .heap
                    .environments
                    .get_declarative_environment(index)
                    .outer_env
            }
            EnvironmentIndex::Function(index) => {
                agent
                    .heap
                    .environments
                    .get_function_environment(index)
                    .outer_env
            }
            EnvironmentIndex::Global(_) => None,
            EnvironmentIndex::Object(index) => {
                agent
                    .heap
                    .environments
                    .get_object_environment(index)
                    .outer_env
            }
        };

        // b. Return ? GetIdentifierReference(outer, name, strict).
        get_identifier_reference(agent, outer, name, strict)
    }
}

impl Environments {
    pub(crate) fn push_declarative_environment(
        &mut self,
        env: DeclarativeEnvironment,
    ) -> DeclarativeEnvironmentIndex {
        self.declarative.push(Some(env));
        DeclarativeEnvironmentIndex::from_u32(self.declarative.len() as u32)
    }

    pub(crate) fn push_function_environment(
        &mut self,
        env: FunctionEnvironment,
    ) -> FunctionEnvironmentIndex {
        self.function.push(Some(env));
        FunctionEnvironmentIndex::from_u32(self.function.len() as u32)
    }

    pub(crate) fn push_global_environment(
        &mut self,
        env: GlobalEnvironment,
    ) -> GlobalEnvironmentIndex {
        self.global.push(Some(env));
        GlobalEnvironmentIndex::from_u32(self.global.len() as u32)
    }

    pub(crate) fn push_object_environment(
        &mut self,
        env: ObjectEnvironment,
    ) -> ObjectEnvironmentIndex {
        self.object.push(Some(env));
        ObjectEnvironmentIndex::from_u32(self.object.len() as u32)
    }

    pub(crate) fn get_declarative_environment(
        &self,
        index: DeclarativeEnvironmentIndex,
    ) -> &DeclarativeEnvironment {
        self.declarative
            .get(index.into_index())
            .expect("DeclarativeEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("DeclarativeEnvironmentIndex pointed to a None")
    }

    pub(crate) fn get_function_environment(
        &self,
        index: FunctionEnvironmentIndex,
    ) -> &FunctionEnvironment {
        self.function
            .get(index.into_index())
            .expect("FunctionEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("FunctionEnvironmentIndex pointed to a None")
    }

    pub(crate) fn get_global_environment(
        &self,
        index: GlobalEnvironmentIndex,
    ) -> &GlobalEnvironment {
        self.global
            .get(index.into_index())
            .expect("GlobalEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("GlobalEnvironmentIndex pointed to a None")
    }

    pub(crate) fn get_global_environment_mut(
        &mut self,
        index: GlobalEnvironmentIndex,
    ) -> &mut GlobalEnvironment {
        self.global
            .get_mut(index.into_index())
            .expect("GlobalEnvironmentIndex did not match to any vector index")
            .as_mut()
            .expect("GlobalEnvironmentIndex pointed to a None")
    }

    pub(crate) fn get_object_environment(
        &self,
        index: ObjectEnvironmentIndex,
    ) -> &ObjectEnvironment {
        self.object
            .get(index.into_index())
            .expect("ObjectEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("ObjectEnvironmentIndex pointed to a None")
    }
}
