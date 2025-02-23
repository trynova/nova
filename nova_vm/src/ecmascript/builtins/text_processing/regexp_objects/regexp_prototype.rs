// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::type_conversion::to_boolean;
use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        abstract_operations::{operations_on_objects::get, type_conversion::to_string},
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsic},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{Object, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct RegExpPrototype;

struct RegExpPrototypeExec;
impl Builtin for RegExpPrototypeExec {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.exec;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::exec);
}
impl BuiltinIntrinsic for RegExpPrototypeExec {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::RegExpPrototypeExec;
}
struct RegExpPrototypeGetDotAll;
impl Builtin for RegExpPrototypeGetDotAll {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_dotAll;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.dotAll.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_dot_all);
}
impl BuiltinGetter for RegExpPrototypeGetDotAll {}
struct RegExpPrototypeGetFlags;
impl Builtin for RegExpPrototypeGetFlags {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_flags;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.flags.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_flags);
}
impl BuiltinGetter for RegExpPrototypeGetFlags {}
struct RegExpPrototypeGetGlobal;
impl Builtin for RegExpPrototypeGetGlobal {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_global;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.global.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_global);
}
impl BuiltinGetter for RegExpPrototypeGetGlobal {}
struct RegExpPrototypeGetHasIndices;
impl Builtin for RegExpPrototypeGetHasIndices {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_hasIndices;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.hasIndices.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_has_indices);
}
impl BuiltinGetter for RegExpPrototypeGetHasIndices {}
struct RegExpPrototypeGetIgnoreCase;
impl Builtin for RegExpPrototypeGetIgnoreCase {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_ignoreCase;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.ignoreCase.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_ignore_case);
}
impl BuiltinGetter for RegExpPrototypeGetIgnoreCase {}
struct RegExpPrototypeMatch;
impl Builtin for RegExpPrototypeMatch {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_match_;
    const KEY: Option<PropertyKey<'static>> = Some(WellKnownSymbolIndexes::Match.to_property_key());
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::r#match);
}
struct RegExpPrototypeMatchAll;
impl Builtin for RegExpPrototypeMatchAll {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_matchAll_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::MatchAll.to_property_key());
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::match_all);
}
struct RegExpPrototypeGetMultiline;
impl Builtin for RegExpPrototypeGetMultiline {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_multiline;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.multiline.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_multiline);
}
impl BuiltinGetter for RegExpPrototypeGetMultiline {}
struct RegExpPrototypeReplace;
impl Builtin for RegExpPrototypeReplace {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_replace_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Replace.to_property_key());
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::replace);
}
struct RegExpPrototypeSearch;
impl Builtin for RegExpPrototypeSearch {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_search_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Search.to_property_key());
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::search);
}
struct RegExpPrototypeGetSource;
impl Builtin for RegExpPrototypeGetSource {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_source;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.source.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_source);
}
impl BuiltinGetter for RegExpPrototypeGetSource {}
struct RegExpPrototypeSplit;
impl Builtin for RegExpPrototypeSplit {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_split_;
    const KEY: Option<PropertyKey<'static>> = Some(WellKnownSymbolIndexes::Split.to_property_key());
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::split);
}
struct RegExpPrototypeGetSticky;
impl Builtin for RegExpPrototypeGetSticky {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_sticky;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.sticky.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_sticky);
}
impl BuiltinGetter for RegExpPrototypeGetSticky {}
struct RegExpPrototypeTest;
impl Builtin for RegExpPrototypeTest {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.test;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::test);
}
struct RegExpPrototypeToString;
impl Builtin for RegExpPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::to_string);
}
struct RegExpPrototypeGetUnicode;
impl Builtin for RegExpPrototypeGetUnicode {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_unicode;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.unicode.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_unicode);
}
impl BuiltinGetter for RegExpPrototypeGetUnicode {}
struct RegExpPrototypeGetUnicodeSets;
impl Builtin for RegExpPrototypeGetUnicodeSets {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_unicodeSets;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.unicodeSets.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::get_unicode_sets);
}
impl BuiltinGetter for RegExpPrototypeGetUnicodeSets {}

