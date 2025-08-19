// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast::RegExpFlags;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                construct, get, set, species_constructor, try_create_data_property_or_throw,
                try_get,
            },
            type_conversion::{to_boolean, to_length, to_string},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsic, array_create,
            ordinary::caches::PropertyLookupCache,
            regexp::{
                advance_string_index, reg_exp_builtin_exec, reg_exp_builtin_test, reg_exp_exec,
                reg_exp_test, require_internal_slot_reg_exp,
            },
        },
        execution::{
            Agent, JsResult, Realm,
            agent::{ExceptionType, JsError, unwrap_try},
        },
        types::{
            BUILTIN_STRING_MEMORY, IntoFunction, IntoObject, IntoValue, Number, Object,
            PropertyKey, String, TryGetResult, Value,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};

use super::regexp_string_iterator_objects::create_reg_exp_string_iterator;

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
            .map(|r| r.map_or(Value::Null, |a| a.into_value()))
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
        if !flags.to_string_lossy(agent).contains("g") {
            // a. Return ? RegExpExec(rx, S).
            reg_exp_exec(
                agent,
                // SAFETY: not shared.
                unsafe { rx.take(agent) },
                // SAFETY: not shared.
                unsafe { s.take(agent) },
                gc,
            )
            .map(|o| o.map_or(Value::Null, |o| o.into_value()))
        } else {
            // 6. Else,
            // a. If flags contains "u" or flags contains "v", let fullUnicode
            //    be true. Otherwise, let fullUnicode be false.
            let full_unicode = flags.to_string_lossy(agent).contains("u")
                || flags.to_string_lossy(agent).contains("v");
            // b. Perform ? Set(rx, "lastIndex", +0𝔽, true).
            set(
                agent,
                rx.get(agent),
                BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                0.into_value(),
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
                        return Ok(unsafe { a.take(agent) }.into_value());
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
                    match_str.into_value(),
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
                        Number::try_from(next_index).unwrap().into_value(),
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
                "R is not an object",
                gc.into_nogc(),
            ));
        };
        let scoped_r = r.scope(agent, gc.nogc());
        // 3. Let S be ? ToString(string).
        let s = to_string(agent, string.unbind(), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 4. Let C be ? SpeciesConstructor(R, %RegExp%).
        let regexp_intrinsic_constructor = agent
            .current_realm_record()
            .intrinsics()
            .reg_exp()
            .into_function()
            .bind(gc.nogc());
        let c = species_constructor(
            agent,
            scoped_r.get(agent),
            regexp_intrinsic_constructor.unbind(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let c = if c
            == agent
                .current_realm_record()
                .intrinsics()
                .reg_exp()
                .into_function()
        {
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
                .into_function()
                .bind(gc.nogc())
        };
        // 6. Let matcher be ? Construct(C, « R, flags »).
        let matcher = construct(
            agent,
            c.unbind(),
            Some(ArgumentsList::from_mut_slice(&mut [
                scoped_r.get(agent).into_value(),
                flags.into_value().unbind(),
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
        let flags = flags.as_bytes(agent);
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
        .into_value())
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

    fn replace<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("RegExp.prototype.replace", gc.into_nogc()))
    }

    fn search<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("RegExp.prototype.search", gc.into_nogc()))
    }

    fn get_source<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("RegExp.prototype.source", gc.into_nogc()))
    }

    fn split<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("RegExp.prototype.split", gc.into_nogc()))
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
                        .into_value(),
                )
                .into()
            {
                return Ok(reg_exp_builtin_test(agent, r.unbind(), s.unbind(), gc)?.into_value());
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
        Ok(r#match.into_value())
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
                    .to_string_lossy(agent)
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
                .into_value()
                .unbind());
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
            pattern.get(agent).bind(gc.nogc()).to_string_lossy(agent),
            flags.to_string_lossy(agent)
        );
        let result = String::from_string(agent, result, gc.into_nogc());
        // 6. Return result.
        Ok(result.into_value())
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
            .into_object()
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
