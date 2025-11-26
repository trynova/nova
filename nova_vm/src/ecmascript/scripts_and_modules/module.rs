// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [16.2 Modules](https://tc39.es/ecma262/#sec-modules)

use module_semantics::{
    ImportAttributeRecord, ModuleRequest, Referrer,
    abstract_module_records::{AbstractModule, AbstractModuleMethods},
    cyclic_module_records::GraphLoadingStateRecord,
    get_module_namespace,
};

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{to_string, to_string_primitive},
        builtins::{
            promise::Promise,
            promise_objects::{
                promise_abstract_operations::{
                    promise_capability_records::{PromiseCapability, if_abrupt_reject_promise_m},
                    promise_reaction_records::PromiseReactionHandler,
                },
                promise_prototype::inner_promise_then,
            },
        },
        execution::{
            Agent, JsResult,
            agent::{get_active_script_or_module, unwrap_try},
        },
        types::{IntoValue, Primitive, Value},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
};
pub mod module_semantics;

/// ### [13.3.10.2 EvaluateImportCall ( specifierExpression \[ , optionsExpression \] )](https://tc39.es/ecma262/#sec-evaluate-import-call)
///
/// The abstract operation EvaluateImportCall takes argument
/// specifierExpression (a Parse Node) and optional argument optionsExpression
/// (a Parse Node) and returns either a normal completion containing a Promise
/// or an abrupt completion.
///
/// > NOTE: This method performs steps 1, 2, and 7 onwards. Thus, the arguments
/// > are already evaluated into Values (optional in the case of options).
pub(crate) fn evaluate_import_call<'gc>(
    agent: &mut Agent,
    specifier: Value,
    options: Option<Value>,
    mut gc: GcScope<'gc, '_>,
) -> Promise<'gc> {
    let specifier = specifier.bind(gc.nogc());
    let mut options = options.bind(gc.nogc());
    // 7. Let promiseCapability be ! NewPromiseCapability(%Promise%).
    let promise_capability = PromiseCapability::new(agent, gc.nogc());
    let scoped_promise = promise_capability.promise.scope(agent, gc.nogc());
    // 8. Let specifierString be Completion(ToString(specifier)).
    let specifier = if let Ok(specifier) = Primitive::try_from(specifier) {
        to_string_primitive(agent, specifier, gc.nogc())
            .unbind()
            .bind(gc.nogc())
    } else {
        let scoped_options = options.map(|o| o.scope(agent, gc.nogc()));
        let specifier = to_string(agent, specifier.unbind(), gc.reborrow())
            .unbind()
            .bind(gc.nogc());
        // SAFETY: not shared.
        options = scoped_options.map(|o| unsafe { o.take(agent) }.bind(gc.nogc()));
        specifier
    };
    // 9. IfAbruptRejectPromise(specifierString, promiseCapability).
    let promise_capability = PromiseCapability {
        promise: scoped_promise.get(agent).bind(gc.nogc()),
        must_be_unresolved: true,
    };
    let specifier = if_abrupt_reject_promise_m!(agent, specifier, promise_capability, gc);
    // 10. Let attributes be a new empty List.
    let attributes: Vec<ImportAttributeRecord> = vec![];
    // 11. If options is not undefined, then
    if let Some(_options) = options {
        // a. If options is not an Object, then
        //         i. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
        //         ii. Return promiseCapability.[[Promise]].
        // b. Let attributesObj be Completion(Get(options, "with")).
        // c. IfAbruptRejectPromise(attributesObj, promiseCapability).
        // d. If attributesObj is not undefined, then
        //         i. If attributesObj is not an Object, then
        //                 1. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
        //                 2. Return promiseCapability.[[Promise]].
        //         ii. Let entries be Completion(EnumerableOwnProperties(attributesObj, key+value)).
        //         iii. IfAbruptRejectPromise(entries, promiseCapability).
        //         iv. For each element entry of entries, do
        //                 1. Let key be ! Get(entry, "0").
        //                 2. Let value be ! Get(entry, "1").
        //                 3. If key is a String, then
        //                         a. If value is not a String, then
        //                                 i. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
        //                                 ii. Return promiseCapability.[[Promise]].
        //                         b. Append the ImportAttribute Record { [[Key]]: key, [[Value]]: value } to attributes.
        // e. If AllImportAttributesSupported(attributes) is false, then
        //         i. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
        //         ii. Return promiseCapability.[[Promise]].
        // f. Sort attributes according to the lexicographic order of their [[Key]] field, treating the value of each such field as a sequence of UTF-16 code unit values. NOTE: This sorting is observable only in that hosts are prohibited from changing behaviour based on the order in which attributes are enumerated.
        todo!()
    }
    let specifier = specifier.unbind();
    let attributes = attributes.unbind();
    let gc = gc.into_nogc();
    let specifier = specifier.bind(gc);
    let attributes = attributes.bind(gc);
    // 12. Let moduleRequest be a new ModuleRequest Record {
    let module_request = ModuleRequest::new_dynamic(
        agent,     // [[Specifier]]: specifierString,
        specifier, // [[Attributes]]: attributes
        attributes, gc,
    );
    // }.
    // 1. Let referrer be GetActiveScriptOrModule().
    // 2. If referrer is null, set referrer to the current Realm Record.
    let referrer: Referrer = get_active_script_or_module(agent, gc)
        .map(|m| m.into())
        .unwrap_or_else(|| agent.current_realm(gc).into());
    // 13. Perform HostLoadImportedModule(referrer, moduleRequest, empty, promiseCapability).
    // Note: this is against the spec. We'll fix it in post.
    // SAFETY: scoped_promise is not shared.
    let promise = unsafe { scoped_promise.take(agent) }.bind(gc);
    let mut payload = GraphLoadingStateRecord::from_promise(promise);
    agent
        .host_hooks
        .load_imported_module(agent, referrer, module_request, None, &mut payload, gc);
    // 14. Return promiseCapability.[[Promise]].
    promise
}

