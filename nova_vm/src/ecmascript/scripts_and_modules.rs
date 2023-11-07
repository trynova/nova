use self::{module::Module, script::Script};

pub mod module;
pub mod script;

#[derive(Debug)]
pub enum ScriptOrModule<'ctx, 'host> {
    Script(Script<'ctx, 'host>),
    Module(Module),
}
