use super::{agent::{HostHooks, PromiseRejectionOperation}, JsResult, Realm};
use crate::ecmascript::{builtins::promise::Promise, types::Function};

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
    
    fn host_load_imported_module(
        &self,
        referrer: (),
        specifier: &str,
        host_defined: Option<Box<dyn std::any::Any>>,
        payload: (),
    ) {
        unreachable!("HostLoadImportedModule does not have a default implementation");
    }
    
    fn host_promise_rejection_tracker(&self, promise: Promise, operation: PromiseRejectionOperation) {
    }

    
}