/// #### [13.3.10.3 ContinueDynamicImport ( promiseCapability, moduleCompletion )](https://tc39.es/ecma262/#sec-ContinueDynamicImport)
///
/// The abstract operation ContinueDynamicImport takes arguments
/// promiseCapability (a PromiseCapability Record) and moduleCompletion (either
/// a normal completion containing a Module Record or a throw completion) and
/// returns unused. It completes the process of a dynamic import originally
/// started by an import() call, resolving or rejecting the promise returned by
/// that call as appropriate.
pub(super) fn continue_dynamic_import<'a>(
    agent: &mut Agent,
    promise_capability: PromiseCapability<'a>,
    module_completion: JsResult<'a, AbstractModule<'a>>,
    gc: NoGcScope<'a, '_>,
) {
    // 1. If moduleCompletion is an abrupt completion,
    let module = match module_completion {
        // then
        Err(err) => {
            // a. Perform ! Call(promiseCapability.[[Reject]], undefined, « moduleCompletion.[[Value]] »).
            promise_capability.reject(agent, err.value(), gc);
            // b. Return unused.
            return;
        }
        // 2. Let module be moduleCompletion.[[Value]].
        Ok(module) => module,
    };
    // 3. Let loadPromise be module.LoadRequestedModules().
    let load_promise = module.load_requested_modules(agent, None, gc);
    // 4. Let rejectedClosure be a new Abstract Closure with parameters
    //    (reason) that captures promiseCapability and performs the following
    //    steps when called:
    // 5. Let onRejected be CreateBuiltinFunction(rejectedClosure, 1, "", « »).
    // 6. Let linkAndEvaluateClosure be a new Abstract Closure with no
    //    parameters that captures module, promiseCapability, and onRejected
    //    and performs the following steps when called:
    // 7. Let linkAndEvaluate be CreateBuiltinFunction(linkAndEvaluateClosure, 0, "", « »).
    // 8. Perform PerformPromiseThen(loadPromise, linkAndEvaluate, onRejected).
    let promise = promise_capability.promise();
    inner_promise_then(
        agent,
        load_promise,
        PromiseReactionHandler::DynamicImport { promise, module },
        PromiseReactionHandler::DynamicImport { promise, module },
        None,
        gc,
    );
    // 9. Return unused.
}

