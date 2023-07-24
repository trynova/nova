//! 9.1 Environment Records
//! https://tc39.es/ecma262/#sec-environment-records

pub mod declarative_environment;
pub mod function_environment;
pub mod global_environment;
pub mod object_environment;
pub mod private_environment;

pub use declarative_environment::DeclarativeEnvironment;
pub use function_environment::FunctionEnvironment;
pub use global_environment::GlobalEnvironment;
pub use object_environment::ObjectEnvironment;
pub use private_environment::PrivateEnvironment;
use std::{cell::RefCell, rc::Rc};

/// 9.1.1 The Environment Record Type Hierarchy
/// https://tc39.es/ecma262/#sec-the-environment-record-type-hierarchy
#[derive(Debug)]
pub enum Environment {
    DeclarativeEnvironment(Rc<RefCell<DeclarativeEnvironment>>),
    ObjectEnvironment(Rc<RefCell<ObjectEnvironment>>),
    FunctionEnvironment(Rc<RefCell<FunctionEnvironment>>),
    GlobalEnvironment(Rc<RefCell<GlobalEnvironment>>),
}
