// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{cmp::max, collections::VecDeque, iter::repeat};

use small_string::SmallString;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call_function, create_array_from_list, get_method},
            testing_and_comparison::{is_callable, require_object_coercible},
            type_conversion::{
                is_trimmable_whitespace, to_integer_or_infinity, to_length, to_number, to_string,
                to_uint32,
            },
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            primitive_objects::{PrimitiveObjectData, PrimitiveObjectHeapData},
            ArgumentsList, Array, Behaviour, Builtin, BuiltinIntrinsic,
        },
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{IntoValue, Number, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
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
struct StringPrototypeLastIndexOf;
impl Builtin for StringPrototypeLastIndexOf {
    const NAME: String = BUILTIN_STRING_MEMORY.lastIndexOf;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::last_index_of);
}
struct StringPrototypeLocaleCompare;
impl Builtin for StringPrototypeLocaleCompare {
    const NAME: String = BUILTIN_STRING_MEMORY.localeCompare;
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
    const NAME: String = BUILTIN_STRING_MEMORY.padStart;
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
    const LENGTH: u8 = 2;
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
struct StringPrototypeSubstring;
impl Builtin for StringPrototypeSubstring {
    const NAME: String = BUILTIN_STRING_MEMORY.substring;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::substring);
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
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::value_of);
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
    const KEY: Option<PropertyKey> = Some(WellKnownSymbolIndexes::Iterator.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringPrototype::iterator);
}

