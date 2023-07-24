mod intrinsics;

use super::{Agent, GlobalEnvironment};
use crate::types::Object;
use intrinsics::Intrinsics;
use std::{
    any::Any,
    cell::{RefCell, RefMut},
    rc::Rc,
};

/// 9.3 Realms
/// https://tc39.es/ecma262/#sec-code-realms
#[derive(Debug)]
pub struct Realm<'ctx, 'host> {
    pub agent: Rc<RefCell<Agent<'ctx, 'host>>>,

    // rng: Xoroshiro128,
    /// [[Intrinsics]]
    pub intrinsics: Intrinsics<'ctx, 'host>,

    /// [[GlobalObject]]
    pub global_object: Object,

    /// [[GlobalEnv]]
    pub global_env: Rc<RefCell<GlobalEnvironment>>,

    /// [[HostDefined]]
    pub host_defined: Option<Rc<RefCell<dyn Any>>>,
    // TODO: [[TemplateMap]], [[LoadedModules]]
}
