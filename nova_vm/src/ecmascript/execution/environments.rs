//! 9.1 Environment Records
//! https://tc39.es/ecma262/#sec-environment-records

pub mod declarative_environment;
pub mod function_environment;
pub mod global_environment;
pub mod object_environment;
pub mod private_environment;

pub use declarative_environment::{DeclarativeEnvironment, DeclarativeEnvironmentIndex};
pub use function_environment::{FunctionEnvironment, FunctionEnvironmentIndex};
pub use global_environment::{GlobalEnvironment, GlobalEnvironmentIndex};
pub use object_environment::{ObjectEnvironment, ObjectEnvironmentIndex};
pub use private_environment::{PrivateEnvironment, PrivateEnvironmentIndex};

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

/// 9.1.1 The Environment Record Type Hierarchy
/// https://tc39.es/ecma262/#sec-the-environment-record-type-hierarchy
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