impl StringPrototype {
    fn at(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;
        // 3. Let len be the length of S.
        let len = i64::try_from(s.utf16_len(agent)).unwrap();
        // 4. Let relativeIndex be ? ToIntegerOrInfinity(pos).
        let relative_index = to_integer_or_infinity(agent, args.get(0))?.into_i64(agent);
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
            let ch = s.utf16_char(agent, usize::try_from(k).unwrap());
            Ok(SmallString::from_code_point(ch).into_value())
        }
    }

    fn char_at(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;
        // 3. Let position be ? ToIntegerOrInfinity(pos).
        let position = to_integer_or_infinity(agent, args.get(0))?.into_i64(agent);
        // 4. Let size be the length of S.
        let size = s.utf16_len(agent);
        // 5. If position < 0 or position ≥ size, return the empty String.
        if position < 0 || position >= i64::try_from(size).unwrap() {
            Ok(String::EMPTY_STRING.into_value())
        } else {
            // 6. Return the substring of S from position to position + 1.
            let ch = s.utf16_char(agent, usize::try_from(position).unwrap());
            Ok(SmallString::from_code_point(ch).into_value())
        }
    }

    fn char_code_at(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;
        // 3. Let position be ? ToIntegerOrInfinity(pos).
        let position = to_integer_or_infinity(agent, args.get(0))?.into_i64(agent);
        // 4. Let size be the length of S.
        let size = s.utf16_len(agent);
        // 5. If position < 0 or position ≥ size, return NaN.
        if position < 0 || position >= i64::try_from(size).unwrap() {
            Ok(Value::nan())
        } else {
            // 6. Return the Number value for the numeric value of the code unit at index position
            // within the String S.
            let ch = s.utf16_char(agent, usize::try_from(position).unwrap());
            Ok(Value::from(ch as u32))
        }
    }

    fn code_point_at(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;
        // 3. Let position be ? ToIntegerOrInfinity(pos).
        let position = to_integer_or_infinity(agent, args.get(0))?.into_i64(agent);
        // 4. Let size be the length of S.
        let size = s.utf16_len(agent);
        // 5. If position < 0 or position ≥ size, return undefined.
        if position < 0 || position >= i64::try_from(size).unwrap() {
            Ok(Value::Undefined)
        } else {
            // 6. Let cp be CodePointAt(S, position).
            // 7. Return 𝔽(cp.[[CodePoint]]).
            // TODO: Deal with lone surrogates.
            let u8_idx = s
                .utf8_index(agent, usize::try_from(position).unwrap())
                .unwrap();
            let ch = s.as_str(agent)[u8_idx..].chars().next().unwrap();
            Ok(Value::from(ch as u32))
        }
    }

    fn concat(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;
        // 3. Let R be S.
        let mut strings = Vec::with_capacity(args.len() + 1);
        strings.push(s);
        // 4. For each element next of args, do
        //     a. Let nextString be ? ToString(next).
        //     b. Set R to the string-concatenation of R and nextString.
        for next in args.iter() {
            strings.push(to_string(agent, *next)?);
        }
        // 5. Return R.
        Ok(String::concat(agent, &strings).into_value())
    }

    fn ends_with(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;

        // 3. Let isRegExp be ? IsRegExp(searchString).
        // 4. If isRegExp is true, throw a TypeError exception.
        // TODO

        // 5. Let searchStr be ? ToString(searchString).
        let search_str = to_string(agent, args.get(0))?;

        // 6. Let len be the length of S.
        // 7. If endPosition is undefined, let pos be len; else let pos be ? ToIntegerOrInfinity(endPosition).
        // 8. Let end be the result of clamping pos between 0 and len.
        let end_position = args.get(1);
        let haystack_str = if end_position.is_undefined() {
            s.as_str(agent)
        } else {
            let pos = to_integer_or_infinity(agent, end_position)?.into_usize(agent);
            let end = if pos != 0 {
                // NOTE: `pos` was already clamped to 0 by `Number::into_usize`.
                pos.min(s.utf16_len(agent))
            } else {
                0
            };
            let u8_idx = s.utf8_index(agent, end).unwrap();
            &s.as_str(agent)[..u8_idx]
        };

        // 9. Let searchLength be the length of searchStr.
        // 10. If searchLength = 0, return true.
        // 11. Let start be end - searchLength.
        // 12. If start < 0, return false.
        // 13. Let substring be the substring of S from start to end.
        // 14. If substring is searchStr, return true.
        // 15. Return false.
        Ok(Value::from(
            haystack_str.ends_with(search_str.as_str(agent)),
        ))
    }

    fn includes(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;

        // 3. Let isRegExp be ? IsRegExp(searchString).
        // 4. If isRegExp is true, throw a TypeError exception.
        // TODO

        // 5. Let searchStr be ? ToString(searchString).
        let search_str = to_string(agent, args.get(0))?;

        // 6. Let pos be ? ToIntegerOrInfinity(position).
        // 7. Assert: If position is undefined, then pos is 0.
        // 8. Let len be the length of S.
        // 9. Let start be the result of clamping pos between 0 and len.
        let haystack_str = {
            let pos = to_integer_or_infinity(agent, args.get(0))?.into_usize(agent);
            let start = if pos != 0 {
                // NOTE: `pos` was already clamped to 0 by `Number::into_usize`.
                pos.min(s.utf16_len(agent))
            } else {
                0
            };
            &s.as_str(agent)[start..]
        };

        // 10. Let index be StringIndexOf(S, searchStr, start).
        // 11. If index is not-found, return false.
        // 12. Return true.
        Ok(Value::from(haystack_str.contains(search_str.as_str(agent))))
    }

    fn index_of(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;
        // 3. Let searchStr be ? ToString(searchString).
        let search_str = to_string(agent, args.get(0))?;
        // 4. Let pos be ? ToIntegerOrInfinity(position).
        // 5. Assert: If position is undefined, then pos is 0.
        let pos = to_integer_or_infinity(agent, args.get(1))?.into_usize(agent);

        // 6. Let len be the length of S.
        // 7. Let start be the result of clamping pos between 0 and len.
        let utf8_start = if pos != 0 {
            let u16_len = s.utf16_len(agent);
            if pos >= u16_len {
                s.len(agent)
            } else {
                s.utf8_index(agent, pos).unwrap()
            }
        } else {
            0
        };

        // 8. Let result be StringIndexOf(S, searchStr, start).
        // 9. If result is not-found, return -1𝔽.
        // 10. Return 𝔽(result).
        if let Some(rel_u8_pos) = s.as_str(agent)[utf8_start..].find(search_str.as_str(agent)) {
            let u8_pos = utf8_start + rel_u8_pos;
            let result = s.utf16_index(agent, u8_pos);
            Ok(Number::try_from(result).unwrap().into_value())
        } else {
            Ok(Number::from(-1).into_value())
        }
    }

    fn is_well_formed(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;

        // 3. Return IsStringWellFormedUnicode(S).
        // TODO: For now, all strings are well-formed Unicode. In the future, `.as_str()` will
        // return None for WTF-8 strings.
        let _: &str = s.as_str(agent);
        Ok(Value::Boolean(true))
    }

    /// ### [22.1.3.11 String.prototype.lastIndexOf ( searchString \[ , position \] )]()
    ///
    /// > #### Note 1
    /// >
    /// > If _searchString_ appears as a substring of the result of converting
    /// > this object to a String at one or more indices that are smaller than
    /// > or equal to _position_, then the greatest such index is returned;
    /// > otherwise, **`-1𝔽`** is returned. If position is **undefined**, the
    /// > length of the String value is assumed, so as to search all of the
    /// > String.
    fn last_index_of(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        let search_string = args.get(0);
        let position = args.get(1);

        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;
        // 3. Let searchStr be ? ToString(searchString).
        let search_str = to_string(agent, search_string)?;
        let pos = if position.is_undefined() {
            // 5. Assert: If position is undefined, then numPos is NaN.
            // 6. If numPos is NaN, let pos be +∞;
            usize::MAX
        } else if let Value::Integer(position) = position {
            position.into_i64().max(0) as usize
        } else {
            // 4. Let numPos be ? ToNumber(position).
            let num_pos = to_number(agent, args.get(1))?;
            if num_pos.is_nan(agent) {
                // 6. If numPos is NaN, let pos be +∞;
                usize::MAX
            } else {
                // otherwise, let pos be! ToIntegerOrInfinity(numPos).
                to_integer_or_infinity(agent, num_pos.into_value())
                    .unwrap()
                    .into_usize(agent)
            }
        };

        // 7. Let len be the length of S.
        // 8. Let searchLen be the length of searchStr.
        // 9. Let start be the result of clamping pos between 0 and len - searchLen.
        // 10. Let result be StringLastIndexOf(S, searchStr, start).
        // NOTE: If start >= (len - searchLen), there is a last index if and
        // only if start.endsWith(searchLen).
        let haystack_str = {
            if pos == usize::MAX {
                s.as_str(agent)
            } else {
                // When starting from a position, the position may mark the
                // start of the search string, so we need to include the search
                // string length in the haystack.
                let utf8_pos = s.utf8_index(agent, pos).unwrap();
                let utf8_len = s.len(agent);
                let search_str_len = search_str.len(agent);
                &s.as_str(agent)[..utf8_len.min(utf8_pos + search_str_len)]
            }
        };
        let search_str = search_str.as_str(agent);
        let utf8_result = haystack_str.rfind(search_str);

        // 11. If result is not-found, return -1𝔽.
        // 12. Return 𝔽(result).
        if let Some(utf8_idx) = utf8_result {
            let result = s.utf16_index(agent, utf8_idx);
            Ok(Number::try_from(result).unwrap().into_value())
        } else {
            Ok(Number::from(-1).into_value())
        }
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

    /// ### [22.1.3.16 String.prototype.padEnd ( maxLength \[ , fillString \] )](https://tc39.es/ecma262/#sec-string.prototype.padend)
    fn pad_end(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let max_length = arguments.get(0);
        let fill_string = arguments.get(1);

        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;

        // 2. Return ? StringPaddingBuiltinsImpl(O, maxLength, fillString, end).
        string_padding_builtins_impl(agent, o, max_length, fill_string, false)
    }

    /// ### [22.1.3.17 String.prototype.padStart ( maxLength \[ , fillString \] )](https://tc39.es/ecma262/#sec-string.prototype.padstart)
    fn pad_start(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let max_length = arguments.get(0);
        let fill_string = arguments.get(1);

        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;

        // 2. Return ? StringPaddingBuiltinsImpl(O, maxLength, fillString, start).
        string_padding_builtins_impl(agent, o, max_length, fill_string, true)
    }

    /// ### [22.1.3.18 String.prototype.repeat ( count )](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.prototype.repeat)
    fn repeat(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let count = arguments.get(0);

        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;

        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;

        // 3. Let n be ? ToIntegerOrInfinity(count).
        let n = to_integer_or_infinity(agent, count)?;

        // 4. If n < 0 or n = +∞, throw a RangeError exception.
        if n.is_pos_infinity(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "count must be less than infinity",
            ));
        }

        let n = n.into_i64(agent);

        if n < 0 {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "count must not be negative",
            ));
        }

        // 5. If n = 0, return the empty String.
        if n == 0 || s.is_empty_string() {
            return Ok(String::EMPTY_STRING.into());
        }

        if n == 1 {
            return Ok(s.into_value());
        }

        // 6. Return the String value that is made from n copies of S appended together.
        Ok(Value::from_string(
            agent,
            s.as_str(agent).repeat(n as usize),
        ))
    }

    /// ### [22.1.3.19 String.prototype.replace ( searchValue, replaceValue )](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.prototype.replace)
    fn replace(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;

        let search_value = args.get(0);
        let replace_value = args.get(1);

        // 2. If searchValue is neither undefined nor null, then
        if !search_value.is_null() && !search_value.is_undefined() {
            // a. Let replacer be ? GetMethod(searchValue, %Symbol.replace%).
            let symbol = WellKnownSymbolIndexes::Replace.into();
            let replacer = get_method(agent, search_value, symbol)?;

            // b. If replacer is not undefined, Return ? Call(replacer, searchValue, « O, replaceValue »).
            if let Some(replacer) = replacer {
                return call_function(
                    agent,
                    replacer,
                    search_value,
                    Some(ArgumentsList(&[o, replace_value])),
                );
            }
        }

        // 3. Let s be ? ToString(O).
        let s = to_string(agent, o)?;

        // 4. Let searchString be ? ToString(searchValue).
        let search_string = to_string(agent, search_value)?;

        // 5. Let functionalReplace be IsCallable(replaceValue).
        if let Some(functional_replace) = is_callable(replace_value) {
            // 7. Let searchLength be the length of searchString.
            let search_length = search_string.len(agent);

            // 8. Let position be StringIndexOf(s, searchString, 0).
            let position = if let Some(position) = s.as_str(agent).find(search_string.as_str(agent))
            {
                position
            } else {
                // 9. If position is not-found, return s.
                return Ok(s.into_value());
            };

            // Let replacement be ? ToString(? Call(replaceValue, undefined, « searchString, 𝔽(position), string »)).
            let result = call_function(
                agent,
                functional_replace,
                Value::Undefined,
                Some(ArgumentsList(&[
                    search_string.into_value(),
                    Number::from(position as u32).into_value(),
                    s.into_value(),
                ])),
            )?;

            let result = to_string(agent, result)?;

            // 10. Let preceding be the substring of s from 0 to position.
            // 11. Let following be the substring of s from position + searchLength.
            // 12. If functionalReplace is true,
            let preceding = &s.as_str(agent)[0..position].to_owned();
            let following = &s.as_str(agent)[position + search_length..].to_owned();

            // 14. Return the string-concatenation of preceding, replacement, and following.
            let concatenated_result = format!("{}{}{}", preceding, result.as_str(agent), following);
            return Ok(String::from_string(agent, concatenated_result).into_value());
        }

        // 6. If functionalReplace is false, Set replaceValue to ? ToString(replaceValue).
        let replace_string = to_string(agent, replace_value)?;
        // Everything are strings: `"foo".replace("o", "a")` => use rust's replace
        let result =
            s.as_str(agent)
                .replacen(search_string.as_str(agent), replace_string.as_str(agent), 1);
        Ok(String::from_string(agent, result).into_value())
    }

    /// ### [22.1.3.20 String.prototype.replaceAll ( searchValue, replaceValue )](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.prototype.replaceall)
    fn replace_all(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;

        let search_value = args.get(0);
        let replace_value = args.get(1);

        // 2. If searchValue is neither undefined nor null, then
        if !search_value.is_null() && !search_value.is_undefined() {
            // a. Let isRegExp be ? IsRegExp(searchValue).
            let is_reg_exp = false;

            // b. If isRegExp is true, then
            if is_reg_exp {
                // i. Let flags be ? Get(searchValue, "flags").
                // ii. Perform ? RequireObjectCoercible(flags).
                // iii. If ? ToString(flags) does not contain "g", throw a TypeError exception.
                todo!();
            }

            // c. Let replacer be ? GetMethod(searchValue, %Symbol.replace%).
            let symbol = WellKnownSymbolIndexes::Replace.into();
            let replacer = get_method(agent, search_value, symbol)?;

            // d. If replacer is not undefined, Return ? Call(replacer, searchValue, « O, replaceValue »).
            if let Some(replacer) = replacer {
                return call_function(
                    agent,
                    replacer,
                    search_value,
                    Some(ArgumentsList(&[o, replace_value])),
                );
            }
        }

        // 3. Let s be ? ToString(O).
        let s = to_string(agent, o)?;

        // 4. Let searchString be ? ToString(searchValue).
        let search_string = to_string(agent, search_value)?;

        // 5. Let functionalReplace be IsCallable(replaceValue).
        if let Some(functional_replace) = is_callable(replace_value) {
            // 7. Let searchLength be the length of searchString.
            let search_length = search_string.len(agent);

            // 8. Let advanceBy be max(1, searchLength).
            let advance_by = max(1, search_length);

            // 9. Let matchPositions be a new empty List.
            let mut match_positions: Vec<usize> = vec![];

            // 10. Let position be StringIndexOf(s, searchString, 0).
            let search_str = search_string.as_str(agent);
            let subject = s.as_str(agent).to_owned();
            let mut position = 0;

            // 11. Repeat, while position is not not-found,
            while let Some(pos) = subject[position..].find(search_str) {
                match_positions.push(position + pos);
                position += advance_by + pos;
            }

            // If none has found, return s.
            if match_positions.is_empty() {
                return Ok(s.into_value());
            }

            // 12. Let endOfLastMatch be 0.
            let mut end_of_last_match = 0;

            // 13. Let result be the empty String.
            let mut result = std::string::String::with_capacity(subject.len());

            // 14. For each element p of matchPositions, do
            for p in match_positions {
                // b. let replacement be ? ToString(? Call(replaceValue, undefined, « searchString, 𝔽(p), string »)).
                let replacement = call_function(
                    agent,
                    functional_replace,
                    Value::Undefined,
                    Some(ArgumentsList(&[
                        search_string.into_value(),
                        Number::from(position as u32).into_value(),
                        s.into_value(),
                    ])),
                )?;

                // a. Let preserved be the substring of string from endOfLastMatch to p.
                let preserved = &subject[end_of_last_match..p];
                // d. Set result to the string-concatenation of result, preserved, and replacement.
                let replacement_str = replacement.to_string(agent)?;
                let replacement_str = replacement_str.as_str(agent);
                result.reserve(preserved.len() + replacement_str.len());
                result.push_str(preserved);
                result.push_str(replacement_str);
                end_of_last_match = p + search_length;
            }

            // 15. If endOfLastMatch < the length of string, set result to the string-concatenation of result and the substring of string from endOfLastMatch.
            if end_of_last_match < subject.len() {
                let preserved = &subject[end_of_last_match..];
                result.push_str(preserved);
            }

            // 16. Return result.
            return Ok(String::from_string(agent, result).into_value());
        }

        // 6. If functionalReplace is false, Set replaceValue to ? ToString(replaceValue).
        let replace_string = to_string(agent, replace_value)?;
        // Everything are strings: `"foo".replaceAll("o", "a")` => use rust's replace
        let result = s
            .as_str(agent)
            .replace(search_string.as_str(agent), replace_string.as_str(agent));
        Ok(String::from_string(agent, result).into_value())
    }

    fn search(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn slice(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;

        // 3. Let len be the length of S.
        // 4. Let intStart be ? ToIntegerOrInfinity(start).
        let int_start = to_integer_or_infinity(agent, args.get(0))?;
        // 5. If intStart = -∞, let from be 0.
        // NOTE: We use `None` when `from` would be `len` in the spec.
        let from = if int_start.is_neg_infinity(agent) {
            Some(0)
        } else if int_start.is_sign_negative(agent) && int_start.is_nonzero(agent) {
            // 6. Else if intStart < 0, let from be max(len + intStart, 0).
            let len = i64::try_from(s.utf16_len(agent)).unwrap();
            let int_start = int_start.into_i64(agent);
            Some(usize::try_from(len + int_start).unwrap_or(0))
        } else {
            // 7. Else, let from be min(intStart, len).
            let len = s.utf16_len(agent);
            let int_start = int_start.into_usize(agent);
            if int_start >= len {
                None
            } else {
                Some(int_start)
            }
        };

        // 8. If end is undefined, let intEnd be len; else let intEnd be ? ToIntegerOrInfinity(end).
        // NOTE: We use `None` when `to` would be `len` in the spec.
        let to = if args.get(1).is_undefined() {
            None
        } else {
            let int_end = to_integer_or_infinity(agent, args.get(1))?;
            // 9. If intEnd = -∞, let to be 0.
            if int_end.is_neg_infinity(agent) {
                Some(0)
            } else if int_end.is_sign_negative(agent) && int_end.is_nonzero(agent) {
                // 10. Else if intEnd < 0, let to be max(len + intEnd, 0).
                let len = i64::try_from(s.utf16_len(agent)).unwrap();
                let int_end = int_end.into_i64(agent);
                Some(usize::try_from(len + int_end).unwrap_or(0))
            } else {
                // 11. Else, let to be min(intEnd, len).
                let len = s.utf16_len(agent);
                let int_end = int_end.into_usize(agent);
                if int_end >= len {
                    None
                } else {
                    Some(int_end)
                }
            }
        };

        // 12. If from ≥ to, return the empty String.
        // 13. Return the substring of S from from to to.
        let substring = match (from, to) {
            (None, _) => "",
            (Some(0), None) => return Ok(s.into_value()),
            (Some(from_idx), None) => {
                let u8_from = s.utf8_index(agent, from_idx).unwrap();
                &s.as_str(agent)[u8_from..]
            }
            (Some(from_idx), Some(to_idx)) if from_idx >= to_idx => "",
            (Some(from_idx), Some(to_idx)) => {
                let u8_from = s.utf8_index(agent, from_idx).unwrap();
                let u8_to = s.utf8_index(agent, to_idx).unwrap();
                &s.as_str(agent)[u8_from..u8_to]
            }
        };
        // SAFETY: The memory for `substring` (and for the WTF-8 representation
        // of `s`) won't be moved or deallocated before this function returns.
        let substring: &'static str = unsafe { std::mem::transmute(substring) };
        Ok(String::from_str(agent, substring).into_value())
    }

    /// ### [22.1.3.23 String.prototype.split ( separator, limit )](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.prototype.split)
    fn split(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;

        // 2. If separator is neither undefined nor null, then
        let separator = args.get(0);

        if matches!(separator, Value::Undefined | Value::Null) {
            let symbol = WellKnownSymbolIndexes::Split.into();

            // If splitter is not undefined, then return ? Call(splitter, separator, « O, limit »).
            if let Ok(Some(splitter)) = get_method(agent, separator, symbol) {
                return call_function(
                    agent,
                    splitter,
                    separator,
                    Some(ArgumentsList(&[o, args.get(1)])),
                );
            }
        }

        // 3. Let S be ? ToString(O).
        let s = to_string(agent, o)?;

        let limit = args.get(1);
        let lim = match limit {
            // 4. If limit is undefined, lim is 2**32 - 1.
            Value::Undefined => u32::MAX,
            // else let lim be ℝ(? ToUint32(limit)).
            // Note: Fast path for integer parameter.
            Value::Integer(value) => value.into_i64() as u32,
            _ => to_uint32(agent, limit)?,
        };

        // 5. Let R be ? ToString(separator).
        let r = to_string(agent, separator)?;

        // 6. If lim is zero, return an empty array
        if lim == 0 {
            return Ok(create_array_from_list(agent, &[]).into_value());
        }

        // 7. If separator is undefined, return an array with the whole string
        if separator.is_undefined() {
            return Ok(create_array_from_list(agent, &[s.into_value()]).into_value());
        }

        // 8. Let separatorLength be the length of R.
        let separator_length = r.len(agent);

        // 9. If separatorLength = 0, the split by characters
        if separator_length == 0 {
            let subject = s.as_str(agent);
            let head = subject.split("");

            let mut results: Vec<Value> = head
                .enumerate()
                .skip(1) // Rust's split inserts an empty string in the beginning.
                .take_while(|(i, _)| *i <= lim as usize)
                .map(|(_, part)| SmallString::try_from(part).unwrap().into_value())
                .collect();

            // Remove the latest empty string if it's needed
            if results.len() < lim as usize {
                results.pop();
            }

            let results = Array::from_slice(agent, results.as_slice());
            return Ok(results.into_value());
        }

        // 10. If S is the empty String, return CreateArrayFromList(« S »).
        if s.is_empty_string() {
            let list: [Value; 1] = [s.into_value()];
            return Ok(create_array_from_list(agent, &list).into_value());
        }

        // 11-17. Normal split
        let subject = s.as_str(agent).to_owned();
        let separator = r.as_str(agent).to_owned();
        let head = subject.split(&separator);
        let mut results: Vec<Value> = Vec::new();

        for (i, part) in head.enumerate() {
            if lim as usize == i {
                break;
            }
            results.push(Value::from_str(agent, part));
        }

        let results = Array::from_slice(agent, results.as_slice());
        Ok(results.into_value())
    }

    fn starts_with(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;

        // 3. Let isRegExp be ? IsRegExp(searchString).
        // 4. If isRegExp is true, throw a TypeError exception.
        // TODO

        // 5. Let searchStr be ? ToString(searchString).
        let search_str = to_string(agent, args.get(0))?;

        // 6. Let len be the length of S.
        // 7. If position is undefined, let pos be 0; else let pos be ? ToIntegerOrInfinity(endPosition).
        // 8. Let start be the result of clamping pos between 0 and len.
        let position = args.get(1);
        let haystack_str = if position.is_undefined() {
            s.as_str(agent)
        } else {
            let pos = to_integer_or_infinity(agent, position)?;
            if pos.is_sign_negative(agent) || pos.is_pos_zero(agent) {
                s.as_str(agent)
            } else {
                let len = s.utf16_len(agent);
                let pos = pos.into_usize(agent);
                if pos >= len {
                    ""
                } else {
                    let u8_idx = s.utf8_index(agent, pos).unwrap();
                    &s.as_str(agent)[u8_idx..]
                }
            }
        };

        // 9. Let searchLength be the length of searchStr.
        // 10. If searchLength = 0, return true.
        // 11. Let start be end - searchLength.
        // 12. If start < 0, return false.
        // 13. Let substring be the substring of S from start to end.
        // 14. If substring is searchStr, return true.
        // 15. Return false.
        Ok(Value::from(
            haystack_str.starts_with(search_str.as_str(agent)),
        ))
    }

    fn substring(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;

        // 3. Let len be the length of S.
        // 4. Let intStart be ? ToIntegerOrInfinity(start).
        let int_start = to_integer_or_infinity(agent, args.get(0))?;
        // 5. If end is undefined, let intEnd be len; else let intEnd be ? ToIntegerOrInfinity(end).
        let int_end = if args.get(1).is_undefined() {
            None
        } else {
            Some(to_integer_or_infinity(agent, args.get(1))?)
        };

        // Fast path: can we return `s` without computing the UTF-16 length?
        // We can if int_start <= 0 and we know int_end must be >= len
        // (i.e. it's either None or +inf).
        if (int_start.is_sign_negative(agent) || !int_start.is_nonzero(agent))
            && (int_end.is_none() || int_end.unwrap().is_pos_infinity(agent))
        {
            return Ok(s.into_value());
        }

        let len = s.utf16_len(agent);
        // 6. Let finalStart be the result of clamping intStart between 0 and len.
        // NOTE: `into_usize` already clamps to 0.
        let final_start = int_start.into_usize(agent).min(len);
        // 7. Let finalEnd be the result of clamping intEnd between 0 and len.
        let final_end = if let Some(int_end) = int_end {
            // NOTE: `into_usize` already clamps to 0.
            int_end.into_usize(agent).min(len)
        } else {
            len
        };

        // 8. Let from be min(finalStart, finalEnd)
        let from = final_start.min(final_end);
        // 9. Let to be max(finalStart, finalEnd).
        let to = final_start.max(final_end);

        // 10. Return the substring of S from from to to.
        let u8_from = if from != len {
            s.utf8_index(agent, from).unwrap()
        } else {
            s.len(agent)
        };
        let u8_to = if to != len {
            s.utf8_index(agent, to).unwrap()
        } else {
            s.len(agent)
        };
        let substring = &s.as_str(agent)[u8_from..u8_to];
        // SAFETY: The memory for `substring` (and for the WTF-8 representation
        // of `s`) won't be moved or deallocated before this function returns.
        let substring: &'static str = unsafe { std::mem::transmute(substring) };
        Ok(String::from_str(agent, substring).into_value())
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

    /// > NOTE: The implementation might not reflect the spec.
    fn to_lower_case(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;

        // 3. Let sText be [StringToCodePoints](https://tc39.es/ecma262/#sec-stringtocodepoints)(S).
        // 4. Let lowerText be toLowercase(sText), according to the Unicode Default Case Conversion algorithm.
        // 5. Let L be [CodePointsToString](https://tc39.es/ecma262/#sec-codepointstostring)(lowerText).
        // 6. Return L.
        let lower_case_string = s.as_str(agent).to_lowercase();
        Ok(String::from_string(agent, lower_case_string).into_value())
    }

    /// > NOTE: The implementation might not reflect the spec.
    fn to_upper_case(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;

        // 3. Let sText be [StringToCodePoints](https://tc39.es/ecma262/#sec-stringtocodepoints)(S).
        // 4. Let upperText be toUppercase(sText), according to the Unicode Default Case Conversion algorithm.
        // 5. Let L be [CodePointsToString](https://tc39.es/ecma262/#sec-codepointstostring)(upperText).
        // 6. Return L.
        let upper_case_string = s.as_str(agent).to_uppercase();
        Ok(String::from_string(agent, upper_case_string).into_value())
    }

    fn to_well_formed(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;

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

        // TODO: For now, all strings are well-formed Unicode. In the future, `.as_str()` will
        // return None for WTF-8 strings.
        let _: &str = s.as_str(agent);
        Ok(s.into_value())
    }

    /// ### [22.1.3.32 String.prototype.trim ( )](https://tc39.es/ecma262/#sec-string.prototype.trim)
    fn trim(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let S be the this value.
        // 2. Return ? TrimString(S, start+end).
        Self::trim_string(agent, this_value, TrimWhere::StartAndEnd)
    }

    /// #### [22.1.3.32.1 String.prototype.trimString ( )](https://tc39.es/ecma262/#sec-trimstring)
    fn trim_string(agent: &mut Agent, value: Value, trim_where: TrimWhere) -> JsResult<Value> {
        // 1. Let str be ? RequireObjectCoercible(string).
        let str = require_object_coercible(agent, value)?;

        // 2. Let S be ? ToString(str)
        let s = to_string(agent, str)?;

        let s_str = s.as_str(agent);

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
            Ok(s.into_value())
        } else {
            let t = String::from_string(agent, t.to_string());
            Ok(t.into_value())
        }
    }

    /// ### [22.1.3.33 String.prototype.trimEnd ( )](https://tc39.es/ecma262/#sec-string.prototype.trimend)
    fn trim_end(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let S be the this value.
        // 2. Return ? TrimString(S, end).
        Self::trim_string(agent, this_value, TrimWhere::End)
    }

    /// ### [22.1.3.34 String.prototype.trimStart ( )](https://tc39.es/ecma262/#sec-string.prototype.trimstart)
    fn trim_start(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Let S be the this value.
        // 2. Return ? TrimString(S, start).
        Self::trim_string(agent, this_value, TrimWhere::Start)
    }

    /// ### [22.1.3.29 String.prototype.toString ( )](https://tc39.es/ecma262/#sec-string.prototype.tostring)
    /// ### [22.1.3.35 String.prototype.valueOf ( )](https://tc39.es/ecma262/#sec-string.prototype.valueof)
    ///
    /// > NOTE: `String.prototype.toString` and `String.prototype.valueOf` are
    /// > different functions but have the exact same steps.
    fn value_of(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        // 1. Return ? ThisStringValue(this value).
        this_string_value(agent, this_value).map(|string| string.into_value())
    }

    fn iterator(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.string_prototype();
        let this_base_object = intrinsics.string_prototype_base_object().into();
        let string_constructor = intrinsics.string();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this_base_object)
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
            .with_builtin_function_property::<StringPrototypeIterator>()
            .build();

        let slot = agent
            .heap
            .primitive_objects
            .get_mut(this.get_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(PrimitiveObjectHeapData {
            object_index: Some(this_base_object),
            data: PrimitiveObjectData::SmallString(SmallString::EMPTY),
        });
    }
}

