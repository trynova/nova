use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::call_function, testing_and_comparison::is_callable,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ordinary::ordinary_create_from_constructor,
            promise::{
                data::{PromiseReactions, PromiseState},
                Promise,
            },
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
        },
        execution::{
            agent::{ExceptionType, PromiseRejectionOperation},
            Agent, JsResult, ProtoIntrinsics, RealmIdentifier,
        },
        types::{
            Function, IntoObject, IntoValue, Object, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

use super::promise_abstract_operations::{
    create_resolving_functions, new_promise_reaction_job,
    promise_capability_records::PromiseCapability,
    promise_reaction_records::{
        PromiseReactionHandler, PromiseReactionRecord, PromiseReactionType,
    },
};

pub(crate) struct PromiseConstructor;
impl Builtin for PromiseConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Promise;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(PromiseConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for PromiseConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Promise;
}
struct PromiseAll;
impl Builtin for PromiseAll {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::all);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.all;
}
struct PromiseAllSettled;
impl Builtin for PromiseAllSettled {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::all_settled);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.allSettled;
}
struct PromiseAny;
impl Builtin for PromiseAny {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::any);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.any;
}
struct PromiseRace;
impl Builtin for PromiseRace {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::race);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.race;
}
struct PromiseReject;
impl Builtin for PromiseReject {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::reject);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.reject;
}
struct PromiseResolve;
impl Builtin for PromiseResolve {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::resolve);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.resolve;
}
struct PromiseWithResolvers;
impl Builtin for PromiseWithResolvers {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::with_resolvers);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.withResolvers;
}
struct PromiseGetSpecies;
impl Builtin for PromiseGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_species_;
}
impl BuiltinGetter for PromiseGetSpecies {
    const KEY: PropertyKey = WellKnownSymbolIndexes::Species.to_property_key();
}

