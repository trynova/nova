pub mod agent;
mod default_host_hooks;
mod environments;
mod execution_context;
mod realm;

pub use agent::{Agent, JsResult};
pub use default_host_hooks::DefaultHostHooks;
pub(crate) use environments::{
    new_declarative_environment, new_function_environment, DeclarativeEnvironment,
    DeclarativeEnvironmentIndex, EnvironmentIndex, Environments, FunctionEnvironment,
    FunctionEnvironmentIndex, GlobalEnvironment, GlobalEnvironmentIndex, ModuleEnvironmentIndex,
    ObjectEnvironment, ObjectEnvironmentIndex, PrivateEnvironment, PrivateEnvironmentIndex,
    ThisBindingStatus,
};
pub(crate) use execution_context::*;
pub use realm::{
    create_realm, initialize_default_realm, initialize_host_defined_realm, set_realm_global_object,
};
pub(crate) use realm::{Intrinsics, ProtoIntrinsics, Realm, RealmIdentifier};