/// ### [22.1.3.17.1 StringPaddingBuiltinsImpl ( O, maxLength, fillString, placement )](https://tc39.es/ecma262/#sec-stringpaddingbuiltinsimpl)
///
/// The abstract operation StringPaddingBuiltinsImpl takes arguments O (an
/// ECMAScript language value), maxLength (an ECMAScript language value),
/// fillString (an ECMAScript language value), and placement (start or end)
/// and returns either a normal completion containing a String or a throw
/// completion.
fn string_padding_builtins_impl(
    agent: &mut Agent,
    o: Value,
    max_length: Value,
    fill_string: Value,
    placement_start: bool,
) -> JsResult<Value> {
    // 1. Let S be ? ToString(O).
    let s = to_string(agent, o)?;

    // 2. Let intMaxLength be ℝ(? ToLength(maxLength)).
    let int_max_length = to_length(agent, max_length)?;

    // 3. Let stringLength be the length of S.
    let string_length = s.utf16_len(agent) as i64;

    // 4. If intMaxLength ≤ stringLength, return S.
    if int_max_length <= string_length {
        return Ok(s.into());
    }

    // 5. If fillString is undefined, set fillString to the String value consisting solely of the code unit 0x0020 (SPACE).
    let fill_string = if fill_string.is_undefined() {
        BUILTIN_STRING_MEMORY.r#__
    } else {
        // 6. Else, set fillString to ? ToString(fillString).
        to_string(agent, fill_string)?
    };

    // 7. Return StringPad(S, intMaxLength, fillString, placement).
    string_pad(agent, s, int_max_length, fill_string, placement_start)
}

