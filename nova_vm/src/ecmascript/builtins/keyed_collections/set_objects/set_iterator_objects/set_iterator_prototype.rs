// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, CollectionIteratorKind,
        ExceptionType, JsResult, Realm, String, Value, builders::OrdinaryObjectBuilder,
        create_array_from_list, create_iter_result_object,
    },
    engine::{Bindable, GcScope},
    heap::{ArenaAccess, ArenaAccessMut, ArenaAccessSoA, WellKnownSymbolIndexes},
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
        _arguments: ArgumentsList<'_, 'static>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
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
        crate::engine::bind!(let iterator = iterator, gc);

        // 24.2.6.1 CreateSetIterator ( set, kind )
        // NOTE: We set `set` to None when the generator in the spec text has returned.
        let Some(set) = iterator.get(agent).local().set else {
            return create_iter_result_object(agent, Value::Undefined, true, gc.into_nogc())
                .map(|o| o.into());
        };

        // b. Let entries be set.[[SetData]].
        // c. Let numEntries be the number of elements in entries.
        // d. Repeat, while index < numEntries,
        while iterator.get(agent).local().next_index < set.get(agent).local().values.len() {
            // i. Let e be entries[index].
            // ii. Set index to index + 1.
            let index = iterator.get(agent).local().next_index;
            iterator.get_mut(agent).next_index += 1;

            // iii. if e is not EMPTY, then
            let Some(e) = set.get(agent).local().values[index] else {
                continue;
            };

            let result = match iterator.get(agent).local().kind {
                CollectionIteratorKind::Key => unreachable!(),
                CollectionIteratorKind::KeyAndValue => {
                    // 1. If kind is KEY+VALUE, then
                    //   a. Let result be CreateArrayFromList(« e, e »).
                    //   b. Perform ? GeneratorYield(CreateIteratorResultObject(result, false)).
                    create_array_from_list(agent, &[e, e], gc).into()
                }
                CollectionIteratorKind::Value => {
                    // 2. Else,
                    //   a. Assert: kind is VALUE.
                    //   b. Perform ? GeneratorYield(CreateIteratorResultObject(e, false)).
                    e
                }
            };

            return create_iter_result_object(agent, result, false, gc.into_nogc())
                .map(|o| o.into());
        }

        debug_assert_eq!(
            iterator.get(agent).local().next_index,
            set.get(agent).local().values.len()
        );

        // e. Return undefined.
        iterator.get_mut(agent).set = None;
        create_iter_result_object(agent, Value::Undefined, true, gc.into_nogc()).map(|o| o.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.set_iterator_prototype();
        let iterator_prototype = intrinsics.iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(2)
            .with_prototype(iterator_prototype)
            .with_builtin_function_property::<SetIteratorPrototypeNext>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Set_Iterator.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
