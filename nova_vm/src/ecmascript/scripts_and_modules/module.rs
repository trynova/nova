// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [16.2 Modules](https://tc39.es/ecma262/#sec-modules)

mod module_semantics;

pub use module_semantics::*;

use crate::{
    ecmascript::{
        Agent, Array, BUILTIN_STRING_MEMORY, EnumerateKeysAndValues, ExceptionType, JsResult,
        Object, Promise, PromiseCapability, PromiseReactionHandler, String, Value,
        enumerable_own_properties, get, get_active_script_or_module, if_abrupt_reject_promise_m,
        inner_promise_then, to_string, unwrap_try,
    },
    engine::{
        Scoped,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
        typeof_operator,
    },
};

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
    if options.is_some_and(|opt| opt.is_undefined()) {
        options.take();
    }
    // 7. Let promiseCapability be ! NewPromiseCapability(%Promise%).
    let promise_capability = PromiseCapability::new(agent, gc.nogc());
    let scoped_promise = promise_capability.promise.scope(agent, gc.nogc());
    // 8. Let specifierString be Completion(ToString(specifier)).
    let specifier = if let Ok(specifier) = String::try_from(specifier) {
        specifier
    } else {
        let scoped_options = options.map(|o| o.scope(agent, gc.nogc()));
        let specifier = to_string(agent, specifier.unbind(), gc.reborrow())
            .unbind()
            .bind(gc.nogc());
        // SAFETY: not shared.
        options = scoped_options.map(|o| unsafe { o.take(agent) }.bind(gc.nogc()));
        // 9. IfAbruptRejectPromise(specifierString, promiseCapability).
        let promise_capability = PromiseCapability {
            promise: scoped_promise.get(agent).bind(gc.nogc()),
            must_be_unresolved: true,
        };
        if_abrupt_reject_promise_m!(agent, specifier, promise_capability, gc)
    };
    // 10. Let attributes be a new empty List.
    // 11. If options is not undefined, then
    let (promise, specifier, attributes, gc) = if let Some(options) = options {
        // a. If options is not an Object, then
        let Ok(options) = Object::try_from(options) else {
            // i. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
            // ii. Return promiseCapability.[[Promise]].
            return reject_import_not_object_or_undefined(
                agent,
                scoped_promise,
                options.unbind(),
                gc.into_nogc(),
            );
        };
        let specifier = specifier.scope(agent, gc.nogc());
        // b. Let attributesObj be Completion(Get(options, "with")).
        let attributes_obj = get(
            agent,
            options.unbind(),
            BUILTIN_STRING_MEMORY.with.to_property_key(),
            gc.reborrow(),
        )
        .unbind()
        .bind(gc.nogc());

        let promise_capability = PromiseCapability {
            promise: scoped_promise.get(agent).bind(gc.nogc()),
            must_be_unresolved: true,
        };
        // c. IfAbruptRejectPromise(attributesObj, promiseCapability).
        let attributes_obj =
            if_abrupt_reject_promise_m!(agent, attributes_obj, promise_capability, gc);
        // d. If attributesObj is not undefined, then
        if !attributes_obj.is_undefined() {
            // i. If attributesObj is not an Object, then
            let Ok(attributes_obj) = Object::try_from(attributes_obj) else {
                // 1. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
                // 2. Return promiseCapability.[[Promise]].
                return reject_import_not_object_or_undefined(
                    agent,
                    scoped_promise,
                    attributes_obj.unbind(),
                    gc.into_nogc(),
                );
            };
            // ii. Let entries be Completion(EnumerableOwnProperties(attributesObj, key+value)).
            let entries = enumerable_own_properties::<EnumerateKeysAndValues>(
                agent,
                attributes_obj.unbind(),
                gc.reborrow(),
            )
            .unbind();
            let gc = gc.into_nogc();
            let entries = entries.bind(gc);

            let promise = unsafe { scoped_promise.take(agent) }.bind(gc);
            let promise_capability = PromiseCapability {
                promise,
                must_be_unresolved: true,
            };
            // iii. IfAbruptRejectPromise(entries, promiseCapability).
            // 1. Assert: value is a Completion Record.
            let entries = match entries {
                // 2. If value is an abrupt completion, then
                Err(err) => {
                    // a. Perform ? Call(capability.[[Reject]], undefined, « value.[[Value]] »).
                    promise_capability.reject(agent, err.value().unbind(), gc);
                    // b. Return capability.[[Promise]].
                    return promise_capability.promise;
                }
                // 3. Else,
                Ok(value) => {
                    // a. Set value to ! value.
                    value
                }
            };
            let mut attributes: Vec<ImportAttributeRecord> = Vec::with_capacity(entries.len());
            // iv. For each element entry of entries, do
            for entry in entries {
                let entry = Array::try_from(entry).unwrap();
                let entry = entry.get_storage(agent).values;
                // 1. Let key be ! Get(entry, "0").
                let key = entry[0].unwrap();
                // 2. Let value be ! Get(entry, "1").
                let value = entry[0].unwrap();
                // 3. If key is a String, then
                if let Ok(key) = String::try_from(key) {
                    // a. If value is not a String, then
                    let Ok(value) = String::try_from(value) else {
                        // i. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
                        // ii. Return promiseCapability.[[Promise]].
                        return reject_unsupported_import_attribute(
                            agent,
                            promise_capability,
                            key.unbind(),
                            gc.into_nogc(),
                        );
                    };
                    // b. Append the ImportAttribute Record { [[Key]]: key, [[Value]]: value } to attributes.
                    attributes.push(ImportAttributeRecord { key, value });
                }
            }
            // e. If AllImportAttributesSupported(attributes) is false, then
            if !all_import_attributes_supported(agent, &attributes) {
                // i. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
                // ii. Return promiseCapability.[[Promise]].
                return reject_unsupported_import_attributes(agent, promise_capability, gc);
            }
            // f. Sort attributes according to the lexicographic order of their
            //    [[Key]] field, treating the value of each such field as a sequence
            //    of UTF-16 code unit values. NOTE: This sorting is observable only
            //    in that hosts are prohibited from changing behaviour based on the
            //    order in which attributes are enumerated.
            attributes.sort_by(|a, b| a.key.as_wtf8_(agent).cmp(b.key.as_wtf8_(agent)));
            let specifier = unsafe { specifier.take(agent) }.bind(gc);
            (promise, specifier, attributes.into_boxed_slice(), gc)
        } else {
            let gc = gc.into_nogc();
            let specifier = unsafe { specifier.take(agent) }.bind(gc);
            let promise = unsafe { scoped_promise.take(agent) }.bind(gc);
            (promise, specifier, Default::default(), gc)
        }
    } else {
        let specifier = specifier.unbind();
        let gc = gc.into_nogc();
        let specifier = specifier.bind(gc);
        let promise = unsafe { scoped_promise.take(agent) }.bind(gc);
        (promise, specifier, Default::default(), gc)
    };
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
    let mut payload = GraphLoadingStateRecord::from_promise(promise);
    agent
        .host_hooks
        .load_imported_module(agent, referrer, module_request, None, &mut payload, gc);
    // 14. Return promiseCapability.[[Promise]].
    promise
}

