// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call, call_function},
            testing_and_comparison::is_constructor,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ordinary::ordinary_create_from_constructor,
            promise::{
                data::{PromiseHeapData, PromiseState},
                Promise,
            },
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{
            Function, IntoObject, IntoValue, Object, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
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

impl<'gc> PromiseConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        args: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let executor = args.get(0);

        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Promise Constructor requires 'new'",
                gc.nogc(),
            ));
        };
        let new_target = new_target.unbind().bind(gc.nogc());

        // We currently don't support Promise subclassing.
        assert_eq!(
            new_target,
            agent.current_realm().intrinsics().promise().into_object()
        );

        // 2. If IsCallable(executor) is false, throw a TypeError exception.
        // TODO: Callable proxies
        let Ok(executor) = Function::try_from(executor) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Not a callable value",
                gc.nogc(),
            ));
        };
        let executor = executor.unbind().bind(gc.nogc()).scope(agent, gc.nogc());

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
        )?
        else {
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
                promise_capability,
                resolve_type: PromiseResolvingFunctionType::Resolve,
            })
            .into_value();
        let reject_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability,
                resolve_type: PromiseResolvingFunctionType::Reject,
            })
            .into_value();

        // 9. Let completion be Completion(Call(executor, undefined, « resolvingFunctions.[[Resolve]], resolvingFunctions.[[Reject]] »)).
        // 10. If completion is an abrupt completion, then
        if let Err(err) = call_function(
            agent,
            executor.get(agent),
            Value::Undefined,
            Some(ArgumentsList(&[resolve_function, reject_function])),
            gc.reborrow(),
        ) {
            // a. Perform ? Call(resolvingFunctions.[[Reject]], undefined, « completion.[[Value]] »).
            promise_capability.reject(agent, err.value());
        }

        // 11. Return promise.
        Ok(scoped_promise.get(agent).into_value())
    }

    fn all(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn all_settled(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }
    fn any(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }
    fn race(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn reject(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        _: GcScope,
    ) -> JsResult<Value<'gc>> {
        // We currently don't support Promise subclassing.
        assert_eq!(
            this_value,
            agent.current_realm().intrinsics().promise().into_value()
        );

        // 1. Let C be the this value.
        // 2. Let promiseCapability be ? NewPromiseCapability(C).
        // 3. Perform ? Call(promiseCapability.[[Reject]], undefined, « r »).
        // 4. Return promiseCapability.[[Promise]].
        // NOTE: Since we don't support promise subclassing, this is equivalent
        // to creating an already-rejected promise.
        let promise = agent.heap.create(PromiseHeapData {
            object_index: None,
            promise_state: PromiseState::Rejected {
                promise_result: arguments.get(0),
                is_handled: false,
            },
        });
        Ok(promise.into_value())
    }

    fn resolve(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // We currently don't support Promise subclassing.
        assert_eq!(
            this_value,
            agent.current_realm().intrinsics().promise().into_value()
        );

        // 3. Return ? PromiseResolve(C, x).
        Ok(Promise::resolve(agent, arguments.get(0), gc).into_value())
    }

    /// Defined in the [`Promise.try` proposal](https://tc39.es/proposal-promise-try)
    fn r#try(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let C be the this value.
        // 2. If C is not an Object, throw a TypeError exception.
        if is_constructor(agent, this_value).is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected the this value to be a constructor.",
                gc.nogc(),
            ));
        }
        // We currently don't support Promise subclassing.
        assert_eq!(
            this_value,
            agent.current_realm().intrinsics().promise().into_value()
        );

        // 3. Let promiseCapability be ? NewPromiseCapability(C).
        // 4. Let status be Completion(Call(callbackfn, undefined, args)).
        let status = call(
            agent,
            arguments.get(0),
            Value::Undefined,
            Some(ArgumentsList(&arguments.0[1..])),
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
                Promise::resolve(agent, result, gc)
            }
        };
        // 7. Return promiseCapability.[[Promise]].
        Ok(promise.into_value())
    }

    fn with_resolvers(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // Step 2 will throw if `this_value` is not a constructor.
        if is_constructor(agent, this_value).is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected the this value to be a constructor.",
                gc.nogc(),
            ));
        }
        // We currently don't support Promise subclassing.
        assert_eq!(
            this_value,
            agent.current_realm().intrinsics().promise().into_value()
        );

        // 1. Let C be the this value.
        // 2. Let promiseCapability be ? NewPromiseCapability(C).
        let promise_capability = PromiseCapability::new(agent);
        let resolve_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability,
                resolve_type: PromiseResolvingFunctionType::Resolve,
            })
            .into_value();
        let reject_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability,
                resolve_type: PromiseResolvingFunctionType::Reject,
            })
            .into_value();

        // 3. Let obj be OrdinaryObjectCreate(%Object.prototype%).
        // 4. Perform ! CreateDataPropertyOrThrow(obj, "promise", promiseCapability.[[Promise]]).
        // 5. Perform ! CreateDataPropertyOrThrow(obj, "resolve", promiseCapability.[[Resolve]]).
        // 6. Perform ! CreateDataPropertyOrThrow(obj, "reject", promiseCapability.[[Reject]]).
        let obj = agent.heap.create_object_with_prototype(
            agent
                .current_realm()
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

    fn get_species(
        _: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        _: GcScope,
    ) -> JsResult<Value<'gc>> {
        Ok(this_value)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
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
