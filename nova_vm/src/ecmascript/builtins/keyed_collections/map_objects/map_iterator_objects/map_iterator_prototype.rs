// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::create_iter_result_object,
            operations_on_objects::create_array_from_list,
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            indexed_collections::array_objects::array_iterator_objects::array_iterator::CollectionIteratorKind,
            ArgumentsList, Builtin,
        },
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct MapIteratorPrototype;

struct MapIteratorPrototypeNext;
impl Builtin for MapIteratorPrototypeNext {
    const NAME: String = BUILTIN_STRING_MEMORY.next;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MapIteratorPrototype::next);
}

impl MapIteratorPrototype {
    fn next(agent: &mut Agent, this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        // 27.5.3.2 GeneratorValidate ( generator, generatorBrand )
        // 3. If generator.[[GeneratorBrand]] is not generatorBrand, throw a TypeError exception.
        let Value::MapIterator(iterator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "MapIterator expected",
            ));
        };

        // 24.1.5.1 CreateMapIterator ( map, kind ), step 2
        // NOTE: We set `map` to None when the generator in the spec text has returned.
        let Some(map) = agent[iterator].map else {
            return Ok(create_iter_result_object(agent, Value::Undefined, true).into_value());
        };

        // a. Let entries be map.[[MapData]].
        // c. Let numEntries be the number of elements in entries.
        // d. Repeat, while index < numEntries,
        while agent[iterator].next_index < agent[map].keys().len() {
            // i. Let e be entries[index].
            // ii. Set index to index + 1.
            let index = agent[iterator].next_index;
            agent[iterator].next_index += 1;

            let result = match agent[iterator].kind {
                CollectionIteratorKind::Key => {
                    // iii. If e.[[Key]] is not EMPTY, then
                    //   1. If kind is KEY, then
                    //     a. Let result be e.[[Key]].
                    let Some(key) = agent[map].keys()[index] else {
                        continue;
                    };
                    key
                }
                CollectionIteratorKind::Value => {
                    // iii. If e.[[Key]] is not EMPTY, then
                    //   2. If kind is VALUE, then
                    //     a. Let result be e.[[Value]].
                    let Some(value) = agent[map].values()[index] else {
                        continue;
                    };
                    value
                }
                CollectionIteratorKind::KeyAndValue => {
                    // iii. If e.[[Key]] is not EMPTY, then
                    //   3. Else,
                    //     a. Assert: kind is KEY+VALUE.
                    //     b. Let result be CreateArrayFromList(« e.[[Key]], e.[[Value]] »).
                    let Some(key) = agent[map].keys()[index] else {
                        continue;
                    };
                    let value = agent[map].values()[index].unwrap();
                    create_array_from_list(agent, &[key, value]).into_value()
                }
            };

            // 4. Perform ? GeneratorYield(CreateIteratorResultObject(result, false)).
            return Ok(create_iter_result_object(agent, result, false).into_value());
        }

        debug_assert_eq!(agent[iterator].next_index, agent[map].keys().len());

        // e. Return undefined.
        agent[iterator].map = None;
        Ok(create_iter_result_object(agent, Value::Undefined, true).into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.map_iterator_prototype();
        let iterator_prototype = intrinsics.iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(2)
            .with_prototype(iterator_prototype)
            .with_builtin_function_property::<MapIteratorPrototypeNext>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Map_Iterator.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
