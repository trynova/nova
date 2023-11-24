pub mod agent;
mod default_host_hooks;
mod environments;
mod execution_context;
mod realm;

pub use agent::{Agent, JsResult};
pub(crate) use environments::{
    DeclarativeEnvironment, DeclarativeEnvironmentIndex, EnvironmentIndex, Environments,
    FunctionEnvironment, FunctionEnvironmentIndex, GlobalEnvironment, GlobalEnvironmentIndex,
    ObjectEnvironment, ObjectEnvironmentIndex, PrivateEnvironment, PrivateEnvironmentIndex,
};
pub use execution_context::{ECMAScriptCode, ExecutionContext};
pub(crate) use realm::ProtoIntrinsics;
pub use realm::{Intrinsics, Realm, RealmIdentifier};
