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
        execution::{Agent, JsResult, RealmIdentifier, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, IntoValue, String, Value},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct SetIteratorPrototype;

struct SetIteratorPrototypeNext;
impl Builtin for SetIteratorPrototypeNext {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.next;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SetIteratorPrototype::next);
}

impl SetIteratorPrototype {
    fn next<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let gc = gc.into_nogc();
        // 27.5.3.2 GeneratorValidate ( generator, generatorBrand )
        // 3. If generator.[[GeneratorBrand]] is not generatorBrand, throw a TypeError exception.
        let Value::SetIterator(iterator) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "SetIterator expected",
                gc,
            ));
        };
        let iterator = iterator.bind(gc);

        // 24.2.6.1 CreateSetIterator ( set, kind )
        // NOTE: We set `set` to None when the generator in the spec text has returned.
        let Some(set) = agent[iterator].set else {
            return Ok(create_iter_result_object(agent, Value::Undefined, true, gc).into_value());
        };

        // b. Let entries be set.[[SetData]].
        // c. Let numEntries be the number of elements in entries.
        // d. Repeat, while index < numEntries,
        while agent[iterator].next_index < agent[set].values(gc).len() {
            // i. Let e be entries[index].
            // ii. Set index to index + 1.
            let index = agent[iterator].next_index;
            agent[iterator].next_index += 1;

            // iii. if e is not EMPTY, then
            let Some(e) = agent[set].values(gc)[index] else {
                continue;
            };

            let result = match agent[iterator].kind {
                CollectionIteratorKind::Key => unreachable!(),
                CollectionIteratorKind::KeyAndValue => {
                    // 1. If kind is KEY+VALUE, then
                    //   a. Let result be CreateArrayFromList(« e, e »).
                    //   b. Perform ? GeneratorYield(CreateIteratorResultObject(result, false)).
                    create_array_from_list(agent, &[e.unbind(), e.unbind()], gc).into_value()
                }
                CollectionIteratorKind::Value => {
                    // 2. Else,
                    //   a. Assert: kind is VALUE.
                    //   b. Perform ? GeneratorYield(CreateIteratorResultObject(e, false)).
                    e
                }
            };

            return Ok(create_iter_result_object(agent, result.unbind(), false, gc).into_value());
        }

        debug_assert_eq!(agent[iterator].next_index, agent[set].values(gc).len());

        // e. Return undefined.
        agent[iterator].set = None;
        Ok(create_iter_result_object(agent, Value::Undefined, true, gc).into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.set_iterator_prototype();
        let iterator_prototype = intrinsics.iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(2)
            .with_prototype(iterator_prototype)
            .with_builtin_function_property::<SetIteratorPrototypeNext>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Set_Iterator.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
