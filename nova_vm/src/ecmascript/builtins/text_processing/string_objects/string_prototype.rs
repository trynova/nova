use crate::{
    ecmascript::{
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct StringPrototype;

struct StringPrototypeGetAt;
impl Builtin for StringPrototypeGetAt {
    const NAME: String = BUILTIN_STRING_MEMORY.at;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::at);
}
struct StringPrototypeCharAt;
impl Builtin for StringPrototypeCharAt {
    const NAME: String = BUILTIN_STRING_MEMORY.charAt;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::char_at);
}
struct StringPrototypeCharCodeAt;
impl Builtin for StringPrototypeCharCodeAt {
    const NAME: String = BUILTIN_STRING_MEMORY.charCodeAt;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::char_code_at);
}
struct StringPrototypeCodePointAt;
impl Builtin for StringPrototypeCodePointAt {
    const NAME: String = BUILTIN_STRING_MEMORY.codePointAt;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::code_point_at);
}
struct StringPrototypeConcat;
impl Builtin for StringPrototypeConcat {
    const NAME: String = BUILTIN_STRING_MEMORY.concat;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::concat);
}
struct StringPrototypeEndsWith;
impl Builtin for StringPrototypeEndsWith {
    const NAME: String = BUILTIN_STRING_MEMORY.endsWith;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::ends_with);
}
struct StringPrototypeIncludes;
impl Builtin for StringPrototypeIncludes {
    const NAME: String = BUILTIN_STRING_MEMORY.includes;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::includes);
}
struct StringPrototypeIndexOf;
impl Builtin for StringPrototypeIndexOf {
    const NAME: String = BUILTIN_STRING_MEMORY.indexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::index_of);
}
struct StringPrototypeIsWellFormed;
impl Builtin for StringPrototypeIsWellFormed {
    const NAME: String = BUILTIN_STRING_MEMORY.isWellFormed;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::is_well_formed);
}
struct StringPrototypeGetTimezoneOffset;
impl Builtin for StringPrototypeGetTimezoneOffset {
    const NAME: String = BUILTIN_STRING_MEMORY.getTimezoneOffset;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::get_timezone_offset);
}
struct StringPrototypeLastIndexOf;
impl Builtin for StringPrototypeLastIndexOf {
    const NAME: String = BUILTIN_STRING_MEMORY.lastIndexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::last_index_of);
}
struct StringPrototypeLocaleCompare;
impl Builtin for StringPrototypeLocaleCompare {
    const NAME: String = BUILTIN_STRING_MEMORY.findIndex;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::locale_compare);
}
struct StringPrototypeMatch;
impl Builtin for StringPrototypeMatch {
    const NAME: String = BUILTIN_STRING_MEMORY.r#match;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::r#match);
}
struct StringPrototypeMatchAll;
impl Builtin for StringPrototypeMatchAll {
    const NAME: String = BUILTIN_STRING_MEMORY.matchAll;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::match_all);
}
struct StringPrototypeNormalize;
impl Builtin for StringPrototypeNormalize {
    const NAME: String = BUILTIN_STRING_MEMORY.normalize;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::normalize);
}
struct StringPrototypePadEnd;
impl Builtin for StringPrototypePadEnd {
    const NAME: String = BUILTIN_STRING_MEMORY.padEnd;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::pad_end);
}
struct StringPrototypePadStart;
impl Builtin for StringPrototypePadStart {
    const NAME: String = BUILTIN_STRING_MEMORY.flatMap;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::pad_start);
}
struct StringPrototypeRepeat;
impl Builtin for StringPrototypeRepeat {
    const NAME: String = BUILTIN_STRING_MEMORY.repeat;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::repeat);
}
struct StringPrototypeReplace;
impl Builtin for StringPrototypeReplace {
    const NAME: String = BUILTIN_STRING_MEMORY.replace;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::replace);
}
struct StringPrototypeReplaceAll;
impl Builtin for StringPrototypeReplaceAll {
    const NAME: String = BUILTIN_STRING_MEMORY.replaceAll;
    const LENGTH: u8 = 3;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::replace_all);
}
struct StringPrototypeSearch;
impl Builtin for StringPrototypeSearch {
    const NAME: String = BUILTIN_STRING_MEMORY.search;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::search);
}
struct StringPrototypeSlice;
impl Builtin for StringPrototypeSlice {
    const NAME: String = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::slice);
}
struct StringPrototypeSplit;
impl Builtin for StringPrototypeSplit {
    const NAME: String = BUILTIN_STRING_MEMORY.split;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::split);
}
struct StringPrototypeStartsWith;
impl Builtin for StringPrototypeStartsWith {
    const NAME: String = BUILTIN_STRING_MEMORY.startsWith;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::starts_with);
}
struct StringPrototypeToLocaleLowerCase;
impl Builtin for StringPrototypeToLocaleLowerCase {
    const NAME: String = BUILTIN_STRING_MEMORY.toLocaleLowerCase;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::to_locale_lower_case);
}
struct StringPrototypeToLocaleUpperCase;
impl Builtin for StringPrototypeToLocaleUpperCase {
    const NAME: String = BUILTIN_STRING_MEMORY.toLocaleUpperCase;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::to_locale_upper_case);
}
struct StringPrototypeToLowerCase;
impl Builtin for StringPrototypeToLowerCase {
    const NAME: String = BUILTIN_STRING_MEMORY.toLowerCase;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::to_lower_case);
}
struct StringPrototypeToString;
impl Builtin for StringPrototypeToString {
    const NAME: String = BUILTIN_STRING_MEMORY.toString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::to_string);
}
struct StringPrototypeToUpperCase;
impl Builtin for StringPrototypeToUpperCase {
    const NAME: String = BUILTIN_STRING_MEMORY.toUpperCase;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::to_upper_case);
}
struct StringPrototypeToWellFormed;
impl Builtin for StringPrototypeToWellFormed {
    const NAME: String = BUILTIN_STRING_MEMORY.toWellFormed;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::to_well_formed);
}
struct StringPrototypeTrim;
impl Builtin for StringPrototypeTrim {
    const NAME: String = BUILTIN_STRING_MEMORY.trim;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::trim);
}
struct StringPrototypeTrimEnd;
impl Builtin for StringPrototypeTrimEnd {
    const NAME: String = BUILTIN_STRING_MEMORY.trimEnd;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::trim_end);
}
impl BuiltinIntrinsic for StringPrototypeTrimEnd {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::StringPrototypeTrimEnd;
}
struct StringPrototypeTrimStart;
impl Builtin for StringPrototypeTrimStart {
    const NAME: String = BUILTIN_STRING_MEMORY.trimStart;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::trim_start);
}
impl BuiltinIntrinsic for StringPrototypeTrimStart {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::StringPrototypeTrimStart;
}
struct StringPrototypeValueOf;
impl Builtin for StringPrototypeValueOf {
    const NAME: String = BUILTIN_STRING_MEMORY.valueOf;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::value_of);
}
struct StringPrototypeIterator;
impl Builtin for StringPrototypeIterator {
    const NAME: String = BUILTIN_STRING_MEMORY._Symbol_iterator_;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::iterator);
}

