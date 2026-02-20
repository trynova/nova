// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::{cmp::max, str::FromStr};
use small_string::SmallString;
use std::{cmp::Ordering, ops::Deref};
use unicode_normalization::{
    IsNormalized, UnicodeNormalization, is_nfc_quick, is_nfd_quick, is_nfkc_quick, is_nfkd_quick,
};
use wtf8::{CodePoint, Wtf8Buf};

use crate::{
    ecmascript::{
        Agent, ArgumentsList, Array, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinIntrinsic,
        ExceptionType, JsResult, Number, Primitive, PrimitiveObjectData, PrimitiveObjectRecord,
        PropertyKey, Realm, String, StringIterator, Value, builders::OrdinaryObjectBuilder,
        call_function, create_array_from_list, is_callable, is_reg_exp, is_trimmable_whitespace,
        require_object_coercible, to_integer_or_infinity, to_integer_or_infinity_number, to_length,
        to_number, to_string, to_string_primitive, to_uint32, try_result_into_js,
        try_to_integer_or_infinity, try_to_length, try_to_string,
    },
    engine::{Bindable, GcScope, NoGcScope, Scopable},
    heap::{ArenaAccess, HeapIndexHandle, IntrinsicFunctionIndexes, WellKnownSymbols},
};
#[cfg(feature = "regexp")]
use crate::{
    ecmascript::{Object, get, get_object_method, invoke, reg_exp_create},
    engine::Scoped,
};

pub(crate) struct StringPrototype;

struct StringPrototypeGetAt;
impl Builtin for StringPrototypeGetAt {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.at;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::at);
}
struct StringPrototypeCharAt;
impl Builtin for StringPrototypeCharAt {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.charAt;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::char_at);
}
struct StringPrototypeCharCodeAt;
impl Builtin for StringPrototypeCharCodeAt {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.charCodeAt;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::char_code_at);
}
struct StringPrototypeCodePointAt;
impl Builtin for StringPrototypeCodePointAt {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.codePointAt;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::code_point_at);
}
struct StringPrototypeConcat;
impl Builtin for StringPrototypeConcat {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.concat;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::concat);
}
struct StringPrototypeEndsWith;
impl Builtin for StringPrototypeEndsWith {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.endsWith;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::ends_with);
}
struct StringPrototypeIncludes;
impl Builtin for StringPrototypeIncludes {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.includes;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::includes);
}
struct StringPrototypeIndexOf;
impl Builtin for StringPrototypeIndexOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.indexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::index_of);
}
struct StringPrototypeIsWellFormed;
impl Builtin for StringPrototypeIsWellFormed {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isWellFormed;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::is_well_formed);
}
struct StringPrototypeLastIndexOf;
impl Builtin for StringPrototypeLastIndexOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.lastIndexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::last_index_of);
}
struct StringPrototypeLocaleCompare;
impl Builtin for StringPrototypeLocaleCompare {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.localeCompare;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::locale_compare);
}
#[cfg(feature = "regexp")]
struct StringPrototypeMatch;
#[cfg(feature = "regexp")]
impl Builtin for StringPrototypeMatch {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.r#match;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::r#match);
}
#[cfg(feature = "regexp")]
struct StringPrototypeMatchAll;
#[cfg(feature = "regexp")]
impl Builtin for StringPrototypeMatchAll {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.matchAll;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::match_all);
}
struct StringPrototypeNormalize;
impl Builtin for StringPrototypeNormalize {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.normalize;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::normalize);
}
struct StringPrototypePadEnd;
impl Builtin for StringPrototypePadEnd {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.padEnd;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::pad_end);
}
struct StringPrototypePadStart;
impl Builtin for StringPrototypePadStart {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.padStart;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::pad_start);
}
struct StringPrototypeRepeat;
impl Builtin for StringPrototypeRepeat {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.repeat;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::repeat);
}
struct StringPrototypeReplace;
impl Builtin for StringPrototypeReplace {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.replace;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::replace);
}
struct StringPrototypeReplaceAll;
impl Builtin for StringPrototypeReplaceAll {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.replaceAll;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::replace_all);
}
#[cfg(feature = "regexp")]
struct StringPrototypeSearch;
#[cfg(feature = "regexp")]
impl Builtin for StringPrototypeSearch {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.search;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::search);
}
struct StringPrototypeSlice;
impl Builtin for StringPrototypeSlice {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::slice);
}
struct StringPrototypeSplit;
impl Builtin for StringPrototypeSplit {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.split;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::split);
}
struct StringPrototypeStartsWith;
impl Builtin for StringPrototypeStartsWith {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.startsWith;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::starts_with);
}
struct StringPrototypeSubstring;
impl Builtin for StringPrototypeSubstring {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.substring;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::substring);
}
struct StringPrototypeToLocaleLowerCase;
impl Builtin for StringPrototypeToLocaleLowerCase {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleLowerCase;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::to_locale_lower_case);
}
struct StringPrototypeToLocaleUpperCase;
impl Builtin for StringPrototypeToLocaleUpperCase {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleUpperCase;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::to_locale_upper_case);
}
struct StringPrototypeToLowerCase;
impl Builtin for StringPrototypeToLowerCase {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLowerCase;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::to_lower_case);
}
struct StringPrototypeToString;
impl Builtin for StringPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::value_of);
}
struct StringPrototypeToUpperCase;
impl Builtin for StringPrototypeToUpperCase {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toUpperCase;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::to_upper_case);
}
struct StringPrototypeToWellFormed;
impl Builtin for StringPrototypeToWellFormed {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toWellFormed;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::to_well_formed);
}
struct StringPrototypeTrim;
impl Builtin for StringPrototypeTrim {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.trim;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::trim);
}
struct StringPrototypeTrimEnd;
impl Builtin for StringPrototypeTrimEnd {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.trimEnd;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::trim_end);
}
impl BuiltinIntrinsic for StringPrototypeTrimEnd {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::StringPrototypeTrimEnd;
}
struct StringPrototypeTrimStart;
impl Builtin for StringPrototypeTrimStart {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.trimStart;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::trim_start);
}
impl BuiltinIntrinsic for StringPrototypeTrimStart {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::StringPrototypeTrimStart;
}
struct StringPrototypeValueOf;
impl Builtin for StringPrototypeValueOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.valueOf;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::value_of);
}
struct StringPrototypeIterator;
impl Builtin for StringPrototypeIterator {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_iterator_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbols::Iterator.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::iterator);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeSubstr;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeSubstr {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.substr;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::substr);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeAnchor;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeAnchor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.anchor;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::anchor);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeBig;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeBig {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.big;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::big);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeBlink;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeBlink {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.blink;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::blink);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeBold;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeBold {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.bold;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::bold);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeFixed;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeFixed {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fixed;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::fixed);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeFontcolor;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeFontcolor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fontcolor;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::fontcolor);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeFontsize;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeFontsize {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fontsize;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::fontsize);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeItalics;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeItalics {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.italics;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::italics);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeLink;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeLink {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.link;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::link);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeSmall;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeSmall {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.small;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::small);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeStrike;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeStrike {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.strike;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::strike);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeSub;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeSub {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.sub;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::sub);
}

#[cfg(feature = "annex-b-string")]
struct StringPrototypeSup;
#[cfg(feature = "annex-b-string")]
impl Builtin for StringPrototypeSup {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.sup;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::sup);
}

