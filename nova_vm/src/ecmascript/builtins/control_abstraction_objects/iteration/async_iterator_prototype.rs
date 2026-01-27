// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, JsResult,
        OrdinaryObjectBuilder, PropertyKey, Realm, String, Value,
    },
    engine::{Bindable, GcScope},
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct AsyncIteratorPrototype;

struct AsyncIteratorPrototypeIterator;
impl Builtin for AsyncIteratorPrototypeIterator {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_asyncIterator_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::AsyncIterator.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(AsyncIteratorPrototype::iterator);
}

impl AsyncIteratorPrototype {
    fn iterator<'gc>(
        _agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Ok(this_value.unbind())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.async_iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(1)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<AsyncIteratorPrototypeIterator>()
            .build();
    }
}
