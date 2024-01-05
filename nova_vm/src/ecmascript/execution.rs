pub mod agent;
mod default_host_hooks;
mod environments;
mod execution_context;
mod realm;

pub use agent::{Agent, JsResult};
pub use default_host_hooks::DefaultHostHooks;
pub(crate) use environments::{
    DeclarativeEnvironment, DeclarativeEnvironmentIndex, EnvironmentIndex, Environments,
    FunctionEnvironment, FunctionEnvironmentIndex, GlobalEnvironment, GlobalEnvironmentIndex,
    ObjectEnvironment, ObjectEnvironmentIndex, PrivateEnvironment, PrivateEnvironmentIndex,
};
pub(crate) use execution_context::{ECMAScriptCode, ExecutionContext};
pub use realm::create_realm;
#[allow(unused_imports)]
pub(crate) use realm::{set_realm_global_object, Realm, RealmIdentifier};
pub(crate) use realm::{Intrinsics, ProtoIntrinsics};
