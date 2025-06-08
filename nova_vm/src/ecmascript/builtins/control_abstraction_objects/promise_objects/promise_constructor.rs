// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::{Bindable, GcScope};
use crate::engine::rootable::Scopable;
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call, call_function},
            testing_and_comparison::is_constructor,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
            ordinary::ordinary_create_from_constructor,
            promise::{
                Promise,
                data::{PromiseHeapData, PromiseState},
            },
        },
        execution::{Agent, JsResult, ProtoIntrinsics, Realm, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, Function, IntoObject, IntoValue, Object, PropertyKey, String,
            Value,
        },
    },
    heap::{CreateHeapData, IntrinsicConstructorIndexes, ObjectEntry, WellKnownSymbolIndexes},
};

use super::promise_abstract_operations::{
    promise_capability_records::PromiseCapability,
    promise_resolving_functions::{PromiseResolvingFunctionHeapData, PromiseResolvingFunctionType},
};

pub(crate) struct PromiseConstructor;
impl Builtin for PromiseConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Promise;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for PromiseConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Promise;
}
struct PromiseAll;
impl Builtin for PromiseAll {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::all);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.all;
}
struct PromiseAllSettled;
impl Builtin for PromiseAllSettled {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::all_settled);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.allSettled;
}
struct PromiseAny;
impl Builtin for PromiseAny {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::any);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.any;
}
struct PromiseRace;
impl Builtin for PromiseRace {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::race);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.race;
}
struct PromiseReject;
impl Builtin for PromiseReject {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::reject);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.reject;
}
struct PromiseResolve;
impl Builtin for PromiseResolve {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::resolve);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.resolve;
}
struct PromiseTry;
impl Builtin for PromiseTry {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::r#try);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.r#try;
}
struct PromiseWithResolvers;
impl Builtin for PromiseWithResolvers {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::with_resolvers);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.withResolvers;
}
struct PromiseGetSpecies;
impl Builtin for PromiseGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Species.to_property_key());
}
impl BuiltinGetter for PromiseGetSpecies {}

