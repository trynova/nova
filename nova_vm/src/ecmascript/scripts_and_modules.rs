use self::script::ScriptIdentifier;

use super::builtins::module::Module;

pub mod module;
pub mod script;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScriptOrModule {
    Script(ScriptIdentifier),
    Module(Module),
}