/// ### [22.1.3.17.2 StringPad ( S, maxLength, fillString, placement )](https://tc39.es/ecma262/#sec-stringpad)
///
/// The abstract operation StringPad takes arguments S (a String),
/// maxLength (a non-negative integer), fillString (a String), and
/// placement (start or end) and returns a String.
fn string_pad(
    agent: &mut Agent,
    s: String,
    max_len: i64,
    fill_string: String,
    placement_start: bool,
) -> JsResult<Value> {
    // 1. Let stringLength be the length of S.
    let string_len = s.utf16_len(agent) as i64;

    // 2. If maxLength ≤ stringLength, return S.
    if max_len <= string_len {
        return Ok(s.into());
    }

    // 3. If fillString is the empty String, return S.
    if fill_string.is_empty_string() {
        return Ok(s.into());
    }

    // 4. Let fillLen be maxLength - stringLength.
    let fill_len = max_len - string_len;
    let fill_string_len = fill_string.utf16_len(agent) as i64;

    // 5. Let truncatedStringFiller be the String value consisting of repeated concatenations of fillString truncated to length fillLen.
    let mut strings = if fill_len == fill_string_len {
        let mut vec = VecDeque::with_capacity(2);
        vec.push_back(fill_string);
        vec
    } else if fill_len % fill_string_len == 0 {
        let fill_count = (fill_len / fill_string_len) as usize;
        let mut vec = VecDeque::with_capacity(fill_count + 1);
        vec.extend(repeat(fill_string).take(fill_count));
        vec
    } else if fill_len < fill_string_len {
        let mut vec = VecDeque::with_capacity(2);
        let mut sub_string = vec![0; fill_len as usize];
        for i in 0..fill_len {
            fill_string
                .utf16_char(agent, i as usize)
                .encode_utf8(&mut sub_string[fill_string.utf8_index(agent, i as usize).unwrap()..]);
        }
        let sub_string = std::str::from_utf8(&sub_string).unwrap();
        // let sub_string = &fill_string.as_str(agent)[..fill_len as usize];
        vec.push_back(String::from_string(agent, sub_string.to_owned()));
        vec
    } else {
        let fill_count = (fill_len / fill_string_len) as usize;
        let mut vec = VecDeque::with_capacity(fill_count + 2);
        vec.extend(repeat(fill_string).take(fill_count));
        let sub_string_len = (fill_len % fill_string_len) as usize;
        let mut sub_string = vec![0; sub_string_len];
        for i in 0..sub_string_len {
            fill_string
                .utf16_char(agent, i)
                .encode_utf8(&mut sub_string[fill_string.utf8_index(agent, i).unwrap()..]);
        }
        let sub_string = std::str::from_utf8(&sub_string).unwrap();
        vec.push_back(String::from_string(agent, sub_string.to_owned()));
        vec
    };

    // 6. If placement is start, return the string-concatenation of truncatedStringFiller and S.
    // 7. Else, return the string-concatenation of S and truncatedStringFiller.
    if placement_start {
        strings.push_back(s);
    } else {
        strings.push_front(s);
    }

    Ok(String::concat(agent, strings.into_iter().collect::<Vec<String>>()).into_value())
}

/// ### [22.1.3.35.1 ThisStringValue ( value )](https://tc39.es/ecma262/#sec-thisstringvalue)
///
/// The abstract operation ThisStringValue takes argument value (an ECMAScript
/// language value) and returns either a normal completion containing a String
/// or a throw completion.
fn this_string_value(agent: &mut Agent, value: Value) -> JsResult<String> {
    match value {
        // 1. If value is a String, return value.
        Value::String(data) => Ok(data.into()),
        Value::SmallString(data) => Ok(data.into()),
        // 2. If value is an Object and value has a [[StringData]] internal slot, then
        Value::PrimitiveObject(obj) if obj.is_string_object(agent) => {
            // a. Let s be value.[[StringData]].
            // b. Assert: s is a String.
            // c. Return s.
            match agent[obj].data {
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
            ))
        }
    }
}

enum TrimWhere {
    Start,
    End,
    StartAndEnd,
}
