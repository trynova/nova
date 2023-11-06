use self::{module::Module, script::Script};
use std::{cell::RefCell, rc::Rc};

pub mod module;
pub mod script;

#[derive(Debug)]
pub enum ScriptOrModule<'ctx, 'host> {
    Script(Script<'ctx, 'host>),
    Module(Module),
}
