// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast::RegExpFlags;
use wtf8::Wtf8Buf;

use crate::{
    ecmascript::{
        Agent, ArgumentsList, Array, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinGetter,
        BuiltinIntrinsic, ExceptionType, Function, JsError, JsResult, Number, Object, PropertyKey,
        PropertyLookupCache, ProtoIntrinsics, Realm, String, TryGetResult, Value,
        advance_string_index, array_create, builders::OrdinaryObjectBuilder, call_function,
        construct, create_reg_exp_string_iterator, get, get_substitution, is_callable,
        length_of_array_like, reg_exp_builtin_exec, reg_exp_builtin_test, reg_exp_exec,
        reg_exp_test, require_internal_slot_reg_exp, same_value, set, species_constructor,
        to_boolean, to_integer_or_infinity, to_length, to_object, to_string, to_uint32,
        try_create_data_property_or_throw, try_get, unwrap_try,
    },
    engine::{Bindable, GcScope, NoGcScope, Scopable, Scoped},
    heap::{IntrinsicFunctionIndexes, WellKnownSymbols},
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
    const KEY: Option<PropertyKey<'static>> = Some(WellKnownSymbols::Match.to_property_key());
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::r#match);
}
struct RegExpPrototypeMatchAll;
impl Builtin for RegExpPrototypeMatchAll {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_matchAll_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbols::MatchAll.to_property_key());
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
        Some(WellKnownSymbols::Replace.to_property_key());
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpPrototype::replace);
}
struct RegExpPrototypeSearch;
impl Builtin for RegExpPrototypeSearch {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_search_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbols::Search.to_property_key());
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
    const KEY: Option<PropertyKey<'static>> = Some(WellKnownSymbols::Split.to_property_key());
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
    /// ### [22.2.6.2 RegExp.prototype.exec ( string )](https://tc39.es/ecma262/#sec-regexpbuiltinexec)
    ///
    /// This method searches string for an occurrence of the regular expression
    /// pattern and returns an Array containing the results of the match, or
    /// null if string did not match.
    fn exec<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let string = arguments.get(0).bind(gc.nogc());
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        // 2. Perform ? RequireInternalSlot(R, [[RegExpMatcher]]).
        let r = require_internal_slot_reg_exp(agent, r, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let r = r.scope(agent, gc.nogc());
        // 3. Let S be ? ToString(string).
        let s = to_string(agent, string.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // SAFETY: not shared.
        let r = unsafe { r.take(agent) }.bind(gc.nogc());
        // 4. Return ? RegExpBuiltinExec(R, S).
        reg_exp_builtin_exec(agent, r.unbind(), s.unbind(), gc)
            .map(|r| r.map_or(Value::Null, |a| a.into()))
    }

    /// ### [22.2.6.3 get RegExp.prototype.dotAll](https://tc39.es/ecma262/#sec-get-regexp.prototype.dotAll)
    ///
    /// RegExp.prototype.dotAll is an accessor property whose set accessor
    /// function is undefined.
    fn get_dot_all<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        // 2. Let cu be the code unit 0x0073 (LATIN SMALL LETTER S).
        // 3. Return ? RegExpHasFlag(R, cu).
        reg_exp_has_flag(agent, r.unbind(), RegExpFlags::S, gc.into_nogc())
            .map(|v| v.map_or(Value::Undefined, |v| v.into()))
    }

    /// ### [22.2.6.4 get RegExp.prototype.flags](https://tc39.es/ecma262/#sec-get-regexp.prototype.flags)
    fn get_flags<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        // 1. Let R be the this value.
        let r = this_value.bind(nogc);

        // 2. If R is not an Object, throw a TypeError exception.
        let Ok(r) = Object::try_from(r) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "value is not object",
                gc.into_nogc(),
            ));
        };

        // 3. Let codeUnits be a new empty List.
        let mut code_units: [u8; 8] = [0; 8];
        let mut i: usize = 0;

        // 4. Let hasIndices be ToBoolean(? Get(R, "hasIndices")).
        let scoped_r = r.scope(agent, nogc);
        let has_indices_args = get(
            agent,
            r.unbind(),
            BUILTIN_STRING_MEMORY.hasIndices.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let has_indices = to_boolean(agent, has_indices_args);

        // 5. If hasIndices is true, append the code unit 0x0064 (LATIN SMALL LETTER D) to codeUnits.
        if has_indices {
            code_units[i] = b'd';
            i += 1;
        };

        // 6. Let global be ToBoolean(? Get(R, "global")).
        let global_args = get(
            agent,
            scoped_r.get(agent),
            BUILTIN_STRING_MEMORY.global.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let global = to_boolean(agent, global_args);

        // 7. If global is true, append the code unit 0x0067 (LATIN SMALL LETTER G) to codeUnits.
        if global {
            code_units[i] = b'g';
            i += 1;
        };

        // 8. Let ignoreCase be ToBoolean(? Get(R, "ignoreCase")).
        let ignore_case_args = get(
            agent,
            scoped_r.get(agent),
            BUILTIN_STRING_MEMORY.ignoreCase.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let ignore_case = to_boolean(agent, ignore_case_args);

        // 9. If ignoreCase is true, append the code unit 0x0069 (LATIN SMALL LETTER I) to codeUnits.
        if ignore_case {
            code_units[i] = b'i';
            i += 1;
        };

        // 10. Let multiline be ToBoolean(? Get(R, "multiline")).
        let mutliline_args = get(
            agent,
            scoped_r.get(agent),
            BUILTIN_STRING_MEMORY.multiline.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let multiline = to_boolean(agent, mutliline_args);

        // 11. If multiline is true, append the code unit 0x006D (LATIN SMALL LETTER M) to codeUnits.
        if multiline {
            code_units[i] = b'm';
        };

        // 12. Let dotAll be ToBoolean(? Get(R, "dotAll")).
        let dot_all_args = get(
            agent,
            scoped_r.get(agent),
            BUILTIN_STRING_MEMORY.dotAll.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let dot_all = to_boolean(agent, dot_all_args);

        // 13. If dotAll is true, append the code unit 0x0073 (LATIN SMALL LETTER S) to codeUnits.
        if dot_all {
            code_units[i] = b's';
            i += 1;
        };

        // 14. Let unicode be ToBoolean(? Get(R, "unicode")).
        let unicode_args = get(
            agent,
            scoped_r.get(agent),
            BUILTIN_STRING_MEMORY.unicode.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let unicode = to_boolean(agent, unicode_args);

        // 15. If unicode is true, append the code unit 0x0075 (LATIN SMALL LETTER U) to codeUnits.
        if unicode {
            code_units[i] = b'u';
            i += 1;
        };

        // 16. Let unicodeSets be ToBoolean(? Get(R, "unicodeSets")).
        let unicode_sets_args = get(
            agent,
            scoped_r.get(agent),
            BUILTIN_STRING_MEMORY.unicodeSets.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let unicode_sets = to_boolean(agent, unicode_sets_args);

        // 17. If unicodeSets is true, append the code unit 0x0076 (LATIN SMALL LETTER V) to codeUnits.
        if unicode_sets {
            code_units[i] = b'v';
            i += 1;
        };

        // 18. Let sticky be ToBoolean(? Get(R, "sticky")).
        let sticky_args = get(
            agent,
            scoped_r.get(agent),
            BUILTIN_STRING_MEMORY.sticky.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
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

    /// ### [22.2.6.5 get RegExp.prototype.global](https://tc39.es/ecma262/#sec-get-regexp.prototype.global)
    ///
    /// RegExp.prototype.global is an accessor property whose set accessor
    /// function is undefined.
    fn get_global<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        // 2. Let cu be the code unit 0x0067 (LATIN SMALL LETTER G).
        // 3. Return ? RegExpHasFlag(R, cu).
        reg_exp_has_flag(agent, r.unbind(), RegExpFlags::G, gc.into_nogc())
            .map(|v| v.map_or(Value::Undefined, |v| v.into()))
    }

    /// ### [22.2.6.6 get RegExp.prototype.hasIndices](https://tc39.es/ecma262/#sec-get-regexp.prototype.hasIndices)
    ///
    /// RegExp.prototype.hasIndices is an accessor property whose set accessor
    /// function is undefined.
    fn get_has_indices<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        // 2. Let cu be the code unit 0x0064 (LATIN SMALL LETTER D).
        // 3. Return ? RegExpHasFlag(R, cu).
        reg_exp_has_flag(agent, r.unbind(), RegExpFlags::D, gc.into_nogc())
            .map(|v| v.map_or(Value::Undefined, |v| v.into()))
    }

    /// ### [22.2.6.7 get RegExp.prototype.ignoreCase](https://tc39.es/ecma262/#sec-get-regexp.prototype.ignorecase)
    ///
    /// RegExp.prototype.ignoreCase is an accessor property whose set accessor
    /// function is undefined.
    fn get_ignore_case<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        // 2. Let cu be the code unit 0x0069 (LATIN SMALL LETTER I).
        // 3. Return ? RegExpHasFlag(R, cu).
        reg_exp_has_flag(agent, r.unbind(), RegExpFlags::I, gc.into_nogc())
            .map(|v| v.map_or(Value::Undefined, |v| v.into()))
    }

    /// ### [22.2.6.8 RegExp.prototype \[ %Symbol.match% \] ( string )](https://tc39.es/ecma262/#sec-regexp.prototype-%symbol.match%)
    fn r#match<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let string = arguments.get(0).bind(gc.nogc());
        // 1. Let rx be the this value.
        let rx = this_value.bind(gc.nogc());
        // 2. If rx is not an Object, throw a TypeError exception.
        let Ok(rx) = Object::try_from(rx) else {
            return Err(throw_not_an_object(agent, gc.into_nogc()));
        };
        let rx = rx.scope(agent, gc.nogc());
        // 3. Let S be ? ToString(string).
        let s = to_string(agent, string.unbind(), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 4. Let flags be ? ToString(? Get(rx, "flags")).
        let flags = get(
            agent,
            rx.get(agent),
            BUILTIN_STRING_MEMORY.flags.to_property_key(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let flags = to_string(agent, flags.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 5. If flags does not contain "g", then
        if !flags.to_string_lossy_(agent).contains("g") {
            // a. Return ? RegExpExec(rx, S).
            reg_exp_exec(
                agent,
                // SAFETY: not shared.
                unsafe { rx.take(agent) },
                // SAFETY: not shared.
                unsafe { s.take(agent) },
                gc,
            )
            .map(|o| o.map_or(Value::Null, |o| o.into()))
        } else {
            // 6. Else,
            // a. If flags contains "u" or flags contains "v", let fullUnicode
            //    be true. Otherwise, let fullUnicode be false.
            let full_unicode = flags.to_string_lossy_(agent).contains("u")
                || flags.to_string_lossy_(agent).contains("v");
            // b. Perform ? Set(rx, "lastIndex", +0𝔽, true).
            set(
                agent,
                rx.get(agent),
                BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                0.into(),
                true,
                gc.reborrow(),
            )
            .unbind()?;
            // c. Let A be ! ArrayCreate(0).
            let a = array_create(agent, 0, 0, None, gc.nogc())
                .unwrap()
                .scope(agent, gc.nogc());
            // d. Let n be 0.
            let mut n = 0u32;
            // e. Repeat,
            loop {
                // i. Let result be ? RegExpExec(rx, S).
                let result = reg_exp_exec(agent, rx.get(agent), s.get(agent), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // ii. If result is null, then
                let Some(result) = result else {
                    // 1. If n = 0, return null.
                    if n == 0 {
                        return Ok(Value::Null);
                    } else {
                        // 2. Return A.
                        // SAFETY: not shared.
                        return Ok(unsafe { a.take(agent) }.into());
                    }
                };
                // iii. Else,
                // 1. Let matchStr be ? ToString(? Get(result, "0")).
                let match_str = get(agent, result.unbind(), 0.into(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                let match_str = to_string(agent, match_str.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // 2. Perform ! CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), matchStr).
                unwrap_try(try_create_data_property_or_throw(
                    agent,
                    a.get(agent),
                    n.into(),
                    match_str.into(),
                    None,
                    gc.nogc(),
                ));
                // 3. If matchStr is the empty String, then
                if match_str.is_empty_string() {
                    // a. Let thisIndex be ℝ(? ToLength(? Get(rx, "lastIndex"))).
                    let this_index = get(
                        agent,
                        rx.get(agent),
                        BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc());
                    let this_index = to_length(agent, this_index.unbind(), gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc());
                    // b. Let nextIndex be AdvanceStringIndex(S, thisIndex, fullUnicode).
                    let next_index = advance_string_index(
                        agent,
                        s.get(agent),
                        this_index as usize,
                        full_unicode,
                    );
                    // c. Perform ? Set(rx, "lastIndex", 𝔽(nextIndex), true).
                    set(
                        agent,
                        rx.get(agent),
                        BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                        Number::try_from(next_index).unwrap().into(),
                        true,
                        gc.reborrow(),
                    )
                    .unbind()?
                }
                // 4. Set n to n + 1.
                n += 1;
            }
        }
    }

    /// ### [22.2.6.9 RegExp.prototype \[ %Symbol.matchAll% \] ( string )](https://tc39.es/ecma262/#sec-regexp-prototype-%symbol.matchall%)
    ///
    /// The value of the "name" property of this method is "\[Symbol.matchAll]".
    fn match_all<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let string = args.get(0).bind(gc.nogc());
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        // 2. If R is not an Object, throw a TypeError exception.
        let Ok(r) = Object::try_from(r) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "this is not an object",
                gc.into_nogc(),
            ));
        };
        let scoped_r = r.scope(agent, gc.nogc());
        // 3. Let S be ? ToString(string).
        let s = to_string(agent, string.unbind(), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 4. Let C be ? SpeciesConstructor(R, %RegExp%).

        let c = species_constructor(
            agent,
            scoped_r.get(agent),
            ProtoIntrinsics::RegExp,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let c = if c == agent.current_realm_record().intrinsics().reg_exp().into() {
            None
        } else {
            Some(c.scope(agent, gc.nogc()))
        };
        // 5. Let flags be ? ToString(? Get(R, "flags")).
        let flags = get(
            agent,
            scoped_r.get(agent),
            BUILTIN_STRING_MEMORY.flags.to_property_key(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let flags = to_string(agent, flags.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let scoped_flags = flags.scope(agent, gc.nogc());
        let c = if let Some(c) = c {
            unsafe { c.take(agent) }.bind(gc.nogc())
        } else {
            agent
                .current_realm_record()
                .intrinsics()
                .reg_exp()
                .bind(gc.nogc())
                .into()
        };
        // 6. Let matcher be ? Construct(C, « R, flags »).
        let matcher = construct(
            agent,
            c.unbind(),
            Some(ArgumentsList::from_mut_slice(&mut [
                scoped_r.get(agent).into(),
                flags.unbind().into(),
            ])),
            None,
            gc.reborrow(),
        )
        .unbind()?
        .scope(agent, gc.nogc());
        // 7. Let lastIndex be ? ToLength(? Get(R, "lastIndex")).
        let last_index = get(
            agent,
            scoped_r.get(agent),
            BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let last_index = to_length(agent, last_index.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 8. Perform ? Set(matcher, "lastIndex", lastIndex, true).
        set(
            agent,
            matcher.get(agent),
            BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
            last_index.try_into().unwrap(),
            true,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let flags = scoped_flags.get(agent).bind(gc.nogc());
        let flags = flags.as_bytes_(agent);
        // 9. If flags contains "g", let global be true.
        // 10. Else, let global be false.
        let global = flags.contains(&b'g');
        // 11. If flags contains "u" or flags contains "v", let fullUnicode be true.
        // 12. Else, let fullUnicode be false.
        let full_unicode = flags.contains(&b'u') | flags.contains(&b'v');
        // 13. Return CreateRegExpStringIterator(matcher, S, global, fullUnicode).
        Ok(create_reg_exp_string_iterator(
            agent,
            matcher.get(agent),
            s.get(agent),
            global,
            full_unicode,
            gc.into_nogc(),
        )
        .into())
    }

    /// ### [22.2.6.10 get RegExp.prototype.multiline](https://tc39.es/ecma262/#sec-get-regexp.prototype.multiline)
    ///
    /// RegExp.prototype.multiline is an accessor property whose set accessor
    /// function is undefined.
    fn get_multiline<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        // 2. Let cu be the code unit 0x006D (LATIN SMALL LETTER M).
        // 3. Return ? RegExpHasFlag(R, cu).
        reg_exp_has_flag(agent, r.unbind(), RegExpFlags::M, gc.into_nogc())
            .map(|v| v.map_or(Value::Undefined, |v| v.into()))
    }

    /// ## [22.2.6.11 RegExp.prototype \[ %Symbol.replace% \] ( string, replaceValue )](https://tc39.es/ecma262/#sec-regexp.prototype-%symbol.replace%)
    ///
    /// The value of the "name" property of this method is "\[Symbol.replace]".
    fn replace<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let string = args.get(0).bind(gc.nogc());
        let replace_value = args.get(1).scope(agent, gc.nogc());
        // 1. Let rx be the this value.
        let rx = this_value.bind(gc.nogc());
        // 2. If rx is not an Object, throw a TypeError exception.
        let Ok(rx) = Object::try_from(rx) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "this is not an object",
                gc.into_nogc(),
            ));
        };
        let rx = rx.scope(agent, gc.nogc());
        // 3. Let S be ? ToString(string).
        let s = to_string(agent, string.unbind(), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 4. Let lengthS be the length of S.
        let length_s = s.get(agent).utf16_len_(agent);
        #[derive(Clone)]
        enum ReplaceValue<'a> {
            Functional(Scoped<'a, Function<'static>>),
            String(Scoped<'a, String<'static>>),
        }
        // 5. Let functionalReplace be IsCallable(replaceValue).
        let (replace_value, functional_replace) = if let Some(functional_replace) =
            is_callable(replace_value.get(agent), gc.nogc())
        {
            // SAFETY: replace_value is not shared.
            (
                ReplaceValue::Functional(unsafe {
                    replace_value.replace_self(agent, functional_replace.unbind())
                }),
                true,
            )
        } else {
            // 6. If functionalReplace is false, then
            // a. Set replaceValue to ? ToString(replaceValue).
            let string = to_string(agent, replace_value.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: replace_value is not shared.
            (
                ReplaceValue::String(unsafe { replace_value.replace_self(agent, string.unbind()) }),
                false,
            )
        };
        // 7. Let flags be ? ToString(? Get(rx, "flags")).
        let flags = get(
            agent,
            rx.get(agent),
            BUILTIN_STRING_MEMORY.flags.to_property_key(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let flags = to_string(agent, flags.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 8. If flags contains "g", let global be true; otherwise let global be false.
        let flags = flags.as_bytes_(agent);
        let global = flags.contains(&b'g');
        // b. If flags contains "u" or flags contains "v", let fullUnicode be
        //    true; otherwise let fullUnicode be false.
        let full_unicode = flags.contains(&b'u') | flags.contains(&b'v');
        // 9. If global is true, then
        if global {
            // a. Perform ? Set(rx, "lastIndex", +0𝔽, true).
            set(
                agent,
                rx.get(agent),
                BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                0.into(),
                true,
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
        }
        // 10. Let results be a new empty List.
        let mut results = vec![];
        // 11. Let done be false.
        // 12. Repeat, while done is false,
        loop {
            // a. Let result be ? RegExpExec(rx, S).
            let result = reg_exp_exec(agent, rx.get(agent), s.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // b. If result is null, then
            let Some(result) = result else {
                // i. Set done to true.
                break;
            };
            // c. Else,
            // i. Append result to results.
            results.push(result.scope(agent, gc.nogc()));
            // ii. If global is false, then
            if !global {
                // 1. Set done to true.
                break;
            }
            // iii. Else,
            // 1. Let matchStr be ? ToString(? Get(result, "0")).
            let match_str = get(agent, result.unbind(), 0.into(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            let match_str = to_string(agent, match_str.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // 2. If matchStr is the empty String, then
            if match_str.is_empty_string() {
                // a. Let thisIndex be ℝ(? ToLength(? Get(rx, "lastIndex"))).
                let this_index = get(
                    agent,
                    rx.get(agent),
                    BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                let this_index = to_length(agent, this_index.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                let this_index = usize::try_from(this_index).expect("thisIndex not valid usize");
                // b. If flags contains "u" or flags contains "v", let
                //    fullUnicode be true; otherwise let fullUnicode be false.
                // c. Let nextIndex be AdvanceStringIndex(S, thisIndex, fullUnicode).
                let next_index =
                    advance_string_index(agent, s.get(agent), this_index, full_unicode);
                // d. Perform ? Set(rx, "lastIndex", 𝔽(nextIndex), true).
                set(
                    agent,
                    rx.get(agent),
                    BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                    Number::try_from(next_index).unwrap().into(),
                    true,
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
            }
        }
        // 13. Let accumulatedResult be the empty String.
        let mut accumulated_result = Wtf8Buf::new();
        // 14. Let nextSourcePosition be 0.
        let mut next_source_position = 0;
        // 15. For each element result of results, do
        for result in results {
            // a. Let resultLength be ? LengthOfArrayLike(result).
            let result_length = length_of_array_like(agent, result.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc()) as u64;
            // b. Let nCaptures be max(resultLength - 1, 0).
            let n_captures = result_length.saturating_sub(1);
            // c. Let matched be ? ToString(? Get(result, "0")).
            let matched = get(agent, result.get(agent), 0.into(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            let matched = to_string(agent, matched.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // d. Let matchLength be the length of matched.
            let match_length = matched.utf16_len_(agent);
            let matched = matched.scope(agent, gc.nogc());
            // e. Let position be ? ToIntegerOrInfinity(? Get(result, "index")).
            let position = get(
                agent,
                result.get(agent),
                BUILTIN_STRING_MEMORY.index.into(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            let position = to_integer_or_infinity(agent, position.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // f. Set position to the result of clamping position between 0 and lengthS.
            let position = position.into_i64().clamp(0, length_s as i64) as usize;
            // g. Let captures be a new empty List.
            let mut captures =
                Vec::with_capacity(n_captures as usize + if functional_replace { 3 } else { 0 });
            // h. Let n be 1.
            // i. Repeat, while n ≤ nCaptures,
            for n in 1..=n_captures {
                // i. Let capN be ? Get(result, ! ToString(𝔽(n))).
                let cap_n = get(
                    agent,
                    result.get(agent),
                    n.try_into().unwrap(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // ii. If capN is not undefined, then
                let cap_n = if cap_n.is_undefined() {
                    None
                } else {
                    // 1. Set capN to ? ToString(capN).
                    Some(
                        to_string(agent, cap_n.unbind(), gc.reborrow())
                            .unbind()?
                            .scope(agent, gc.nogc()),
                    )
                };
                // iii. Append capN to captures.
                captures.push(cap_n);
                // iv. NOTE: When n = 1, the preceding step puts the first
                //     element into captures (at index 0). More generally, the
                //     nth capture (the characters captured by the nth set of
                //     capturing parentheses) is at captures[n - 1].
                // v. Set n to n + 1.
            }
            // j. Let namedCaptures be ? Get(result, "groups").
            let named_captures = get(
                agent,
                result.get(agent),
                BUILTIN_STRING_MEMORY.groups.to_property_key(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // k. If functionalReplace is true, then
            let replacement_string = match replace_value.clone() {
                ReplaceValue::Functional(replace_value) => {
                    // i. Let replacerArgs be the list-concatenation of
                    //    « matched », captures, and « 𝔽(position), S ».
                    let mut replacer_args = captures
                        .into_iter()
                        .map(|s| {
                            s.map_or(Value::Undefined, |s| s.get(agent).bind(gc.nogc()).into())
                        })
                        .collect::<Vec<_>>();
                    replacer_args.insert(0, matched.get(agent).into());
                    replacer_args.push(Number::try_from(position).unwrap().into());
                    // ii. If namedCaptures is not undefined, then
                    if !named_captures.is_undefined() {
                        // 1. Append namedCaptures to replacerArgs.
                        replacer_args.push(named_captures);
                    }
                    // iii. Let replacementValue be ? Call(replaceValue, undefined, replacerArgs).
                    let replacement_value = call_function(
                        agent,
                        replace_value.get(agent),
                        Value::Undefined,
                        Some(ArgumentsList::from_mut_slice(&mut replacer_args.unbind())),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc());
                    // iv. Let replacementString be ? ToString(replacementValue).
                    to_string(agent, replacement_value.unbind(), gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc())
                }
                ReplaceValue::String(replace_value) => {
                    // l. Else,
                    // i. If namedCaptures is not undefined, then
                    let named_captures = if named_captures.is_undefined() {
                        None
                    } else {
                        // 1. Set namedCaptures to ? ToObject(namedCaptures).
                        Some(
                            to_object(agent, named_captures, gc.nogc())
                                .unbind()?
                                .bind(gc.nogc()),
                        )
                    };
                    // ii. Let replacementString be
                    //     ? GetSubstitution(
                    get_substitution(
                        agent,
                        // matched,
                        matched,
                        // S,
                        s.clone(),
                        // position,
                        position,
                        // captures,
                        captures,
                        // namedCaptures,
                        named_captures.unbind(),
                        // replaceValue
                        replace_value,
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc())
                    // ).
                }
            };
            // m. If position ≥ nextSourcePosition, then
            if position >= next_source_position {
                // i. NOTE: position should not normally move backwards. If it
                //    does, it is an indication of an ill-behaving RegExp
                //    subclass or use of an access triggered side-effect to
                //    change the global flag or other characteristics of rx. In
                //    such cases, the corresponding substitution is ignored.
                // ii. Set accumulatedResult to the string-concatenation of
                //     accumulatedResult, the substring of S from
                //     nextSourcePosition to position, and replacementString.
                let s = s.get(agent).bind(gc.nogc());
                let next_source_position_utf8 = s.utf8_index_(agent, next_source_position).unwrap();
                let position_utf8 = s.utf8_index_(agent, position).unwrap();
                accumulated_result.push_wtf8(
                    s.as_wtf8_(agent)
                        .slice(next_source_position_utf8, position_utf8),
                );
                accumulated_result.push_wtf8(replacement_string.as_wtf8_(agent));
                // iii. Set nextSourcePosition to position + matchLength.
                next_source_position = position + match_length;
            }
        }
        // 16. If nextSourcePosition ≥ lengthS, return accumulatedResult.
        if next_source_position < length_s {
            // 17. Return the string-concatenation of accumulatedResult and the
            // substring of S from nextSourcePosition.
            let s = s.get(agent).bind(gc.nogc());
            let next_source_position_utf8 = s.utf8_index_(agent, next_source_position).unwrap();
            accumulated_result.push_wtf8(s.as_wtf8_(agent).slice_from(next_source_position_utf8));
        }
        Ok(String::from_wtf8_buf(agent, accumulated_result, gc.into_nogc()).into())
    }

    /// ### [22.2.6.12 RegExp.prototype \[ %Symbol.search% \] ( string )](https://tc39.es/ecma262/#sec-regexp.prototype-%symbol.search%)
    ///
    /// The value of the "name" property of this method is "\[Symbol.search]".
    ///
    /// > NOTE: The "lastIndex" and "global" properties of this RegExp object are
    /// > ignored when performing the search. The "lastIndex" property is left
    /// > unchanged.
    fn search<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let string = args.get(0).bind(gc.nogc());
        // 1. Let rx be the this value.
        let rx = this_value.bind(gc.nogc());
        // 2. If rx is not an Object, throw a TypeError exception.
        let Ok(rx) = Object::try_from(rx) else {
            return Err(throw_not_an_object(agent, gc.into_nogc()));
        };
        let rx = rx.scope(agent, gc.nogc());
        // 3. Let S be ? ToString(string).
        let s = to_string(agent, string.unbind(), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 4. Let previousLastIndex be ? Get(rx, "lastIndex").
        let previous_last_index = get(
            agent,
            rx.get(agent),
            BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let scoped_previous_last_index = previous_last_index.scope(agent, gc.nogc());
        // 5. If previousLastIndex is not +0𝔽, then
        if previous_last_index != Number::pos_zero().into() {
            // a. Perform ? Set(rx, "lastIndex", +0𝔽, true).
            set(
                agent,
                rx.get(agent),
                BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                Number::pos_zero().into(),
                true,
                gc.reborrow(),
            )
            .unbind()?;
        }
        // 6. Let result be ? RegExpExec(rx, S).
        let result = reg_exp_exec(agent, rx.get(agent), s.get(agent), gc.reborrow())
            .unbind()?
            .map(|r| r.scope(agent, gc.nogc()));
        // 7. Let currentLastIndex be ? Get(rx, "lastIndex").
        let current_last_index = get(
            agent,
            rx.get(agent),
            BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let previous_last_index = unsafe { scoped_previous_last_index.take(agent) }.bind(gc.nogc());
        // 8. If SameValue(currentLastIndex, previousLastIndex) is false, then
        if !same_value(agent, current_last_index, previous_last_index) {
            // a. Perform ? Set(rx, "lastIndex", previousLastIndex, true).
            set(
                agent,
                rx.get(agent),
                BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                previous_last_index.unbind(),
                true,
                gc.reborrow(),
            )
            .unbind()?;
        }
        if let Some(result) = result.map(|r| unsafe { r.take(agent) }.bind(gc.nogc())) {
            // 10. Return ? Get(result, "index").
            get(
                agent,
                result.unbind(),
                BUILTIN_STRING_MEMORY.index.to_property_key(),
                gc,
            )
        } else {
            // 9. If result is null, return -1𝔽.
            Ok(Number::from(-1).into())
        }
    }

    /// ### [22.2.6.13 get RegExp.prototype.source](https://tc39.es/ecma262/#sec-get-regexp.prototype.source)
    ///
    /// RegExp.prototype.source is an accessor property whose set accessor
    /// function is undefined.
    fn get_source<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        // 2. If R is not an Object, throw a TypeError exception.
        let Ok(r) = Object::try_from(r) else {
            return Err(throw_not_an_object(agent, gc.into_nogc()));
        };
        // 3. If R does not have an [[OriginalSource]] internal slot, then
        let Object::RegExp(r) = r else {
            // a. If SameValue(R, %RegExp.prototype%) is true, return "(?:)".
            if r == agent
                .current_realm_record()
                .intrinsics()
                .reg_exp_prototype()
                .into()
            {
                return Ok(String::from_small_string("(?:)").into());
            }
            // b. Otherwise, throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected RegExp object or %RegExp.prototype% intrinsic object",
                gc.into_nogc(),
            ));
        };
        // 4. Assert: R has an [[OriginalFlags]] internal slot.
        // 5. Let src be R.[[OriginalSource]].
        let src = r.original_source(agent);
        // 6. Let flags be R.[[OriginalFlags]].
        let flags = r.original_flags(agent);
        if src.is_empty_string() {
            Ok(String::from_small_string("(?:)").into())
        } else {
            // 7. Return EscapeRegExpPattern(src, flags).
            Ok(escape_reg_exp_pattern(agent, src.unbind(), flags, gc.into_nogc()).into())
        }
    }

    /// ### [22.2.6.14 RegExp.prototype \[ %Symbol.split% \] ( string, limit )](https://tc39.es/ecma262/#sec-regexp.prototype-%symbol.split%)
    ///
    /// > NOTE 1: This method returns an Array into which substrings of the
    /// > result of converting `string` to a String have been stored. The
    /// > substrings are determined by searching from left to right for matches
    /// > of the this value regular expression; these occurrences are not part
    /// > of any String in the returned array, but serve to divide up the
    /// > String value.
    /// >
    /// > The `this value` may be an empty regular expression or a regular
    /// > expression that can match an empty String. In this case, the regular
    /// > expression does not match the empty substring at the beginning or end
    /// > of the input String, nor does it match the empty substring at the end
    /// > of the previous separator match. (For example, if the regular
    /// > expression matches the empty String, the String is split up into
    /// > individual code unit elements; the length of the result array equals
    /// > the length of the String, and each substring contains one code unit.)
    /// > Only the first match at a given index of the String is considered,
    /// > even if backtracking could yield a non-empty substring match at that
    /// > index. (For example, `/a*?/[Symbol.split]("ab")` evaluates to the
    /// > array `["a", "b"]`, while `/a*/[Symbol.split]("ab")` evaluates to the
    /// > array `["","b"]`.)
    /// >
    /// > If `string` is (or converts to) the empty String, the result depends
    /// > on whether the regular expression can match the empty String. If it
    /// > can, the result array contains no elements. Otherwise, the result
    /// > array contains one element, which is the empty String.
    /// >
    /// > If the regular expression contains capturing parentheses, then each
    /// > time separator is matched the results (including any undefined
    /// > results) of the capturing parentheses are spliced into the output
    /// > array. For example,
    /// >
    /// > ```javascript
    /// > /<(\/)?([^<>]+)>/[Symbol.split]("A<B>bold</B>and<CODE>coded</CODE>")
    /// > ```
    /// >
    /// > evaluates to the array
    /// >
    /// > ```javascript
    /// > ["A", undefined, "B", "bold", "/", "B", "and", undefined, "CODE", "coded", "/", "CODE", ""]
    /// > ```
    /// >
    /// > If `limit` is not undefined, then the output array is truncated so
    /// > that it contains no more than `limit` elements.
    ///
    /// The value of the "name" property of this method is "\[Symbol.split]".
    ///
    /// > NOTE 2: This method ignores the value of the "global" and "sticky"
    /// > properties of this RegExp object.
    fn split<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let string = args.get(0).bind(gc.nogc());
        let limit = args.get(1).scope(agent, gc.nogc());
        // 1. Let rx be the this value.
        let rx = this_value.bind(gc.nogc());
        // 2. If rx is not an Object, throw a TypeError exception.
        let Ok(rx) = Object::try_from(rx) else {
            return Err(throw_not_an_object(agent, gc.into_nogc()));
        };
        let rx = rx.scope(agent, gc.nogc());
        // 3. Let S be ? ToString(string).
        let s = to_string(agent, string.unbind(), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 4. Let C be ? SpeciesConstructor(rx, %RegExp%).

        let c = species_constructor(agent, rx.get(agent), ProtoIntrinsics::RegExp, gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 5. Let flags be ? ToString(? Get(rx, "flags")).
        let flags = get(
            agent,
            rx.get(agent),
            BUILTIN_STRING_MEMORY.flags.to_property_key(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let flags = to_string(agent, flags.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let flag_bytes = flags.as_bytes_(agent);
        // 6. If flags contains "u" or flags contains "v", let unicodeMatching be true.
        // 7. Else, let unicodeMatching be false.
        let unicode_matching = flag_bytes.contains(&b'u') | flag_bytes.contains(&b'v');
        // 8. If flags contains "y", let newFlags be flags.
        let new_flags = if flag_bytes.contains(&b'y') {
            flags
        } else {
            // 9. Else, let newFlags be the string-concatenation of flags and "y".
            let mut buf = Wtf8Buf::with_capacity(flag_bytes.len() + 1);
            buf.push_wtf8(flags.as_wtf8_(agent));
            buf.push_char('y');
            String::from_wtf8_buf(agent, buf, gc.nogc())
        };
        // SAFETY: not shared.
        let c = unsafe { c.take(agent) }.bind(gc.nogc());
        // 10. Let splitter be ? Construct(C, « rx, newFlags »).
        let splitter = construct(
            agent,
            c.unbind(),
            Some(ArgumentsList::from_mut_slice(&mut [
                rx.get(agent).into(),
                new_flags.unbind().into(),
            ])),
            None,
            gc.reborrow(),
        )
        .unbind()?
        .scope(agent, gc.nogc());
        // 11. Let A be ! ArrayCreate(0).
        let a = Array::new(agent, gc.nogc()).scope(agent, gc.nogc());
        // 12. Let lengthA be 0.
        let mut length_a: u32 = 0;
        // SAFETY: not shared.
        let limit = unsafe { limit.take(agent) }.bind(gc.nogc());
        // 13. If limit is undefined,
        let lim = if limit.is_undefined() {
            // let lim be 2**32 - 1;
            u32::MAX
        } else {
            // else let lim be ℝ(? ToUint32(limit)).
            to_uint32(agent, limit.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
        };
        // 14. If lim = 0, return A.
        if lim == 0 {
            // SAFETY: not shared.
            return Ok(unsafe { a.take(agent) }.into());
        }
        // 15. If S is the empty String, then
        if s.is_empty_string() {
            // a. Let z be ? RegExpExec(splitter, S).
            let z = reg_exp_exec(agent, splitter.get(agent), s.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // b. If z is not null, return A.
            if z.is_some() {
                // SAFETY: not shared.
                return Ok(unsafe { a.take(agent) }.into());
            }
            let gc = gc.into_nogc();
            let a = unsafe { a.take(agent) }.bind(gc);
            let s = unsafe { s.take(agent) }.bind(gc);
            // c. Perform ! CreateDataPropertyOrThrow(A, "0", S).
            if let Err(err) = a.push(agent, s.into()) {
                return Err(agent.throw_allocation_exception(err, gc));
            }
            // d. Return A.
            return Ok(a.into());
        }
        // 16. Let size be the length of S.
        let size = s.get(agent).utf16_len_(agent);
        // 17. Let p be 0.
        let mut p = 0;
        // 18. Let q be p.
        let mut q = 0;
        // 19. Repeat, while q < size,
        while q < size {
            // a. Perform ? Set(splitter, "lastIndex", 𝔽(q), true).
            let f_q = Number::try_from(q).unwrap();
            set(
                agent,
                splitter.get(agent),
                BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                f_q.into(),
                true,
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // b. Let z be ? RegExpExec(splitter, S).
            let z = reg_exp_exec(agent, splitter.get(agent), s.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // c. If z is null, then
            if let Some(z) = z {
                let z = z.scope(agent, gc.nogc());
                // d. Else,
                // i. Let e be ℝ(? ToLength(? Get(splitter, "lastIndex"))).
                let e = get(
                    agent,
                    splitter.get(agent),
                    BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                let e = to_length(agent, e.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc()) as u64;
                let e = usize::try_from(e).unwrap();
                // ii. Set e to min(e, size).
                let e = e.min(size);
                // iii. If e = p, then
                if e == p {
                    // 1. Set q to AdvanceStringIndex(S, q, unicodeMatching).
                    q = advance_string_index(agent, s.get(agent), q, unicode_matching);
                } else {
                    // iv. Else,
                    let s_local = s.get(agent).bind(gc.nogc());
                    let a_local = a.get(agent).bind(gc.nogc());
                    let p_utf8 = s_local
                        .utf8_index_(agent, p)
                        .expect("p splits two surrogates into unmatched pairs");
                    let q_utf8 = s_local
                        .utf8_index_(agent, q)
                        .expect("q splits two surrogates into unmatched pairs");
                    // 1. Let T be the substring of S from p to q.
                    let t = s_local.as_wtf8_(agent).slice(p_utf8, q_utf8);
                    let mut t_buf = Wtf8Buf::with_capacity(t.len());
                    t_buf.push_wtf8(t);
                    let t = String::from_wtf8_buf(agent, t_buf, gc.nogc());
                    // 2. Perform ! CreateDataPropertyOrThrow(A, ! ToString(𝔽(lengthA)), T).
                    if let Err(err) = a_local.push(agent, t.into()) {
                        return Err(agent.throw_allocation_exception(err, gc.into_nogc()));
                    };
                    // 3. Set lengthA to lengthA + 1.
                    length_a += 1;
                    // 4. If lengthA = lim,
                    if length_a == lim {
                        // return A.
                        return Ok(a_local.unbind().into());
                    }
                    // 5. Set p to e.
                    p = e;
                    // 6. Let numberOfCaptures be ? LengthOfArrayLike(z).
                    let number_of_captures =
                        length_of_array_like(agent, z.get(agent), gc.reborrow())
                            .unbind()?
                            .bind(gc.nogc()) as u64;
                    // 7. Set numberOfCaptures to max(numberOfCaptures - 1, 0).
                    let number_of_captures = number_of_captures.saturating_sub(1);
                    // 8. Let i be 1.
                    let mut i = 1;
                    // 9. Repeat, while i ≤ numberOfCaptures,
                    while i <= number_of_captures {
                        // a. Let nextCapture be ? Get(z, ! ToString(𝔽(i))).
                        let next_capture = get(
                            agent,
                            z.get(agent),
                            PropertyKey::try_from(i).unwrap(),
                            gc.reborrow(),
                        )
                        .unbind()?
                        .bind(gc.nogc());
                        // b. Perform ! CreateDataPropertyOrThrow(A, ! ToString(𝔽(lengthA)), nextCapture).
                        if let Err(err) = a.get(agent).push(agent, next_capture) {
                            return Err(agent.throw_allocation_exception(err, gc.into_nogc()));
                        };
                        // c. Set i to i + 1.
                        i += 1;
                        // d. Set lengthA to lengthA + 1.
                        length_a += 1;
                        // e. If lengthA = lim, return A.
                        if length_a == lim {
                            // SAFETY: not shared.
                            return Ok(unsafe { a.take(agent) }.into());
                        }
                    }
                    // 10. Set q to p.
                    q = p;
                }
            } else {
                // i. Set q to AdvanceStringIndex(S, q, unicodeMatching).
                q = advance_string_index(agent, s.get(agent), q, unicode_matching);
            }
        }
        let gc = gc.into_nogc();
        let a = unsafe { a.take(agent) }.bind(gc);
        let result = if p == size {
            a.push(agent, String::EMPTY_STRING.into())
        } else {
            let s = unsafe { s.take(agent) }.bind(gc);
            let p_utf8 = s
                .utf8_index_(agent, p)
                .expect("p splits two surrogates into unmatched pairs");
            // 20. Let T be the substring of S from p to size.
            let t = s.as_wtf8_(agent).slice(p_utf8, size);
            let mut t_buf = Wtf8Buf::with_capacity(t.len());
            t_buf.push_wtf8(t);
            let t = String::from_wtf8_buf(agent, t_buf, gc);
            // 21. Perform ! CreateDataPropertyOrThrow(A, ! ToString(𝔽(lengthA)), T).
            a.push(agent, t.into())
        };
        if let Err(err) = result {
            return Err(agent.throw_allocation_exception(err, gc));
        };

        // 22. Return A.
        Ok(a.into())
    }

    /// ### [22.2.6.15 get RegExp.prototype.sticky](https://tc39.es/ecma262/#sec-get-regexp.prototype.sticky)
    ///
    /// RegExp.prototype.sticky is an accessor property whose set accessor
    /// function is undefined.
    fn get_sticky<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        // 2. Let cu be the code unit 0x0079 (LATIN SMALL LETTER Y).
        // 3. Return ? RegExpHasFlag(R, cu).
        reg_exp_has_flag(agent, r.unbind(), RegExpFlags::Y, gc.into_nogc())
            .map(|v| v.map_or(Value::Undefined, |v| v.into()))
    }

    /// ### [22.2.6.16 RegExp.prototype.test ( S )](https://tc39.es/ecma262/#sec-regexp.prototype.test)
    fn test<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let s = arguments.get(0).bind(gc.nogc());
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        if let (Ok(s), Value::RegExp(r)) = (String::try_from(s), r) {
            let key = BUILTIN_STRING_MEMORY.exec.to_property_key();
            let exec = try_get(
                agent,
                r,
                key,
                PropertyLookupCache::get(agent, key),
                gc.nogc(),
            );
            if exec
                == TryGetResult::Value(
                    agent
                        .current_realm_record()
                        .intrinsics()
                        .reg_exp_prototype_exec()
                        .into(),
                )
                .into()
            {
                return Ok(reg_exp_builtin_test(agent, r.unbind(), s.unbind(), gc)?.into());
            }
        }
        // 2. If R is not an Object, throw a TypeError exception.
        let Ok(r) = Object::try_from(r) else {
            return Err(throw_not_an_object(agent, gc.into_nogc()));
        };
        let r = r.scope(agent, gc.nogc());
        // 3. Let string be ? ToString(S).
        let string = to_string(agent, s.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let r = unsafe { r.take(agent) }.bind(gc.nogc());
        // 4. Let match be ? RegExpExec(R, string).
        let r#match = reg_exp_test(agent, r.unbind(), string.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 5. If match is not null, return true; else return false.
        Ok(r#match.into())
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
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        // 1. Let R be the this value.
        // 2. If R is not an Object, throw a TypeError exception.
        let Ok(r) = Object::try_from(this_value) else {
            let error_message = format!(
                "{} is not an object",
                this_value
                    .unbind()
                    .string_repr(agent, gc.reborrow())
                    .to_string_lossy_(agent)
            );
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                error_message,
                gc.into_nogc(),
            ));
        };
        if let Object::RegExp(r) = r {
            // Fast path for RegExp objects: This is not actually proper as it
            // does not take into account prototype mutations.
            let regexp_string = r.create_regexp_string(agent);
            return Ok(String::from_wtf8_buf(agent, regexp_string, nogc)
                .unbind()
                .into());
        }
        let scoped_r = r.scope(agent, nogc);
        // 3. Let pattern be ? ToString(? Get(R, "source")).
        let pattern = get(
            agent,
            r.unbind(),
            BUILTIN_STRING_MEMORY.source.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let pattern = to_string(agent, pattern.unbind(), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 4. Let flags be ? ToString(? Get(R, "flags")).
        let flags = get(
            agent,
            scoped_r.get(agent),
            BUILTIN_STRING_MEMORY.flags.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let flags = to_string(agent, flags.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 5. Let result be the string-concatenation of "/", pattern, "/", and flags.
        let result = format!(
            "/{}/{}",
            pattern.get(agent).bind(gc.nogc()).to_string_lossy_(agent),
            flags.to_string_lossy_(agent)
        );
        let result = String::from_string(agent, result, gc.into_nogc());
        // 6. Return result.
        Ok(result.into())
    }

    /// ### [22.2.6.18 get RegExp.prototype.unicode](https://tc39.es/ecma262/#sec-get-regexp.prototype.unicode)
    ///
    /// RegExp.prototype.unicode is an accessor property whose set accessor
    /// function is undefined.
    fn get_unicode<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        // 2. Let cu be the code unit 0x0075 (LATIN SMALL LETTER U).
        // 3. Return ? RegExpHasFlag(R, cu).
        reg_exp_has_flag(agent, r.unbind(), RegExpFlags::U, gc.into_nogc())
            .map(|v| v.map_or(Value::Undefined, |v| v.into()))
    }

    /// ### [22.2.6.19 get RegExp.prototype.unicodeSets](https://tc39.es/ecma262/#sec-get-regexp.prototype.unicodesets)
    ///
    /// RegExp.prototype.unicodeSets is an accessor property whose set accessor
    /// function is undefined.
    fn get_unicode_sets<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let R be the this value.
        let r = this_value.bind(gc.nogc());
        // 2. Let cu be the code unit 0x0076 (LATIN SMALL LETTER V).
        // 3. Return ? RegExpHasFlag(R, cu).
        reg_exp_has_flag(agent, r.unbind(), RegExpFlags::V, gc.into_nogc())
            .map(|v| v.map_or(Value::Undefined, |v| v.into()))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
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

/// 22.2.6.4.1 RegExpHasFlag ( R, codeUnit )
///
/// The abstract operation RegExpHasFlag takes arguments R (an ECMAScript
/// language value) and codeUnit (a code unit) and returns either a normal
/// completion containing either a Boolean or undefined, or a throw completion.
fn reg_exp_has_flag<'a>(
    agent: &mut Agent,
    r: Value,
    code_unit: RegExpFlags,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, Option<bool>> {
    // 1. If R is not an Object, throw a TypeError exception.
    let Ok(r) = Object::try_from(r) else {
        return Err(throw_not_an_object(agent, gc));
    };
    // 2. If R does not have an [[OriginalFlags]] internal slot, then
    let Object::RegExp(r) = r else {
        // a. If SameValue(R, %RegExp.prototype%) is true, return undefined.
        if r == agent
            .current_realm_record()
            .intrinsics()
            .reg_exp_prototype()
            .into()
        {
            return Ok(None);
        }
        // b. Otherwise, throw a TypeError exception.
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "not a RegExp object",
            gc,
        ));
    };
    // 3. Let flags be R.[[OriginalFlags]].
    let flags = r.original_flags(agent);
    // 4. If flags contains codeUnit, return true.
    // 5. Return false.
    Ok(Some((flags & code_unit).bits() > 0))
}

fn throw_not_an_object<'a>(agent: &mut Agent, gc: NoGcScope<'a, '_>) -> JsError<'a> {
    agent.throw_exception_with_static_message(ExceptionType::TypeError, "not an object", gc)
}

/// ### [22.2.6.13.1 EscapeRegExpPattern ( P, F )](https://tc39.es/ecma262/#sec-escaperegexppattern)
///
/// The abstract operation EscapeRegExpPattern takes arguments P (a String) and
/// F (a String) and returns a String.
///
/// > NOTE: Despite having similar names, RegExp.escape and EscapeRegExpPattern
/// > do not perform similar actions. The former escapes a string for
/// > representation inside a pattern, while this function escapes a pattern
/// > for representation as a string.
fn escape_reg_exp_pattern<'a>(
    agent: &mut Agent,
    p: String,
    f: RegExpFlags,
    gc: NoGcScope<'a, '_>,
) -> String<'a> {
    // 1. If F contains "v", then
    let _pattern_symbol = if f.contains(RegExpFlags::V) {
        // a. Let patternSymbol be Pattern[+UnicodeMode, +UnicodeSetsMode].
        true
    } else if f.contains(RegExpFlags::U) {
        // 2. Else if F contains "u", then
        // a. Let patternSymbol be Pattern[+UnicodeMode, ~UnicodeSetsMode].
        true
    } else {
        // 3. Else,
        // a. Let patternSymbol be Pattern[~UnicodeMode, ~UnicodeSetsMode].
        false
    };
    // 4. Let S be a String in the form of a patternSymbol equivalent to P
    //    interpreted as UTF-16 encoded Unicode code points (6.1.4), in which
    //    certain code points are escaped as described below. S may or may not
    //    differ from P; however, the Abstract Closure that would result from
    //    evaluating S as a patternSymbol must behave identically to the
    //    Abstract Closure given by the constructed object's [[RegExpMatcher]]
    //    internal slot. Multiple calls to this abstract operation using the
    //    same values for P and F must produce identical results.
    // 5. The code points / or any LineTerminator occurring in the pattern
    //    shall be escaped in S as necessary to ensure that the
    //    string-concatenation of "/", S, "/", and F can be parsed (in an
    //    appropriate lexical context) as a RegularExpressionLiteral that
    //    behaves identically to the constructed regular expression. For
    //    example, if P is "/", then S could be "\/" or "\u002F", among other
    //    possibilities, but not "/", because /// followed by F would be parsed
    //    as a SingleLineComment rather than a RegularExpressionLiteral. If P
    //    is the empty String, this specification can be met by letting S be
    //    "(?:)".

    let p_wtf8 = p.as_wtf8_(agent);
    let byte_length = p_wtf8.len();
    let mut s = Wtf8Buf::with_capacity(byte_length + (byte_length >> 4));
    for cp in p_wtf8.code_points() {
        if let Some(c) = cp.to_char() {
            match c {
                '\u{0008}' => s.push_str("\\b"),
                '\t' => s.push_str("\\t"),
                '\n' => s.push_str("\\n"),
                '\u{000C}' => s.push_str("\\f"),
                '\u{000D}' => s.push_str("\\r"),
                '/' => s.push_str("\\/"),
                _ => s.push_char(c),
            }
        } else {
            s.push(cp);
        }
    }
    String::from_wtf8_buf(agent, s, gc)
    // 6. Return S.
}
