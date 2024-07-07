// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{operations_on_objects::get, testing_and_comparison::is_callable},
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            map::Map, ordinary::ordinary_create_from_constructor, ArgumentsList, Behaviour,
            Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{
            Function, IntoObject, IntoValue, Object, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct MapConstructor;
impl Builtin for MapConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Map;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(MapConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for MapConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Map;
}
struct MapGroupBy;
impl Builtin for MapGroupBy {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapConstructor::group_by);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.groupBy;
}
struct MapGetSpecies;
impl Builtin for MapGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_species_;
}
impl BuiltinGetter for MapGetSpecies {
    const KEY: PropertyKey = WellKnownSymbolIndexes::Species.to_property_key();
}

impl MapConstructor {
    fn behaviour(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        // If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(
                agent.throw_exception(ExceptionType::TypeError, "Constructor Map requires 'new'")
            );
        };
        let new_target = Function::try_from(new_target).unwrap();
        // 2. Let map be ? OrdinaryCreateFromConstructor(NewTarget, "%Map.prototype%", « [[MapData]] »).
        let map = Map::try_from(ordinary_create_from_constructor(
            agent,
            new_target,
            ProtoIntrinsics::Map,
        )?)
        .unwrap();
        // 3. Set map.[[MapData]] to a new empty List.
        let iterable = arguments.get(0);
        // 4. If iterable is either undefined or null, return map.
        if iterable.is_undefined() || iterable.is_null() {
            Ok(map.into_value())
        } else {
            // Note
            // If the parameter iterable is present, it is expected to be an
            // object that implements an @@iterator method that returns an
            // iterator object that produces a two element array-like object
            // whose first element is a value that will be used as a Map key
            // and whose second element is the value to associate with that
            // key.

            // 5. Let adder be ? Get(map, "set").
            let adder = get(
                agent,
                map.into_object(),
                BUILTIN_STRING_MEMORY.set.to_property_key(),
            )?;
            // 6. If IsCallable(adder) is false, throw a TypeError exception.
            if !is_callable(adder) {
                Err(agent.throw_exception(
                    ExceptionType::TypeError,
                    "Map.prototype.set is not callable",
                ))
            } else {
                // 7. Return ? AddEntriesFromIterable(map, iterable, adder).
                todo!();
            }
        }
    }

    fn group_by(
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
        let map_prototype = intrinsics.map_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<MapConstructor>(agent, realm)
            .with_property_capacity(3)
            .with_builtin_function_property::<MapGroupBy>()
            .with_prototype_property(map_prototype.into_object())
            .with_builtin_function_getter_property::<MapGetSpecies>()
            .build();
    }
}
