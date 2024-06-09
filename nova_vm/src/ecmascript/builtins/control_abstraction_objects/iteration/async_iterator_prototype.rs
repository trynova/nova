use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct AsyncIteratorPrototype;

struct AsyncIteratorPrototypeIterator;
impl Builtin for AsyncIteratorPrototypeIterator {
    const NAME: String = BUILTIN_STRING_MEMORY._Symbol_asyncIterator_;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(AsyncIteratorPrototype::iterator);
}
impl BuiltinGetter for AsyncIteratorPrototypeIterator {
    const KEY: PropertyKey = WellKnownSymbolIndexes::AsyncIterator.to_property_key();
}
impl AsyncIteratorPrototype {
    fn iterator(_agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        Ok(this_value)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.async_iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(1)
            .with_prototype(object_prototype)
            .with_builtin_function_getter_property::<AsyncIteratorPrototypeIterator>()
            .build();
    }
}
