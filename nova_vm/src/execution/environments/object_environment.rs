use super::Environment;
use crate::types::Object;

/// 9.1.1.2 Object Environment Records
/// https://tc39.es/ecma262/#sec-object-environment-records
#[derive(Debug)]
pub struct ObjectEnvironment {
    /// [[BindingObject]]
    binding_object: Object,

    /// [[IsWithEnvironment]]
    is_with_environment: bool,

    /// [[OuterEnv]]
    outer_env: Option<Environment>,
}
