// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::builtins::Behaviour;
use crate::engine::context::{Bindable, GcScope};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::create_iter_result_object,
            operations_on_objects::create_array_from_list,
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Builtin,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::CollectionIteratorKind,
        },
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, IntoValue, String, Value},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct MapIteratorPrototype;

struct MapIteratorPrototypeNext;
impl Builtin for MapIteratorPrototypeNext {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.next;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(MapIteratorPrototype::next);
}

impl MapIteratorPrototype {
    fn next<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let this_value = this_value.bind(gc);
        // 27.5.3.2 GeneratorValidate ( generator, generatorBrand )
        // 3. If generator.[[GeneratorBrand]] is not generatorBrand, throw a TypeError exception.
        let Value::MapIterator(iterator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "MapIterator expected",
                gc,
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
        while agent[iterator].next_index < agent[map].keys(gc).len() {
            // i. Let e be entries[index].
            // ii. Set index to index + 1.
            let index = agent[iterator].next_index;
            agent[iterator].next_index += 1;

            let result = match agent[iterator].kind {
                CollectionIteratorKind::Key => {
                    // iii. If e.[[Key]] is not EMPTY, then
                    //   1. If kind is KEY, then
                    //     a. Let result be e.[[Key]].
                    let Some(key) = agent[map].keys(gc)[index] else {
                        continue;
                    };
                    key
                }
                CollectionIteratorKind::Value => {
                    // iii. If e.[[Key]] is not EMPTY, then
                    //   2. If kind is VALUE, then
                    //     a. Let result be e.[[Value]].
                    let Some(value) = agent[map].values(gc)[index] else {
                        continue;
                    };
                    value
                }
                CollectionIteratorKind::KeyAndValue => {
                    // iii. If e.[[Key]] is not EMPTY, then
                    //   3. Else,
                    //     a. Assert: kind is KEY+VALUE.
                    //     b. Let result be CreateArrayFromList(« e.[[Key]], e.[[Value]] »).
                    let Some(key) = agent[map].keys(gc)[index] else {
                        continue;
                    };
                    let value = agent[map].values(gc)[index].unwrap();
                    create_array_from_list(agent, &[key, value], gc).into_value()
                }
            };

            // 4. Perform ? GeneratorYield(CreateIteratorResultObject(result, false)).
            return Ok(create_iter_result_object(agent, result, false).into_value());
        }

        debug_assert_eq!(agent[iterator].next_index, agent[map].keys(gc).len());

        // e. Return undefined.
        agent[iterator].map = None;
        Ok(create_iter_result_object(agent, Value::Undefined, true).into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
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
