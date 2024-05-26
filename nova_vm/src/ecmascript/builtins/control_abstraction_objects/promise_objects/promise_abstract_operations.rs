use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        abstract_operations::testing_and_comparison::is_constructor,
        builtins::{
            promise::{data::PromiseHeapData, Promise},
            ArgumentsList,
        },
        execution::{
            agent::{ExceptionType, JobCallbackRecord},
            Agent, JsResult,
        },
        types::{
            AbstractClosureHeapData, Function, IntoFunction, IntoObject, IntoValue, Object, String,
            Value,
        },
    },
    heap::{
        indexes::{BaseIndex, BoundFunctionIndex, ObjectIndex},
        CreateHeapData, Heap,
    },
};

use self::{
    promise_capability_records::{PromiseCapability, PromiseCapabilityRecord},
    promise_reaction_records::PromiseReaction,
};

pub(crate) mod promise_capability_records;
pub(crate) mod promise_reaction_records;

pub(crate) struct PromiseResolvingFunctions {
    pub(crate) resolve: Function,
    pub(crate) reject: Function,
}

/// ### [27.2.1.3 CreateResolvingFunctions ( promise )]()
///
/// The abstract operation CreateResolvingFunctions takes argument promise (a
/// Promise) and returns a Record with fields \[\[Resolve\]\] (a function
/// object) and \[\[Reject\]\] (a function object).
pub(crate) fn create_resolving_functions(
    agent: &mut Agent,
    promise: Promise,
) -> PromiseResolvingFunctions {
    // 1. Let alreadyResolved be the Record { [[Value]]: false }.
    let already_resolved = false;
    // 2. Let stepsResolve be the algorithm steps defined in Promise Resolve Functions.
    // 3. Let lengthResolve be the number of non-optional parameters of the function definition in Promise Resolve Functions.
    // 4. Let resolve be CreateBuiltinFunction(stepsResolve, lengthResolve, "", « [[Promise]], [[AlreadyResolved]] »).
    // TODO
    let resolve = Function::BoundFunction(BoundFunctionIndex::from_u32_index(0));
    // 5. Set resolve.[[Promise]] to promise.
    // 6. Set resolve.[[AlreadyResolved]] to alreadyResolved.
    // 7. Let stepsReject be the algorithm steps defined in Promise Reject Functions.
    // 8. Let lengthReject be the number of non-optional parameters of the function definition in Promise Reject Functions.
    // 9. Let reject be CreateBuiltinFunction(stepsReject, lengthReject, "", « [[Promise]], [[AlreadyResolved]] »).
    let reject = PromiseRejectFunctionHeapData {
        promise,
        already_resolved,
        object_index: None,
    };
    // 10. Set reject.[[Promise]] to promise.
    // 11. Set reject.[[AlreadyResolved]] to alreadyResolved.
    agent.heap.promise_reject_functions.push(Some(reject));
    let reject = BuiltinPromiseRejectFunction(BuiltinPromiseRejectFunctionIndex::last(
        &agent.heap.promise_reject_functions,
    ))
    .into_function();
    // 12. Return the Record { [[Resolve]]: resolve, [[Reject]]: reject }.
    PromiseResolvingFunctions { resolve, reject }
}

/// ### [27.2.1.3.1 Promise Reject Functions]()
///
/// A promise reject function is an anonymous built-in function that has
/// \[\[Promise\]\] and \[\[AlreadyResolved\]\] internal slots.
///
/// The "length" property of a promise reject function is 1𝔽.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PromiseRejectFunctionHeapData {
    /// \[\[Promise\]\]
    pub(crate) promise: Promise,
    /// \[\[AlreadyResolved\]\]
    pub(crate) already_resolved: bool,
    pub(crate) object_index: Option<ObjectIndex>,
}

pub(crate) type BuiltinPromiseRejectFunctionIndex = BaseIndex<PromiseRejectFunctionHeapData>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BuiltinPromiseRejectFunction(pub(crate) BuiltinPromiseRejectFunctionIndex);

impl Index<BuiltinPromiseRejectFunction> for Agent {
    type Output = PromiseRejectFunctionHeapData;

