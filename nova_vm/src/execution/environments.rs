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
    pub fn push_declarative_environment<'a>(
        &'a mut self,
        env: DeclarativeEnvironment,
    ) -> DeclarativeEnvironmentIndex {
        self.declarative.push(Some(env));
        DeclarativeEnvironmentIndex::from_u32_index(self.declarative.len() as u32)
    }

    pub fn push_function_environment<'a>(
        &'a mut self,
        env: FunctionEnvironment,
    ) -> FunctionEnvironmentIndex {
        self.function.push(Some(env));
        FunctionEnvironmentIndex::from_u32_index(self.function.len() as u32)
    }

    pub fn push_global_environment<'a>(
        &'a mut self,
        env: GlobalEnvironment,
    ) -> GlobalEnvironmentIndex {
        self.global.push(Some(env));
        GlobalEnvironmentIndex::from_u32_index(self.global.len() as u32)
    }

    pub fn push_object_environment<'a>(
        &'a mut self,
        env: ObjectEnvironment,
    ) -> ObjectEnvironmentIndex {
        self.object.push(Some(env));
        ObjectEnvironmentIndex::from_u32_index(self.object.len() as u32)
    }

    pub fn get_declarative_environment<'a>(
        &'a self,
        index: DeclarativeEnvironmentIndex,
    ) -> &'a DeclarativeEnvironment {
        self.declarative
            .get(index.into_index())
            .expect("DeclarativeEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("DeclarativeEnvironmentIndex pointed to a None")
    }

    pub fn get_function_environment<'a>(
        &'a self,
        index: FunctionEnvironmentIndex,
    ) -> &'a FunctionEnvironment {
        self.function
            .get(index.into_index())
            .expect("FunctionEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("FunctionEnvironmentIndex pointed to a None")
    }

    pub fn get_global_environment<'a>(
        &'a self,
        index: GlobalEnvironmentIndex,
    ) -> &'a GlobalEnvironment {
        self.global
            .get(index.into_index())
            .expect("GlobalEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("GlobalEnvironmentIndex pointed to a None")
    }

    pub fn get_object_environment<'a>(
        &'a self,
        index: ObjectEnvironmentIndex,
    ) -> &'a ObjectEnvironment {
        self.object
            .get(index.into_index())
            .expect("ObjectEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("ObjectEnvironmentIndex pointed to a None")
    }
}
