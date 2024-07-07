use small_string::SmallString;

use crate::{
    ecmascript::{
        abstract_operations::{
            testing_and_comparison::require_object_coercible,
            type_conversion::{to_integer_or_infinity, to_number, to_string},
        },
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{
            primitive_objects::{PrimitiveObjectData, PrimitiveObjectHeapData},
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic,
        },
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{IntoValue, Number, String, Value, BUILTIN_STRING_MEMORY},
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
        let len = s.utf16_len(agent);
        // 4. Let relativeIndex be ? ToIntegerOrInfinity(pos).
        let relative_index = to_integer_or_infinity(agent, args.get(0))?.into_i64(agent);
        // 5. If relativeIndex ≥ 0, then
        let k = if relative_index >= 0 {
            // a. Let k be relativeIndex.
            relative_index
        } else {
            // 6. Else,
            //   a. Let k be len + relativeIndex.
            i64::try_from(len).unwrap() + relative_index
        };
        // 7. If k < 0 or k ≥ len, return undefined.
        if k < 0 || k >= i64::try_from(len).unwrap() {
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

    fn last_index_of(agent: &mut Agent, this_value: Value, args: ArgumentsList) -> JsResult<Value> {
        // 1. Let O be ? RequireObjectCoercible(this value).
        let o = require_object_coercible(agent, this_value)?;
        // 2. Let S be ? ToString(O).
        let s = to_string(agent, o)?;
        // 3. Let searchStr be ? ToString(searchString).
        let search_str = to_string(agent, args.get(0))?;
        // 4. Let numPos be ? ToNumber(position).
        let num_pos = to_number(agent, args.get(1))?;
        // 5. Assert: If position is undefined, then numPos is NaN.
        // 6. If numPos is NaN, let pos be +∞; otherwise, let pos be ! ToIntegerOrInfinity(numPos).
        let pos = if num_pos.is_nan(agent) {
            Number::pos_inf()
        } else {
            to_integer_or_infinity(agent, num_pos.into_value()).unwrap()
        };

        // 7. Let len be the length of S.
        // 8. Let searchLen be the length of searchStr.
        // 9. Let start be the result of clamping pos between 0 and len - searchLen.
        // 10. Let result be StringLastIndexOf(S, searchStr, start).
        // NOTE: If start >= (len - searchLen), there is a last index if and
        // only if start.endsWith(searchLen).
        let haystack_str = {
            let pos = pos.into_usize(agent);
            if pos == 0 {
                s.as_str(agent)
            } else {
                let utf16_len = s.utf16_len(agent);
                if pos >= utf16_len {
                    ""
                } else {
                    let utf8_pos = s.utf8_index(agent, pos).unwrap();
                    &s.as_str(agent)[utf8_pos..]
                }
            }
        };
        let search_str = search_str.as_str(agent);
        let utf8_result = if haystack_str.len() <= search_str.len() {
            if haystack_str.ends_with(search_str) {
                Some(haystack_str.len() - search_str.len())
            } else {
                None
            }
        } else {
            haystack_str.rfind(search_str)
        };

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

    fn split(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
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
        let final_start = int_start.into_usize(agent).min(len);
        // 7. Let finalEnd be the result of clamping intEnd between 0 and len.
        let final_end = if let Some(int_end) = int_end {
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

    fn to_lower_case(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_upper_case(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
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

    fn trim(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn trim_end(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn trim_start(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
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
            Err(agent.throw_exception(ExceptionType::TypeError, "Not a string value"))
        }
    }
}
