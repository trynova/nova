// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [16.2.1 Module Semantics](https://tc39.es/ecma262/#sec-module-semantics)

use cyclic_module_records::{GraphLoadingStateRecord, continue_module_loading};
use source_text_module_records::SourceTextModule;

use crate::{
    ecmascript::execution::{Agent, JsResult},
    engine::context::{Bindable, NoGcScope},
};
pub mod abstract_module_records;
pub mod cyclic_module_records;
pub mod source_text_module_records;

/// ### [16.2.1.9 GetImportedModule ( referrer, request )](https://tc39.es/ecma262/#sec-GetImportedModule)
///
/// The abstract operation GetImportedModule takes arguments referrer (a Cyclic
/// Module Record) and request (a ModuleRequest Record) and returns a Module
/// Record.
fn get_imported_module<'a>(
    agent: &mut Agent,
    referrer: SourceTextModule<'a>,
    request: &str,
    gc: NoGcScope<'a, '_>,
) -> SourceTextModule<'a> {
    // 1. Let records be a List consisting of each LoadedModuleRequest Record r
    //    of referrer.[[LoadedModules]] such that ModuleRequestsEqual(r,
    //    request) is true.
    // 2. Assert: records has exactly one element, since LoadRequestedModules
    //    has completed successfully on referrer prior to invoking this
    //    abstract operation.
    // 3. Let record be the sole element of records.
    // 4. Return record.[[Module]].
    referrer
        .get_loaded_module(agent, request)
        .expect("Could not find loaded module for request")
        .bind(gc)
}

/// ### [16.2.1.11 FinishLoadingImportedModule ( referrer, moduleRequest, payload, result )](https://tc39.es/ecma262/#sec-FinishLoadingImportedModule)
///
/// The abstract operation FinishLoadingImportedModule takes arguments referrer
/// (a Script Record, a Cyclic Module Record, or a Realm Record), moduleRequest
/// (a ModuleRequest Record), payload (a GraphLoadingState Record or a
/// PromiseCapability Record), and result (either a normal completion
/// containing a Module Record or a throw completion) and returns unused.
pub fn finish_loading_imported_module<'a>(
    agent: &mut Agent,
    referrer: SourceTextModule<'a>,
    module_request: &str,
    payload: &mut GraphLoadingStateRecord<'a>,
    result: JsResult<'a, SourceTextModule<'a>>,
    gc: NoGcScope<'a, '_>,
) {
    // 1. If result is a normal completion, then
    if let Ok(result) = result {
        // a. If referrer.[[LoadedModules]] contains a LoadedModuleRequest
        //    Record record such that ModuleRequestsEqual(record,
        //    moduleRequest) is true, then
        // i. Assert: record.[[Module]] and result.[[Value]] are the same
        //    Module Record.
        // b. Else,
        // i. Append the LoadedModuleRequest Record {
        // [[Specifier]]: moduleRequest.[[Specifier]],
        // [[Attributes]]: moduleRequest.[[Attributes]],
        // [[Module]]: result.[[Value]]
        // } to referrer.[[LoadedModules]].
        referrer.insert_loaded_module(agent, module_request, result);
    }
    // 2. If payload is a GraphLoadingState Record, then
    // a. Perform ContinueModuleLoading(payload, result).
    continue_module_loading(agent, payload, result, gc);
    // 3. Else,
    // a. Perform ContinueDynamicImport(payload, result).
    // 4. Return unused.
}
