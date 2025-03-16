// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::operations_on_iterator_objects::{
    IteratorRecord, get_iterator_direct, if_abrupt_close_iterator, iterator_close,
    iterator_close_with_error, iterator_step_value,
};
use crate::ecmascript::abstract_operations::operations_on_objects::{call, throw_not_callable};
use crate::ecmascript::abstract_operations::testing_and_comparison::is_callable;
use crate::ecmascript::abstract_operations::type_conversion::to_boolean;
use crate::ecmascript::builtins::Array;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::types::{IntoValue, Object};
use crate::engine::Scoped;
use crate::engine::context::{Bindable, GcScope};
use crate::engine::rootable::Scopable;
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{BUILTIN_STRING_MEMORY, PropertyKey, String, Value},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct IteratorPrototype;

struct IteratorPrototypeIterator;
impl Builtin for IteratorPrototypeIterator {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_iterator_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Iterator.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(IteratorPrototype::iterator);
}

struct IteratorPrototypeEvery;
impl Builtin for IteratorPrototypeEvery {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.every;
    const KEY: Option<PropertyKey<'static>> = None;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(IteratorPrototype::every);
}

struct IteratorPrototypeFind;
impl Builtin for IteratorPrototypeFind {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.find;
    const KEY: Option<PropertyKey<'static>> = None;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(IteratorPrototype::find);
}

struct IteratorPrototypeForEach;
impl Builtin for IteratorPrototypeForEach {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.forEach;
    const KEY: Option<PropertyKey<'static>> = None;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(IteratorPrototype::for_each);
}

struct IteratorPrototypeSome;
impl Builtin for IteratorPrototypeSome {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.some;
    const KEY: Option<PropertyKey<'static>> = None;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(IteratorPrototype::some);
}

struct IteratorPrototypeReduce;
impl Builtin for IteratorPrototypeReduce {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.reduce;
    const KEY: Option<PropertyKey<'static>> = None;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(IteratorPrototype::reduce);
}

struct IteratorPrototypeToArray;
impl Builtin for IteratorPrototypeToArray {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toArray;
    const KEY: Option<PropertyKey<'static>> = None;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(IteratorPrototype::to_array);
}

