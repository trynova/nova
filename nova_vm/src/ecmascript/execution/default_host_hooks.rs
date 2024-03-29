use super::{agent::HostHooks, JsResult, Realm};
use crate::ecmascript::types::Function;

#[derive(Debug)]
pub struct DefaultHostHooks;

impl HostHooks for DefaultHostHooks {
    /// ### [19.2.1.2 HostEnsureCanCompileStrings ( calleeRealm )](https://tc39.es/ecma262/#sec-hostensurecancompilestrings)
    fn host_ensure_can_compile_strings(&self, _: &mut Realm) -> JsResult<()> {
        Ok(())
    }

    /// ### [20.2.5 HostHasSourceTextAvailable ( func )](https://tc39.es/ecma262/#sec-hosthassourcetextavailable)
    fn host_has_source_text_available(&self, _: Function) -> bool {
        // The default implementation of HostHasSourceTextAvailable is to return true.
        true
    }
}