impl RegExpPrototype {
    fn exec<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn get_dot_all<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    /// ### [22.2.6.4 get RegExp.prototype.flags](https://tc39.es/ecma262/#sec-get-regexp.prototype.flags)
    fn get_flags<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let R be the this value.
        let r = this_value;

        // 2. If R is not an Object, throw a TypeError exception.
        let Ok(r) = Object::try_from(r) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "value is not object",
                gc.nogc(),
            ));
        };

        // 3. Let codeUnits be a new empty List.
        let mut code_units: [u8; 8] = [0; 8];
        let mut i: usize = 0;

        // 4. Let hasIndices be ToBoolean(? Get(R, "hasIndices")).
        let r = r.scope(agent, gc.nogc());
        let has_indices_args = get(
            agent,
            r.get(agent),
            BUILTIN_STRING_MEMORY.hasIndices.into(),
            gc.reborrow(),
        )?;
        let has_indices = to_boolean(agent, has_indices_args);

        // 5. If hasIndices is true, append the code unit 0x0064 (LATIN SMALL LETTER D) to codeUnits.
        if has_indices {
            code_units[i] = b'd';
            i += 1;
        };

        // 6. Let global be ToBoolean(? Get(R, "global")).
        let global_args = get(
            agent,
            r.get(agent),
            BUILTIN_STRING_MEMORY.global.into(),
            gc.reborrow(),
        )?;
        let global = to_boolean(agent, global_args);

        // 7. If global is true, append the code unit 0x0067 (LATIN SMALL LETTER G) to codeUnits.
        if global {
            code_units[i] = b'g';
            i += 1;
        };

        // 8. Let ignoreCase be ToBoolean(? Get(R, "ignoreCase")).
        let ignore_case_args = get(
            agent,
            r.get(agent),
            BUILTIN_STRING_MEMORY.ignoreCase.into(),
            gc.reborrow(),
        )?;
        let ignore_case = to_boolean(agent, ignore_case_args);

        // 9. If ignoreCase is true, append the code unit 0x0069 (LATIN SMALL LETTER I) to codeUnits.
        if ignore_case {
            code_units[i] = b'i';
            i += 1;
        };

        // 10. Let multiline be ToBoolean(? Get(R, "multiline")).
        let mutliline_args = get(
            agent,
            r.get(agent),
            BUILTIN_STRING_MEMORY.multiline.into(),
            gc.reborrow(),
        )?;
        let multiline = to_boolean(agent, mutliline_args);

        // 11. If multiline is true, append the code unit 0x006D (LATIN SMALL LETTER M) to codeUnits.
        if multiline {
            code_units[i] = b'm';
        };

        // 12. Let dotAll be ToBoolean(? Get(R, "dotAll")).
        let dot_all_args = get(
            agent,
            r.get(agent),
            BUILTIN_STRING_MEMORY.dotAll.into(),
            gc.reborrow(),
        )?;
        let dot_all = to_boolean(agent, dot_all_args);

        // 13. If dotAll is true, append the code unit 0x0073 (LATIN SMALL LETTER S) to codeUnits.
        if dot_all {
            code_units[i] = b's';
            i += 1;
        };

        // 14. Let unicode be ToBoolean(? Get(R, "unicode")).
        let unicode_args = get(
            agent,
            r.get(agent),
            BUILTIN_STRING_MEMORY.unicode.into(),
            gc.reborrow(),
        )?;
        let unicode = to_boolean(agent, unicode_args);

        // 15. If unicode is true, append the code unit 0x0075 (LATIN SMALL LETTER U) to codeUnits.
        if unicode {
            code_units[i] = b'u';
            i += 1;
        };

        // 16. Let unicodeSets be ToBoolean(? Get(R, "unicodeSets")).
        let unicode_sets_args = get(
            agent,
            r.get(agent),
            BUILTIN_STRING_MEMORY.unicodeSets.into(),
            gc.reborrow(),
        )?;
        let unicode_sets = to_boolean(agent, unicode_sets_args);

        // 17. If unicodeSets is true, append the code unit 0x0076 (LATIN SMALL LETTER V) to codeUnits.
        if unicode_sets {
            code_units[i] = b'v';
            i += 1;
        };

        // 18. Let sticky be ToBoolean(? Get(R, "sticky")).
        let sticky_args = get(
            agent,
            r.get(agent),
            BUILTIN_STRING_MEMORY.sticky.into(),
            gc.reborrow(),
        )?;
        let sticky = to_boolean(agent, sticky_args);

        // 19. If sticky is true, append the code unit 0x0079 (LATIN SMALL LETTER Y) to codeUnits.
        if sticky {
            code_units[i] = b'v';
            i += 1;
        };

        // 20. Return the String value whose code units are the elements of the List codeUnits. If codeUnits has no elements, the empty String is returned.
        let res = unsafe { core::str::from_utf8_unchecked(&code_units[0..i]) };
        Ok(Value::from_string(agent, res.to_string(), gc.nogc()).unbind())
    }

    fn get_global<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn get_has_indices<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn get_ignore_case<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn r#match<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn match_all<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn get_multiline<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn replace<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn search<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn get_source<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn split<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn get_sticky<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn test<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    /// ### [22.2.6.17 RegExp.prototype.toString ( )](https://tc39.es/ecma262/#sec-regexp.prototype.tostring)
    ///
    /// > #### Note
    /// >
    /// > The returned String has the form of a RegularExpressionLiteral that
    /// > evaluates to another RegExp object with the same behaviour as this
    /// > object.
    fn to_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let R be the this value.
        // 2. If R is not an Object, throw a TypeError exception.
        let Ok(r) = Object::try_from(this_value) else {
            let error_message = format!(
                "{} is not an object",
                this_value.string_repr(agent, gc.reborrow()).as_str(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc.nogc()));
        };
        if let Object::RegExp(r) = r {
            // Fast path for RegExp objects: This is not actually proper as it
            // does not take into account prototype mutations.
            let data = &agent[r];
            let string_length = data.original_source.len(agent);
            let flags_length = data.original_flags.iter().count();
            let mut regexp_string =
                std::string::String::with_capacity(1 + string_length + 1 + flags_length);
            regexp_string.push('/');
            regexp_string.push_str(data.original_source.as_str(agent));
            regexp_string.push('/');
            data.original_flags.iter_names().for_each(|(flag, _)| {
                regexp_string.push_str(flag);
            });
            return Ok(String::from_string(agent, regexp_string, gc.nogc()).into_value());
        }
        // 3. Let pattern be ? ToString(? Get(R, "source")).
        let pattern = get(agent, r, BUILTIN_STRING_MEMORY.source.into(), gc.reborrow())?;
        let pattern = to_string(agent, pattern, gc.reborrow())?
            .unbind()
            .scope(agent, gc.nogc());
        // 4. Let flags be ? ToString(? Get(R, "flags")).
        let flags = get(agent, r, BUILTIN_STRING_MEMORY.flags.into(), gc.reborrow())?;
        let flags = to_string(agent, flags, gc.reborrow())?
            .unbind()
            .bind(gc.nogc());
        // 5. Let result be the string-concatenation of "/", pattern, "/", and flags.
        let result = format!(
            "/{}/{}",
            pattern.get(agent).bind(gc.nogc()).as_str(agent),
            flags.as_str(agent)
        );
        let result = String::from_string(agent, result, gc.nogc());
        // 6. Return result.
        Ok(result.into_value())
    }

    fn get_unicode<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn get_unicode_sets<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
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
            .with_builtin_function_property::<RegExpPrototypeMatch>()
            .with_builtin_function_property::<RegExpPrototypeMatchAll>()
            .with_builtin_function_getter_property::<RegExpPrototypeGetMultiline>()
            .with_builtin_function_property::<RegExpPrototypeReplace>()
            .with_builtin_function_property::<RegExpPrototypeSearch>()
            .with_builtin_function_property::<RegExpPrototypeSplit>()
            .build();
    }
}
