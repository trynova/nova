use self::{module::ModuleIdentifier, script::ScriptIdentifier};

pub mod module;
pub mod script;

#[derive(Debug, Clone, Copy)]
pub enum ScriptOrModule<'ctx, 'host> {
    Script(ScriptIdentifier<'ctx, 'host>),
    Module(ModuleIdentifier<'ctx, 'host>),
}
