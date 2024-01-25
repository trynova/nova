pub mod agent;
mod default_host_hooks;
mod environments;
mod execution_context;
mod realm;

pub use agent::{Agent, JsResult};
pub use default_host_hooks::DefaultHostHooks;
pub(crate) use environments::{
    new_function_environment, DeclarativeEnvironment, DeclarativeEnvironmentIndex,
    EnvironmentIndex, Environments, FunctionEnvironment, FunctionEnvironmentIndex,
    GlobalEnvironment, GlobalEnvironmentIndex, ObjectEnvironment, ObjectEnvironmentIndex,
    PrivateEnvironment, PrivateEnvironmentIndex, ThisBindingStatus,
};
pub(crate) use execution_context::*;
pub use realm::{create_realm, set_realm_global_object};
pub(crate) use realm::{Intrinsics, ProtoIntrinsics, Realm, RealmIdentifier};