impl PromiseConstructor {
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                "Promise constructor cannot be called as a function",
            ));
        };
        // 2. If IsCallable(executor) is false, throw a TypeError exception.
        let executor = arguments.get(0);
        if !is_callable(executor) {
            return Err(
                agent.throw_exception(ExceptionType::TypeError, "Executor is not a constructor")
            );
        }
        let executor = Function::try_from(executor).unwrap();
        let new_target = Function::try_from(new_target).unwrap();
        // 3. Let promise be ? OrdinaryCreateFromConstructor(NewTarget, "%Promise.prototype%", « [[PromiseState]], [[PromiseResult]], [[PromiseFulfillReactions]], [[PromiseRejectReactions]], [[PromiseIsHandled]] »).
        let promise = Promise::try_from(ordinary_create_from_constructor(
            agent,
            new_target,
            ProtoIntrinsics::Promise,
        )?)
        .unwrap();

        // All of these steps are done by the heap data default builder.
        // 4. Set promise.[[PromiseState]] to pending.
        // 5. Set promise.[[PromiseFulfillReactions]] to a new empty List.
        // 6. Set promise.[[PromiseRejectReactions]] to a new empty List.
        // 7. Set promise.[[PromiseIsHandled]] to false.

        // 8. Let resolvingFunctions be CreateResolvingFunctions(promise).
        let resolving_functions = create_resolving_functions(agent, promise);
        // 9. Let completion be Completion(Call(executor, undefined, « resolvingFunctions.[[Resolve]], resolvingFunctions.[[Reject]] »)).
        let completion = call_function(
            agent,
            executor,
            Value::Undefined,
            Some(ArgumentsList(&[
                resolving_functions.resolve.into_value(),
                resolving_functions.reject.into_value(),
            ])),
        );
        // 10. If completion is an abrupt completion, then
        match completion {
            Ok(_) => {
                // 11. Return promise.
                Ok(promise.into_value())
            }
            Err(err) => {
                // a. Perform ? Call(resolvingFunctions.[[Reject]], undefined, « completion.[[Value]] »).
                call_function(
                    agent,
                    resolving_functions.reject,
                    Value::Undefined,
                    Some(ArgumentsList(&[err.value()])),
                )
            }
        }

        // Note
        // The executor argument must be a function object. It is called for
        // initiating and reporting completion of the possibly deferred action
        // represented by this Promise. The executor is called with two
        // arguments: resolve and reject. These are functions that may be used
        // by the executor function to report eventual completion or failure of
        // the deferred computation. Returning from the executor function does
        // not mean that the deferred action has been completed but only that
        // the request to eventually perform the deferred action has been
        // accepted.
        // The resolve function that is passed to an executor function accepts
        // a single argument. The executor code may eventually call the resolve
        // function to indicate that it wishes to resolve the associated
        // Promise. The argument passed to the resolve function represents the
        // eventual value of the deferred action and can be either the actual
        // fulfillment value or another promise which will provide the value if
        // it is fulfilled.
        // The reject function that is passed to an executor function accepts a
        // single argument. The executor code may eventually call the reject
        // function to indicate that the associated Promise is rejected and
        // will never be fulfilled. The argument passed to the reject function
        // is used as the rejection value of the promise. Typically it will be
        // an Error object.
        // The resolve and reject functions passed to an executor function by
        // the Promise constructor have the capability to actually resolve and
        // reject the associated promise. Subclasses may have different
        // constructor behaviour that passes in customized values for resolve
        // and reject.
    }

    fn all(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn all_settled(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }
    fn any(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn race(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn reject(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }
    fn resolve(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }
    fn with_resolvers(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_species(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let promise_prototype = intrinsics.promise_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<PromiseConstructor>(agent, realm)
            .with_property_capacity(9)
            .with_builtin_function_property::<PromiseAll>()
            .with_builtin_function_property::<PromiseAllSettled>()
            .with_builtin_function_property::<PromiseAny>()
            .with_prototype_property(promise_prototype.into_object())
            .with_builtin_function_property::<PromiseRace>()
            .with_builtin_function_property::<PromiseReject>()
            .with_builtin_function_property::<PromiseResolve>()
            .with_builtin_function_property::<PromiseWithResolvers>()
            .with_builtin_function_getter_property::<PromiseGetSpecies>()
            .build();
    }
}

/// ### [27.2.5.4.1 PerformPromiseThen ( promise, onFulfilled, onRejected \[ , resultCapability \] )](https://tc39.es/ecma262/#sec-performpromisethen)
///
/// The abstract operation PerformPromiseThen takes arguments promise (a
/// Promise), onFulfilled (an ECMAScript language value), and onRejected (an
/// ECMAScript language value) and optional argument resultCapability (a
/// PromiseCapability Record) and returns an ECMAScript language value. It
/// performs the “then” operation on promise using onFulfilled and onRejected
/// as its settlement actions. If resultCapability is passed, the result is
/// stored by updating resultCapability's promise. If it is not passed, then
/// PerformPromiseThen is being called by a specification-internal operation
/// where the result does not matter.
pub(crate) fn perform_promise_then(
    agent: &mut Agent,
    promise: Promise,
    on_fulfilled: Value,
    on_rejected: Value,
    result_capability: Option<PromiseCapability>,
) -> Option<Object> {
    // 1. Assert: IsPromise(promise) is true.
    // Already asserted by type.

    // 2. If resultCapability is not present, then
    // a. Set resultCapability to undefined.
    // 3. If IsCallable(onFulfilled) is false, then
    let on_fulfilled_job_callback = if !is_callable(on_fulfilled) {
        // a. Let onFulfilledJobCallback be empty.
        None
    } else {
        // 4. Else,
        // a. Let onFulfilledJobCallback be HostMakeJobCallback(onFulfilled).
        // host_make_job_callback(on_fulfilled);
        Some(())
    };
    let on_rejected_job_callback = if !is_callable(on_rejected) {
        // 5. If IsCallable(onRejected) is false, then
        // a. Let onRejectedJobCallback be empty.
        None
    } else {
        // 6. Else,
        // a. Let onRejectedJobCallback be HostMakeJobCallback(onRejected).
        // host_make_job_callback(on_rejected);
        Some(())
    };
    // 7. Let fulfillReaction be the PromiseReaction Record { [[Capability]]: resultCapability, [[Type]]: fulfill, [[Handler]]: onFulfilledJobCallback }.
    let fulfill_reaction = PromiseReactionRecord {
        capability: result_capability,
        handler: on_fulfilled_job_callback.map_or_else(
            || PromiseReactionHandler::Empty(PromiseReactionType::Fulfill),
            |job| PromiseReactionHandler::JobCallback(job),
        ),
    };
    // 8. Let rejectReaction be the PromiseReaction Record { [[Capability]]: resultCapability, [[Type]]: reject, [[Handler]]: onRejectedJobCallback }.
    let reject_reaction = PromiseReactionRecord {
        capability: result_capability,
        handler: on_rejected_job_callback.map_or_else(
            || PromiseReactionHandler::Empty(PromiseReactionType::Reject),
            |job| PromiseReactionHandler::JobCallback(job),
        ),
    };
    // 9. If promise.[[PromiseState]] is pending, then
    match agent[promise].promise_state {
        PromiseState::Pending => {
            // a. Append fulfillReaction to promise.[[PromiseFulfillReactions]].
            let existing_fulfill_reactions = &mut agent[promise].promise_fulfill_reactions;
            match existing_fulfill_reactions {
                Some(existing) => match existing {
                    PromiseReactions::One(previous) => {
                        *existing = PromiseReactions::Many(vec![*previous, fulfill_reaction]);
                    }
                    PromiseReactions::Many(multiple) => {
                        multiple.push(fulfill_reaction);
                    }
                },
                None => {
                    *existing_fulfill_reactions = Some(PromiseReactions::One(fulfill_reaction));
                }
            }
            // b. Append rejectReaction to promise.[[PromiseRejectReactions]].
            let existing_reject_reactions = &mut agent[promise].promise_reject_reactions;
            match existing_reject_reactions {
                Some(existing) => match existing {
                    PromiseReactions::One(previous) => {
                        *existing = PromiseReactions::Many(vec![*previous, reject_reaction]);
                    }
                    PromiseReactions::Many(multiple) => {
                        multiple.push(reject_reaction);
                    }
                },
                None => {
                    *existing_reject_reactions = Some(PromiseReactions::One(reject_reaction));
                }
            }
        }
        // 10. Else if promise.[[PromiseState]] is fulfilled, then
        PromiseState::Fulfilled {
            promise_result: value,
        } => {
            // a. Let value be promise.[[PromiseResult]].
            // b. Let fulfillJob be NewPromiseReactionJob(fulfillReaction, value).
            let fulfill_job = new_promise_reaction_job(agent, fulfill_reaction, value);
            // c. Perform HostEnqueuePromiseJob(fulfillJob.[[Job]], fulfillJob.[[Realm]]).
            agent
                .host_hooks
                .host_enqueue_promise_job(fulfill_job.job, fulfill_job.realm);
        }
        // 11. Else,
        PromiseState::Rejected {
            promise_result: reason,
        } => {
            // a. Assert: The value of promise.[[PromiseState]] is rejected.
            // b. Let reason be promise.[[PromiseResult]].
            // c. If promise.[[PromiseIsHandled]] is false, perform HostPromiseRejectionTracker(promise, "handle").
            if !agent[promise].promise_is_handled {
                agent
                    .host_hooks
                    .host_promise_rejection_tracker(promise, PromiseRejectionOperation::Handle);
            }
            // d. Let rejectJob be NewPromiseReactionJob(rejectReaction, reason).
            let reject_job = new_promise_reaction_job(agent, reject_reaction, reason);
            // e. Perform HostEnqueuePromiseJob(rejectJob.[[Job]], rejectJob.[[Realm]]).
            agent
                .host_hooks
                .host_enqueue_promise_job(reject_job.job, reject_job.realm);
        }
    }
    // 12. Set promise.[[PromiseIsHandled]] to true.
    agent[promise].promise_is_handled = true;
    // 13. If resultCapability is undefined, then
    //     a. Return undefined.
    // 14. Else,
    //     a. Return resultCapability.[[Promise]].
    result_capability.map(|result_capability| agent[result_capability].promise)
}
