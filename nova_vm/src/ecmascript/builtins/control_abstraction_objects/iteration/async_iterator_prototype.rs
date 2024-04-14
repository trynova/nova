use crate::{
    ecmascript::{
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoFunction, String, Value, BUILTIN_STRING_MEMORY},
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

impl AsyncIteratorPrototype {
    fn iterator(_agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        Ok(this_value)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.async_iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(1)
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::AsyncIterator.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<AsyncIteratorPrototypeIterator>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