impl StringPrototype {
    /// ### [22.1.3.1 String.prototype.at ( index )](https://tc39.es/ecma262/#sec-string.prototype.at)
    fn at<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let index = args.get(0).bind(nogc);
        let (s, relative_index) = if let (Ok(s), Value::Integer(relative_index)) =
            (String::try_from(this_value), index)
        {
            (s, relative_index.into_i64())
        } else {
            let index = index.scope(agent, nogc);
            // 1. Let O be ? RequireObjectCoercible(this value).
            let o = require_object_coercible(agent, this_value, nogc)
                .unbind()?
                .bind(nogc);
            // 2. Let S be ? ToString(O).
            let s = to_string(agent, o.unbind(), gc.reborrow())
                .unbind()?
                .scope(agent, gc.nogc());
            // 4. Let relativeIndex be ? ToIntegerOrInfinity(pos).
            let relative_index = to_integer_or_infinity(agent, index.get(agent), gc.reborrow())
                .unbind()?
                .into_i64();
            (s.get(agent).bind(gc.nogc()), relative_index)
        };
        // 3. Let len be the length of S.
        let len = i64::try_from(s.utf16_len_(agent)).unwrap();
        // 5. If relativeIndex ≥ 0, then
        let k = if relative_index >= 0 {
            // a. Let k be relativeIndex.
            relative_index
        } else {
            // 6. Else,
            //   a. Let k be len + relativeIndex.
            len + relative_index
        };
        // 7. If k < 0 or k ≥ len, return undefined.
        if k < 0 || k >= len {
            Ok(Value::Undefined)
        } else {
            // 8. Return the substring of S from k to k + 1.
            let ch = s.char_code_at_(agent, usize::try_from(k).unwrap());
            Ok(SmallString::from_code_point(ch).into())
        }
    }

    /// ### [22.1.3.2 String.prototype.charAt ( pos )](https://tc39.es/ecma262/#sec-string.prototype.charat)
    fn char_at<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let pos = args.get(0).bind(nogc);
        let (s, position) =
            if let (Ok(s), Value::Integer(position)) = (String::try_from(this_value), pos) {
                (s, position.into_i64())
            } else {
                let pos = pos.scope(agent, nogc);
                // 1. Let O be ? RequireObjectCoercible(this value).
                let o = require_object_coercible(agent, this_value, nogc)
                    .unbind()?
                    .bind(nogc);
                // 2. Let S be ? ToString(O).
                let s = to_string(agent, o.unbind(), gc.reborrow())
                    .unbind()?
                    .scope(agent, gc.nogc());
                // 3. Let position be ? ToIntegerOrInfinity(pos).
                let position = to_integer_or_infinity(agent, pos.get(agent), gc.reborrow())
                    .unbind()?
                    .into_i64();
                (s.get(agent).bind(gc.nogc()), position)
            };
        // 4. Let size be the length of S.
        let size = s.utf16_len_(agent);
        // 5. If position < 0 or position ≥ size, return the empty String.
        if position < 0 || position >= i64::try_from(size).unwrap() {
            Ok(String::EMPTY_STRING.into())
        } else {
            // 6. Return the substring of S from position to position + 1.
            let ch = s.char_code_at_(agent, usize::try_from(position).unwrap());
            Ok(SmallString::from_code_point(ch).into())
        }
    }

    /// ### [22.1.3.3 String.prototype.charCodeAt ( pos )](https://tc39.es/ecma262/#sec-string.prototype.charcodeat)
    fn char_code_at<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let pos = args.get(0).bind(nogc);
        let (s, position) =
            if let (Ok(s), Value::Integer(position)) = (String::try_from(this_value), pos) {
                (s, position.into_i64())
            } else {
                let pos = pos.scope(agent, nogc);
                // 1. Let O be ? RequireObjectCoercible(this value).
                let o = require_object_coercible(agent, this_value, nogc)
                    .unbind()?
                    .bind(nogc);
                // 2. Let S be ? ToString(O).
                let s = to_string(agent, o.unbind(), gc.reborrow())
                    .unbind()?
                    .scope(agent, gc.nogc());
                // 3. Let position be ? ToIntegerOrInfinity(pos).
                let position = to_integer_or_infinity(agent, pos.get(agent), gc.reborrow())
                    .unbind()?
                    .into_i64();
                (s.get(agent).bind(gc.nogc()), position)
            };
        // 4. Let size be the length of S.
        let size = s.utf16_len_(agent);
        // 5. If position < 0 or position ≥ size, return NaN.
        if position < 0 || position >= i64::try_from(size).unwrap() {
            Ok(Value::nan())
        } else {
            // 6. Return the Number value for the numeric value of the code unit at index position
            // within the String S.
            let ch = s.char_code_at_(agent, usize::try_from(position).unwrap());
            Ok(Value::from(ch.to_u32()))
        }
    }

    /// ### [22.1.3.4 String.prototype.codePointAt ( pos )](https://tc39.es/ecma262/#sec-string.prototype.codepointat)
    ///
    /// > NOTE 1: This method returns a non-negative integral Number less than
    /// > or equal to 0x10FFFF𝔽 that is the numeric value of the UTF-16 encoded
    /// > code point (6.1.4) starting at the string element at index pos within
    /// > the String resulting from converting this object to a String. If
    /// > there is no element at that index, the result is undefined. If a
    /// > valid UTF-16 surrogate pair does not begin at pos, the result is the
    /// > code unit at pos.
    ///
    /// > NOTE 2: This method is intentionally generic; it does not require
    /// > that its this value be a String object. Therefore it can be
    /// > transferred to other kinds of objects for use as a method.
    fn code_point_at<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let pos = args.get(0).bind(nogc);
        let (s, position) =
            if let (Ok(s), Value::Integer(position)) = (String::try_from(this_value), pos) {
                (s, position.into_i64())
            } else {
                let pos = pos.scope(agent, nogc);
                // 1. Let O be ? RequireObjectCoercible(this value).
                let o = require_object_coercible(agent, this_value, nogc)
                    .unbind()?
                    .bind(nogc);
                // 2. Let S be ? ToString(O).
                let s = to_string(agent, o.unbind(), gc.reborrow())
                    .unbind()?
                    .scope(agent, gc.nogc());
                // 3. Let position be ? ToIntegerOrInfinity(pos).
                let position = to_integer_or_infinity(agent, pos.get(agent), gc.reborrow())
                    .unbind()?
                    .into_i64();
                (s.get(agent).bind(gc.nogc()), position)
            };
        // 4. Let size be the length of S.
        let size = s.utf16_len_(agent);
        // 5. If position < 0 or position ≥ size, return undefined.
        if position < 0 || position >= i64::try_from(size).unwrap() {
            Ok(Value::Undefined)
        } else {
            // 6. Let cp be CodePointAt(S, position).
            let cp = s.code_point_at_(agent, position as usize);
            // 7. Return 𝔽(cp.[[CodePoint]]).
            Ok(Value::from(cp.to_u32()))
        }
    }

    /// ### [22.1.3.5 String.prototype.concat ( ...args )](https://tc39.es/ecma262/#sec-string.prototype.concat)
    ///
    /// > NOTE 1: When this method is called it returns the String value
    /// > consisting of the code units of the this value (converted to a
    /// > String) followed by the code units of each of the arguments converted
    /// > to a String. The result is a String value, not a String object.
    ///
    /// > NOTE 2: This method is intentionally generic; it does not require
    /// > that its this value be a String object. Therefore it can be
    /// > transferred to other kinds of objects for use as a method.
    fn concat<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let (s, args) = if let Ok(s) = String::try_from(this_value) {
            (s, &args[..])
        } else {
            // 1. Let O be ? RequireObjectCoercible(this value).
            let o = require_object_coercible(agent, this_value, nogc)
                .unbind()?
                .bind(nogc);
            if let Some(s) = try_result_into_js(try_to_string(agent, o, nogc))
                .unbind()?
                .bind(gc.nogc())
            {
                (s, &args[..])
            } else {
                let scoped_args = args
                    .iter()
                    .map(|arg| arg.scope(agent, nogc))
                    .collect::<Vec<_>>();
                // 2. Let S be ? ToString(O).
                let s = to_string(agent, o.unbind(), gc.reborrow())
                    .unbind()?
                    .scope(agent, gc.nogc());
                let mut scoped_string_args = scoped_args
                    .into_iter()
                    .map(|arg| {
                        let stack_arg = arg.get(agent);
                        let stack_arg = to_string(agent, stack_arg, gc.reborrow())
                            .unbind()?
                            .bind(gc.nogc());
                        // SAFETY: args are never shared
                        Ok(unsafe { arg.replace_self(agent, stack_arg.unbind()) })
                    })
                    .collect::<JsResult<Vec<_>>>()?;
                scoped_string_args.insert(0, s);
                let nogc = gc.into_nogc();
                let string_args = scoped_string_args
                    .into_iter()
                    .map(|arg| arg.get(agent).bind(nogc))
                    .collect::<Vec<_>>();
                return Ok(String::concat(agent, &string_args, nogc).into());
            }
        };
        // 3. Let R be S.
        let (strings, nogc) = if args.iter().all(|arg| arg.is_primitive()) {
            let mut strings = Vec::with_capacity(args.len() + 1);
            let s = s.unbind();
            let nogc = gc.into_nogc();
            strings.push(s.bind(nogc));
            for next in args.iter() {
                strings.push(
                    to_string_primitive(agent, Primitive::try_from(*next).unwrap(), nogc)
                        .unbind()?
                        .bind(nogc),
                );
            }
            (strings, nogc)
        } else {
            let mut string_roots = Vec::with_capacity(args.len() + 1);
            string_roots.push(s.scope(agent, gc.nogc()));
            for next in args.iter() {
                string_roots.push(
                    to_string(agent, *next, gc.reborrow())
                        .unbind()?
                        .scope(agent, gc.nogc()),
                );
            }
            let nogc = gc.into_nogc();
            let strings = string_roots
                .into_iter()
                .map(|string_root| string_root.get(agent).bind(nogc))
                .collect::<Vec<_>>();
            (strings, nogc)
        };
        // 4. For each element next of args, do
        //     a. Let nextString be ? ToString(next).
        //     b. Set R to the string-concatenation of R and nextString.
        // 5. Return R.
        Ok(String::concat(agent, &strings, nogc).into())
    }

    /// ### [22.1.3.7 String.prototype.endsWith ( searchString \[ , endPosition \] )](https://tc39.es/ecma262/#sec-string.prototype.endswith)
    ///
    /// > NOTE 1: This method returns true if the sequence of code units of
    /// > searchString converted to a String is the same as the corresponding
    /// > code units of this object (converted to a String) starting at
    /// > `endPosition - length(this)`. Otherwise it returns false.
    ///
    /// > NOTE 2: Throwing an exception if the first argument is a RegExp is
    /// > specified in order to allow future editions to define extensions that
    /// > allow such argument values.
    ///
    /// > NOTE 3: This method is intentionally generic; it does not require
    /// > that its this value be a String object. Therefore it can be
    /// > transferred to other kinds of objects for use as a method.
    fn ends_with<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let search_string = args.get(0).bind(nogc);
        let end_position = args.get(1).bind(nogc);

        let (s, search_str, pos) = if let (Ok(s), Ok(search_str), Value::Undefined) = (
            String::try_from(this_value),
            String::try_from(search_string),
            end_position,
        ) {
            (s, search_str, usize::MAX)
        } else if let (Ok(s), Ok(search_str), Value::Integer(position)) = (
            String::try_from(this_value),
            String::try_from(search_string),
            end_position,
        ) {
            (s, search_str, position.into_i64().max(0) as usize)
        } else {
            let search_string = search_string.scope(agent, nogc);
            let end_position = end_position.scope(agent, nogc);
            // 1. Let O be ? RequireObjectCoercible(this value).
            let o = require_object_coercible(agent, this_value, gc.nogc())
                .unbind()?
                .bind(gc.nogc());
            // 2. Let S be ? ToString(O).
            let s = to_string(agent, o.unbind(), gc.reborrow())
                .unbind()?
                .scope(agent, gc.nogc());

            // 3. Let isRegExp be ? IsRegExp(searchString).
            // 4. If isRegExp is true, throw a TypeError exception.
            if is_reg_exp(agent, search_string.get(agent), gc.reborrow()).unbind()? {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "searchString is RegExp",
                    gc.into_nogc(),
                ));
            }

            // 5. Let searchStr be ? ToString(searchString).
            let search_str = to_string(agent, search_string.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: search_string is not shared.
            let search_str = unsafe { search_string.replace_self(agent, search_str.unbind()) };

            // 6. Let len be the length of S.
            // 7. If endPosition is undefined, let pos be len;
            let end_position = end_position.get(agent);
            let pos = if end_position.is_undefined() {
                usize::MAX
            } else {
                // else let pos be ? ToIntegerOrInfinity(endPosition).
                to_integer_or_infinity(agent, end_position, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc())
                    .into_i64()
                    .max(0) as usize
            };

            let nogc = gc.nogc();
            (
                s.get(agent).bind(nogc),
                search_str.get(agent).bind(nogc),
                pos,
            )
        };

        // 6. Let len be the length of S.
        // 8. Let end be the result of clamping pos between 0 and len.
        let haystack_str = if pos == usize::MAX {
            s.as_bytes_(agent)
        } else {
            let end = if pos != 0 {
                // NOTE: `pos` was already clamped to 0.
                pos.min(s.utf16_len_(agent))
            } else {
                0
            };
            let u8_idx = s.utf8_index_(agent, end).unwrap();
            &s.as_bytes_(agent)[..u8_idx]
        };

        // 9. Let searchLength be the length of searchStr.
        // 10. If searchLength = 0, return true.
        // 11. Let start be end - searchLength.
        // 12. If start < 0, return false.
        // 13. Let substring be the substring of S from start to end.
        // 14. If substring is searchStr, return true.
        // 15. Return false.
        Ok(Value::from(
            haystack_str.ends_with(search_str.as_bytes_(agent)),
        ))
    }

    /// ### [22.1.3.8 String.prototype.includes ( searchString \[ , position \] )](https://tc39.es/ecma262/#sec-string.prototype.includes)
    ///
    /// > NOTE 1: If searchString appears as a substring of the result of
    /// > converting this object to a String, at one or more indices that are
    /// > greater than or equal to position, this function returns true;
    /// > otherwise, it returns false. If position is undefined, 0 is assumed,
    /// > so as to search all of the String.
    ///
    /// > NOTE 2: Throwing an exception if the first argument is a RegExp is
    /// > specified in order to allow future editions to define extensions that
    /// > allow such argument values.
    ///
    /// > NOTE 3: This method is intentionally generic; it does not require
    /// > that its this value be a String object. Therefore it can be
    /// > transferred to other kinds of objects for use as a method.
    fn includes<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let search_string = args.get(0).bind(nogc);
        let position = args.get(1).bind(nogc);

        let (s, search_str, pos) = if let (Ok(s), Ok(search_str), Value::Undefined) = (
            String::try_from(this_value),
            String::try_from(search_string),
            position,
        ) {
            (s, search_str, 0usize)
        } else if let (Ok(s), Ok(search_str), Value::Integer(position)) = (
            String::try_from(this_value),
            String::try_from(search_string),
            position,
        ) {
            (s, search_str, position.into_i64().max(0) as usize)
        } else {
            let position = position.scope(agent, nogc);
            let search_string = search_string.scope(agent, nogc);
            // 1. Let O be ? RequireObjectCoercible(this value).
            let o = require_object_coercible(agent, this_value, gc.nogc())
                .unbind()?
                .bind(gc.nogc());
            // 2. Let S be ? ToString(O).
            let s = to_string(agent, o.unbind(), gc.reborrow())
                .unbind()?
                .scope(agent, gc.nogc());

            // 3. Let isRegExp be ? IsRegExp(searchString).
            // 4. If isRegExp is true, throw a TypeError exception.
            if is_reg_exp(agent, search_string.get(agent), gc.reborrow()).unbind()? {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "searchString is RegExp",
                    gc.into_nogc(),
                ));
            }

            // 5. Let searchStr be ? ToString(searchString).
            let search_str = to_string(agent, search_string.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());

            // SAFETY: search_string is not shared.
            let search_str = unsafe { search_string.replace_self(agent, search_str.unbind()) };
            // 6. Let pos be ? ToIntegerOrInfinity(position).
            let position = position.get(agent);
            let position_is_undefined = position.is_undefined();
            let pos = to_integer_or_infinity(agent, position, gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
                .into_i64()
                .max(0) as usize;
            // 7. Assert: If position is undefined, then pos is 0.
            assert!(!position_is_undefined || pos == 0);

            let nogc = gc.nogc();
            (
                s.get(agent).bind(nogc),
                search_str.get(agent).bind(nogc),
                pos,
            )
        };

        let search_str = search_str.bind(gc.nogc());

        // 8. Let len be the length of S.
        // 9. Let start be the result of clamping pos between 0 and len.
        let haystack_str = {
            let start = if pos != 0 {
                // NOTE: `pos` was already clamped to 0.
                pos.min(s.utf16_len_(agent))
            } else {
                0
            };
            &s.to_string_lossy_(agent)[start..]
        };

        // 10. Let index be StringIndexOf(S, searchStr, start).
        // 11. If index is not-found, return false.
        // 12. Return true.
        Ok(Value::from(
            haystack_str.contains(search_str.to_string_lossy_(agent).deref()),
        ))
    }

    /// ### [22.1.3.9 String.prototype.indexOf ( searchString \[ , position \] )](https://tc39.es/ecma262/#sec-string.prototype.indexof)
    ///
    /// > NOTE 1: If searchString appears as a substring of the result of
    /// > converting this object to a String, at one or more indices that are
    /// > greater than or equal to position, then the smallest such index is
    /// > returned; otherwise, -1𝔽 is returned. If position is undefined, +0𝔽
    /// > is assumed, so as to search all of the String.
    ///
    /// > NOTE 2: This method is intentionally generic; it does not require
    /// > that its this value be a String object. Therefore it can be
    /// > transferred to other kinds of objects for use as a method.
    fn index_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let search_string = args.get(0).bind(nogc);
        let position = args.get(1).bind(nogc);

        let (s, search_str, pos) = if let (Ok(s), Ok(search_str), Value::Undefined) = (
            String::try_from(this_value),
            String::try_from(search_string),
            position,
        ) {
            (s, search_str, 0usize)
        } else if let (Ok(s), Ok(search_str), Value::Integer(position)) = (
            String::try_from(this_value),
            String::try_from(search_string),
            position,
        ) {
            (s, search_str, position.into_i64().max(0) as usize)
        } else {
            let search_string = search_string.scope(agent, nogc);
            let position = position.scope(agent, nogc);
            // 1. Let O be ? RequireObjectCoercible(this value).
            let o = require_object_coercible(agent, this_value, nogc)
                .unbind()?
                .bind(nogc);
            // 2. Let S be ? ToString(O).
            let s = to_string(agent, o.unbind(), gc.reborrow())
                .unbind()?
                .scope(agent, gc.nogc());
            // 3. Let searchStr be ? ToString(searchString).
            let search_str = to_string(agent, search_string.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: search_str is not shared.
            let search_str = unsafe { search_string.replace_self(agent, search_str.unbind()) };
            // 4. Let pos be ? ToIntegerOrInfinity(position).
            // 5. Assert: If position is undefined, then pos is 0.
            let pos = to_integer_or_infinity(agent, position.get(agent), gc.reborrow())
                .unbind()?
                .into_i64()
                .max(0) as usize;

            let nogc = gc.nogc();
            (
                s.get(agent).bind(nogc),
                search_str.get(agent).bind(nogc),
                pos,
            )
        };

        // 6. Let len be the length of S.
        // 7. Let start be the result of clamping pos between 0 and len.
        let utf8_start = if pos != 0 {
            let u16_len = s.utf16_len_(agent);
            if pos >= u16_len {
                s.len_(agent)
            } else {
                s.utf8_index_(agent, pos).unwrap()
            }
        } else {
            0
        };

        // 8. Let result be StringIndexOf(S, searchStr, start).
        // 9. If result is not-found, return -1𝔽.
        // 10. Return 𝔽(result).
        if let Some(rel_u8_pos) =
            s.to_string_lossy_(agent)[utf8_start..].find(search_str.to_string_lossy_(agent).deref())
        {
            let u8_pos = utf8_start + rel_u8_pos;
            let result = s.utf16_index_(agent, u8_pos);
            Ok(Number::try_from(result).unwrap().into())
        } else {
            Ok(Number::from(-1).into())
        }
    }

    /// ### [22.1.3.10 String.prototype.isWellFormed ( )](https://tc39.es/ecma262/#sec-string.prototype.iswellformed)
    fn is_well_formed<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o.unbind(), gc)?;

        // 3. Return IsStringWellFormedUnicode(S).
        Ok(Value::Boolean(s.as_str_(agent).is_some()))
    }

    /// ### [22.1.3.11 String.prototype.lastIndexOf ( searchString \[ , position \] )](https://tc39.es/ecma262/#sec-string.prototype.lastindexof)
    ///
    /// > NOTE 1: If _searchString_ appears as a substring of the result of
    /// > converting this object to a String at one or more indices that are
    /// > smaller than or equal to _position_, then the greatest such index is
    /// > returned; otherwise, **`-1𝔽`** is returned. If position is
    /// > **undefined**, the length of the String value is assumed, so as to
    /// > search all of the String.
    ///
    /// > NOTE 2: This method is intentionally generic; it does not require
    /// > that its this value be a String object. Therefore it can be
    /// > transferred to other kinds of objects for use as a method.
    fn last_index_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let search_string = args.get(0).bind(nogc);
        let position = args.get(1).bind(nogc);

        let (s, search_str, pos) = if let (Ok(s), Ok(search_str), Value::Undefined) = (
            String::try_from(this_value),
            String::try_from(search_string),
            position,
        ) {
            (s, search_str, usize::MAX)
        } else if let (Ok(s), Ok(search_str), Value::Integer(position)) = (
            String::try_from(this_value),
            String::try_from(search_string),
            position,
        ) {
            (s, search_str, position.into_i64().max(0) as usize)
        } else {
            let search_string = search_string.scope(agent, nogc);
            let position = position.scope(agent, nogc);
            // 1. Let O be ? RequireObjectCoercible(this value).
            let o = require_object_coercible(agent, this_value, nogc)
                .unbind()?
                .bind(nogc);
            // 2. Let S be ? ToString(O).
            let s = to_string(agent, o.unbind(), gc.reborrow())
                .unbind()?
                .scope(agent, gc.nogc());
            // 3. Let searchStr be ? ToString(searchString).
            let mut search_str = to_string(agent, search_string.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());

            let position = position.get(agent).bind(gc.nogc());
            let pos = if position.is_undefined() {
                // 5. Assert: If position is undefined, then numPos is NaN.
                // 6. If numPos is NaN, let pos be +∞;
                usize::MAX
            } else if let Value::Integer(position) = position {
                position.into_i64().max(0) as usize
            } else {
                // 4. Let numPos be ? ToNumber(position).
                let scoped_search_str = search_str.scope(agent, gc.nogc());
                let num_pos = to_number(agent, position.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                search_str = scoped_search_str.get(agent).bind(gc.nogc());
                if num_pos.is_nan_(agent) {
                    // 6. If numPos is NaN, let pos be +∞;
                    usize::MAX
                } else {
                    // otherwise, let pos be! ToIntegerOrInfinity(numPos).
                    to_integer_or_infinity_number(agent, num_pos)
                        .into_i64()
                        .max(0) as usize
                }
            };

            (s.get(agent).bind(gc.nogc()), search_str, pos)
        };

        // 7. Let len be the length of S.
        // 8. Let searchLen be the length of searchStr.
        // 9. Let start be the result of clamping pos between 0 and len - searchLen.
        // 10. Let result be StringLastIndexOf(S, searchStr, start).
        // NOTE: If start >= (len - searchLen), there is a last index if and
        // only if start.endsWith(searchLen).
        let haystack_str = {
            if pos == usize::MAX {
                s.to_string_lossy_(agent)
            } else {
                // When starting from a position, the position may mark the
                // start of the search string, so we need to include the search
                // string length in the haystack.
                let utf8_pos = s.utf8_index_(agent, pos).unwrap();
                let utf8_len = s.len_(agent);
                let search_str_len = search_str.len_(agent);
                s.as_wtf8_(agent)
                    .slice_to(utf8_len.min(utf8_pos + search_str_len))
                    .to_string_lossy()
            }
        };
        let search_str = search_str.to_string_lossy_(agent);
        let utf8_result = haystack_str.rfind(search_str.deref());

        // 11. If result is not-found, return -1𝔽.
        // 12. Return 𝔽(result).
        if let Some(utf8_idx) = utf8_result {
            let result = s.utf16_index_(agent, utf8_idx);
            Ok(Number::try_from(result).unwrap().into())
        } else {
            Ok(Number::from(-1).into())
        }
    }

    /// ### [22.1.3.12 String.prototype.localeCompare ( that \[ , reserved1 \[ , reserved2 \] \] )](https://tc39.es/ecma262/#sec-string.prototype.localecompare)
    ///
    /// An ECMAScript implementation that includes the ECMA-402
    /// Internationalization API must implement this method as specified in the
    /// ECMA-402 specification. If an ECMAScript implementation does not
    /// include the ECMA-402 API the following specification of this method is
    /// used:
    ///
    /// This method returns a Number other than NaN representing the result of
    /// an implementation-defined locale-sensitive String comparison of the
    /// this value (converted to a String S) with that (converted to a String
    /// thatValue). The result is intended to correspond with a sort order of
    /// String values according to conventions of the host environment's
    /// current locale, and will be negative when S is ordered before
    /// thatValue, positive when S is ordered after thatValue, and zero in all
    /// other cases (representing no relative ordering between S and
    /// thatValue).
    ///
    /// Before performing the comparisons, this method performs the following
    /// steps to prepare the Strings:
    ///
    /// ```text
    /// 1. Let O be ? RequireObjectCoercible(this value).
    /// 2. Let S be ? ToString(O).
    /// 3. Let thatValue be ? ToString(that).
    /// ```
    ///
    /// The meaning of the optional second and third parameters to this method
    /// are defined in the ECMA-402 specification; implementations that do not
    /// include ECMA-402 support must not assign any other interpretation to
    /// those parameter positions.
    ///
    /// The actual return values are implementation-defined to permit encoding
    /// additional information in them, but this method, when considered as a
    /// method of two arguments, is required to be a consistent comparator
    /// defining a total ordering on the set of all Strings. This method is
    /// also required to recognize and honour canonical equivalence according
    /// to the Unicode Standard, including returning +0𝔽 when comparing
    /// distinguishable Strings that are canonically equivalent.
    ///
    /// > NOTE 1: This method itself is not directly suitable as an argument to
    /// > `Array.prototype.sort` because the latter requires a function of two
    /// > arguments.
    ///
    /// > NOTE 2: This method may rely on whatever language- and/or
    /// > locale-sensitive comparison functionality is available to the
    /// > ECMAScript environment from the host environment, and is intended to
    /// > compare according to the conventions of the host environment's
    /// > current locale. However, regardless of comparison capabilities, this
    /// > method must recognize and honour canonical equivalence according to
    /// > the Unicode Standard—for example, the following comparisons must all
    /// > return +0𝔽:
    /// >
    /// > ```javascript
    /// > // Å ANGSTROM SIGN vs.
    /// > // Å LATIN CAPITAL LETTER A + COMBINING RING ABOVE
    /// > "\u212B".localeCompare("A\u030A")
    /// >
    /// > // Ω OHM SIGN vs.
    /// > // Ω GREEK CAPITAL LETTER OMEGA
    /// > "\u2126".localeCompare("\u03A9")
    /// >
    /// > // ṩ LATIN SMALL LETTER S WITH DOT BELOW AND DOT ABOVE vs.
    /// > // ṩ LATIN SMALL LETTER S + COMBINING DOT ABOVE + COMBINING DOT BELOW
    /// > "\u1E69".localeCompare("s\u0307\u0323")
    /// >
    /// > // ḍ̇ LATIN SMALL LETTER D WITH DOT ABOVE + COMBINING DOT BELOW vs.
    /// > // ḍ̇ LATIN SMALL LETTER D WITH DOT BELOW + COMBINING DOT ABOVE
    /// > "\u1E0B\u0323".localeCompare("\u1E0D\u0307")
    /// >
    /// > // 가 HANGUL CHOSEONG KIYEOK + HANGUL JUNGSEONG A vs.
    /// > // 가 HANGUL SYLLABLE GA
    /// > "\u1100\u1161".localeCompare("\uAC00")
    /// > ```
    /// >
    /// > For a definition and discussion of canonical equivalence see the
    /// > Unicode Standard, chapters 2 and 3, as well as Unicode Standard Annex
    /// > #15, Unicode Normalization Forms and Unicode Technical Note #5,
    /// > Canonical Equivalence in Applications. Also see Unicode Technical
    /// > Standard #10, Unicode Collation Algorithm.
    /// >
    /// > It is recommended that this method should not honour Unicode
    /// > compatibility equivalents or compatibility decompositions as defined
    /// > in the Unicode Standard, chapter 3, section 3.7.
    ///
    /// > NOTE 3: This method is intentionally generic; it does not require
    /// > that its this value be a String object. Therefore, it can be
    /// > transferred to other kinds of objects for use as a method.
    fn locale_compare<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let that = args.get(0).bind(gc.nogc());
        let (s, that_value) = if let (Ok(s), Ok(that_value)) =
            (String::try_from(this_value), String::try_from(that))
        {
            (s, that_value)
        } else {
            let scoped_that = that.scope(agent, gc.nogc());
            // 1. Let O be ? RequireObjectCoercible(this value).
            let o = require_object_coercible(agent, this_value, gc.nogc())
                .unbind()?
                .bind(gc.nogc());
            // 2. Let S be ? ToString(O).
            let s = to_string(agent, o.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            let that = scoped_that.get(agent).bind(gc.nogc());
            // SAFETY: not shared.
            let s = unsafe { scoped_that.replace_self(agent, s.unbind()) };
            // 3. Let thatValue be ? ToString(that).
            let that_value = to_string(agent, that.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            let s = unsafe { s.take(agent) }.bind(gc.nogc());
            (s, that_value)
        };
        match s.as_wtf8_(agent).cmp(that_value.as_wtf8_(agent)) {
            Ordering::Less => Ok((-1).into()),
            Ordering::Equal => Ok(0.into()),
            Ordering::Greater => Ok(1.into()),
        }
    }

    /// ### [22.1.3.13 String.prototype.match ( regexp )](https://tc39.es/ecma262/#sec-string.prototype.match)
    ///
    /// > NOTE: This method is intentionally generic; it does not require that
    /// > its this value be a String object. Therefore, it can be transferred
    /// > to other kinds of objects for use as a method.
    #[cfg(feature = "regexp")]
    fn r#match<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let regexp = arguments.get(0).bind(gc.nogc());
        let scoped_regexp = regexp.scope(agent, gc.nogc());

        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let o = o.scope(agent, gc.nogc());
        // See: https://github.com/tc39/ecma262/pull/3009
        // 2. If regexp is an Object, then
        if let Ok(regexp) = Object::try_from(regexp) {
            // a. Let matcher be ? GetMethod(regexp, %Symbol.match%).
            let matcher = get_object_method(
                agent,
                regexp.unbind(),
                WellKnownSymbols::Match.to_property_key(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // b. If matcher is not undefined, then
            if let Some(matcher) = matcher {
                // i. Return ? Call(matcher, regexp, « O »).
                return call_function(
                    agent,
                    matcher.unbind(),
                    // SAFETY: not shared.
                    unsafe { scoped_regexp.take(agent) },
                    Some(ArgumentsList::from_mut_value(
                        // SAFETY: not shared.
                        &mut unsafe { o.take(agent) },
                    )),
                    gc,
                );
            }
        }
        // 3. Let S be ? ToString(O).
        // SAFETY: o is not shared.
        let s = to_string(agent, unsafe { o.take(agent) }, gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 4. Let rx be ? RegExpCreate(regexp, undefined).
        let rx = reg_exp_create(agent, scoped_regexp, None, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 5. Return ? Invoke(rx, %Symbol.match%, « S »).
        invoke(
            agent,
            rx.unbind().into(),
            WellKnownSymbols::Match.to_property_key(),
            Some(ArgumentsList::from_mut_value(&mut unsafe {
                s.take(agent).into()
            })),
            gc,
        )
    }

    /// ### [22.1.3.14 String.prototype.matchAll ( regexp )](https://tc39.es/ecma262/#sec-string.prototype.matchall)
    ///
    /// This method performs a regular expression match of the String
    /// representing the this value against regexp and returns an iterator that
    /// yields match results. Each match result is an Array containing the
    /// matched portion of the String as the first element, followed by the
    /// portions matched by any capturing groups. If the regular expression
    /// never matches, the returned iterator does not yield any match results.
    ///
    /// > NOTE 1: This method is intentionally generic, it does not require
    /// > that its this value be a String object. Therefore, it can be
    /// > transferred to other kinds of objects for use as a method.
    ///
    /// > NOTE 2: Similarly to `String.prototype.split`,
    /// > `String.prototype.matchAll` is designed to typically act without
    /// > mutating its inputs.
    #[cfg(feature = "regexp")]
    fn match_all<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let regexp = args.get(0).bind(gc.nogc());
        let scoped_regexp = regexp.scope(agent, gc.nogc());
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // See: https://github.com/tc39/ecma262/pull/3009
        // 2. If regexp is an Object, then
        if let Ok(mut regexp) = Object::try_from(regexp) {
            // a. Let isRegExp be ? IsRegExp(regexp).
            let is_reg_exp = is_reg_exp(agent, regexp.unbind().into(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());

            // SAFETY: regexp is an Object.
            regexp = unsafe {
                Object::try_from(scoped_regexp.get(agent).bind(gc.nogc())).unwrap_unchecked()
            };

            // b. If isRegExp is true, then
            if is_reg_exp {
                // i. Let flags be ? Get(regexp, "flags").
                let flags = get(
                    agent,
                    regexp.unbind(),
                    BUILTIN_STRING_MEMORY.flags.to_property_key(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // ii. Perform ? RequireObjectCoercible(flags).
                let flags = require_object_coercible(agent, flags, gc.nogc())
                    .unbind()?
                    .bind(gc.nogc());
                // iii. If ? ToString(flags) does not contain "g",
                let flags = to_string(agent, flags.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                if !flags.as_bytes_(agent).contains(&b'g') {
                    // throw a TypeError exception.
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "replaceAll must be called with a global RegExp",
                        gc.into_nogc(),
                    ));
                }
                // SAFETY: regexp is an Object.
                regexp = unsafe {
                    Object::try_from(scoped_regexp.get(agent).bind(gc.nogc())).unwrap_unchecked()
                };
            }
            // c. Let matcher be ? GetMethod(regexp, %Symbol.matchAll%).
            let matcher = get_object_method(
                agent,
                regexp.unbind(),
                WellKnownSymbols::MatchAll.to_property_key(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // d. If matcher is not undefined, then
            if let Some(matcher) = matcher {
                // i. Return ? Call(matcher, regexp, « O »).
                return call_function(
                    agent,
                    matcher.unbind(),
                    // SAFETY: not shared.
                    unsafe { scoped_regexp.take(agent) },
                    Some(ArgumentsList::from_mut_value(
                        // SAFETY: not shared.
                        &mut unsafe { o.take(agent) },
                    )),
                    gc,
                );
            }
        }
        // 3. Let S be ? ToString(O).
        // SAFETY: not shared.
        let s = to_string(agent, unsafe { o.take(agent) }, gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 4. Let rx be ? RegExpCreate(regexp, "g").
        let rx = reg_exp_create(
            agent,
            scoped_regexp,
            Some(String::from_small_string("g")),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let s = unsafe { s.take(agent) }.bind(gc.nogc());
        // 5. Return ? Invoke(rx, %Symbol.matchAll%, « S »).
        invoke(
            agent,
            rx.unbind().into(),
            WellKnownSymbols::MatchAll.to_property_key(),
            Some(ArgumentsList::from_mut_value(&mut s.unbind().into())),
            gc,
        )
    }

    /// ### [22.1.3.15 String.prototype.normalize ( \[ form \] )](https://tc39.es/ecma262/#sec-string.prototype.normalize)
    ///
    /// > NOTE: This method is intentionally generic, it does not require that
    /// > its this value be a String object. Therefore, it can be transferred
    /// > to other kinds of objects for use as a method.
    fn normalize<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let mut form = arguments.get(0).bind(nogc);

        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, nogc)
            .unbind()?
            .bind(nogc);

        // 2. Let S be ? ToString(O).
        let mut s = if let Some(s) = try_result_into_js(try_to_string(agent, o, nogc))
            .unbind()?
            .bind(gc.nogc())
        {
            s
        } else {
            let scoped_form = form.scope(agent, nogc);
            let s = to_string(agent, o.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            form = scoped_form.get(agent).bind(gc.nogc());
            s
        };

        // 3. If form is undefined, let f be "NFC".
        let f = if form.is_undefined() {
            NormalizeForm::Nfc
        } else {
            // 4. Else, let f be ? ToString(form).
            let f = if let Some(f) = try_result_into_js(try_to_string(agent, form, gc.nogc()))
                .unbind()?
                .bind(gc.nogc())
            {
                f
            } else {
                let scoped_s = s.scope(agent, gc.nogc());
                let f = to_string(agent, form.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                s = scoped_s.get(agent).bind(gc.nogc());
                f
            };
            let form_result = f.as_str_(agent).ok_or(()).and_then(NormalizeForm::from_str);
            match form_result {
                Ok(form) => form,
                // 5. If f is not one of "NFC", "NFD", "NFKC", or "NFKD", throw a RangeError exception.
                Err(()) => {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "The normalization form should be one of NFC, NFD, NFKC, NFKD.",
                        gc.into_nogc(),
                    ));
                }
            }
        };

        // 6. Let ns be the String value that is the result of normalizing S
        //    into the normalization form named by f as specified in the latest
        //    Unicode Standard, Normalization Forms.
        match unicode_normalize(&s.to_string_lossy_(agent), f) {
            // 7. Return ns.
            None => Ok(s.unbind().into()),
            Some(ns) => Ok(Value::from_string(agent, ns, gc.into_nogc())),
        }
    }

    /// ### [22.1.3.16 String.prototype.padEnd ( maxLength \[ , fillString \] )](https://tc39.es/ecma262/#sec-string.prototype.padend)
    fn pad_end<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let max_length = arguments.get(0).bind(nogc);
        let fill_string = arguments.get(1).bind(nogc);

        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, nogc)
            .unbind()?
            .bind(nogc);

        // 2. Return ? StringPaddingBuiltinsImpl(O, maxLength, fillString, end).
        string_padding_builtins_impl(
            agent,
            o.unbind(),
            max_length.unbind(),
            fill_string.unbind(),
            false,
            gc,
        )
    }

    /// ### [22.1.3.17 String.prototype.padStart ( maxLength \[ , fillString \] )](https://tc39.es/ecma262/#sec-string.prototype.padstart)
    fn pad_start<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let max_length = arguments.get(0).bind(nogc);
        let fill_string = arguments.get(1).bind(nogc);

        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, nogc)
            .unbind()?
            .bind(nogc);

        // 2. Return ? StringPaddingBuiltinsImpl(O, maxLength, fillString, start).
        string_padding_builtins_impl(
            agent,
            o.unbind(),
            max_length.unbind(),
            fill_string.unbind(),
            true,
            gc,
        )
    }

    /// ### [22.1.3.18 String.prototype.repeat ( count )](https://tc39.es/ecma262/#sec-string.prototype.repeat)
    ///
    /// > NOTE 1: This method creates the String value consisting of the code
    /// > units of the this value (converted to String) repeated count times.
    ///
    /// > NOTE 2: This method is intentionally generic; it does not require
    /// > that its this value be a String object. Therefore, it can be
    /// > transferred to other kinds of objects for use as a method.
    fn repeat<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let count = arguments.get(0).scope(agent, nogc);

        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, nogc)
            .unbind()?
            .bind(nogc);

        // 2. Let S be ? ToString(O).
        let mut s = to_string(agent, o.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // 3. Let n be ? ToIntegerOrInfinity(count).
        // SAFETY: count is not shared.
        let count = unsafe { count.take(agent) };
        let n = if let Some(n) =
            try_result_into_js(try_to_integer_or_infinity(agent, count, gc.nogc())).unbind()?
        {
            n
        } else {
            let scoped_s = s.scope(agent, gc.nogc());
            let result = to_integer_or_infinity(agent, count, gc.reborrow()).unbind()?;
            s = scoped_s.get(agent).bind(gc.nogc());
            result
        };

        // 4. If n < 0 or n = +∞, throw a RangeError exception.
        if n.is_pos_infinity() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "count must be less than infinity",
                gc.into_nogc(),
            ));
        }

        let n = n.into_i64();

        if n < 0 {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "count must not be negative",
                gc.into_nogc(),
            ));
        }

        // 5. If n = 0, return the empty String.
        if n == 0 || s.is_empty_string() {
            return Ok(String::EMPTY_STRING.into());
        }

        if n == 1 {
            return Ok(s.unbind().into());
        }

        // 6. Return the String value that is made from n copies of S appended together.
        Ok(Value::from_string(
            agent,
            s.to_string_lossy_(agent).repeat(n as usize),
            gc.into_nogc(),
        ))
    }

    /// ### [22.1.3.19 String.prototype.replace ( searchValue, replaceValue )](https://tc39.es/ecma262/#sec-string.prototype.replace)
    ///
    /// > NOTE: This method is intentionally generic; it does not require that
    /// > its this value be a String object. Therefore, it can be transferred
    /// > to other kinds of objects for use as a method.
    fn replace<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let search_value = args.get(0).bind(nogc);
        let replace_value = args.get(1).scope(agent, nogc);

        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);

        let scoped_search_value = search_value.scope(agent, nogc);
        // See: https://github.com/tc39/ecma262/pull/3009
        // 2. If searchValue is an Object, then
        #[cfg(feature = "regexp")]
        if let Ok(search_value) = Object::try_from(search_value) {
            // a. Let replacer be ? GetMethod(searchValue, %Symbol.replace%).
            let symbol = WellKnownSymbols::Replace.into();
            let replacer = get_object_method(agent, search_value.unbind(), symbol, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());

            // b. If replacer is not undefined, Return ? Call(replacer, searchValue, « O, replaceValue »).
            if let Some(replacer) = replacer {
                return call_function(
                    agent,
                    replacer.unbind(),
                    scoped_search_value.get(agent),
                    Some(ArgumentsList::from_mut_slice(&mut [
                        o.get(agent),
                        replace_value.get(agent),
                    ])),
                    gc,
                );
            }
        }

        // 3. Let s be ? ToString(O).
        let s = to_string(agent, o.get(agent), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());

        // 4. Let searchString be ? ToString(searchValue).
        let search_string = to_string(agent, scoped_search_value.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // 5. Let functionalReplace be IsCallable(replaceValue).
        if let Some(functional_replace) = is_callable(replace_value.get(agent), gc.nogc()) {
            // 7. Let searchLength be the length of searchString.
            let search_length = search_string.len_(agent);

            // 8. Let position be StringIndexOf(s, searchString, 0).
            let position = if let Some(position) = s
                .to_string_lossy(agent)
                .find(search_string.to_string_lossy_(agent).deref())
            {
                position
            } else {
                // 9. If position is not-found, return s.
                return Ok(s.get(agent).into());
            };

            // Let replacement be ? ToString(? Call(replaceValue, undefined, « searchString, 𝔽(position), string »)).
            let result = call_function(
                agent,
                functional_replace.unbind(),
                Value::Undefined,
                Some(ArgumentsList::from_mut_slice(&mut [
                    search_string.unbind().into(),
                    Number::from(position as u32).into(),
                    s.get(agent).unbind().into(),
                ])),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());

            let result = to_string(agent, result.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());

            // 10. Let preceding be the substring of s from 0 to position.
            // 11. Let following be the substring of s from position + searchLength.
            // 12. If functionalReplace is true,
            let preceding = &s.to_string_lossy(agent)[0..position];
            let following = &s.to_string_lossy(agent)[position + search_length..];

            // 14. Return the string-concatenation of preceding, replacement, and following.
            let concatenated_result = format!(
                "{}{}{}",
                preceding,
                result.to_string_lossy_(agent),
                following
            );
            return Ok(String::from_string(agent, concatenated_result, gc.into_nogc()).into());
        }

        let search_string_root = search_string.scope(agent, gc.nogc());

        // 6. If functionalReplace is false, Set replaceValue to ? ToString(replaceValue).
        let replace_string = to_string(agent, replace_value.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // Everything are strings: `"foo".replace("o", "a")` => use rust's replace
        let result = s.to_string_lossy(agent).into_owned().replacen(
            search_string_root.to_string_lossy(agent).deref(),
            &replace_string.to_string_lossy_(agent),
            1,
        );
        Ok(String::from_string(agent, result, gc.into_nogc()).into())
    }

    /// ### [22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue )](https://tc39.es/ecma262/#sec-string.prototype.replaceall)
    fn replace_all<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let search_value = args.get(0).bind(nogc);
        let replace_value = args.get(1).scope(agent, nogc);
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);

        let scoped_search_value = search_value.scope(agent, nogc);

        // 2. If searchValue is an Object, then
        #[cfg(feature = "regexp")]
        if let Ok(mut search_value) = Object::try_from(search_value) {
            // a. Let isRegExp be ? IsRegExp(searchValue).
            let is_reg_exp = is_reg_exp(agent, search_value.unbind().into(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());

            // SAFETY: searchValue is an Object.
            search_value = unsafe {
                Object::try_from(scoped_search_value.get(agent).bind(gc.nogc())).unwrap_unchecked()
            };

            // b. If isRegExp is true, then
            if is_reg_exp {
                // i. Let flags be ? Get(searchValue, "flags").
                let flags = get(
                    agent,
                    search_value.unbind(),
                    BUILTIN_STRING_MEMORY.flags.to_property_key(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // ii. Perform ? RequireObjectCoercible(flags).
                let flags = require_object_coercible(agent, flags, gc.nogc())
                    .unbind()?
                    .bind(gc.nogc());
                // iii. If ? ToString(flags) does not contain "g",
                let flags = to_string(agent, flags.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                if !flags.as_bytes_(agent).contains(&b'g') {
                    // throw a TypeError exception.
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "replaceAll must be called with a global RegExp",
                        gc.into_nogc(),
                    ));
                }
                // SAFETY: searchValue is an Object.
                search_value = unsafe {
                    Object::try_from(scoped_search_value.get(agent).bind(gc.nogc()))
                        .unwrap_unchecked()
                };
            }

            // c. Let replacer be ? GetMethod(searchValue, %Symbol.replace%).
            let symbol = WellKnownSymbols::Replace.into();
            let replacer = get_object_method(agent, search_value.unbind(), symbol, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());

            // d. If replacer is not undefined, Return ? Call(replacer, searchValue, « O, replaceValue »).
            if let Some(replacer) = replacer {
                return call_function(
                    agent,
                    replacer.unbind(),
                    scoped_search_value.get(agent),
                    Some(ArgumentsList::from_mut_slice(&mut [
                        o.get(agent),
                        replace_value.get(agent),
                    ])),
                    gc,
                );
            }
        }

        // 3. Let s be ? ToString(O).
        let s = to_string(agent, o.get(agent), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());

        // 4. Let searchString be ? ToString(searchValue).
        let mut search_string = to_string(agent, scoped_search_value.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let search_string_root = search_string.scope(agent, gc.nogc());

        // 5. Let functionalReplace be IsCallable(replaceValue).
        if let Some(functional_replace) = is_callable(replace_value.get(agent), gc.nogc()) {
            // 7. Let searchLength be the length of searchString.
            let search_length = search_string.len_(agent);

            // 8. Let advanceBy be max(1, searchLength).
            let advance_by = max(1, search_length);

            // 9. Let matchPositions be a new empty List.
            let mut match_positions: Vec<usize> = vec![];

            // 10. Let position be StringIndexOf(s, searchString, 0).
            let search_str = search_string.to_string_lossy_(agent);
            let subject = s.to_string_lossy(agent).into_owned();
            let mut position = 0;

            // 11. Repeat, while position is not not-found,
            while let Some(pos) = subject
                .split_at_checked(position)
                .and_then(|(_, str)| str.find(search_str.deref()))
            {
                // a. Append position to matchPositions.
                match_positions.push(position + pos);
                // b. Set position to StringIndexOf(string, searchString,
                //    position + advanceBy).
                position += advance_by + pos;
            }

            // If none has found, return s.
            if match_positions.is_empty() {
                return Ok(s.get(agent).into());
            }

            // 12. Let endOfLastMatch be 0.
            let mut end_of_last_match = 0;

            // 13. Let result be the empty String.
            let mut result = std::string::String::with_capacity(subject.len());

            // 14. For each element p of matchPositions, do
            let functional_replace = functional_replace.scope(agent, gc.nogc());
            for p in match_positions {
                // b. let replacement be ? ToString(? Call(replaceValue, undefined, « searchString, 𝔽(p), string »)).
                let replacement = call_function(
                    agent,
                    functional_replace.get(agent),
                    Value::Undefined,
                    Some(ArgumentsList::from_mut_slice(&mut [
                        search_string_root.get(agent).into(),
                        Number::from(p as u32).into(),
                        s.get(agent).into(),
                    ])),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                let replacement = to_string(agent, replacement.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());

                // a. Let preserved be the substring of string from endOfLastMatch to p.
                let preserved = &subject[end_of_last_match..p];
                // d. Set result to the string-concatenation of result, preserved, and replacement.
                let replacement_str = replacement.to_string_lossy_(agent);
                result.reserve(preserved.len() + replacement_str.len());
                result.push_str(preserved);
                result.push_str(&replacement_str);
                end_of_last_match = p + search_length;
            }

            // 15. If endOfLastMatch < the length of string, set result to the string-concatenation of result and the substring of string from endOfLastMatch.
            if end_of_last_match < subject.len() {
                let preserved = &subject[end_of_last_match..];
                result.push_str(preserved);
            }

            // 16. Return result.
            return Ok(String::from_string(agent, result, gc.into_nogc()).into());
        }

        // 6. If functionalReplace is false, Set replaceValue to ? ToString(replaceValue).
        let replace_string = to_string(agent, replace_value.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // Everything are strings: `"foo".replaceAll("o", "a")` => use rust's replace
        search_string = search_string_root.get(agent).bind(gc.nogc());
        let s = s.get(agent).bind(gc.nogc());
        let result = s.to_string_lossy_(agent).into_owned().replace(
            search_string.to_string_lossy_(agent).deref(),
            &replace_string.to_string_lossy_(agent),
        );
        Ok(String::from_string(agent, result, gc.into_nogc()).into())
    }

    /// ### [22.1.3.21 String.prototype.search ( regexp )](https://tc39.es/ecma262/#sec-string.prototype.search)
    ///
    /// > NOTE: This method is intentionally generic; it does not require that
    /// > its this value be a String object. Therefore, it can be transferred
    /// > to other kinds of objects for use as a method.
    #[cfg(feature = "regexp")]
    fn search<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let regexp = args.get(0).bind(gc.nogc());
        // 1. Let O be ? RequireObjectCoercible(this value).
        let mut o = require_object_coercible(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let scoped_regexp = regexp.scope(agent, gc.nogc());
        // See: https://github.com/tc39/ecma262/pull/3009
        // 2. If regexp is an Object, then
        if let Ok(regexp) = Object::try_from(regexp) {
            let scoped_o = o.scope(agent, gc.nogc());
            // a. Let searcher be ? GetMethod(regexp, %Symbol.search%).
            let searcher = get_object_method(
                agent,
                regexp.unbind(),
                WellKnownSymbols::Search.to_property_key(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            o = unsafe { scoped_o.take(agent) }.bind(gc.nogc());
            // b. If searcher is not undefined, then
            if let Some(searcher) = searcher {
                let regexp = unsafe { scoped_regexp.take(agent) }.bind(gc.nogc());
                // i. Return ? Call(searcher, regexp, « O »).
                return call_function(
                    agent,
                    searcher.unbind(),
                    regexp.unbind(),
                    Some(ArgumentsList::from_mut_value(&mut o.unbind())),
                    gc,
                );
            }
        }
        // 3. Let string be ? ToString(O).
        let string = to_string(agent, o.unbind(), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());
        // 4. Let rx be ? RegExpCreate(regexp, undefined).
        let rx = reg_exp_create(agent, scoped_regexp, None, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let string = unsafe { string.take(agent) }.bind(gc.nogc());
        // 5. Return ? Invoke(rx, %Symbol.search%, « string »).
        invoke(
            agent,
            rx.unbind().into(),
            WellKnownSymbols::Search.to_property_key(),
            Some(ArgumentsList::from_mut_value(&mut string.unbind().into())),
            gc,
        )
    }

    /// ### [22.1.3.22 String.prototype.slice ( start, end )](https://tc39.es/ecma262/#sec-string.prototype.slice)
    ///
    /// > NOTE: This method is intentionally generic; it does not require that
    /// > its this value be a String object. Therefore, it can be transferred
    /// > to other kinds of objects for use as a method.
    fn slice<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let start = args.get(0).scope(agent, nogc);
        let end = args.get(1).scope(agent, nogc);
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, nogc)
            .unbind()?
            .bind(nogc);
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o.unbind(), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc());

        // 3. Let len be the length of S.
        // 4. Let intStart be ? ToIntegerOrInfinity(start).
        let int_start = to_integer_or_infinity(agent, start.get(agent), gc.reborrow()).unbind()?;
        // 5. If intStart = -∞, let from be 0.
        // NOTE: We use `None` when `from` would be `len` in the spec.
        let from = if int_start.is_neg_infinity() {
            Some(0)
        } else if int_start.is_negative() {
            // 6. Else if intStart < 0, let from be max(len + intStart, 0).
            let len = i64::try_from(s.get(agent).utf16_len_(agent)).unwrap();
            let int_start = int_start.into_i64();
            Some((len + int_start).max(0) as usize)
        } else {
            // 7. Else, let from be min(intStart, len).
            let len = s.get(agent).utf16_len_(agent);
            let int_start = int_start.into_i64() as usize;
            if int_start >= len {
                None
            } else {
                Some(int_start)
            }
        };

        // 8. If end is undefined, let intEnd be len; else let intEnd be ? ToIntegerOrInfinity(end).
        // NOTE: We use `None` when `to` would be `len` in the spec.
        let end = end.get(agent).bind(gc.nogc());
        let to = if end.is_undefined() {
            None
        } else {
            let int_end = to_integer_or_infinity(agent, end.unbind(), gc.reborrow()).unbind()?;
            // 9. If intEnd = -∞, let to be 0.
            if int_end.is_neg_infinity() {
                Some(0)
            } else if int_end.is_negative() {
                // 10. Else if intEnd < 0, let to be max(len + intEnd, 0).
                let len = i64::try_from(s.get(agent).utf16_len_(agent)).unwrap();
                let int_end = int_end.into_i64();
                Some((len + int_end).max(0) as usize)
            } else {
                // 11. Else, let to be min(intEnd, len).
                let len = s.get(agent).utf16_len_(agent);
                let int_end = int_end.into_i64() as usize;
                if int_end >= len { None } else { Some(int_end) }
            }
        };

        let gc = gc.into_nogc();
        let s = s.get(agent).bind(gc);

        // 12. If from ≥ to, return the empty String.
        // 13. Return the substring of S from from to to.
        let substring = match (from, to) {
            (None, _) => "",
            (Some(0), None) => return Ok(s.into()),
            (Some(from_idx), None) => {
                let u8_from = s.utf8_index_(agent, from_idx).unwrap();
                &s.to_string_lossy_(agent)[u8_from..]
            }
            (Some(from_idx), Some(to_idx)) if from_idx >= to_idx => "",
            (Some(from_idx), Some(to_idx)) => {
                let u8_from = s.utf8_index_(agent, from_idx).unwrap();
                let u8_to = s.utf8_index_(agent, to_idx).unwrap();
                &s.to_string_lossy_(agent)[u8_from..u8_to]
            }
        };
        // SAFETY: The memory for `substring` (and for the WTF-8 representation
        // of `s`) won't be moved or deallocated before this function returns.
        let substring: &'static str = unsafe { core::mem::transmute(substring) };
        Ok(String::from_str(agent, substring, gc).into())
    }

    /// ### [22.1.3.23 String.prototype.split ( separator, limit )](https://tc39.es/ecma262/#sec-string.prototype.split)
    ///
    /// > NOTE 1: The value of separator may be an empty String. In this case,
    /// > separator does not match the empty substring at the beginning or end
    /// > of the input String, nor does it match the empty substring at the end
    /// > of the previous separator match. If separator is the empty String,
    /// > the String is split up into individual code unit elements; the length
    /// > of the result array equals the length of the String, and each
    /// > substring contains one code unit.
    /// >
    /// > If the this value is (or converts to) the empty String, the result
    /// > depends on whether separator can match the empty String. If it can,
    /// > the result array contains no elements. Otherwise, the result array
    /// > contains one element, which is the empty String.
    /// >
    /// > If separator is undefined, then the result array contains just one
    /// > String, which is the this value (converted to a String). If limit is
    /// > not undefined, then the output array is truncated so that it contains
    /// > no more than limit elements.
    ///
    /// > NOTE 2: This method is intentionally generic; it does not require
    /// > that its this value be a String object. Therefore, it can be
    /// > transferred to other kinds of objects for use as a method.
    fn split<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let separator = args.get(0).bind(nogc);
        let scoped_separator = separator.scope(agent, nogc);
        let limit = args.get(1).scope(agent, nogc);
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);

        // 2. If separator is an object, then
        #[cfg(feature = "regexp")]
        if let Ok(separator) = Object::try_from(separator) {
            let symbol = WellKnownSymbols::Split.into();
            // a. Let splitter be ? GetMethod(separator, %Symbol.split%).
            let splitter = get_object_method(agent, separator.unbind(), symbol, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());

            // b. If splitter is not undefined, then
            if let Some(splitter) = splitter {
                // i. Return ? Call(splitter, separator, « O, limit »).
                return call_function(
                    agent,
                    splitter.unbind(),
                    scoped_separator.get(agent),
                    Some(ArgumentsList::from_mut_slice(&mut [
                        o.get(agent),
                        limit.get(agent),
                    ])),
                    gc,
                );
            }
        }

        // 3. Let S be ? ToString(O).
        let s = to_string(agent, o.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let scoped_s = s.scope(agent, gc.nogc());

        // SAFETY: not shared.
        let limit = unsafe { limit.take(agent).bind(gc.nogc()) };
        let lim = match limit {
            // 4. If limit is undefined, lim is 2**32 - 1.
            Value::Undefined => u32::MAX,
            // else let lim be ℝ(? ToUint32(limit)).
            // Note: Fast path for integer parameter.
            Value::Integer(value) => value.into_i64() as u32,
            _ => to_uint32(agent, limit.unbind(), gc.reborrow()).unbind()?,
        };

        // 5. Let R be ? ToString(separator).
        let r = to_string(agent, scoped_separator.get(agent), gc.reborrow()).unbind()?;
        let gc = gc.into_nogc();
        // SAFETY: not shared.
        let separator = unsafe { scoped_separator.take(agent) }.bind(gc);
        let s = scoped_s.get(agent).bind(gc);
        let r = r.bind(gc);

        // 6. If lim is zero, return an empty array
        if lim == 0 {
            return Ok(create_array_from_list(agent, &[], gc).into());
        }

        // 7. If separator is undefined, return an array with the whole string
        if separator.is_undefined() {
            return Ok(create_array_from_list(agent, &[s.into()], gc).into());
        }

        // 8. Let separatorLength be the length of R.
        let separator_length = r.len_(agent);

        // 9. If separatorLength = 0, then split into characters
        if separator_length == 0 {
            let subject = s.to_string_lossy_(agent);
            let head = subject.split("");

            let mut results: Vec<Value> = head
                .enumerate()
                .skip(1) // Rust's split inserts an empty string in the beginning.
                .take_while(|(i, _)| *i <= lim as usize)
                .map(|(_, part)| SmallString::try_from(part).unwrap().into())
                .collect();

            // Remove the latest empty string if it's needed
            if results.len() < lim as usize {
                results.pop();
            }

            let results = Array::from_slice(agent, results.as_slice(), gc);
            return Ok(results.into());
        }

        // 10. If S is the empty String, return CreateArrayFromList(« S »).
        let s = scoped_s.get(agent).bind(gc);
        if s.is_empty_string() {
            let list: [Value; 1] = [s.unbind().into()];
            return Ok(create_array_from_list(agent, &list, gc).into());
        }

        // 11-17. Normal split
        let subject = s.to_string_lossy_(agent);
        let separator = r.to_string_lossy_(agent);
        let head = subject
            .split(separator.deref())
            .take(lim as usize)
            .map(|part| part.to_owned())
            .collect::<Vec<_>>();
        let mut results: Vec<Value> = Vec::new();

        for part in head {
            results.push(Value::from_string(agent, part, gc));
        }

        let results = Array::from_slice(agent, results.as_slice(), gc);
        Ok(results.into())
    }

    /// ### [22.1.3.24 String.prototype.startsWith ( searchString \[ , position \] )](https://tc39.es/ecma262/#sec-string.prototype.startswith)
    ///
    /// > NOTE 1: This method returns **true** if the sequence of code units of
    /// > _searchString_ converted to a String is the same as the corresponding
    /// > code units of this object (converted to a String) starting at index
    /// > position. Otherwise it returns **false**.
    ///
    /// > NOTE 2: Throwing an exception if the first argument is a RegExp is
    /// > specified in order to allow future editions to define extensions that
    /// > allow such argument values.
    ///
    /// > NOTE 3: This method is intentionally generic; it does not require
    /// > that its **this** value be a String object. Therefore, it can be
    /// > transferred to other kinds of objects for use as a method.
    fn starts_with<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let search_string = args.get(0).bind(nogc);
        let position = args.get(1).bind(nogc);

        let position_option = if position.is_undefined() {
            Some(0)
        } else if let Value::Integer(position) = position {
            Some(position.into_i64().max(0) as usize)
        } else {
            None
        };

        let (s, search_str, start) = if let (Ok(s), Ok(search_string), Some(position)) = (
            String::try_from(this_value),
            String::try_from(search_string),
            position_option,
        ) {
            (s, search_string, position)
        } else {
            let search_string = search_string.scope(agent, nogc);
            let position = position.scope(agent, nogc);

            // 1. Let O be ? RequireObjectCoercible(this value).
            let o = require_object_coercible(agent, this_value, nogc)
                .unbind()?
                .bind(nogc);
            // 2. Let S be ? ToString(O).
            let s = to_string(agent, o.unbind(), gc.reborrow())
                .unbind()?
                .scope(agent, gc.nogc());

            // 3. Let isRegExp be ? IsRegExp(searchString).
            // 4. If isRegExp is true, throw a TypeError exception.
            if is_reg_exp(agent, search_string.get(agent), gc.reborrow()).unbind()? {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "searchString is RegExp",
                    gc.into_nogc(),
                ));
            }

            // 5. Let searchStr be ? ToString(searchString).
            let search_str = to_string(agent, search_string.get(agent), gc.reborrow())
                .unbind()?
                .scope(agent, gc.nogc());

            // 6. Let len be the length of S.
            // 7. If position is undefined, let pos be 0; else let pos be
            //    ? ToIntegerOrInfinity(endPosition).
            // 8. Let start be the result of clamping pos between 0 and len.
            let position = position.get(agent);
            let start = if position.is_undefined() {
                0
            } else {
                to_integer_or_infinity(agent, position, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc())
                    .into_i64()
                    .max(0) as usize
            };

            (
                s.get(agent).bind(gc.nogc()),
                search_str.get(agent).bind(gc.nogc()),
                start,
            )
        };

        // 9. Let searchLength be the length of searchStr.
        // 10. If searchLength = 0, return true.
        if search_str.len_(agent) == 0 {
            return Ok(true.into());
        }
        // 11. Let end be start + searchLength.
        // 12. If end > len, return false.
        // 13. Let substring be the substring of S from start to end.
        // 14. If substring is searchStr, return true.
        // 15. Return false.
        let haystack_str = if start == 0 {
            s.to_string_lossy_(agent)
        } else {
            let len = s.utf16_len_(agent);
            if start >= len {
                "".into()
            } else {
                let start = s.utf8_index_(agent, start).unwrap();
                s.as_wtf8_(agent).slice_from(start).to_string_lossy()
            }
        };
        Ok(haystack_str
            .starts_with(search_str.to_string_lossy_(agent).deref())
            .into())
    }

    /// ### [22.1.3.25 String.prototype.substring ( start, end )](https://tc39.es/ecma262/#sec-string.prototype.substring)
    ///
    /// > NOTE: This method is intentionally generic; it does not require that
    /// > its this value be a String object. Therefore, it can be transferred
    /// > to other kinds of objects for use as a method.
    fn substring<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let start = args.get(0).scope(agent, nogc);
        let end = args.get(1).scope(agent, nogc);
        let mut s = if let Ok(s) = String::try_from(this_value) {
            s.bind(nogc)
        } else {
            // 1. Let O be ? RequireObjectCoercible(this value).
            let o = require_object_coercible(agent, this_value, nogc)
                .unbind()?
                .bind(nogc);
            // 2. Let S be ? ToString(O).
            to_string(agent, o.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
        };

        let mut scoped_s = None;

        // 3. Let len be the length of S.
        // 4. Let intStart be ? ToIntegerOrInfinity(start).
        // SAFETY: Never shared.
        let start = unsafe { start.take(agent) }.bind(gc.nogc());
        let int_start = if let Some(int_start) =
            try_result_into_js(try_to_integer_or_infinity(agent, start, gc.nogc())).unbind()?
        {
            int_start
        } else {
            let local_scoped_s = s.scope(agent, gc.nogc());
            let int_start =
                to_integer_or_infinity(agent, start.unbind(), gc.reborrow()).unbind()?;
            s = local_scoped_s.get(agent).bind(gc.nogc());
            scoped_s = Some(local_scoped_s);
            int_start
        };
        // 5. If end is undefined, let intEnd be len; else let intEnd be ? ToIntegerOrInfinity(end).
        let end = end.get(agent).bind(gc.nogc());
        let int_end = if end.is_undefined() {
            None
        } else if let Some(int_end) =
            try_result_into_js(try_to_integer_or_infinity(agent, end, gc.nogc())).unbind()?
        {
            Some(int_end)
        } else {
            let local_scoped_s = scoped_s.unwrap_or_else(|| s.scope(agent, gc.nogc()));
            let int_end = to_integer_or_infinity(agent, end.unbind(), gc.reborrow()).unbind()?;
            s = local_scoped_s.get(agent).bind(gc.nogc());
            Some(int_end)
        };

        // Fast path: can we return `s` without computing the UTF-16 length?
        // We can if int_start <= 0 and we know int_end must be >= len
        // (i.e. it's either None or is greater than the UTF-8 length).
        if int_start.into_i64() <= 0
            && int_end
                .map(|int_end| int_end.into_i64() >= s.len_(agent) as i64)
                .unwrap_or(true)
        {
            return Ok(s.unbind().into());
        }

        let len = s.utf16_len_(agent);
        // 6. Let finalStart be the result of clamping intStart between 0 and len.
        let final_start = (int_start.into_i64().max(0) as usize).min(len);
        // 7. Let finalEnd be the result of clamping intEnd between 0 and len.
        let final_end = if let Some(int_end) = int_end {
            (int_end.into_i64().max(0) as usize).min(len)
        } else {
            len
        };

        // 8. Let from be min(finalStart, finalEnd)
        let from = final_start.min(final_end);
        // 9. Let to be max(finalStart, finalEnd).
        let to = final_start.max(final_end);

        // 10. Return the substring of S from from to to.
        let u8_from = if from != len {
            s.utf8_index_(agent, from).unwrap()
        } else {
            s.len_(agent)
        };
        let u8_to = if to != len {
            s.utf8_index_(agent, to).unwrap()
        } else {
            s.len_(agent)
        };
        let substring = &s.to_string_lossy_(agent)[u8_from..u8_to];
        // SAFETY: The memory for `substring` (and for the WTF-8 representation
        // of `s`) won't be moved or deallocated before this function returns.
        let substring: &'static str = unsafe { core::mem::transmute(substring) };
        Ok(String::from_str(agent, substring, gc.into_nogc()).into())
    }

    /// ### [22.1.3.26 String.prototype.toLocaleLowerCase ( \[ reserved1 \[ , reserved2 \] \] )](https://tc39.es/ecma262/#sec-string.prototype.tolocalelowercase)
    fn to_locale_lower_case<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o: Value = require_object_coercible(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // 3. Let sText be [StringToCodePoints](https://tc39.es/ecma262/#sec-stringtocodepoints)(S).
        // 4. Let lowerText be toLowercase(sText), according to the Unicode Default Case Conversion algorithm.
        // 5. Let L be [CodePointsToString](https://tc39.es/ecma262/#sec-codepointstostring)(lowerText).
        // 6. Return L.
        let lower_case_string: std::string::String = s.to_string_lossy_(agent).to_lowercase();
        Ok(String::from_string(agent, lower_case_string, gc.into_nogc()).into())
    }

    /// ### [22.1.3.27 String.prototype.toLocaleUpperCase ( \[ reserved1 \[ , reserved2 \] \] )](https://tc39.es/ecma262/#sec-string.prototype.tolocaleuppercase)
    fn to_locale_upper_case<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // 3. Let sText be [StringToCodePoints](https://tc39.es/ecma262/#sec-stringtocodepoints)(S).
        // 4. Let upperText be toUppercase(sText), according to the Unicode Default Case Conversion algorithm.
        // 5. Let L be [CodePointsToString](https://tc39.es/ecma262/#sec-codepointstostring)(upperText).
        // 6. Return L.
        let upper_case_string = s.to_string_lossy_(agent).to_uppercase();
        Ok(String::from_string(agent, upper_case_string, gc.into_nogc()).into())
    }

    /// ### [22.1.3.28 String.prototype.toLowerCase ( )](https://tc39.es/ecma262/#sec-string.prototype.tolowercase)
    ///
    /// The implementation might not reflect the specification.
    ///
    /// > NOTE: This method is intentionally generic; it does not require that
    /// > its this value be a String object. Therefore, it can be transferred
    /// > to other kinds of objects for use as a method.
    fn to_lower_case<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // 3. Let sText be [StringToCodePoints](https://tc39.es/ecma262/#sec-stringtocodepoints)(S).
        // 4. Let lowerText be toLowercase(sText), according to the Unicode Default Case Conversion algorithm.
        // 5. Let L be [CodePointsToString](https://tc39.es/ecma262/#sec-codepointstostring)(lowerText).
        // 6. Return L.
        let lower_case_string = s.to_string_lossy_(agent).to_lowercase();
        Ok(String::from_string(agent, lower_case_string, gc.into_nogc()).into())
    }

    /// ### [22.1.3.30 String.prototype.toUpperCase ( )](https://tc39.es/ecma262/#sec-string.prototype.touppercase)
    ///
    /// The implementation might not reflect the specification.
    ///
    /// > NOTE: This method is intentionally generic; it does not require that
    /// > its this value be a String object. Therefore, it can be transferred
    /// > to other kinds of objects for use as a method.
    fn to_upper_case<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // 3. Let sText be [StringToCodePoints](https://tc39.es/ecma262/#sec-stringtocodepoints)(S).
        // 4. Let upperText be toUppercase(sText), according to the Unicode Default Case Conversion algorithm.
        // 5. Let L be [CodePointsToString](https://tc39.es/ecma262/#sec-codepointstostring)(upperText).
        // 6. Return L.
        let upper_case_string = s.to_string_lossy_(agent).to_uppercase();
        Ok(String::from_string(agent, upper_case_string, gc.into_nogc()).into())
    }

    /// ### [22.1.3.31 String.prototype.toWellFormed ( )](https://tc39.es/ecma262/#sec-string.prototype.towellformed)
    ///
    /// This method returns a String representation of this object with all
    /// leading surrogates and trailing surrogates that are not part of a
    /// surrogate pair replaced with U+FFFD (REPLACEMENT CHARACTER).
    fn to_well_formed<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // 3. Let strLen be the length of S.
        // 4. Let k be 0.
        // 5. Let result be the empty String.
        // 6. Repeat, while k < strLen,
        //     a. Let cp be CodePointAt(S, k).
        //     b. If cp.[[IsUnpairedSurrogate]] is true, then
        //         i. Set result to the string-concatenation of result and 0xFFFD (REPLACEMENT CHARACTER).
        //     c. Else,
        //         i. Set result to the string-concatenation of result and UTF16EncodeCodePoint(cp.[[CodePoint]]).
        //     d. Set k to k + cp.[[CodeUnitCount]].
        // 7. Return result.

        match s.to_string_lossy_(agent) {
            // String is already well-formed UTF-8.
            std::borrow::Cow::Borrowed(_) => Ok(s.unbind().into()),
            std::borrow::Cow::Owned(string) => {
                // String was ill-formed UTF-8 and a well-formed copy was created.
                Ok(String::from_string(agent, string, gc.into_nogc()).into())
            }
        }
    }

    /// ### [22.1.3.32 String.prototype.trim ( )](https://tc39.es/ecma262/#sec-string.prototype.trim)
    ///
    /// This method interprets a String value as a sequence of UTF-16 encoded
    /// code points, as described in 6.1.4.
    ///
    /// > NOTE: This method is intentionally generic; it does not require that
    /// > its this value be a String object. Therefore, it can be transferred
    /// > to other kinds of objects for use as a method.
    fn trim<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? TrimString(S, start+end).
        Self::trim_string(agent, this_value, TrimWhere::StartAndEnd, gc)
    }

    /// ### [22.1.3.32.1 TrimString ( string, where )](https://tc39.es/ecma262/#sec-trimstring)
    fn trim_string<'gc>(
        agent: &mut Agent,
        value: Value,
        trim_where: TrimWhere,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let str be ? RequireObjectCoercible(string).
        let str = require_object_coercible(agent, value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());

        // 2. Let S be ? ToString(str)
        let s = to_string(agent, str.unbind(), gc.reborrow()).unbind()?;
        let gc = gc.into_nogc();
        let s = s.bind(gc);

        let s_str = s.to_string_lossy_(agent);

        let t = match trim_where {
            // 3. If where is start, then
            //   a. Let T be the String value that is a copy of S with leading white space removed.
            TrimWhere::Start => s_str.trim_start_matches(is_trimmable_whitespace),
            // 4. Else if where is end, then
            //   a. Let T be the String value that is a copy of S with trailing white space removed.
            TrimWhere::End => s_str.trim_end_matches(is_trimmable_whitespace),
            // 5. Else,
            //   a. Assert: where is start+end.
            //   b. Let T be the String value that is a copy of S with both leading and trailing white space removed.
            TrimWhere::StartAndEnd => s_str.trim_matches(is_trimmable_whitespace),
        };

        if t == s_str {
            // No need to allocate a String if the string was not trimmed
            Ok(s.into())
        } else {
            let t = String::from_string(agent, t.to_string(), gc);
            Ok(t.into())
        }
    }

    /// ### [22.1.3.33 String.prototype.trimEnd ( )](https://tc39.es/ecma262/#sec-string.prototype.trimend)
    ///
    /// This method interprets a String value as a sequence of UTF-16 encoded
    /// code points, as described in 6.1.4.
    ///
    /// > NOTE: This method is intentionally generic; it does not require that
    /// > its this value be a String object. Therefore, it can be transferred
    /// > to other kinds of objects for use as a method.
    fn trim_end<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? TrimString(S, end).
        Self::trim_string(agent, this_value, TrimWhere::End, gc)
    }

    /// ### [22.1.3.34 String.prototype.trimStart ( )](https://tc39.es/ecma262/#sec-string.prototype.trimstart)
    ///
    /// This method interprets a String value as a sequence of UTF-16 encoded
    /// code points, as described in 6.1.4.
    ///
    /// > NOTE: This method is intentionally generic; it does not require that
    /// > its this value be a String object. Therefore, it can be transferred
    /// > to other kinds of objects for use as a method.
    fn trim_start<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? TrimString(S, start).
        Self::trim_string(agent, this_value, TrimWhere::Start, gc)
    }

    /// ### [22.1.3.29 String.prototype.toString ( )](https://tc39.es/ecma262/#sec-string.prototype.tostring)
    /// ### [22.1.3.35 String.prototype.valueOf ( )](https://tc39.es/ecma262/#sec-string.prototype.valueof)
    ///
    /// > NOTE: `String.prototype.toString` and `String.prototype.valueOf` are
    /// > different functions but have the exact same steps.
    fn value_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Return ? ThisStringValue(this value).
        this_string_value(agent, this_value, gc.into_nogc()).map(|string| string.into())
    }

    /// ### [22.1.3.36 String.prototype \[ %Symbol.iterator% \] ( )](https://tc39.es/ecma262/#sec-string.prototype-%symbol.iterator%)
    ///
    /// This method returns an iterator object that iterates over the code
    /// points of a String value, returning each code point as a String value.
    fn iterator<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value, nogc)
            .unbind()?
            .bind(nogc);
        // 2. Let s be ? ToString(O).
        let s = to_string(agent, o.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // d. Return undefined.
        // 4. Return CreateIteratorFromClosure(closure, "%StringIteratorPrototype%", %StringIteratorPrototype%).
        Ok(StringIterator::create(agent, s.unbind(), gc.into_nogc()).into())
    }

    /// ### [B.2.2.1 String.prototype.substr ( start, length )](https://tc39.es/ecma262/#sec-string.prototype.substr)
    ///
    /// This method returns a substring of the result of converting the this
    /// value to a String, starting from index start and running for length
    /// code units (or through the end of the String if length is undefined).
    /// If start is negative, it is treated as sourceLength + start where
    /// sourceLength is the length of the String. The result is a String value,
    /// not a String object.
    #[cfg(feature = "annex-b-string")]
    fn substr<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let start = args.get(0).scope(agent, nogc);
        let length = args.get(1).scope(agent, nogc);

        let s = if let Ok(s) = String::try_from(this_value) {
            s
        } else {
            // 1. Let O be ? RequireObjectCoercible(this value).
            let o = require_object_coercible(agent, this_value, nogc)
                .unbind()?
                .bind(nogc);
            // 2. Let S be ? ToString(O).
            to_string(agent, o.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
        };
        let scoped_s = s.scope(agent, gc.nogc());

        // 3. Let size be the length of S.
        let size = s.utf16_len_(agent) as i64;

        // 4. Let intStart be ? ToIntegerOrInfinity(start).
        let int_start = to_integer_or_infinity(agent, start.get(agent), gc.reborrow()).unbind()?;

        // 5. If intStart = -∞, set intStart to 0.
        let int_start = if int_start.is_neg_infinity() {
            0
        } else if int_start.is_negative() {
            // 6. Else if intStart < 0, set intStart to max(size + intStart, 0).
            (int_start.into_i64() + size).max(0)
        } else {
            // 7. Else, set intStart to min(intStart, size).
            int_start.into_i64().min(size)
        };

        // 8. If length is undefined, let intLength be size; otherwise let intLength be ? ToIntegerOrInfinity(length).
        let int_length = if length.get(agent).is_undefined() {
            size
        } else {
            to_integer_or_infinity(agent, length.get(agent), gc.reborrow())
                .unbind()?
                .into_i64()
        };

        // 9. Set intLength to the result of clamping intLength between 0 and size.
        let int_length = int_length.clamp(0, size);

        // 10. Let intEnd be min(intStart + intLength, size).
        let int_end = (int_start + int_length).min(size);

        // 11. Return the substring of S from intStart to intEnd.
        let gc = gc.into_nogc();
        let s = scoped_s.get(agent).bind(gc);
        let s_str = s.to_string_lossy_(agent);
        Ok(String::from_string(
            agent,
            s_str[int_start as usize..int_end as usize].to_string(),
            gc,
        )
        .into())
    }

    /// ### [B.2.2.2 String.prototype.anchor ( name )](https://tc39.es/ecma262/#sec-string.prototype.anchor)
    #[cfg(feature = "annex-b-string")]
    fn anchor<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let name = args.get(0).bind(gc.nogc());

        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "a", "name", name).
        create_html(agent, this_value, "a", Some(("name", name.unbind())), gc).map(Value::from)
    }

    /// ### [B.2.2.3 String.prototype.big ( )](https://tc39.es/ecma262/#sec-string.prototype.big)
    #[cfg(feature = "annex-b-string")]
    fn big<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "big", "", "").
        create_html(agent, this_value, "big", None, gc).map(Value::from)
    }

    /// ### [B.2.2.4 String.prototype.blink ( )](https://tc39.es/ecma262/#sec-string.prototype.blink)
    #[cfg(feature = "annex-b-string")]
    fn blink<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "blink", "", "").
        create_html(agent, this_value, "blink", None, gc).map(Value::from)
    }

    /// ### [B.2.2.5 String.prototype.bold ( )](https://tc39.es/ecma262/#sec-string.prototype.bold)
    #[cfg(feature = "annex-b-string")]
    fn bold<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "b", "", "").
        create_html(agent, this_value, "b", None, gc).map(Value::from)
    }

    /// ### [B.2.2.6 String.prototype.fixed ( )](https://tc39.es/ecma262/#sec-string.prototype.fixed)
    #[cfg(feature = "annex-b-string")]
    fn fixed<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "tt", "", "").
        create_html(agent, this_value, "tt", None, gc).map(Value::from)
    }

    /// ### [B.2.2.7 String.prototype.fontcolor ( colour )](https://tc39.es/ecma262/#sec-string.prototype.fontcolor)
    #[cfg(feature = "annex-b-string")]
    fn fontcolor<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let colour = args.get(0).bind(gc.nogc());

        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "font", "colour", colour).
        create_html(
            agent,
            this_value,
            "font",
            Some(("colour", colour.unbind())),
            gc,
        )
        .map(Value::from)
    }

    /// ### [B.2.2.8 String.prototype.fontsize ( size )](https://tc39.es/ecma262/#sec-string.prototype.fontsize)
    #[cfg(feature = "annex-b-string")]
    fn fontsize<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let size = args.get(0).bind(gc.nogc());

        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "font", "size", size).
        create_html(agent, this_value, "font", Some(("size", size.unbind())), gc).map(Value::from)
    }

    /// ### [B.2.2.9 String.prototype.italics ( )](https://tc39.es/ecma262/#sec-string.prototype.italics)
    #[cfg(feature = "annex-b-string")]
    fn italics<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "tt", "", "").
        create_html(agent, this_value, "i", None, gc).map(Value::from)
    }

    /// ### [B.2.2.10 String.prototype.link ( url )](https://tc39.es/ecma262/#sec-string.prototype.link)
    #[cfg(feature = "annex-b-string")]
    fn link<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let url = args.get(0).bind(gc.nogc());

        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "a", "href", url).
        create_html(agent, this_value, "a", Some(("href", url.unbind())), gc).map(Value::from)
    }

    /// ### [B.2.2.11 String.prototype.small ( )](https://tc39.es/ecma262/#sec-string.prototype.small)
    #[cfg(feature = "annex-b-string")]
    fn small<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "small", "", "").
        create_html(agent, this_value, "small", None, gc).map(Value::from)
    }

    /// ### [B.2.2.12 String.prototype.strike ( )](https://tc39.es/ecma262/#sec-string.prototype.strike)
    #[cfg(feature = "annex-b-string")]
    fn strike<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "strike", "", "").
        create_html(agent, this_value, "strike", None, gc).map(Value::from)
    }

    /// ### [B.2.2.13 String.prototype.sub ( )](https://tc39.es/ecma262/#sec-string.prototype.sub)
    #[cfg(feature = "annex-b-string")]
    fn sub<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "sub", "", "").
        create_html(agent, this_value, "sub", None, gc).map(Value::from)
    }

    /// ### [B.2.2.14 String.prototype.sup ( )](https://tc39.es/ecma262/#sec-string.prototype.sup)
    #[cfg(feature = "annex-b-string")]
    fn sup<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let S be the this value.
        // 2. Return ? CreateHTML(S, "sup", "", "").
        create_html(agent, this_value, "sup", None, gc).map(Value::from)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.string_prototype();
        let this_base_object = intrinsics.string_prototype_backing_object();
        let string_constructor = intrinsics.string();
        #[cfg(feature = "annex-b-string")]
        let prototype_trim_start = intrinsics.string_prototype_trim_start();
        #[cfg(feature = "annex-b-string")]
        let prototype_trim_end = intrinsics.string_prototype_trim_end();

        let regexp_property_count = if cfg!(feature = "regexp") { 3 } else { 0 };
        let annex_b_property_count = if cfg!(feature = "regexp") { 16 } else { 0 };

        let builder = OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this_base_object)
            .with_property_capacity(33 + regexp_property_count + annex_b_property_count)
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
            .with_builtin_function_property::<StringPrototypeLastIndexOf>()
            .with_builtin_function_property::<StringPrototypeLocaleCompare>();
        #[cfg(feature = "regexp")]
        let builder = builder
            .with_builtin_function_property::<StringPrototypeMatch>()
            .with_builtin_function_property::<StringPrototypeMatchAll>();
        let builder = builder
            .with_builtin_function_property::<StringPrototypeNormalize>()
            .with_builtin_function_property::<StringPrototypePadEnd>()
            .with_builtin_function_property::<StringPrototypePadStart>()
            .with_builtin_function_property::<StringPrototypeRepeat>()
            .with_builtin_function_property::<StringPrototypeReplace>()
            .with_builtin_function_property::<StringPrototypeReplaceAll>();

        #[cfg(feature = "regexp")]
        let builder = builder.with_builtin_function_property::<StringPrototypeSearch>();
        let builder = builder
            .with_builtin_function_property::<StringPrototypeSlice>()
            .with_builtin_function_property::<StringPrototypeSplit>()
            .with_builtin_function_property::<StringPrototypeStartsWith>()
            .with_builtin_function_property::<StringPrototypeSubstring>()
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
            .with_builtin_function_property::<StringPrototypeIterator>();

        #[cfg(feature = "annex-b-string")]
        let builder = builder
            .with_builtin_function_property::<StringPrototypeSubstr>()
            .with_builtin_function_property::<StringPrototypeAnchor>()
            .with_builtin_function_property::<StringPrototypeBig>()
            .with_builtin_function_property::<StringPrototypeBlink>()
            .with_builtin_function_property::<StringPrototypeBold>()
            .with_builtin_function_property::<StringPrototypeFixed>()
            .with_builtin_function_property::<StringPrototypeFontcolor>()
            .with_builtin_function_property::<StringPrototypeFontsize>()
            .with_builtin_function_property::<StringPrototypeItalics>()
            .with_builtin_function_property::<StringPrototypeLink>()
            .with_builtin_function_property::<StringPrototypeSmall>()
            .with_builtin_function_property::<StringPrototypeStrike>()
            .with_builtin_function_property::<StringPrototypeSub>()
            .with_builtin_function_property::<StringPrototypeSup>()
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.trimLeft.to_property_key())
                    .with_value(prototype_trim_start.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.trimRight.to_property_key())
                    .with_value(prototype_trim_end.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            });

        builder.build();

        let slot = agent
            .heap
            .primitive_objects
            .get_mut(this.get_index())
            .unwrap();
        *slot = PrimitiveObjectRecord {
            object_index: Some(this_base_object),
            data: PrimitiveObjectData::SmallString(SmallString::EMPTY),
        };
    }
}

