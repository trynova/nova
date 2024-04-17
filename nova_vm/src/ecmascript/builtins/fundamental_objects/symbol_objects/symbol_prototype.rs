use crate::{
    ecmascript::{
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Builtin, BuiltinGetter},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoValue, PropertyKey, String, SymbolHeapData, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct SymbolPrototype;

struct SymbolPrototypeGetDescription;
impl Builtin for SymbolPrototypeGetDescription {
    const NAME: String = BUILTIN_STRING_MEMORY.get_description;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(SymbolPrototype::get_description);
}
impl BuiltinGetter for SymbolPrototypeGetDescription {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.description.to_property_key();
}

struct SymbolPrototypeToString;
impl Builtin for SymbolPrototypeToString {
    const NAME: String = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(SymbolPrototype::to_string);
}

struct SymbolPrototypeValueOf;
impl Builtin for SymbolPrototypeValueOf {
    const NAME: String = BUILTIN_STRING_MEMORY.valueOf;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(SymbolPrototype::value_of);
}

struct SymbolPrototypeToPrimitive;
impl Builtin for SymbolPrototypeToPrimitive {
    const NAME: String = BUILTIN_STRING_MEMORY._Symbol_toPrimitive_;

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

        agent.heap.symbols.extend_from_slice(
            &[
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_asyncIterator),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_hasInstance),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_isConcatSpreadable),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_iterator),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_match),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_matchAll),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_replace),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_search),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_species),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_split),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_toPrimitive),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_toStringTag),
                },
                SymbolHeapData {
                    descriptor: Some(BUILTIN_STRING_MEMORY.Symbol_unscopables),
                },
            ]
            .map(Some),
        );

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(6)
            .with_constructor_property(symbol_constructor)
            .with_builtin_function_getter_property::<SymbolPrototypeGetDescription>()
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
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Symbol.into_value())
                    .with_enumerable(false)
                    .build()
            })
            .build();
    }
}
