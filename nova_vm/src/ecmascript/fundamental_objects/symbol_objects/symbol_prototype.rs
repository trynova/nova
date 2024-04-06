use crate::{
    ecmascript::{
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoFunction, IntoValue, String, Value},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct SymbolPrototype;

struct SymbolPrototypeGetDescription;
impl Builtin for SymbolPrototypeGetDescription {
    const NAME: &'static str = "get description";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(SymbolPrototype::get_description);
}

struct SymbolPrototypeToString;
impl Builtin for SymbolPrototypeToString {
    const NAME: &'static str = "toString";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(SymbolPrototype::to_string);
}

struct SymbolPrototypeValueOf;
impl Builtin for SymbolPrototypeValueOf {
    const NAME: &'static str = "valueOf";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(SymbolPrototype::value_of);
}

struct SymbolPrototypeToPrimitive;
impl Builtin for SymbolPrototypeToPrimitive {
    const NAME: &'static str = "[Symbol.toPrimitive]";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(SymbolPrototype::value_of);
}

impl SymbolPrototype {
    fn get_description(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn to_string(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn value_of(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.symbol_prototype();
        let symbol_constructor = intrinsics.symbol();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(6)
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key_from_str("constructor")
                    .with_value(symbol_constructor.into_value())
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key_from_str("description")
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<SymbolPrototypeGetDescription>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_builtin_function_property::<SymbolPrototypeToString>()
            .with_builtin_function_property::<SymbolPrototypeValueOf>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToPrimitive.into())
                    .with_value_creator_readonly(|agent| {
                        BuiltinFunctionBuilder::new::<SymbolPrototypeToPrimitive>(agent, realm)
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(String::from_small_string("Symbol").into_value())
                    .with_enumerable(false)
                    .build()
            })
            .build();
    }
}
