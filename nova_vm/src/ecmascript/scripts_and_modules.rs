use self::{module::Module, script::Script};
use std::{cell::RefCell, rc::Rc};

pub mod module;
pub mod script;

#[derive(Debug, Clone)]
pub enum ScriptOrModule<'ctx, 'host> {
    Script(Rc<RefCell<Script<'ctx, 'host>>>),
    Module(Rc<RefCell<Module>>),
}
