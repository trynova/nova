use self::{module::ModuleIdentifier, script::ScriptIdentifier};

pub mod module;
pub mod script;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScriptOrModule {
    Script(ScriptIdentifier),
    Module(ModuleIdentifier),
}