impl IteratorPrototype {
    fn iterator<'gc>(
        _agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        Ok(this_value.unbind())
    }

    /// ### [27.1.4.3 Iterator.prototype.every ( predicate )](https://tc39.es/ecma262/#sec-iterator.prototype.every)
    fn every<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let predicate = arguments.get(0).bind(nogc);

        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'this' is not an object",
                nogc,
            ));
        };

        // 3. Let iterated be the Iterator Record { [[Iterator]]: O, [[NextMethod]]: undefined, [[Done]]: false }.
        // 4. If IsCallable(predicate) is false, then
        let Some(predicate) = is_callable(predicate, nogc).unbind().bind(nogc) else {
            // a. Let error be ThrowCompletion(a newly created TypeError object).
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'predicate' is not callable",
                nogc,
            );
            // b. Return ? IteratorClose(iterated, error).
            return iterator_close(agent, o.unbind(), Err(error), gc);
        };
        let scoped_predicate = Value::from(predicate).scope(agent, nogc);

        // 5. Set iterated to ? GetIteratorDirect(O).
        let iterated = get_iterator_direct(agent, o.unbind(), gc.reborrow())
            .unbind()
            .bind(gc.nogc())?;
        let Some(IteratorRecord {
            iterator,
            next_method,
            ..
        }) = iterated
        else {
            return Err(throw_not_callable(agent, gc.into_nogc()));
        };
        let iterator = iterator.scope(agent, gc.nogc());
        let next_method = next_method.scope(agent, gc.nogc());

        // 6. Let counter be 0.
        let mut counter = 0;

        // 7. Repeat,
        loop {
            // a. Let value be ? IteratorStepValue(iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let value = iterator_step_value(agent, iterated, gc.reborrow())
                .unbind()
                .bind(gc.nogc())?;

            // b. If value is done, return true.
            let Some(value) = value else {
                return Ok(Value::from(true));
            };

            // c. Let result be Completion(Call(predicate, undefined, « value, 𝔽(counter) »)).
            let result = call(
                agent,
                scoped_predicate.get(agent),
                Value::Undefined,
                Some(ArgumentsList(&[value.unbind(), counter.into()])),
                gc.reborrow(),
            )
            .unbind()
            .bind(gc.nogc());

            // d. IfAbruptCloseIterator(result, iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let result = if_abrupt_close_iterator!(agent, result, iterated, gc);

            // e. If ToBoolean(result) is false, return ? IteratorClose(iterated, NormalCompletion(false)).
            if !to_boolean(agent, result) {
                return iterator_close(agent, iterator.get(agent), Ok(Value::from(false)), gc);
            }

            // f. Set counter to counter + 1.
            counter += 1;
        }
    }

    /// ### [27.1.4.5 Iterator.prototype.find ( predicate )](https://tc39.es/ecma262/#sec-iterator.prototype.find)
    fn find<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let predicate = arguments.get(0).bind(nogc);

        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'this' is not an object",
                nogc,
            ));
        };

        // 3. Let iterated be the Iterator Record { [[Iterator]]: O, [[NextMethod]]: undefined, [[Done]]: false }.
        // 4. If IsCallable(predicate) is false, then
        let Some(predicate) = is_callable(predicate, nogc).unbind().bind(nogc) else {
            // a. Let error be ThrowCompletion(a newly created TypeError object).
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'predicate' is not callable",
                nogc,
            );
            // b. Return ? IteratorClose(iterated, error).
            return Err(iterator_close_with_error(agent, o.unbind(), error, gc));
        };
        let scoped_predicate = Value::from(predicate).scope(agent, nogc);

        // 5. Set iterated to ? GetIteratorDirect(O).
        let iterated = get_iterator_direct(agent, o.unbind(), gc.reborrow())
            .unbind()
            .bind(gc.nogc())?;
        let Some(IteratorRecord {
            iterator,
            next_method,
            ..
        }) = iterated
        else {
            return Err(throw_not_callable(agent, gc.into_nogc()));
        };
        let iterator = iterator.scope(agent, gc.nogc());
        let next_method = next_method.scope(agent, gc.nogc());

        // 6. Let counter be 0.
        let mut counter = 0;

        let mut scoped_value = Value::Undefined.scope_static(gc.nogc());

        // 7. Repeat,
        loop {
            // a. Let value be ? IteratorStepValue(iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let value = iterator_step_value(agent, iterated, gc.reborrow())
                .unbind()
                .bind(gc.nogc())?;

            // b. If value is done, return undefined.
            let Some(value) = value else {
                return Ok(Value::Undefined);
            };
            // SAFETY: scoped_value is never shared.
            unsafe { scoped_value.replace(agent, value.unbind()) };

            // c. Let result be Completion(Call(predicate, undefined, « value, 𝔽(counter) »)).
            let result = call(
                agent,
                scoped_predicate.get(agent),
                Value::Undefined,
                Some(ArgumentsList(&[value.unbind(), counter.into()])),
                gc.reborrow(),
            )
            .unbind()
            .bind(gc.nogc());

            // d. IfAbruptCloseIterator(result, iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let result = if_abrupt_close_iterator!(agent, result, iterated, gc);

            // e. If ToBoolean(result) is true, return ? IteratorClose(iterated, NormalCompletion(value)).
            if to_boolean(agent, result) {
                return iterator_close(agent, iterator.get(agent), Ok(scoped_value.get(agent)), gc);
            }

            // f. Set counter to counter + 1.
            counter += 1;
        }
    }

    /// ### [27.1.4.7 Iterator.prototype.forEach ( procedure )](https://tc39.es/ecma262/#sec-iterator.prototype.foreach)
    fn for_each<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let procedure = arguments.get(0).bind(nogc);

        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'this' is not an object",
                nogc,
            ));
        };

        // 3. Let iterated be the Iterator Record { [[Iterator]]: O, [[NextMethod]]: undefined, [[Done]]: false }.
        // 4. If IsCallable(procedure) is false, then
        let Some(procedure) = is_callable(procedure, nogc).unbind().bind(nogc) else {
            // a. Let error be ThrowCompletion(a newly created TypeError object).
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'procedure' is not callable",
                nogc,
            );
            // b. Return ? IteratorClose(iterated, error).
            return iterator_close(agent, o.unbind(), Err(error), gc);
        };
        let scoped_procedure = Value::from(procedure).scope(agent, nogc);

        // 5. Set iterated to ? GetIteratorDirect(O).
        let iterated = get_iterator_direct(agent, o.unbind(), gc.reborrow())
            .unbind()
            .bind(gc.nogc())?;
        let Some(IteratorRecord {
            iterator,
            next_method,
            ..
        }) = iterated
        else {
            return Err(throw_not_callable(agent, gc.into_nogc()));
        };
        let iterator = iterator.scope(agent, gc.nogc());
        let next_method = next_method.scope(agent, gc.nogc());

        // 6. Let counter be 0.
        let mut counter = 0;

        // 7. Repeat,
        loop {
            // a. Let value be ? IteratorStepValue(iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let value = iterator_step_value(agent, iterated, gc.reborrow())
                .unbind()
                .bind(gc.nogc())?;
            // b. If value is done, return undefined.
            let Some(value) = value else {
                return Ok(Value::Undefined);
            };

            // c. Let result be Completion(Call(procedure, undefined, « value, 𝔽(counter) »)).
            let result = call(
                agent,
                scoped_procedure.get(agent),
                Value::Undefined,
                Some(ArgumentsList(&[value.unbind(), counter.into()])),
                gc.reborrow(),
            )
            .unbind()
            .bind(gc.nogc());

            // d. IfAbruptCloseIterator(result, iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            if_abrupt_close_iterator!(agent, result, iterated, gc);

            // e. Set counter to counter + 1.
            counter += 1;
        }
    }

    /// ### [27.1.4.9 Iterator.prototype.reduce ( reducer \[ , initialValue \] )](https://tc39.es/ecma262/#sec-iterator.prototype.reduce)
    fn reduce<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let reducer = arguments.get(0).bind(nogc);
        let has_initial_value = arguments.len() > 1;
        let initial_value = arguments.get(1).bind(nogc);
        let scoped_initial_value = initial_value.scope(agent, nogc);

        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'this' is not an object",
                nogc,
            ));
        };

        // 3. Let iterated be the Iterator Record { [[Iterator]]: O, [[NextMethod]]: undefined, [[Done]]: false }.
        // 4. If IsCallable(reducer) is false, then
        let Some(reducer) = is_callable(reducer, nogc).unbind().bind(nogc) else {
            // a. Let error be ThrowCompletion(a newly created TypeError object).
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'reducer' is not callable",
                nogc,
            );
            // b. Return ? IteratorClose(iterated, error).
            return iterator_close(agent, o.unbind(), Err(error), gc);
        };
        let scoped_reducer = Value::from(reducer).scope(agent, nogc);

        // 5. Set iterated to ? GetIteratorDirect(O).
        let iterated = get_iterator_direct(agent, o.unbind(), gc.reborrow())
            .unbind()
            .bind(gc.nogc())?;
        let Some(IteratorRecord {
            iterator,
            next_method,
            ..
        }) = iterated
        else {
            return Err(throw_not_callable(agent, gc.into_nogc()));
        };
        let iterator = iterator.scope(agent, gc.nogc());
        let next_method = next_method.scope(agent, gc.nogc());

        let (mut accumulator, mut counter) = if !has_initial_value {
            // 6. If initialValue is not present, then
            // a. Let accumulator be ? IteratorStepValue(iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let accumulator = iterator_step_value(agent, iterated, gc.reborrow())
                .unbind()
                .bind(gc.nogc())?;

            // b. If accumulator is done, throw a TypeError exception.
            let Some(accumulator) = accumulator else {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "'this' was done",
                    gc.into_nogc(),
                ));
            };

            // c. Let counter be 1.
            (accumulator.scope(agent, gc.nogc()), 1)
        } else {
            // 7. Else,
            // a. Let accumulator be initialValue.
            // b. Let counter be 0.
            (scoped_initial_value, 0)
        };

        // 8. Repeat,
        loop {
            // a. Let value be ? IteratorStepValue(iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let value = iterator_step_value(agent, iterated, gc.reborrow())
                .unbind()
                .bind(gc.nogc())?;

            // b. If value is done, return accumulator.
            let Some(value) = value else {
                return Ok(accumulator.get(agent));
            };

            // c. Let result be Completion(Call(reducer, undefined, « accumulator, value, 𝔽(counter) »)).
            let result = call(
                agent,
                scoped_reducer.get(agent),
                Value::Undefined,
                Some(ArgumentsList(&[
                    accumulator.get(agent),
                    value.unbind(),
                    counter.into(),
                ])),
                gc.reborrow(),
            )
            .unbind()
            .bind(gc.nogc());

            // d. IfAbruptCloseIterator(result, iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let result = if_abrupt_close_iterator!(agent, result, iterated, gc);

            // e. Set accumulator to result.
            // SAFETY: accumulator is never shared.
            unsafe { accumulator.replace(agent, result.unbind()) };

            // f. Set counter to counter + 1.
            counter += 1;
        }
    }

    /// ### [27.1.4.10 Iterator.prototype.some ( predicate )](https://tc39.es/ecma262/#sec-iterator.prototype.some)
    fn some<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let predicate = arguments.get(0).bind(nogc);

        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'this' is not an object",
                nogc,
            ));
        };

        // 3. Let iterated be the Iterator Record { [[Iterator]]: O, [[NextMethod]]: undefined, [[Done]]: false }.
        // 4. If IsCallable(predicate) is false, then
        let Some(predicate) = is_callable(predicate, nogc).unbind().bind(nogc) else {
            // a. Let error be ThrowCompletion(a newly created TypeError object).
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'predicate' is not callable",
                nogc,
            );
            // b. Return ? IteratorClose(iterated, error).
            return iterator_close(agent, o.unbind(), Err(error), gc);
        };
        let scoped_predicate = Value::from(predicate).scope(agent, nogc);

        // 5. Set iterated to ? GetIteratorDirect(O).
        let iterated = get_iterator_direct(agent, o.unbind(), gc.reborrow())
            .unbind()
            .bind(gc.nogc())?;
        let Some(IteratorRecord {
            iterator,
            next_method,
            ..
        }) = iterated
        else {
            return Err(throw_not_callable(agent, gc.into_nogc()));
        };
        let iterator = iterator.scope(agent, gc.nogc());
        let next_method = next_method.scope(agent, gc.nogc());

        // 6. Let counter be 0.
        let mut counter = 0;

        // 7. Repeat,
        loop {
            // a. Let value be ? IteratorStepValue(iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let value = iterator_step_value(agent, iterated, gc.reborrow())
                .unbind()
                .bind(gc.nogc())?;

            // b. If value is done, return false.
            let Some(value) = value else {
                return Ok(Value::from(false));
            };

            // c. Let result be Completion(Call(predicate, undefined, « value, 𝔽(counter) »)).
            let result = call(
                agent,
                scoped_predicate.get(agent),
                Value::Undefined,
                Some(ArgumentsList(&[value.unbind(), counter.into()])),
                gc.reborrow(),
            )
            .unbind()
            .bind(gc.nogc());

            // d. IfAbruptCloseIterator(result, iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let result = if_abrupt_close_iterator!(agent, result, iterated, gc);

            // e. If ToBoolean(result) is true, return ? IteratorClose(iterated, NormalCompletion(true)).
            if to_boolean(agent, result) {
                return iterator_close(agent, iterator.get(agent), Ok(Value::from(true)), gc);
            }

            // f. Set counter to counter + 1.
            counter += 1;
        }
    }

    /// 27.1.4.12 Iterator.prototype.toArray ( )
    fn to_array<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);

        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'this' is not an object",
                nogc,
            ));
        };

        // 3. Let iterated be ? GetIteratorDirect(O).
        let iterated = get_iterator_direct(agent, o.unbind(), gc.reborrow())
            .unbind()
            .bind(gc.nogc())?;
        let Some(IteratorRecord {
            iterator,
            next_method,
            ..
        }) = iterated
        else {
            return Err(throw_not_callable(agent, gc.into_nogc()));
        };
        let iterator = iterator.scope(agent, gc.nogc());
        let next_method = next_method.scope(agent, gc.nogc());

        // 4. Let items be a new empty List.
        let mut items: Vec<Scoped<Value>> = Vec::new();

        // 5. Repeat,
        loop {
            // a. Let value be ? IteratorStepValue(iterated).
            let iterated = IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            };
            let value = iterator_step_value(agent, iterated, gc.reborrow())
                .unbind()
                .bind(gc.nogc())?
                .map(|x| x.scope(agent, gc.nogc()));

            // b. If value is done, return CreateArrayFromList(items).
            let Some(value) = value else {
                let gc = gc.into_nogc();

                // should reuse the allocation
                let unscoped: Vec<Value> =
                    items.into_iter().map(|x| x.get(agent).bind(gc)).collect();

                return Ok(Array::from_slice(agent, &unscoped, gc).into_value());
            };

            // c. Append value to items.
            items.push(value);
        }
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(7)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<IteratorPrototypeIterator>()
            .with_builtin_function_property::<IteratorPrototypeEvery>()
            .with_builtin_function_property::<IteratorPrototypeFind>()
            .with_builtin_function_property::<IteratorPrototypeForEach>()
            .with_builtin_function_property::<IteratorPrototypeSome>()
            .with_builtin_function_property::<IteratorPrototypeReduce>()
            .with_builtin_function_property::<IteratorPrototypeToArray>()
            .build();
    }
}