/// ### [22.1.3.17.1 StringPaddingBuiltinsImpl ( O, maxLength, fillString, placement )](https://tc39.es/ecma262/#sec-stringpaddingbuiltinsimpl)
///
/// The abstract operation StringPaddingBuiltinsImpl takes arguments O (an
/// ECMAScript language value), maxLength (an ECMAScript language value),
/// fillString (an ECMAScript language value), and placement (start or end)
/// and returns either a normal completion containing a String or a throw
/// completion.
fn string_padding_builtins_impl<'gc>(
    agent: &mut Agent,
    o: Value,
    max_length: Value,
    fill_string: Value,
    placement_start: bool,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let nogc = gc.nogc();
    let o = o.bind(nogc);
    let mut max_length = max_length.bind(nogc);
    let mut fill_string = fill_string.bind(nogc);
    let mut scoped_fill_string = None;
    // 1. Let S be ? ToString(O).
    let mut s = if let Some(s) = try_result_into_js(try_to_string(agent, o, nogc))
        .unbind()?
        .bind(gc.nogc())
    {
        s
    } else {
        scoped_fill_string = Some(fill_string.scope(agent, nogc));
        let scoped_max_length = max_length.scope(agent, nogc);
        let s = to_string(agent, o.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // SAFETY: never leaked outside this call path.
        max_length = unsafe { scoped_max_length.take(agent).bind(gc.nogc()) };
        fill_string = scoped_fill_string
            .as_ref()
            .unwrap()
            .get(agent)
            .bind(gc.nogc());
        s
    };
    let mut scoped_s = None;

    // 2. Let intMaxLength be ℝ(? ToLength(maxLength)).
    let int_max_length = if let Some(int_max_length) =
        try_result_into_js(try_to_length(agent, max_length, gc.nogc()))
            .unbind()?
            .bind(gc.nogc())
    {
        int_max_length
    } else {
        scoped_s = Some(s.scope(agent, gc.nogc()));
        scoped_fill_string =
            scoped_fill_string.or_else(|| Some(fill_string.scope(agent, gc.nogc())));
        let int_max_length = to_length(agent, max_length.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        s = scoped_s.as_ref().unwrap().get(agent).bind(gc.nogc());
        fill_string = scoped_fill_string
            .as_ref()
            .unwrap()
            .get(agent)
            .bind(gc.nogc());
        int_max_length
    };

    // 3. Let stringLength be the length of S.
    let string_length = s.utf16_len_(agent) as i64;

    // 4. If intMaxLength ≤ stringLength, return S.
    if int_max_length <= string_length {
        return Ok(s.unbind().into());
    }

    // 5. If fillString is undefined, set fillString to the String value consisting solely of the code unit 0x0020 (SPACE).
    let fill_string = if fill_string.is_undefined() {
        BUILTIN_STRING_MEMORY.r#__
    } else if let Ok(fill_string) = String::try_from(fill_string) {
        fill_string
    } else {
        if scoped_s.is_none() {
            scoped_s = Some(s.scope(agent, gc.nogc()));
        }
        // 6. Else, set fillString to ? ToString(fillString).
        let result = to_string(agent, fill_string.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        s = scoped_s.unwrap().get(agent).bind(gc.nogc());
        result
    };

    let s = s.unbind();
    let fill_string = fill_string.unbind();
    let gc = gc.into_nogc();
    let s = s.bind(gc);
    let fill_string = fill_string.bind(gc);

    // 7. Return StringPad(S, intMaxLength, fillString, placement).
    string_pad(agent, s, int_max_length, fill_string, placement_start, gc)
}

/// ### [22.1.3.17.2 StringPad ( S, maxLength, fillString, placement )](https://tc39.es/ecma262/#sec-stringpad)
///
/// The abstract operation StringPad takes arguments S (a String),
/// maxLength (a non-negative integer), fillString (a String), and
/// placement (start or end) and returns a String.
fn string_pad<'gc>(
    agent: &mut Agent,
    s: String<'gc>,
    max_len: i64,
    fill_string: String<'gc>,
    placement_start: bool,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    // 1. Let stringLength be the length of S.
    let string_len = s.utf16_len_(agent) as i64;

    // 2. If maxLength ≤ stringLength, return S.
    if max_len <= string_len {
        return Ok(s.into());
    }

    // 3. If fillString is the empty String, return S.
    if fill_string.is_empty_string() {
        return Ok(s.into());
    }

    // 4. Let fillLen be maxLength - stringLength.
    // Note: we checked that max_len > string_len above; this is always >0.
    let fill_len = max_len.wrapping_sub(string_len) as usize;

    let fill_string_len = fill_string.utf16_len_(agent);

    // 5. Let truncatedStringFiller be the String value consisting of repeated
    //    concatenations of fillString truncated to length fillLen.
    let fill_count = fill_len / fill_string_len;
    let overflow_len = fill_len % fill_string_len;
    let fill_buf = fill_string.as_wtf8_(agent);
    let wtf8_index = fill_string
        .utf8_index_(agent, overflow_len)
        .unwrap_or(overflow_len * 3);
    let mut buf = Wtf8Buf::with_capacity(s.len_(agent) + fill_count * fill_buf.len() + wtf8_index);

    // 6. If placement is start, return the string-concatenation of
    //    truncatedStringFiller and S.
    if !placement_start {
        buf.push_wtf8(s.as_wtf8_(agent));
    }
    for _ in 0..fill_count {
        buf.push_wtf8(fill_buf);
    }
    if overflow_len > 0 {
        for cp in char::decode_utf16(fill_buf.to_ill_formed_utf16().take(overflow_len)) {
            match cp {
                Ok(char) => buf.push_char(char),
                // SAFETY: unpaired surrogates fail parsing, and they're in the
                // CodePoint range.
                Err(code) => buf.push(unsafe {
                    CodePoint::from_u32_unchecked(code.unpaired_surrogate() as u32)
                }),
            }
        }
    }
    // 7. Else, return the string-concatenation of S and truncatedStringFiller.
    if placement_start {
        buf.push_wtf8(s.as_wtf8_(agent));
    }

    Ok(String::from_wtf8_buf(agent, buf, gc).into())
}

/// ### [22.1.3.17.3 ToZeroPaddedDecimalString ( n, minLength )](https://tc39.es/ecma262/#sec-tozeropaddeddecimalstring)
///
/// The abstract operation ToZeroPaddedDecimalString takes arguments n
/// (a non-negative integer) and minLength (a non-negative integer) and returns
/// a String.
#[cfg(feature = "date")]
pub(crate) fn to_zero_padded_decimal_string(
    n: impl ToString,
    min_length: usize,
) -> std::string::String {
    // 1. Let S be the String representation of n, formatted as a decimal number.
    let s = n.to_string();
    // 2. Return StringPad(S, minLength, "0", start).
    format!("{s:0>min_length$}")
}

/// ### [22.1.3.19.1 GetSubstitution ( matched, str, position, captures, namedCaptures, replacementTemplate )](https://tc39.es/ecma262/#sec-getsubstitution)
///
/// The abstract operation GetSubstitution takes arguments matched (a String),
/// str (a String), position (a non-negative integer), captures (a List of
/// either Strings or undefined), namedCaptures (an Object or undefined), and
/// replacementTemplate (a String) and returns either a normal completion
/// containing a String or a throw completion. For the purposes of this
/// abstract operation, a decimal digit is a code unit in the inclusive
/// interval from 0x0030 (DIGIT ZERO) to 0x0039 (DIGIT NINE).
#[allow(clippy::too_many_arguments)]
#[cfg(feature = "regexp")]
pub(crate) fn get_substitution<'gc, 'scope>(
    agent: &mut Agent,
    scoped_matched: Scoped<'scope, String>,
    scoped_str: Scoped<'scope, String>,
    position: usize,
    scoped_captures: Vec<Option<Scoped<'scope, String>>>,
    named_captures: Option<Object>,
    scoped_replacement_template: Scoped<'scope, String>,
    mut gc: GcScope<'gc, 'scope>,
) -> JsResult<'gc, String<'gc>> {
    let named_captures = named_captures.map(|c| c.scope(agent, gc.nogc()));
    let str = scoped_str.to_string_lossy(agent);
    // SAFETY: Scoped string data cannot be mutated or dropped.
    let str = unsafe { core::mem::transmute::<&str, &'scope str>(str.as_ref()) };
    let matched = scoped_matched.to_string_lossy(agent);
    // SAFETY: Scoped string data cannot be mutated or dropped.
    let matched = unsafe { core::mem::transmute::<&str, &'scope str>(matched.as_ref()) };
    // 1. Let stringLength be the length of str.
    let utf8_string_length = str.len();
    // 2. Assert: position ≤ stringLength.
    debug_assert!(position <= scoped_str.get(agent).utf16_len_(agent));
    let utf8_position = scoped_str
        .get(agent)
        .utf8_index_(agent, position)
        .expect("Invalid UTF-8 position");
    // 3. Let result be the empty String.
    let mut result = Wtf8Buf::new();
    // 4. Let templateRemainder be replacementTemplate.
    let template_remainder = scoped_replacement_template.to_string_lossy(agent);
    // SAFETY: Scoped string data cannot be mutated or dropped.
    let mut template_remainder =
        unsafe { core::mem::transmute::<&str, &'scope str>(template_remainder.as_ref()) };
    // 5. Repeat, while templateRemainder is not the empty String,
    while !template_remainder.is_empty() {
        let template_remainder_bytes = template_remainder.as_bytes();
        // a. NOTE: The following steps isolate ref (a prefix of
        //    templateRemainder), determine refReplacement (its replacement),
        //    and then append that replacement to result.
        let mut r#ref = template_remainder;
        let mut ref_replacement = std::borrow::Cow::Borrowed(template_remainder);
        if template_remainder_bytes.len() == 1 {
            // h. Else,
            // i. Let ref be the substring of templateRemainder from 0 to 1.
            // ii. Let refReplacement be ref.
        } else if template_remainder_bytes[0] == b'$' {
            if template_remainder_bytes[1] == b'$' {
                // b. If templateRemainder starts with "$$", then
                // i. Let ref be "$$".
                r#ref = "$$";
                // ii. Let refReplacement be "$".
                ref_replacement = "$".into();
            } else if template_remainder_bytes[1] == b'`' {
                // c. Else if templateRemainder starts with "$`", then
                // i. Let ref be "$`".
                r#ref = "$`";
                // ii. Let refReplacement be the substring of str from 0 to position.
                ref_replacement = str[0..utf8_position].into()
            } else if template_remainder_bytes[1] == b'&' {
                // d. Else if templateRemainder starts with "$&", then
                // i. Let ref be "$&".
                r#ref = "$&";
                // ii. Let refReplacement be matched.
                ref_replacement = matched.into();
            } else if template_remainder_bytes[1] == b'\'' {
                // e. Else if templateRemainder starts with "$'" (0x0024
                //    (DOLLAR SIGN) followed by 0x0027 (APOSTROPHE)), then
                // i. Let ref be "$'".
                r#ref = "$'";
                // ii. Let matchLength be the length of matched.
                let match_length = matched.len();
                // iii. Let tailPos be position + matchLength.
                let tail_pos = position.saturating_add(match_length);
                // iv. Let refReplacement be the substring of str from
                //     min(tailPos, stringLength).
                ref_replacement = str[tail_pos.min(utf8_string_length)..].into();
                // v. NOTE: tailPos can exceed stringLength only if this
                //    abstract operation was invoked by a call to the intrinsic
                //    %Symbol.replace% method of %RegExp.prototype% on an
                //    object whose "exec" property is not the intrinsic
                //    %RegExp.prototype.exec%.
            } else if template_remainder_bytes[1].is_ascii_digit() {
                // f. Else if templateRemainder starts with "$" followed by 1
                //    or more decimal digits, then
                // i. If templateRemainder starts with "$" followed by 2 or
                //    more decimal digits, let digitCount be 2; otherwise let
                //    digitCount be 1.
                let mut digit_count = if template_remainder_bytes.len() > 2
                    && template_remainder_bytes[2].is_ascii_digit()
                {
                    2
                } else {
                    1
                };
                // ii. Let digits be the substring of templateRemainder from 1 to 1 + digitCount.
                let mut digits = &template_remainder[1..1 + digit_count];
                // iii. Let index be ℝ(StringToNumber(digits)).
                let index: u8 = digits.parse().unwrap();
                // iv. Assert: 0 ≤ index ≤ 99.
                debug_assert!(index <= 99);
                let mut index = index as usize;
                // v. Let captureLen be the number of elements in captures.
                let capture_len = scoped_captures.len();
                // vi. If index > captureLen and digitCount = 2, then
                if index > capture_len && digit_count == 2 {
                    // 1. NOTE: When a two-digit replacement pattern specifies
                    //    an index exceeding the count of capturing groups, it
                    //    is treated as a one-digit replacement pattern
                    //    followed by a literal digit.
                    // 2. Set digitCount to 1.
                    digit_count = 1;
                    // 3. Set digits to the substring of digits from 0 to 1.
                    digits = &digits[0..1];
                    // 4. Set index to ℝ(StringToNumber(digits)).
                    index = digits.parse::<u8>().unwrap() as usize;
                }
                // vii. Let ref be the substring of templateRemainder from 0 to
                //      1 + digitCount.
                r#ref = &template_remainder[0..1 + digit_count];
                // viii. If 1 ≤ index ≤ captureLen, then
                if 1 <= index && index <= capture_len {
                    // 1. Let capture be captures[index - 1].
                    let capture = scoped_captures[index - 1].clone();
                    if let Some(capture) = capture {
                        // 3. Else,
                        // a. Let refReplacement be capture.
                        ref_replacement = capture.to_string_lossy(agent).to_string().into();
                    } else {
                        // 2. If capture is undefined, then
                        // a. Let refReplacement be the empty String.
                        ref_replacement = "".into();
                    }
                } else {
                    // ix. Else,
                    // 1. Let refReplacement be ref.
                    ref_replacement = r#ref.into();
                }
            } else if template_remainder_bytes[1] == b'<' {
                // g. Else if templateRemainder starts with "$<", then
                // i. Let gtPos be StringIndexOf(templateRemainder, ">", 0).
                let gt_pos = template_remainder.find(">");
                if let (Some(gt_pos), Some(named_captures)) = (gt_pos, named_captures.clone()) {
                    // iii. Else,
                    // 1. Let ref be the substring of templateRemainder from 0 to gtPos + 1.
                    r#ref = &template_remainder[0..gt_pos + 1];
                    // 2. Let groupName be the substring of templateRemainder from 2 to gtPos.
                    let group_name = &template_remainder[2..gt_pos];
                    let group_name = String::from_str(agent, group_name, gc.nogc());
                    // 3. Assert: namedCaptures is an Object.
                    // 4. Let capture be ? Get(namedCaptures, groupName).
                    let capture = get(
                        agent,
                        named_captures.get(agent),
                        group_name.to_property_key().unbind(),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc());
                    // 5. If capture is undefined, then
                    if capture.is_undefined() {
                        // a. Let refReplacement be the empty String.
                        ref_replacement = "".into();
                    } else {
                        // 6. Else,
                        // a. Let refReplacement be ? ToString(capture).
                        let capture = to_string(agent, capture.unbind(), gc.reborrow())
                            .unbind()?
                            .bind(gc.nogc());
                        ref_replacement = capture.to_string_lossy_(agent).to_string().into();
                    }
                } else {
                    // ii. If gtPos is not-found or namedCaptures is undefined, then
                    // 1. Let ref be "$<".
                    r#ref = "$<";
                    // 2. Let refReplacement be ref.
                    ref_replacement = r#ref.into();
                }
            }
        }
        // i. Let refLength be the length of ref.
        let ref_length = r#ref.len();
        // j. Set templateRemainder to the substring of templateRemainder from refLength.
        template_remainder = &template_remainder[ref_length..];
        // k. Set result to the string-concatenation of result and refReplacement.
        result.push_str(&ref_replacement);
    }
    // 6. Return result.
    Ok(String::from_wtf8_buf(agent, result, gc.into_nogc()))
}

/// ### [22.1.3.35.1 ThisStringValue ( value )](https://tc39.es/ecma262/#sec-thisstringvalue)
///
/// The abstract operation ThisStringValue takes argument value (an ECMAScript
/// language value) and returns either a normal completion containing a String
/// or a throw completion.
fn this_string_value<'gc>(
    agent: &mut Agent,
    value: Value,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, String<'gc>> {
    match value.bind(gc) {
        // 1. If value is a String, return value.
        Value::String(data) => Ok(data.into()),
        Value::SmallString(data) => Ok(data.into()),
        // 2. If value is an Object and value has a [[StringData]] internal slot, then
        Value::PrimitiveObject(obj) if obj.is_string_object(agent) => {
            // a. Let s be value.[[StringData]].
            // b. Assert: s is a String.
            // c. Return s.
            match obj.get(agent).data {
                PrimitiveObjectData::String(data) => Ok(data.into()),
                PrimitiveObjectData::SmallString(data) => Ok(data.into()),
                _ => unreachable!(),
            }
        }
        _ => {
            // 3. Throw a TypeError exception.
            Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Not a string value",
                gc,
            ))
        }
    }
}

/// ### [B.2.2.2.1 CreateHTML ( string, tag, attribute, value )](https://tc39.es/ecma262/#sec-createhtml)
///
/// The abstract operation CreateHTML takes arguments string (an ECMAScript
/// language value), tag (a String), attribute (a String), and value (an
/// ECMAScript language value) and returns either a normal completion
/// containing a String or a throw completion.
#[cfg(feature = "annex-b-string")]
fn create_html<'gc>(
    agent: &mut Agent,
    string: Value,
    tag: &str,
    attribute_and_value: Option<(&str, Value)>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, String<'gc>> {
    use crate::engine::Scopable;

    let nogc = gc.nogc();
    // 1. Let str be ? RequireObjectCoercible(string).
    let string = string.bind(nogc);
    let mut attribute_and_value =
        attribute_and_value.map(|(attribute, value)| (attribute, value.bind(nogc)));
    let str = require_object_coercible(agent, string, nogc)
        .unbind()?
        .bind(nogc);

    // 2. Let S be ? ToString(str)
    let mut s = if let Some(s) = try_result_into_js(try_to_string(agent, str.unbind(), nogc))
        .unbind()?
        .bind(gc.nogc())
    {
        s
    } else {
        let attribute_and_scoped_value =
            attribute_and_value.map(|(attribute, value)| (attribute, value.scope(agent, nogc)));
        let s = to_string(agent, str.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        attribute_and_value = attribute_and_scoped_value
            .map(|(attribute, scoped_value)| (attribute, scoped_value.get(agent).bind(gc.nogc())));
        s
    };

    // 3. Let p1 be the string-concatenation of "<" and tag.
    // 4. If attribute is not the empty String, then
    // c. Set p1 to the string-concatenation of:
    // - p1
    // - the code unit 0x0020 (SPACE)
    // - attribute
    // - the code unit 0x003D (EQUALS SIGN)
    // - the code unit 0x0022 (QUOTATION MARK)
    // - escapedV
    // - the code unit 0x0022 (QUOTATION MARK)
    // 5. Let p2 be the string-concatenation of p1 and ">".
    // 6. Let p3 be the string-concatenation of p2 and S.
    // 7. Let p4 be the string-concatenation of p3, "</", tag, and ">".
    // 8. Return p4.
    if let Some((attribute, value)) = attribute_and_value {
        // a. Let V be ? ToString(value).
        let v = if let Some(v) = try_result_into_js(try_to_string(agent, value, gc.nogc()))
            .unbind()?
            .bind(gc.nogc())
        {
            v
        } else {
            let scoped_s = s.scope(agent, gc.nogc());
            let v = to_string(agent, value.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            s = scoped_s.get(agent).bind(gc.nogc());
            v
        };
        // b. Let escapedV be the String value that is the same as V except that each occurrence of the code unit 0x0022 (QUOTATION MARK) in V has been replaced with the six code unit sequence "&quot;".
        let escaped_v = v.to_string_lossy_(agent).replace('"', "&quot;");

        let s_str = s.to_string_lossy_(agent);
        Ok(String::from_string(
            agent,
            format!("<{tag} {attribute}=\"{escaped_v}\">{s_str}</{tag}>"),
            gc.into_nogc(),
        ))
    } else {
        let s_str = s.to_string_lossy_(agent);
        Ok(String::from_string(
            agent,
            format!("<{tag}>{s_str}</{tag}>"),
            gc.into_nogc(),
        ))
    }
}

enum TrimWhere {
    Start,
    End,
    StartAndEnd,
}

enum NormalizeForm {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

impl FromStr for NormalizeForm {
    type Err = ();

    fn from_str(input: &str) -> Result<NormalizeForm, Self::Err> {
        match input {
            "NFC" => Ok(NormalizeForm::Nfc),
            "NFD" => Ok(NormalizeForm::Nfd),
            "NFKC" => Ok(NormalizeForm::Nfkc),
            "NFKD" => Ok(NormalizeForm::Nfkd),
            _ => Err(()),
        }
    }
}

fn unicode_normalize(s: &str, f: NormalizeForm) -> Option<std::string::String> {
    match f {
        NormalizeForm::Nfc => match is_nfc_quick(s.chars()) {
            IsNormalized::Yes => None,
            _ => Some(s.nfc().collect::<std::string::String>()),
        },
        NormalizeForm::Nfd => match is_nfd_quick(s.chars()) {
            IsNormalized::Yes => None,
            _ => Some(s.nfd().collect::<std::string::String>()),
        },
        NormalizeForm::Nfkc => match is_nfkc_quick(s.chars()) {
            IsNormalized::Yes => None,
            _ => Some(s.nfkc().collect::<std::string::String>()),
        },
        NormalizeForm::Nfkd => match is_nfkd_quick(s.chars()) {
            IsNormalized::Yes => None,
            _ => Some(s.nfkd().collect::<std::string::String>()),
        },
    }
}