#[cold]
#[inline(never)]
fn reject_import_not_object_or_undefined<'gc>(
    agent: &mut Agent,
    scoped_promise: Scoped<Promise>,
    value: Value,
    gc: NoGcScope<'gc, '_>,
) -> Promise<'gc> {
    let value = value.bind(gc);
    let promise_capability = PromiseCapability {
        // SAFETY: not shared.
        promise: unsafe { scoped_promise.take(agent) }.bind(gc),
        must_be_unresolved: true,
    };
    // i. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
    let message = format!(
        "import: expected object or undefined, got {}",
        typeof_operator(agent, value, gc).to_string_lossy_(agent)
    );
    let error = agent.throw_exception(ExceptionType::TypeError, message, gc);
    promise_capability.reject(agent, error.value(), gc);
    // ii. Return promiseCapability.[[Promise]].
    promise_capability.promise
}

#[cold]
#[inline(never)]
fn reject_unsupported_import_attribute<'gc>(
    agent: &mut Agent,
    promise_capability: PromiseCapability<'gc>,
    key: String<'gc>,
    gc: NoGcScope<'gc, '_>,
) -> Promise<'gc> {
    // i. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
    let message = format!(
        "Unsupported import attribute: {}",
        key.to_string_lossy_(agent)
    );
    let error = agent.throw_exception(ExceptionType::TypeError, message, gc);
    promise_capability.reject(agent, error.value(), gc);
    // ii. Return promiseCapability.[[Promise]].
    promise_capability.promise
}

#[cold]
#[inline(never)]
fn reject_unsupported_import_attributes<'gc>(
    agent: &mut Agent,
    promise_capability: PromiseCapability<'gc>,
    gc: NoGcScope<'gc, '_>,
) -> Promise<'gc> {
    // i. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created TypeError object »).
    let error = agent.throw_exception_with_static_message(
        ExceptionType::TypeError,
        "Unsupported import attributes",
        gc,
    );
    promise_capability.reject(agent, error.value(), gc);
    // ii. Return promiseCapability.[[Promise]].
    promise_capability.promise
}

/// ### [13.3.10.3 ContinueDynamicImport ( promiseCapability, moduleCompletion )](https://tc39.es/ecma262/#sec-ContinueDynamicImport)
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
                    namespace.into(),
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
        namespace.into(),
        gc,
    ));
    // iii. Return unused.
}
