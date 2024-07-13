// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsic},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoValue, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct RegExpPrototype;

struct RegExpPrototypeExec;
impl Builtin for RegExpPrototypeExec {
    const NAME: String = BUILTIN_STRING_MEMORY.exec;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::exec);
}
impl BuiltinIntrinsic for RegExpPrototypeExec {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::RegExpPrototypeExec;
}
struct RegExpPrototypeGetDotAll;
impl Builtin for RegExpPrototypeGetDotAll {
    const NAME: String = BUILTIN_STRING_MEMORY.get_dotAll;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_dot_all);
}
impl BuiltinGetter for RegExpPrototypeGetDotAll {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.dotAll.to_property_key();
}
struct RegExpPrototypeGetFlags;
impl Builtin for RegExpPrototypeGetFlags {
    const NAME: String = BUILTIN_STRING_MEMORY.get_flags;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_flags);
}
impl BuiltinGetter for RegExpPrototypeGetFlags {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.flags.to_property_key();
}
struct RegExpPrototypeGetGlobal;
impl Builtin for RegExpPrototypeGetGlobal {
    const NAME: String = BUILTIN_STRING_MEMORY.get_global;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_global);
}
impl BuiltinGetter for RegExpPrototypeGetGlobal {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.global.to_property_key();
}
struct RegExpPrototypeGetHasIndices;
impl Builtin for RegExpPrototypeGetHasIndices {
    const NAME: String = BUILTIN_STRING_MEMORY.get_hasIndices;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_has_indices);
}
impl BuiltinGetter for RegExpPrototypeGetHasIndices {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.hasIndices.to_property_key();
}
struct RegExpPrototypeGetIgnoreCase;
impl Builtin for RegExpPrototypeGetIgnoreCase {
    const NAME: String = BUILTIN_STRING_MEMORY.get_ignoreCase;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_ignore_case);
}
impl BuiltinGetter for RegExpPrototypeGetIgnoreCase {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.ignoreCase.to_property_key();
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
impl BuiltinGetter for RegExpPrototypeGetMultiline {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.multiline.to_property_key();
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
impl BuiltinGetter for RegExpPrototypeGetSource {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.source.to_property_key();
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
impl BuiltinGetter for RegExpPrototypeGetSticky {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.sticky.to_property_key();
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
impl BuiltinGetter for RegExpPrototypeGetUnicode {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.unicode.to_property_key();
}
struct RegExpPrototypeGetUnicodeSets;
impl Builtin for RegExpPrototypeGetUnicodeSets {
    const NAME: String = BUILTIN_STRING_MEMORY.get_unicodeSets;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_unicode_sets);
}
impl BuiltinGetter for RegExpPrototypeGetUnicodeSets {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.unicodeSets.to_property_key();
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
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.reg_exp_prototype();
        let reg_exp_constructor = intrinsics.reg_exp();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(19)
            .with_prototype(object_prototype)
            .with_constructor_property(reg_exp_constructor)
            .with_builtin_intrinsic_function_property::<RegExpPrototypeExec>()
            .with_builtin_function_getter_property::<RegExpPrototypeGetDotAll>()
            .with_builtin_function_getter_property::<RegExpPrototypeGetFlags>()
            .with_builtin_function_getter_property::<RegExpPrototypeGetGlobal>()
            .with_builtin_function_getter_property::<RegExpPrototypeGetHasIndices>()
            .with_builtin_function_getter_property::<RegExpPrototypeGetIgnoreCase>()
            .with_builtin_function_getter_property::<RegExpPrototypeGetSource>()
            .with_builtin_function_getter_property::<RegExpPrototypeGetSticky>()
            .with_builtin_function_property::<RegExpPrototypeTest>()
            .with_builtin_function_property::<RegExpPrototypeToString>()
            .with_builtin_function_getter_property::<RegExpPrototypeGetUnicode>()
            .with_builtin_function_getter_property::<RegExpPrototypeGetUnicodeSets>()
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
            .with_builtin_function_getter_property::<RegExpPrototypeGetMultiline>()
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
                    .with_key(WellKnownSymbolIndexes::Split.into())
                    .with_value_creator_readonly(|agent| {
                        BuiltinFunctionBuilder::new::<RegExpPrototypeSplit>(agent, realm)
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .build()
            })
            .build();
    }
}