impl PromiseConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        args: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let executor = args.get(0).bind(gc.nogc());

        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Promise Constructor requires 'new'",
                gc.into_nogc(),
            ));
        };
        let new_target = new_target.unbind().bind(gc.nogc());

        assert_eq!(
            new_target,
            agent
                .current_realm_record()
                .intrinsics()
                .promise()
                .into_object(),
            "We currently don't support Promise subclassing."
        );

        // 2. If IsCallable(executor) is false, throw a TypeError exception.
        // TODO: Callable proxies
        let Ok(executor) = Function::try_from(executor) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Not a callable value",
                gc.into_nogc(),
            ));
        };
        let executor = executor.unbind().scope(agent, gc.nogc());

        // 3. Let promise be ? OrdinaryCreateFromConstructor(NewTarget, "%Promise.prototype%", « [[PromiseState]], [[PromiseResult]], [[PromiseFulfillReactions]], [[PromiseRejectReactions]], [[PromiseIsHandled]] »).
        // 4. Set promise.[[PromiseState]] to pending.
        // 5. Set promise.[[PromiseFulfillReactions]] to a new empty List.
        // 6. Set promise.[[PromiseRejectReactions]] to a new empty List.
        // 7. Set promise.[[PromiseIsHandled]] to false.
        let Object::Promise(promise) = ordinary_create_from_constructor(
            agent,
            Function::try_from(new_target.unbind()).unwrap(),
            ProtoIntrinsics::Promise,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc()) else {
            unreachable!()
        };
        let promise = promise.unbind().bind(gc.nogc());
        let scoped_promise = promise.scope(agent, gc.nogc());

        // 8. Let resolvingFunctions be CreateResolvingFunctions(promise).
        let promise_capability = PromiseCapability::from_promise(promise, true);
        let resolve_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability: promise_capability.clone(),
                resolve_type: PromiseResolvingFunctionType::Resolve,
            })
            .into_value();
        let reject_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability: promise_capability.clone(),
                resolve_type: PromiseResolvingFunctionType::Reject,
            })
            .into_value();

        // 9. Let completion be Completion(Call(executor, undefined, « resolvingFunctions.[[Resolve]], resolvingFunctions.[[Reject]] »)).
        // 10. If completion is an abrupt completion, then
        if let Err(err) = call_function(
            agent,
            executor.get(agent),
            Value::Undefined,
            Some(ArgumentsList::from_mut_slice(&mut [
                resolve_function.unbind(),
                reject_function.unbind(),
            ])),
            gc.reborrow(),
        ) {
            // a. Perform ? Call(resolvingFunctions.[[Reject]], undefined, « completion.[[Value]] »).
            let promise_capability =
                PromiseCapability::from_promise(scoped_promise.get(agent), true);
            promise_capability.reject(agent, err.value().unbind(), gc.nogc());
        }

        // 11. Return promise.
        Ok(scoped_promise.get(agent).into_value())
    }

    fn all<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Promise.all", gc.into_nogc()))
    }

    fn all_settled<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Promise.allSettled", gc.into_nogc()))
    }
    fn any<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Promise.any", gc.into_nogc()))
    }
    fn race<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Promise.race", gc.into_nogc()))
    }

    fn reject<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let r = arguments.get(0).bind(gc);
        assert_eq!(
            this_value,
            agent
                .current_realm_record()
                .intrinsics()
                .promise()
                .into_value(),
            "We currently don't support Promise subclassing."
        );

        // 1. Let C be the this value.
        // 2. Let promiseCapability be ? NewPromiseCapability(C).
        // 3. Perform ? Call(promiseCapability.[[Reject]], undefined, « r »).
        // 4. Return promiseCapability.[[Promise]].
        // NOTE: Since we don't support promise subclassing, this is equivalent
        // to creating an already-rejected promise.
        Ok(Promise::new_rejected(agent, r, gc).into_value())
    }

    fn resolve<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        assert_eq!(
            this_value,
            agent
                .current_realm_record()
                .intrinsics()
                .promise()
                .into_value(),
            "We currently don't support Promise subclassing."
        );

        // 3. Return ? PromiseResolve(C, x).
        Ok(Promise::resolve(agent, arguments.get(0), gc).into_value())
    }

    /// ### [1 Promise.try ( callbackfn, ...args )](https://tc39.es/proposal-promise-try)
    ///
    /// `Promise.try` proposal.
    fn r#try<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let callback_fn = arguments.get(0).bind(gc.nogc());
        let args = arguments.slice_from(1).bind(gc.nogc());
        // 1. Let C be the this value.
        // 2. If C is not an Object, throw a TypeError exception.
        if is_constructor(agent, this_value).is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected the this value to be a constructor.",
                gc.into_nogc(),
            ));
        }
        assert_eq!(
            this_value,
            agent
                .current_realm_record()
                .intrinsics()
                .promise()
                .into_value(),
            "We currently don't support Promise subclassing."
        );

        // 3. Let promiseCapability be ? NewPromiseCapability(C).
        // 4. Let status be Completion(Call(callbackfn, undefined, args)).
        let status = call(
            agent,
            callback_fn.unbind(),
            Value::Undefined,
            Some(args.unbind()),
            gc.reborrow(),
        );
        let promise = match status {
            // 5. If status is an abrupt completion, then
            Err(err) => {
                // a. Perform ? Call(promiseCapability.[[Reject]], undefined, « status.[[Value]] »).
                // 7. Return promiseCapability.[[Promise]].
                agent.heap.create(PromiseHeapData {
                    object_index: None,
                    promise_state: PromiseState::Rejected {
                        promise_result: err.value(),
                        is_handled: false,
                    },
                })
            }
            // 6. Else,
            Ok(result) => {
                // a. Perform ? Call(promiseCapability.[[Resolve]], undefined, « status.[[Value]] »).
                Promise::resolve(agent, result.unbind(), gc)
            }
        };
        // 7. Return promiseCapability.[[Promise]].
        Ok(promise.into_value().unbind())
    }

    fn with_resolvers<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // Step 2 will throw if `this_value` is not a constructor.
        if is_constructor(agent, this_value).is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected the this value to be a constructor.",
                gc,
            ));
        }
        assert_eq!(
            this_value,
            agent
                .current_realm_record()
                .intrinsics()
                .promise()
                .into_value(),
            "We currently don't support Promise subclassing."
        );

        // 1. Let C be the this value.
        // 2. Let promiseCapability be ? NewPromiseCapability(C).
        let promise_capability = PromiseCapability::new(agent, gc);
        let resolve_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability: promise_capability.clone(),
                resolve_type: PromiseResolvingFunctionType::Resolve,
            })
            .into_value();
        let reject_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability: promise_capability.clone(),
                resolve_type: PromiseResolvingFunctionType::Reject,
            })
            .into_value();

        // 3. Let obj be OrdinaryObjectCreate(%Object.prototype%).
        // 4. Perform ! CreateDataPropertyOrThrow(obj, "promise", promiseCapability.[[Promise]]).
        // 5. Perform ! CreateDataPropertyOrThrow(obj, "resolve", promiseCapability.[[Resolve]]).
        // 6. Perform ! CreateDataPropertyOrThrow(obj, "reject", promiseCapability.[[Reject]]).
        let obj = agent.heap.create_object_with_prototype(
            agent
                .current_realm_record()
                .intrinsics()
                .object_prototype()
                .into_object(),
            &[
                ObjectEntry::new_data_entry(
                    BUILTIN_STRING_MEMORY.promise.into(),
                    promise_capability.promise().into_value(),
                ),
                ObjectEntry::new_data_entry(
                    BUILTIN_STRING_MEMORY.resolve.into(),
                    resolve_function.into_value(),
                ),
                ObjectEntry::new_data_entry(
                    BUILTIN_STRING_MEMORY.reject.into(),
                    reject_function.into_value(),
                ),
            ],
        );

        // 7. Return obj.
        Ok(obj.into_value())
    }

    fn get_species<'gc>(
        _: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        _: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Ok(this_value.unbind())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let promise_prototype = intrinsics.promise_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<PromiseConstructor>(agent, realm)
            .with_property_capacity(10)
            .with_builtin_function_property::<PromiseAll>()
            .with_builtin_function_property::<PromiseAllSettled>()
            .with_builtin_function_property::<PromiseAny>()
            .with_prototype_property(promise_prototype.into_object())
            .with_builtin_function_property::<PromiseRace>()
            .with_builtin_function_property::<PromiseReject>()
            .with_builtin_function_property::<PromiseResolve>()
            .with_builtin_function_property::<PromiseTry>()
            .with_builtin_function_property::<PromiseWithResolvers>()
            .with_builtin_function_getter_property::<PromiseGetSpecies>()
            .build();
    }
}