    fn index(&self, index: BuiltinPromiseRejectFunction) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<BuiltinPromiseRejectFunction> for Agent {
    fn index_mut(&mut self, index: BuiltinPromiseRejectFunction) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<BuiltinPromiseRejectFunctionIndex> for Agent {
    type Output = PromiseRejectFunctionHeapData;

    fn index(&self, index: BuiltinPromiseRejectFunctionIndex) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<BuiltinPromiseRejectFunctionIndex> for Agent {
    fn index_mut(&mut self, index: BuiltinPromiseRejectFunctionIndex) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl From<BuiltinPromiseRejectFunction> for Function {
    fn from(value: BuiltinPromiseRejectFunction) -> Self {
        Self::BuiltinPromiseRejectFunction(value.0)
    }
}

impl IntoFunction for BuiltinPromiseRejectFunction {
    fn into_function(self) -> Function {
        self.into()
    }
}

impl From<BuiltinPromiseRejectFunction> for Object {
    fn from(value: BuiltinPromiseRejectFunction) -> Self {
        Self::BuiltinPromiseRejectFunction(value.0)
    }
}

impl IntoObject for BuiltinPromiseRejectFunction {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<BuiltinPromiseRejectFunction> for Value {
    fn from(value: BuiltinPromiseRejectFunction) -> Self {
        Self::BuiltinPromiseRejectFunction(value.0)
    }
}

impl IntoValue for BuiltinPromiseRejectFunction {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl Index<BuiltinPromiseRejectFunction> for Heap {
    type Output = PromiseRejectFunctionHeapData;

    fn index(&self, index: BuiltinPromiseRejectFunction) -> &Self::Output {
        &self[index.0]
    }
}

impl IndexMut<BuiltinPromiseRejectFunction> for Heap {
    fn index_mut(&mut self, index: BuiltinPromiseRejectFunction) -> &mut Self::Output {
        &mut self[index.0]
    }
}

impl Index<BuiltinPromiseRejectFunctionIndex> for Heap {
    type Output = PromiseRejectFunctionHeapData;

    fn index(&self, index: BuiltinPromiseRejectFunctionIndex) -> &Self::Output {
        self.promise_reject_functions
            .get(index.into_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_ref()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl IndexMut<BuiltinPromiseRejectFunctionIndex> for Heap {
    fn index_mut(&mut self, index: BuiltinPromiseRejectFunctionIndex) -> &mut Self::Output {
        self.promise_reject_functions
            .get_mut(index.into_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_mut()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl PromiseRejectFunctionHeapData {
    /// When a promise reject function is called with argument reason, the
    /// following steps are taken:
    pub(crate) fn call(agent: &mut Agent, reason: Value) {
        // 1. Let F be the active function object.
        let f = agent.running_execution_context().function.unwrap();
        // 2. Assert: F has a [[Promise]] internal slot whose value is an Object.
        let Function::BuiltinPromiseRejectFunction(f) = f else {
            unreachable!();
        };
        // 3. Let promise be F.[[Promise]].
        // 4. Let alreadyResolved be F.[[AlreadyResolved]].
        let PromiseRejectFunctionHeapData {
            promise,
            already_resolved,
            ..
        } = agent[BuiltinPromiseRejectFunction(f)];
        // 5. If alreadyResolved.[[Value]] is true, return undefined.
        if !already_resolved {
            // 6. Set alreadyResolved.[[Value]] to true.
            agent[BuiltinPromiseRejectFunction(f)].already_resolved = true;
            // 7. Perform RejectPromise(promise, reason).
            reject_promise(agent, promise, reason);
            // 8. Return undefined.
        }
    }
}

/// ### [27.2.1.3.2 Promise Resolve Functions]()
///
/// A promise resolve function is an anonymous built-in function that has [[Promise]] and [[AlreadyResolved]] internal slots.

// When a promise resolve function is called with argument resolution, the following steps are taken:

// 1. Let F be the active function object.
// 2. Assert: F has a [[Promise]] internal slot whose value is an Object.
// 3. Let promise be F.[[Promise]].
// 4. Let alreadyResolved be F.[[AlreadyResolved]].
// 5. If alreadyResolved.[[Value]] is true, return undefined.
// 6. Set alreadyResolved.[[Value]] to true.
// 7. If SameValue(resolution, promise) is true, then
// a. Let selfResolutionError be a newly created TypeError object.
// b. Perform RejectPromise(promise, selfResolutionError).
// c. Return undefined.
// 8. If resolution is not an Object, then
// a. Perform FulfillPromise(promise, resolution).
// b. Return undefined.
// 9. Let then be Completion(Get(resolution, "then")).
// 10. If then is an abrupt completion, then
// a. Perform RejectPromise(promise, then.[[Value]]).
// b. Return undefined.
// 11. Let thenAction be then.[[Value]].
// 12. If IsCallable(thenAction) is false, then
// a. Perform FulfillPromise(promise, resolution).
// b. Return undefined.
// 13. Let thenJobCallback be HostMakeJobCallback(thenAction).
// 14. Let job be NewPromiseResolveThenableJob(promise, resolution, thenJobCallback).
// 15. Perform HostEnqueuePromiseJob(job.[[Job]], job.[[Realm]]).
// 16. Return undefined.

// The "length" property of a promise resolve function is 1𝔽.
/// ### [27.2.1.4 FulfillPromise ( promise, value )]()
///
/// The abstract operation FulfillPromise takes arguments promise (a Promise)
/// and value (an ECMAScript language value) and returns unused.
pub(crate) fn fulfill_promise(agent: &mut Agent, promise: Promise, value: Value) {
    // 1. Assert: The value of promise.[[PromiseState]] is pending.
    // 2. Let reactions be promise.[[PromiseFulfillReactions]].
    // 3. Set promise.[[PromiseResult]] to value.
    // 4. Set promise.[[PromiseFulfillReactions]] to undefined.
    // 5. Set promise.[[PromiseRejectReactions]] to undefined.
    // 6. Set promise.[[PromiseState]] to fulfilled.
    // 7. Perform TriggerPromiseReactions(reactions, value).
    // 8. Return unused.
}

pub(crate) fn new_intrinsic_promise_capability(agent: &mut Agent) -> PromiseCapability {
    // 5. Let executor be CreateBuiltinFunction(executorClosure, 2, "", « »).
    // 6. Let promise be ? Construct(C, « executor »).
    // From Promise Constructor: OrdinaryCreateFromConstructor
    let promise = agent.heap.create(PromiseHeapData::default());
    // From Promise Constructor: Let resolvingFunctions be CreateResolvingFunctions(promise).
    let resolving_functions =
        create_resolving_functions(agent, Promise::try_from(promise).unwrap());
    // From Promise Constructor: Call(executor, undefined, « resolvingFunctions.[[Resolve]], resolvingFunctions.[[Reject]] »).

    // 7. If IsCallable(resolvingFunctions.[[Resolve]]) is false, throw a TypeError exception.
    // 8. If IsCallable(resolvingFunctions.[[Reject]]) is false, throw a TypeError exception.
    // 9. Return the PromiseCapability Record { [[Promise]]: promise, [[Resolve]]: resolvingFunctions.[[Resolve]], [[Reject]]: resolvingFunctions.[[Reject]] }.
    let record = PromiseCapabilityRecord {
        promise,
        resolve: resolving_functions.resolve,
        reject: resolving_functions.reject,
    };
    agent.heap.create(record)
}

/// ### [27.2.1.5 NewPromiseCapability ( C )]()
///
/// The abstract operation NewPromiseCapability takes argument C (an ECMAScript
/// language value) and returns either a normal completion containing a
/// PromiseCapability Record or a throw completion. It attempts to use C as a
/// constructor in the fashion of the built-in Promise constructor to create a
/// promise and extract its resolve and reject functions. The promise plus the
/// resolve and reject functions are used to initialize a new PromiseCapability
/// Record.
///
/// NOTE: The argument `c` can take None to signify that the current realm's
/// %Promise% intrinsic should be used as the constructor.
pub(crate) fn new_promise_capability(agent: &mut Agent, c: Value) -> JsResult<PromiseCapability> {
    // 2. NOTE: C is assumed to be a constructor function that supports the parameter conventions of the Promise constructor (see 27.2.3.1).
    if c == agent.current_realm().intrinsics().promise().into_value() {
        return Ok(new_intrinsic_promise_capability(agent));
    }

    // 1. If IsConstructor(C) is false, throw a TypeError exception.
    if !is_constructor(agent, c) {
        return Err(agent.throw_exception(ExceptionType::TypeError, "Not a constructor"));
    }

    // 3. Let resolvingFunctions be the Record { [[Resolve]]: undefined, [[Reject]]: undefined }.
    struct SettableResolvingFunction {
        resolve: Option<Function>,
        reject: Option<Function>,
    }
    let resolving_functions = SettableResolvingFunction {
        resolve: None,
        reject: None,
    };

    // 4. Let executorClosure be a new Abstract Closure with parameters (resolve, reject) that captures resolvingFunctions and performs the following steps when called:
    agent
        .heap
        .abstract_closures
        .push(Some(AbstractClosureHeapData {
            object_index: None,
            length: 2,
            realm: agent.current_realm_id(),
            initial_name: Some(String::EMPTY_STRING),
            behaviour: Box::new(
                |agent: &mut Agent, this_value: Value, arguments: Option<ArgumentsList>| {
                    // a. If resolvingFunctions.[[Resolve]] is not undefined, throw a TypeError exception.

                    // b. If resolvingFunctions.[[Reject]] is not undefined, throw a TypeError exception.
                    // c. Set resolvingFunctions.[[Resolve]] to resolve.
                    // d. Set resolvingFunctions.[[Reject]] to reject.
                    // e. Return undefined.
                    Ok(Value::Undefined)
                },
            ),
        }));
    // 5. Let executor be CreateBuiltinFunction(executorClosure, 2, "", « »).
    // 6. Let promise be ? Construct(C, « executor »).
    // 7. If IsCallable(resolvingFunctions.[[Resolve]]) is false, throw a TypeError exception.
    // 8. If IsCallable(resolvingFunctions.[[Reject]]) is false, throw a TypeError exception.
    // 9. Return the PromiseCapability Record { [[Promise]]: promise, [[Resolve]]: resolvingFunctions.[[Resolve]], [[Reject]]: resolvingFunctions.[[Reject]] }.
    todo!();
    // Note

    // This abstract operation supports Promise subclassing, as it is generic
    // on any constructor that calls a passed executor function argument in the
    // same way as the Promise constructor. It is used to generalize static
    // methods of the Promise constructor to any subclass.
}

/// ### [27.2.1.6 IsPromise ( x )]()
///
/// The abstract operation IsPromise takes argument x (an ECMAScript language
/// value) and returns a Boolean. It checks for the promise brand on an object.
pub(crate) fn is_promise(agent: &mut Agent, x: Value) -> bool {
    // 1. If x is not an Object, return false.
    // 2. If x does not have a [[PromiseState]] internal slot, return false.
    // 3. Return true.
    matches!(x, Value::Promise(_))
}

/// ### [27.2.1.7 RejectPromise ( promise, reason )]()
///
/// The abstract operation RejectPromise takes arguments promise (a Promise)
/// and reason (an ECMAScript language value) and returns unused.
pub(crate) fn reject_promise(agent: &mut Agent, promise: Promise, reason: Value) {
    // 1. Assert: The value of promise.[[PromiseState]] is pending.
    let promise = &mut agent[promise];
    // 2. Let reactions be promise.[[PromiseRejectReactions]].
    // 3. Set promise.[[PromiseResult]] to reason.
    // 4. Set promise.[[PromiseFulfillReactions]] to undefined.
    // 5. Set promise.[[PromiseRejectReactions]] to undefined.
    // 6. Set promise.[[PromiseState]] to rejected.
    // 7. If promise.[[PromiseIsHandled]] is false, perform HostPromiseRejectionTracker(promise, "reject").
    // 8. Perform TriggerPromiseReactions(reactions, reason).
    // 9. Return unused.
}

/// ### [27.2.1.8 TriggerPromiseReactions ( reactions, argument )]()
///
/// The abstract operation TriggerPromiseReactions takes arguments reactions (a
/// List of PromiseReaction Records) and argument (an ECMAScript language
/// value) and returns unused. It enqueues a new Job for each record in
/// reactions. Each such Job processes the \[\[Type\]\] and \[\[Handler\]\] of
/// the PromiseReaction Record, and if the \[\[Handler\]\] is not empty, calls
/// it passing the given argument. If the \[\[Handler\]\] is empty, the
/// behaviour is determined by the \[\[Type\]\].
pub(crate) fn trigger_promise_reactions(
    agent: &mut Agent,
    reactions: &[PromiseReaction],
    argument: Value,
) {
    // 1. For each element reaction of reactions, do
    // a. Let job be NewPromiseReactionJob(reaction, argument).
    // b. Perform HostEnqueuePromiseJob(job.[[Job]], job.[[Realm]]).
    // 2. Return unused.
}

/// ### [27.2.2.1 NewPromiseReactionJob ( reaction, argument )](https://tc39.es/ecma262/#sec-newpromisereactionjob)
///
/// The abstract operation NewPromiseReactionJob takes arguments reaction (a
/// PromiseReaction Record) and argument (an ECMAScript language value) and
/// returns a Record with fields \[\[Job\]\] (a Job Abstract Closure) and
/// \[\[Realm\]\] (a Realm Record or null). It returns a new Job Abstract
/// Closure that applies the appropriate handler to the incoming value, and
/// uses the handler's return value to resolve or reject the derived promise
/// associated with that handler.
pub(crate) fn new_promise_reaction_job(
    agent: &mut Agent,
    reaction: PromiseReaction,
    argument: Value,
) {
    // 1. Let job be a new Job Abstract Closure with no parameters that captures reaction and argument and performs the following steps when called:
    //     a. Let promiseCapability be reaction.[[Capability]].
    //     b. Let type be reaction.[[Type]].
    //     c. Let handler be reaction.[[Handler]].
    //     d. If handler is empty, then
    //         i. If type is fulfill, then
    //             1. Let handlerResult be NormalCompletion(argument).
    //         ii. Else,
    //             1. Assert: type is reject.
    //             2. Let handlerResult be ThrowCompletion(argument).
    //     e. Else,
    //         i. Let handlerResult be Completion(HostCallJobCallback(handler, undefined, « argument »)).
    //     f. If promiseCapability is undefined, then
    //         i. Assert: handlerResult is not an abrupt completion.
    //         ii. Return empty.
    //     g. Assert: promiseCapability is a PromiseCapability Record.
    //     h. If handlerResult is an abrupt completion, then
    //         i. Return ? Call(promiseCapability.[[Reject]], undefined, « handlerResult.[[Value]] »).
    //     i. Else,
    //         i. Return ? Call(promiseCapability.[[Resolve]], undefined, « handlerResult.[[Value]] »).
    // 2. Let handlerRealm be null.
    // 3. If reaction.[[Handler]] is not empty, then
    //     a. Let getHandlerRealmResult be Completion(GetFunctionRealm(reaction.[[Handler]].[[Callback]])).
    //     b. If getHandlerRealmResult is a normal completion, set handlerRealm to getHandlerRealmResult.[[Value]].
    //     c. Else, set handlerRealm to the current Realm Record.
    //     d. NOTE: handlerRealm is never null unless the handler is undefined. When the handler is a revoked Proxy and no ECMAScript code runs, handlerRealm is used to create error objects.
    // 4. Return the Record { [[Job]]: job, [[Realm]]: handlerRealm }.
}

/// 27.2.2.2 NewPromiseResolveThenableJob ( promiseToResolve, thenable, then )
///
/// The abstract operation NewPromiseResolveThenableJob takes arguments
/// promiseToResolve (a Promise), thenable (an Object), and then (a JobCallback
/// Record) and returns a Record with fields \[\[Job\]\] (a Job Abstract
/// Closure) and \[\[Realm\]\] (a Realm Record).
///
/// #### Note
///
/// This Job uses the supplied thenable and its then method to resolve the
/// given promise. This process must take place as a Job to ensure that the
/// evaluation of the then method occurs after evaluation of any surrounding
/// code has completed.
pub(crate) fn new_promise_resolve_thenable_job(
    agent: &mut Agent,
    promise_to_resolve: Promise,
    thenable: Object,
    then: JobCallbackRecord,
) {
    // 1. Let job be a new Job Abstract Closure with no parameters that captures promiseToResolve, thenable, and then and performs the following steps when called:
    //     a. Let resolvingFunctions be CreateResolvingFunctions(promiseToResolve).
    //     b. Let thenCallResult be Completion(HostCallJobCallback(then, thenable, « resolvingFunctions.[[Resolve]], resolvingFunctions.[[Reject]] »)).
    //     c. If thenCallResult is an abrupt completion, then
    //         i. Return ? Call(resolvingFunctions.[[Reject]], undefined, « thenCallResult.[[Value]] »).
    //     d. Return ? thenCallResult.
    // 2. Let getThenRealmResult be Completion(GetFunctionRealm(then.[[Callback]])).
    // 3. If getThenRealmResult is a normal completion, let thenRealm be getThenRealmResult.[[Value]].
    // 4. Else, let thenRealm be the current Realm Record.
    // 5. NOTE: thenRealm is never null. When then.[[Callback]] is a revoked Proxy and no code runs, thenRealm is used to create error objects.
    // 6. Return the Record { [[Job]]: job, [[Realm]]: thenRealm }.
}
