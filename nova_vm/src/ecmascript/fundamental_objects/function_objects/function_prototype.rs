use crate::ecmascript::{
    builders::ordinary_object_builder::OrdinaryObjectBuilder,
    execution::{Agent, RealmIdentifier},
    types::IntoValue,
};

pub(crate) struct BooleanPrototype;

struct BooleanPrototypeToString;

// impl Builtin for BooleanPrototypeToString {
//     const NAME: &'static str = "toString";

//     const LENGTH: u8 = 0;

//     const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
//         crate::ecmascript::builtins::Behaviour::Regular(BooleanPrototype::to_string);
// }

impl BooleanPrototype {
    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.boolean_prototype();
        let boolean_constructor = intrinsics.boolean();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(1)
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key_from_str("constructor")
                    .with_value(boolean_constructor.into_value())
                    .build()
            })
            .build();
    }
}