/// 6. Let linkAndEvaluateClosure be a new Abstract Closure with no
///    parameters that captures module, promiseCapability, and onRejected
///    and performs the following steps when called:
pub(crate) fn link_and_evaluate(
    agent: &mut Agent,
    promise: Promise,
    module: AbstractModule,
    mut gc: GcScope,
) {
    let promise = promise.bind(gc.nogc());
    let module = module.bind(gc.nogc());
    // a. Let link be Completion(module.Link()).
    let link = module.link(agent, gc.nogc());
    // b. If link is an abrupt completion, then
    if let Err(err) = link {
        // i. Perform ! Call(promiseCapability.[[Reject]], undefined, « link.[[Value]] »).
        PromiseCapability::from_promise(promise, true).reject(agent, err.value(), gc.nogc());
        // ii. Return unused.
        return;
    }
    let scoped_module = module.scope(agent, gc.nogc());
    let promise = promise.scope(agent, gc.nogc());
    // c. Let evaluatePromise be module.Evaluate().
    let evaluate_promise = module
        .unbind()
        .evaluate(agent, gc.reborrow())
        .unbind()
        .bind(gc.nogc());
    // SAFETY: not shared.
    let promise = unsafe { promise.take(agent) }.bind(gc.nogc());
    // SAFETY: not shared.
    let module = unsafe { scoped_module.take(agent) }.bind(gc.nogc());
    if let Some(result) = evaluate_promise.try_get_result(agent, gc.nogc()) {
        // Synchronous evaluation finish.
        match result {
            Ok(_) => {
                // i. Let namespace be GetModuleNamespace(module).
                let namespace = get_module_namespace(agent, module, gc.nogc());
                // ii. Perform ! Call(promiseCapability.[[Resolve]], undefined, « namespace »).
                unwrap_try(PromiseCapability::from_promise(promise, true).try_resolve(
                    agent,
                    namespace.into_value(),
                    gc.nogc(),
                ));
                // iii. Return unused.
            }
            Err(err) => {
                // i. Perform ! Call(promiseCapability.[[Reject]], undefined, « link.[[Value]] »).
                PromiseCapability::from_promise(promise, true).reject(
                    agent,
                    err.value(),
                    gc.nogc(),
                );
                // ii. Return unused.
            }
        }
        return;
    };
    // d. Let fulfilledClosure be a new Abstract Closure with no parameters
    //    that captures module and promiseCapability and performs the following
    //    steps when called:
    // e. Let onFulfilled be CreateBuiltinFunction(fulfilledClosure, 0, "", « »).
    // f. Perform PerformPromiseThen(evaluatePromise, onFulfilled, onRejected).
    inner_promise_then(
        agent,
        evaluate_promise,
        PromiseReactionHandler::DynamicImportEvaluate { promise, module },
        PromiseReactionHandler::DynamicImportEvaluate { promise, module },
        None,
        gc.nogc(),
    );
    // g. Return unused.
}

/// d. Let fulfilledClosure be a new Abstract Closure with no parameters
///    that captures module and promiseCapability and performs the following
///    steps when called:
pub(crate) fn import_get_module_namespace(
    agent: &mut Agent,
    promise: Promise,
    module: AbstractModule,
    gc: NoGcScope,
) {
    let promise = promise.bind(gc);
    let module = module.bind(gc);
    // i. Let namespace be GetModuleNamespace(module).
    let namespace = get_module_namespace(agent, module, gc);
    // ii. Perform ! Call(promiseCapability.[[Resolve]], undefined, « namespace »).
    unwrap_try(PromiseCapability::from_promise(promise, true).try_resolve(
        agent,
        namespace.into_value(),
        gc,
    ));
    // iii. Return unused.
}
