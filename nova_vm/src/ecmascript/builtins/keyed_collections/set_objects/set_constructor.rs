// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{
                get_iterator, if_abrupt_close_iterator, iterator_step_value,
            },
            operations_on_objects::{call_function, get, get_method},
            testing_and_comparison::{is_callable, same_value},
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ordinary::ordinary_create_from_constructor, set::Set, ArgumentsList, Behaviour,
            Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{
            Function, IntoFunction, IntoObject, IntoValue, Object, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct SetConstructor;
impl Builtin for SetConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Map;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(SetConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for SetConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Set;
}
struct SetGetSpecies;
impl Builtin for SetGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_species_;
}
impl BuiltinGetter for SetGetSpecies {
    const KEY: PropertyKey = WellKnownSymbolIndexes::Species.to_property_key();
}

impl SetConstructor {
    /// ### [24.2.2.1 Set ( \[ iterable \] )](https://tc39.es/ecma262/#sec-set-iterable)
    fn behaviour(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        let iterable = arguments.get(0);
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(
                agent.throw_exception(ExceptionType::TypeError, "Cannot call Set as a function")
            );
        };
        // 2. Let set be ? OrdinaryCreateFromConstructor(NewTarget, "%Set.prototype%", « [[SetData]] »).
        let new_target = Function::try_from(new_target).unwrap();
        let set = Set::try_from(ordinary_create_from_constructor(
            agent,
            new_target,
            ProtoIntrinsics::Set,
        )?)
        .unwrap();
        // 3. Set set.[[SetData]] to a new empty List.
        // 4. If iterable is either undefined or null, return set.
        if iterable.is_undefined() || iterable.is_null() {
            return Ok(set.into_value());
        }
        // 5. Let adder be ? Get(set, "add").
        let adder = get(agent, set, BUILTIN_STRING_MEMORY.add.into())?;
        // 6. If IsCallable(adder) is false, throw a TypeError exception.
        if !is_callable(adder) {
            return Err(agent.throw_exception(ExceptionType::TypeError, "Invalid adder function"));
        }
        let adder = Function::try_from(adder).unwrap();
        if let Value::Array(iterable) = iterable {
            if iterable.is_trivial(agent)
                && iterable.is_dense(agent)
                && get_method(
                    agent,
                    iterable.into_value(),
                    PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into()),
                )? == Some(
                    agent
                        .current_realm()
                        .intrinsics()
                        .array_prototype_values()
                        .into_function(),
                )
            {
                // Accessorless, holeless array with standard Array values
                // iterator. We can fast-path this.

                // SAFETY: The array slice borrow cannot be invalidated by our
                // incoming mutation of the Set data.
                let mut set_data = iterable.as_slice(agent).to_vec();
                set_data.dedup_by(|a, b| same_value(agent, *a, *b));
                agent[set].set = set_data;
                return Ok(set.into_value());
            }
        }
        // 7. Let iteratorRecord be ? GetIterator(iterable, SYNC).
        let iterator_record = get_iterator(agent, iterable, false)?;
        // 8. Repeat,
        loop {
            // a. Let next be ? IteratorStepValue(iteratorRecord).
            let next = iterator_step_value(agent, &iterator_record)?;
            // b. If next is DONE, return set.
            let Some(next) = next else {
                return Ok(set.into_value());
            };
            // c. Let status be Completion(Call(adder, set, « next »)).
            let status =
                call_function(agent, adder, set.into_value(), Some(ArgumentsList(&[next])));
            // d. IfAbruptCloseIterator(status, iteratorRecord).
            let _ = if_abrupt_close_iterator(agent, status, &iterator_record)?;
        }
    }

    fn get_species(_: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        Ok(this_value)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let set_prototype = intrinsics.set_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<SetConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype_property(set_prototype.into_object())
            .with_builtin_function_getter_property::<SetGetSpecies>()
            .build();
    }
}
