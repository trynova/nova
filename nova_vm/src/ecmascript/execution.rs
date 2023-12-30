pub mod agent;
mod default_host_hooks;
mod environments;
mod execution_context;
mod realm;

pub use agent::{Agent, JsResult};
pub use default_host_hooks::DefaultHostHooks;
pub(crate) use environments::{
    EnvironmentIndex, Environments, GlobalEnvironment, GlobalEnvironmentIndex,
    PrivateEnvironmentIndex,
};
pub(crate) use execution_context::{ECMAScriptCode, ExecutionContext};
pub use realm::create_realm;
pub(crate) use realm::ProtoIntrinsics;
#[allow(unused_imports)]
pub(crate) use realm::{set_realm_global_object, Realm, RealmIdentifier};
