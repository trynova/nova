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
pub enum Environment {
    // Leave 0 for None option
    DeclarativeEnvironment(DeclarativeEnvironmentIndex) = 1,
    FunctionEnvironment(FunctionEnvironmentIndex),
    GlobalEnvironment(GlobalEnvironmentIndex),
    ObjectEnvironment(ObjectEnvironmentIndex),
}
