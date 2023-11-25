pub mod agent;
mod default_host_hooks;
mod environments;
mod execution_context;
pub mod realm;

pub use agent::{Agent, JsResult};
pub use default_host_hooks::DefaultHostHooks;
pub(crate) use environments::{
    DeclarativeEnvironment, DeclarativeEnvironmentIndex, EnvironmentIndex, Environments,
    FunctionEnvironment, FunctionEnvironmentIndex, GlobalEnvironment, GlobalEnvironmentIndex,
    ObjectEnvironment, ObjectEnvironmentIndex, PrivateEnvironment, PrivateEnvironmentIndex,
};
pub use execution_context::{ECMAScriptCode, ExecutionContext};
pub(crate) use realm::ProtoIntrinsics;
pub use realm::{
    create_realm, initialize_host_defined_realm, set_realm_global_object, Intrinsics, Realm,
    RealmIdentifier,
};
