use std::any::Any;

use oxc_span::Atom;

use super::{
    agent::{HostHooks, PromiseRejectionOperation},
    JsResult, Realm,
};
use crate::ecmascript::{
    builtins::{module::cyclic_module_records::GraphLoadingStateRecord, promise::Promise},
    types::Function,
};

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
        specifier: Atom<'static>,
        host_defined: Option<&dyn Any>,
        payload: &mut GraphLoadingStateRecord,
    ) {
        unreachable!("HostLoadImportedModule does not have a default implementation");
    }

    fn host_promise_rejection_tracker(
        &self,
        promise: Promise,
        operation: PromiseRejectionOperation,
    ) {
    }

    fn host_enqueue_generic_job(&self, job: (), realm: Realm) {
        unreachable!("HostEnqueueGenericJob does not have a default implementation");
    }

    fn host_enqueue_promise_job(&self, job: (), realm: Realm) {
        unreachable!("HostEnqueuePromiseJob does not have a default implementation");
    }

    fn host_enqueue_timeout_job(&self, job: (), realm: Realm) {
        unreachable!("HostEnqueueTimeoutJob does not have a default implementation");
    }
}
