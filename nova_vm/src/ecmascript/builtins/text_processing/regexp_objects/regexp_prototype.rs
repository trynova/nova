use crate::{
    ecmascript::{
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoFunction, IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct RegExpPrototype;

struct RegExpPrototypeExec;
impl Builtin for RegExpPrototypeExec {
    const NAME: String = BUILTIN_STRING_MEMORY.exec;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::exec);
}
struct RegExpPrototypeGetDotAll;
impl Builtin for RegExpPrototypeGetDotAll {
    const NAME: String = BUILTIN_STRING_MEMORY.get_dotAll;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_dot_all);
}
struct RegExpPrototypeGetFlags;
impl Builtin for RegExpPrototypeGetFlags {
    const NAME: String = BUILTIN_STRING_MEMORY.get_flags;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_flags);
}
struct RegExpPrototypeGetGlobal;
impl Builtin for RegExpPrototypeGetGlobal {
    const NAME: String = BUILTIN_STRING_MEMORY.get_global;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_global);
}
struct RegExpPrototypeGetHasIndices;
impl Builtin for RegExpPrototypeGetHasIndices {
    const NAME: String = BUILTIN_STRING_MEMORY.get_hasIndices;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_has_indices);
}
struct RegExpPrototypeGetIgnoreCase;
impl Builtin for RegExpPrototypeGetIgnoreCase {
    const NAME: String = BUILTIN_STRING_MEMORY.get_ignoreCase;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_ignore_case);
}
struct RegExpPrototypeMatch;
impl Builtin for RegExpPrototypeMatch {
    const NAME: String = BUILTIN_STRING_MEMORY._Symbol_match_;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::r#match);
}
struct RegExpPrototypeMatchAll;
impl Builtin for RegExpPrototypeMatchAll {
    const NAME: String = BUILTIN_STRING_MEMORY._Symbol_matchAll_;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::match_all);
}
struct RegExpPrototypeGetMultiline;
impl Builtin for RegExpPrototypeGetMultiline {
    const NAME: String = BUILTIN_STRING_MEMORY.get_multiline;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_multiline);
}
struct RegExpPrototypeReplace;
impl Builtin for RegExpPrototypeReplace {
    const NAME: String = BUILTIN_STRING_MEMORY._Symbol_replace_;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::replace);
}
struct RegExpPrototypeSearch;
impl Builtin for RegExpPrototypeSearch {
    const NAME: String = BUILTIN_STRING_MEMORY._Symbol_search_;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::search);
}
struct RegExpPrototypeGetSource;
impl Builtin for RegExpPrototypeGetSource {
    const NAME: String = BUILTIN_STRING_MEMORY.get_source;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_source);
}
struct RegExpPrototypeSplit;
impl Builtin for RegExpPrototypeSplit {
    const NAME: String = BUILTIN_STRING_MEMORY._Symbol_split_;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::split);
}
struct RegExpPrototypeGetSticky;
impl Builtin for RegExpPrototypeGetSticky {
    const NAME: String = BUILTIN_STRING_MEMORY.get_sticky;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_sticky);
}
struct RegExpPrototypeTest;
impl Builtin for RegExpPrototypeTest {
    const NAME: String = BUILTIN_STRING_MEMORY.test;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::test);
}
struct RegExpPrototypeToString;
impl Builtin for RegExpPrototypeToString {
    const NAME: String = BUILTIN_STRING_MEMORY.toString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::to_string);
}
struct RegExpPrototypeGetUnicode;
impl Builtin for RegExpPrototypeGetUnicode {
    const NAME: String = BUILTIN_STRING_MEMORY.get_unicode;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_unicode);
}
struct RegExpPrototypeGetUnicodeSets;
impl Builtin for RegExpPrototypeGetUnicodeSets {
    const NAME: String = BUILTIN_STRING_MEMORY.get_unicodeSets;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_unicode_sets);
}

impl RegExpPrototype {
    fn exec(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_dot_all(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_flags(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_global(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_has_indices(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_ignore_case(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn r#match(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn match_all(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_multiline(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn replace(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn search(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_source(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn split(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_sticky(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn test(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_string(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_unicode(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_unicode_sets(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.reg_exp_prototype();
        let reg_exp_constructor = intrinsics.reg_exp();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(19)
            .with_constructor_property(reg_exp_constructor)
            .with_builtin_function_property::<RegExpPrototypeExec>()
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.dotAll.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeGetDotAll>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.flags.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeGetFlags>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.global.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeGetGlobal>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.hasIndices.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeGetHasIndices>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.ignoreCase.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeGetIgnoreCase>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Match.into())
                    .with_value_creator_readonly(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeMatch>(agent, realm)
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::MatchAll.into())
                    .with_value_creator_readonly(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeMatchAll>(agent, realm)
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.multiline.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeGetMultiline>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Replace.into())
                    .with_value_creator_readonly(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeReplace>(agent, realm)
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Search.into())
                    .with_value_creator_readonly(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeSearch>(agent, realm)
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.source.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeGetSource>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Split.into())
                    .with_value_creator_readonly(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeSplit>(agent, realm)
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.sticky.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeGetSticky>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_builtin_function_property::<RegExpPrototypeTest>()
            .with_builtin_function_property::<RegExpPrototypeToString>()
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.unicode.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeGetUnicode>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.unicodeSets.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeGetUnicodeSets>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .build();
    }
}