impl StringPrototype {
    fn at(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn char_at(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn char_code_at(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn code_point_at(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn concat(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn ends_with(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn includes(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn index_of(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn is_well_formed(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_timezone_offset(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn last_index_of(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn locale_compare(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn r#match(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn match_all(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn normalize(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn pad_end(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn pad_start(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn repeat(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn replace(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn replace_all(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn search(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn slice(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn split(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn starts_with(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_locale_lower_case(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn to_locale_upper_case(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn to_lower_case(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_string(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_upper_case(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_well_formed(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn trim(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn trim_end(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn trim_start(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn value_of(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn iterator(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.string_prototype();
        let string_constructor = intrinsics.string();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(36)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<StringPrototypeGetAt>()
            .with_builtin_function_property::<StringPrototypeCharAt>()
            .with_builtin_function_property::<StringPrototypeCharCodeAt>()
            .with_builtin_function_property::<StringPrototypeCodePointAt>()
            .with_builtin_function_property::<StringPrototypeConcat>()
            .with_constructor_property(string_constructor)
            .with_builtin_function_property::<StringPrototypeEndsWith>()
            .with_builtin_function_property::<StringPrototypeIncludes>()
            .with_builtin_function_property::<StringPrototypeIndexOf>()
            .with_builtin_function_property::<StringPrototypeIsWellFormed>()
            .with_builtin_function_property::<StringPrototypeGetTimezoneOffset>()
            .with_builtin_function_property::<StringPrototypeLastIndexOf>()
            .with_builtin_function_property::<StringPrototypeLocaleCompare>()
            .with_builtin_function_property::<StringPrototypeMatch>()
            .with_builtin_function_property::<StringPrototypeMatchAll>()
            .with_builtin_function_property::<StringPrototypeNormalize>()
            .with_builtin_function_property::<StringPrototypePadEnd>()
            .with_builtin_function_property::<StringPrototypePadStart>()
            .with_builtin_function_property::<StringPrototypeRepeat>()
            .with_builtin_function_property::<StringPrototypeReplace>()
            .with_builtin_function_property::<StringPrototypeReplaceAll>()
            .with_builtin_function_property::<StringPrototypeSearch>()
            .with_builtin_function_property::<StringPrototypeSlice>()
            .with_builtin_function_property::<StringPrototypeSplit>()
            .with_builtin_function_property::<StringPrototypeStartsWith>()
            .with_builtin_function_property::<StringPrototypeToLocaleLowerCase>()
            .with_builtin_function_property::<StringPrototypeToLocaleUpperCase>()
            .with_builtin_function_property::<StringPrototypeToLowerCase>()
            .with_builtin_function_property::<StringPrototypeToString>()
            .with_builtin_function_property::<StringPrototypeToUpperCase>()
            .with_builtin_function_property::<StringPrototypeToWellFormed>()
            .with_builtin_function_property::<StringPrototypeTrim>()
            .with_builtin_intrinsic_function_property::<StringPrototypeTrimEnd>()
            .with_builtin_intrinsic_function_property::<StringPrototypeTrimStart>()
            .with_builtin_function_property::<StringPrototypeValueOf>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::Iterator.into())
                    .with_value_creator_readonly(|agent| {
                        BuiltinFunctionBuilder::new::<StringPrototypeIterator>(agent, realm)
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
