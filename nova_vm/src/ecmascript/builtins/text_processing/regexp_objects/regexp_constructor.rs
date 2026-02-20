// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use wtf8::CodePoint;

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinGetter,
        BuiltinIntrinsicConstructor, ExceptionType, Function, JsResult, Object, PropertyKey, Realm,
        String, Value, builders::BuiltinFunctionBuilder, get, is_reg_exp, reg_exp_alloc,
        reg_exp_initialize,
    },
    engine::{Bindable, GcScope, Scopable},
    heap::{IntrinsicConstructorIndexes, WellKnownSymbols},
};

pub(crate) struct RegExpConstructor;

impl Builtin for RegExpConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
    const LENGTH: u8 = 2;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.RegExp;
}
impl BuiltinIntrinsicConstructor for RegExpConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::RegExp;
}

struct RegExpEscape;
impl Builtin for RegExpEscape {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.escape;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpConstructor::escape);
}

struct RegExpGetSpecies;
impl Builtin for RegExpGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbols::Species.to_property_key());
}
impl BuiltinGetter for RegExpGetSpecies {}

impl RegExpConstructor {
    /// ### [22.2.4.1 RegExp ( pattern, flags )](https://tc39.es/ecma262/#sec-regexp-pattern-flags)
    ///
    /// > NOTE: If pattern is supplied using a StringLiteral, the usual escape
    /// > sequence substitutions are performed before the String is processed
    /// > by this function. If pattern must contain an escape sequence to be
    /// > recognized by this function, any U+005C (REVERSE SOLIDUS) code points
    /// > must be escaped within the StringLiteral to prevent them being
    /// > removed when the contents of the StringLiteral are formed.
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let pattern = arguments.get(0).bind(gc.nogc());
        let flags = arguments.get(1).bind(gc.nogc());
        let scoped_pattern = pattern.scope(agent, gc.nogc());
        let scoped_flags = flags.scope(agent, gc.nogc());
        let flags_is_undefined = flags.is_undefined();
        // 1. Let patternIsRegExp be ? IsRegExp(pattern).
        let pattern_is_reg_exp = is_reg_exp(agent, pattern.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 2. If NewTarget is undefined, then
        let new_target = if new_target.is_none() {
            // a. Let newTarget be the active function object.
            let new_target = agent.active_function_object(gc.nogc());
            // b. If patternIsRegExp is true and flags is undefined, then
            if pattern_is_reg_exp && flags_is_undefined {
                let new_target = new_target.scope(agent, gc.nogc());
                // i. Let patternConstructor be ? Get(pattern, "constructor").
                let pattern_constructor = get(
                    agent,
                    Object::try_from(scoped_pattern.get(agent)).unwrap(),
                    BUILTIN_STRING_MEMORY.constructor.into(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // SAFETY: not shared.
                let new_target = unsafe { new_target.take(agent) }.bind(gc.nogc());
                // ii. If SameValue(newTarget, patternConstructor) is true, return pattern.
                if Value::from(new_target) == pattern_constructor {
                    return Ok(scoped_pattern.get(agent));
                }
                new_target.into()
            } else {
                new_target.into()
            }
        } else {
            // 3. Else,
            // a. Let newTarget be NewTarget.
            // SAFETY: checked above.
            unsafe { new_target.unwrap_unchecked() }
        };
        let new_target = new_target.scope(agent, gc.nogc());
        let pattern = scoped_pattern.get(agent).bind(gc.nogc());
        // 4. If pattern is an Object and pattern has a [[RegExpMatcher]] internal slot, then
        let (p, f) = if let Value::RegExp(pattern) = pattern {
            // a. Let P be pattern.[[OriginalSource]].
            let p = pattern.original_source(agent);
            // b. If flags is undefined, let F be pattern.[[OriginalFlags]].
            let f = if flags_is_undefined {
                Ok(pattern.original_flags(agent))
            } else {
                // c. Else, let F be flags.
                Err(scoped_flags)
            };
            (p.into(), f)
        } else if pattern_is_reg_exp {
            // 5. Else if patternIsRegExp is true, then
            // a. Let P be ? Get(pattern, "source").
            let mut p = get(
                agent,
                Object::try_from(pattern).unwrap().unbind(),
                BUILTIN_STRING_MEMORY.source.into(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // b. If flags is undefined, then
            let f = if flags_is_undefined {
                let scoped_p = p.scope(agent, gc.nogc());
                // i. Let F be ? Get(pattern, "flags").
                let f = get(
                    agent,
                    // SAFETY: not shared.
                    Object::try_from(unsafe { scoped_pattern.take(agent) }).unwrap(),
                    BUILTIN_STRING_MEMORY.flags.into(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // SAFETY: not shared
                p = unsafe { scoped_p.take(agent) }.bind(gc.nogc());
                f.scope(agent, gc.nogc())
            } else {
                // c. Else,
                // i. Let F be flags.
                scoped_flags
            };
            (p, Err(f))
        } else {
            // 6. Else,
            // a. Let P be pattern.
            let p = pattern;
            // b. Let F be flags.
            // SAFETY: not shared
            (p, Err(scoped_flags))
        };
        let p = p.scope(agent, gc.nogc());
        // 7. Let O be ? RegExpAlloc(newTarget).
        let o = reg_exp_alloc(
            agent,
            // SAFETY: not shared.
            Function::try_from(unsafe { new_target.take(agent) })
                .expect("Proxy constructors not yet supported"),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 8. Return ? RegExpInitialize(O, P, F).
        reg_exp_initialize(agent, o.unbind(), p, f, gc).map(|o| o.into())
    }

    /// ### [22.2.5.1 RegExp.escape ( S )](https://tc39.es/ecma262/#sec-regexp.escape)
    ///
    /// This function returns a copy of S in which characters that are
    /// potentially special in a regular expression Pattern have been replaced
    /// by equivalent escape sequences.
    ///
    /// > NOTE: Despite having similar names, `EscapeRegExpPattern` and
    /// > `RegExp.escape` do not perform similar actions. The former escapes a
    /// > pattern for representation as a string, while this function escapes a
    /// > string for representation inside a pattern.
    fn escape<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let s = args.get(0).bind(gc);
        // 1. If S is not a String, throw a TypeError exception.
        let Ok(s) = String::try_from(s) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "expected string",
                gc,
            ));
        };
        // 2. Let escaped be the empty String.
        let mut escaped = std::string::String::with_capacity(s.len_(agent));
        // 3. Let cpList be StringToCodePoints(S).
        let mut cp_list = s.as_wtf8_(agent).code_points();
        // 4. For each code point cp of cpList, do
        let Some(first_cp) = cp_list.next() else {
            return Ok(String::EMPTY_STRING.into());
        };
        // a. If escaped is the empty String
        if let Some(cp) = first_cp.to_char()
            && cp.is_ascii_alphanumeric()
        {
            // and cp is matched by either DecimalDigit or AsciiLetter, then
            // i. NOTE: Escaping a leading digit ensures that output
            //    corresponds with pattern text which may be used after a \0
            //    character escape or a DecimalEscape such as \1 and still
            //    match S rather than be interpreted as an extension of the
            //    preceding escape sequence. Escaping a leading ASCII letter
            //    does the same for the context after \c.
            // ii. Let numericValue be the numeric value of cp.
            let cp = cp as u8;
            // iii. Let hex be Number::toString(𝔽(numericValue), 16).
            // iv. Assert: The length of hex is 2.
            // v. Set escaped to the string-concatenation of the code unit
            //    0x005C (REVERSE SOLIDUS), "x", and hex.
            escaped.push('\\');
            escaped.push('x');
            let upper = cp / 16;
            let lower = cp % 16;

            push_hex(&mut escaped, upper);
            push_hex(&mut escaped, lower);
        } else {
            // b. Else,
            encode_for_reg_exp_escape(&mut escaped, first_cp);
        }
        // 4. For each code point cp of cpList, do
        for cp in cp_list {
            // i. Set escaped to the string-concatenation of escaped and EncodeForRegExpEscape(cp).
            encode_for_reg_exp_escape(&mut escaped, cp);
        }
        // 5. Return escaped.
        Ok(String::from_string(agent, escaped, gc).into())
    }

    fn get_species<'gc>(
        _: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Ok(this_value.unbind())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let regexp_prototype = intrinsics.reg_exp_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<RegExpConstructor>(agent, realm)
            .with_property_capacity(3)
            .with_prototype_property(regexp_prototype.into())
            .with_builtin_function_property::<RegExpEscape>()
            .with_builtin_function_getter_property::<RegExpGetSpecies>()
            .build();
    }
}

/// ### [22.2.5.1.1 EncodeForRegExpEscape ( cp )](https://tc39.es/ecma262/#sec-encodeforregexpescape)
///
/// The abstract operation EncodeForRegExpEscape takes argument cp (a code
/// point) and returns a String. It returns a String representing a Pattern for
/// matching cp. If cp is white space or an ASCII punctuator, the returned
/// value is an escape sequence. Otherwise, the returned value is a String
/// representation of cp itself.
fn encode_for_reg_exp_escape(s: &mut std::string::String, cp: CodePoint) {
    if let Some(cp) = cp.to_char() {
        match cp {
            // 1. If cp is matched by SyntaxCharacter or cp is U+002F
            //    (SOLIDUS), then
            '^' | '$' | '\\' | '/' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}'
            | '|' => {
                // a. Return the string-concatenation of 0x005C (REVERSE
                //    SOLIDUS) and UTF16EncodeCodePoint(cp).
                s.push('\\');
                s.push(cp);
                return;
            }
            // 2. Else if cp is a code point listed in the “Code Point” column
            //    of Table 65, then
            // a. Return the string-concatenation of 0x005C (REVERSE SOLIDUS)
            //    and the string in the “ControlEscape” column of the row whose
            //    “Code Point” column contains cp.
            '\t' => {
                s.push_str("\\t");
                return;
            }
            '\n' => {
                s.push_str("\\n");
                return;
            }
            '\u{000B}' => {
                s.push_str("\\v");
                return;
            }
            '\u{000C}' => {
                s.push_str("\\f");
                return;
            }
            '\r' => {
                s.push_str("\\r");
                return;
            }
            // 3. Let otherPunctuators be the string-concatenation of
            //    ",-=<>#&!%:;@~'`" and the code unit 0x0022 (QUOTATION MARK).
            // 4. Let toEscape be StringToCodePoints(otherPunctuators).
            // 5. If toEscape contains cp, cp is matched by either WhiteSpace
            //    or LineTerminator, or cp has the same numeric value as a
            //    leading surrogate or trailing surrogate, then
            // ### otherPunctuators
            ',' | '-' | '=' | '<' | '>' | '#' | '&' | '!' | '%' | ':' | ';' | '@' | '~' | '\''
            | '`' | '"' => {
                // fallthrough to "then"
            }
            // ### WhiteSpace
            _ if cp.is_whitespace() || cp == '\u{feff}' => {
                // fallthrough to "then"
            }
            // ### LineTerminator
            '\u{2028}' | '\u{2029}' => {
                // fallthrough to "then"
            }
            _ => {
                // 6. Return UTF16EncodeCodePoint(cp).
                s.push(cp);
                return;
            }
        }
        // a. Let cpNum be the numeric value of cp.
        let cp_num = cp as u32;
        // b. If cpNum ≤ 0xFF, then
        if cp_num <= 0xFF {
            let cp_num = cp_num as u8;
            // i. Let hex be Number::toString(𝔽(cpNum), 16).
            // ii. Return the string-concatenation of the code unit 0x005C (REVERSE
            //     SOLIDUS), "x", and StringPad(hex, 2, "0", start).
            let upper = cp_num / 16;
            let lower = cp_num % 16;
            s.push_str("\\x");
            push_hex(s, upper);
            push_hex(s, lower);
            return;
        }
        // c. Let escaped be the empty String.
        // d. Let codeUnits be UTF16EncodeCodePoint(cp).
        let mut scratch = [0u16; 2];
        let code_units = cp.encode_utf16(&mut scratch);
        // e. For each code unit cu of codeUnits, do
        for cu in code_units {
            // i. Set escaped to the string-concatenation of escaped and UnicodeEscape(cu).
            unicode_escape(s, *cu);
        }
        // f. Return escaped.
    } else {
        // Lone surrogate!
        unicode_escape(s, cp.to_u32() as u16);
    }
}

/// ### [25.5.2.4 UnicodeEscape ( C )](https://tc39.es/ecma262/#sec-unicodeescape)
///
/// The abstract operation UnicodeEscape takes argument C (a code unit) and
/// returns a String. It represents C as a Unicode escape sequence.
fn unicode_escape(s: &mut std::string::String, n: u16) {
    // 1. Let n be the numeric value of C.
    // 2. Assert: n ≤ 0xFFFF.
    // 3. Let hex be the String representation of n, formatted as a lowercase
    //    hexadecimal number.
    // 4. Return the string-concatenation of the code unit 0x005C (REVERSE
    //    SOLIDUS), "u", and StringPad(hex, 4, "0", start).
    s.push_str("\\u");
    let h0 = (n >> 12) & 0xF;
    let h1 = (n >> 8) & 0xF;
    let h2 = (n >> 4) & 0xF;
    let h3 = n & 0xF;
    push_hex(s, h0 as u8);
    push_hex(s, h1 as u8);
    push_hex(s, h2 as u8);
    push_hex(s, h3 as u8);
}

fn push_hex(s: &mut std::string::String, hex_half: u8) {
    match hex_half {
        0 => s.push('0'),
        1 => s.push('1'),
        2 => s.push('2'),
        3 => s.push('3'),
        4 => s.push('4'),
        5 => s.push('5'),
        6 => s.push('6'),
        7 => s.push('7'),
        8 => s.push('8'),
        9 => s.push('9'),
        10 => s.push('a'),
        11 => s.push('b'),
        12 => s.push('c'),
        13 => s.push('d'),
        14 => s.push('e'),
        15 => s.push('f'),
        _ => unreachable!(),
    }
}
