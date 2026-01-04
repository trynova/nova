// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::type_conversion::to_integer_or_infinity,
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin,
            primitive_objects::{PrimitiveObject, PrimitiveObjectData, PrimitiveObjectRecord},
        },
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, Number, String, Value},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
};

pub(crate) struct NumberPrototype;

struct NumberPrototypeToExponential;
impl Builtin for NumberPrototypeToExponential {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toExponential;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(NumberPrototype::to_exponential);
}

struct NumberPrototypeToFixed;
impl Builtin for NumberPrototypeToFixed {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toFixed;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(NumberPrototype::to_fixed);
}

struct NumberPrototypeToLocaleString;
impl Builtin for NumberPrototypeToLocaleString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(NumberPrototype::to_locale_string);
}

struct NumberPrototypeToPrecision;
impl Builtin for NumberPrototypeToPrecision {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toPrecision;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(NumberPrototype::to_precision);
}

struct NumberPrototypeToString;
impl Builtin for NumberPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(NumberPrototype::to_string);
}

struct NumberPrototypeValueOf;
impl Builtin for NumberPrototypeValueOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.valueOf;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(NumberPrototype::value_of);
}

impl NumberPrototype {
    fn to_exponential<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let fraction_digits = arguments.get(0).bind(nogc);
        // Let x be ? ThisNumberValue(this value).
        let fraction_digits_is_undefined = fraction_digits.is_undefined();
        let x = this_number_value(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let f be ? ToIntegerOrInfinity(fractionDigits).
        let f = to_integer_or_infinity(agent, fraction_digits.unbind(), gc.reborrow()).unbind()?;
        // No GC can happen after this point.
        let gc = gc.into_nogc();
        let x = x.get(agent).bind(gc);

        // 3. Assert: If fractionDigits is undefined, then f is 0.
        debug_assert!(!fraction_digits_is_undefined || f.into_i64() == 0);
        // 4. If x is not finite, return Number::toString(x, 10).
        if !x.is_finite(agent) {
            return Ok(Number::to_string_radix_10(agent, x, gc).into());
        }
        let f = f.into_i64();
        // 5. If f < 0 or f > 100, throw a RangeError exception.
        if !(0..=100).contains(&f) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Fraction digits count out of range",
                gc,
            ));
        }
        let f = f as usize;

        // 6. Set x to ℝ(x).
        let mut x = x.into_f64(agent);
        // This gets rid of -0.0
        if x == 0.0 {
            x = 0.0;
        };
        if f == 0 {
            Ok(f64_to_exponential(agent, x, gc).into())
        } else {
            Ok(f64_to_exponential_with_precision(agent, x, f, gc).into())
        }
    }

    /// ### [21.1.3.3 Number.prototype.toFixed ( fractionDigits )](https://tc39.es/ecma262/#sec-number.prototype.tofixed)
    ///
    /// > NOTE 1: This method returns a String containing this Number value
    /// > represented in decimal fixed-point notation with fractionDigits
    /// > digits after the decimal point. If fractionDigits is undefined, 0 is
    /// > assumed.
    ///
    /// > NOTE 2: The output of toFixed may be more precise than toString for
    /// > some values because toString only prints enough significant digits to
    /// > distinguish the number from adjacent Number values. For example,
    /// > `(1000000000000000128).toString()` returns `"1000000000000000100"`,
    /// > while `(1000000000000000128).toFixed(0)` returns
    /// > `"1000000000000000128"`.
    fn to_fixed<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let fraction_digits = arguments.get(0).bind(nogc);
        // Let x be ? ThisNumberValue(this value).
        let x = this_number_value(agent, this_value, nogc)
            .unbind()?
            .scope(agent, nogc);
        // 2. Let f be ? ToIntegerOrInfinity(fractionDigits).
        let fraction_digits_is_undefined = fraction_digits.is_undefined();
        let f = to_integer_or_infinity(agent, fraction_digits.unbind(), gc.reborrow()).unbind()?;
        // No GC is possible after this point.
        let gc = gc.into_nogc();
        let x = x.get(agent).bind(gc);
        // 3. Assert: If fractionDigits is undefined, then f is 0.
        debug_assert!(!fraction_digits_is_undefined || f.into_i64() == 0);
        // 4. If f is not finite, throw a RangeError exception.
        if !f.is_finite() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Fraction digits count out of range",
                gc,
            ));
        }
        let f = f.into_i64();
        // 5. If f < 0 or f > 100, throw a RangeError exception.
        if !(0..=100).contains(&f) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Fraction digits count out of range",
                gc,
            ));
        }
        // 6. If x is not finite, return Number::toString(x, 10).
        if !x.is_finite(agent) {
            return Ok(Number::to_string_radix_10(agent, x, gc).into());
        }
        // 7. Set x to ℝ(x).
        let x = x.into_f64(agent);
        let mut buffer = ryu_js::Buffer::new();
        let string = buffer.format_to_fixed(x, f as u8);
        Ok(Value::from_str(agent, string, gc))
    }

    /// ### [21.1.3.4 Number.prototype.toLocaleString ( \[ reserved1 \[ , reserved2 \] \] )](https://tc39.es/ecma262/#sec-number.prototype.tolocalestring)
    fn to_locale_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Self::to_string(agent, this_value, arguments, gc)
    }

    /// ### [21.1.3.5 Number.prototype.toPrecision ( precision )](https://tc39.es/ecma262/#sec-number.prototype.toprecision)
    ///
    /// Copied from Boa JS engine. Source https://github.com/boa-dev/boa/blob/6f1d7d11ce49040eafe54e5ff2da379be4d998c2/core/engine/src/builtins/number/mod.rs#L412
    ///
    /// Copyright (c) 2019 Jason Williams
    fn to_precision<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let precision = arguments.get(0).bind(nogc);

        // 1. Let x be ? ThisNumberValue(this value).
        let x = this_number_value(agent, this_value, nogc)
            .unbind()?
            .bind(nogc);

        // 2. If precision is undefined, return ! ToString(x).
        if precision.is_undefined() {
            // Skip: We know ToString calls Number::toString(argument, 10).
            // Note: That is not `Number.prototype.toString`, but the abstract
            // operation Number::toString.
            return Ok(Number::to_string_radix_10(agent, x.unbind(), gc.into_nogc()).into());
        }

        let x = x.scope(agent, nogc);

        // 3. Let p be ? ToIntegerOrInfinity(precision).
        let p = to_integer_or_infinity(agent, precision.unbind(), gc.reborrow()).unbind()?;
        // No GC can occur after this point.
        let gc = gc.into_nogc();

        let x = x.get(agent).bind(gc);

        // 4. If x is not finite, return Number::toString(x, 10).
        if !x.is_finite(agent) {
            return Ok(Number::to_string_radix_10(agent, x, gc).into());
        }

        // 5. If p < 1 or p > 100, throw a RangeError exception.
        let precision = p.into_i64();
        if !(1..=100).contains(&precision) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Precision out of range",
                gc,
            ));
        }
        let precision = precision as u8;

        // 6. Set x to ℝ(x).
        let mut x_f64 = x.into_f64(agent);

        // 7. Let s be the empty String.
        let mut s = std::string::String::new();
        let mut m: std::string::String;
        let mut e: i32;

        // 8. If x < 0, then
        if x_f64 < 0. {
            // a. Set s to the code unit 0x002D (HYPHEN-MINUS).
            s.push('-');
            // b. Set x to -x.
            x_f64 = -x_f64;
        }

        // 9. If x = 0, then
        if x_f64 == 0. {
            // a. Let m be the String value consisting of p occurrences of the
            // code unit 0x0030 (DIGIT ZERO).
            m = "0".repeat(precision as usize);
            // b. Let e be 0.
            e = 0;
        } else {
            // 10. Else,

            // Due to f64 limitations, this part differs a bit from the spec,
            // but has the same effect. It manipulates the string constructed
            // by `format`: digits with an optional dot between two of them.
            m = format!("{x_f64:.100}");

            // a: getting an exponent
            e = Self::flt_str_to_exp(&m);

            // b: getting relevant digits only
            if e < 0 {
                m = m.split_off((1 - e) as usize);
            } else if let Some(n) = m.find('.') {
                m.remove(n);
            }

            // impl: having exactly `precision` digits in `suffix`
            if Self::round_to_precision(&mut m, precision as usize) {
                e += 1;
            }

            // c. If e < -6 or e ≥ p, then
            // Note: This is switching to scientific notation.
            if e < -6 || e >= precision as i32 {
                // i. Assert: e ≠ 0.
                assert_ne!(e, 0);

                // ii. If p ≠ 1, then
                //     1. Let a be the first code unit of m.
                //     2. Let b be the other p - 1 code units of m.
                //     3. Set m to the string-concatenation of a, ".", and b.
                if precision > 1 {
                    m.insert(1, '.');
                }

                // vi. Return the string-concatenation of s, m, the code unit
                // 0x0065 (LATIN SMALL LETTER E), c, and d.
                m.push('e');

                // iii. If e > 0, then
                if e >= precision as i32 {
                    // 1. Let c be the code unit 0x002B (PLUS SIGN).
                    m.push('+');
                }

                // iv. Else,
                //     1. Assert: e < 0.
                //     2. Let c be the code unit 0x002D (HYPHEN-MINUS).
                //     3. Set e to -e.
                // v. Let d be the String value consisting of the digits of
                // the decimal representation of e (in order, with no leading
                // zeroes).
                m.push_str(&e.to_string());

                return Ok(String::from_string(agent, s + &m, gc).into());
            }
        }

        // 11. If e = p - 1, return the string-concatenation of s and m.
        let e_inc = e + 1;
        if e_inc == precision as i32 {
            return Ok(String::from_string(agent, s + &m, gc).into());
        }

        // 12. If e ≥ 0, then
        if e >= 0 {
            // a. Set m to the string-concatenation of the first e + 1 code
            // units of m, the code unit 0x002E (FULL STOP), and the remaining
            // p - (e + 1) code units of m.
            m.insert(e_inc as usize, '.');
        } else {
            // 13. Else,
            // a. Set m to the string-concatenation of the code unit 0x0030
            // (DIGIT ZERO), the code unit 0x002E (FULL STOP), -(e + 1)
            // occurrences of the code unit 0x0030 (DIGIT ZERO), and the String
            // m.
            s.push('0');
            s.push('.');
            s.push_str(&"0".repeat(-e_inc as usize));
        }

        // 14. Return the string-concatenation of s and m.
        Ok(String::from_string(agent, s + &m, gc).into())
    }

    /// round_to_precision - used in to_precision
    ///
    /// This procedure has two roles:
    /// - If there are enough or more than enough digits in the
    ///   string to show the required precision, the number
    ///   represented by these digits is rounded using string
    ///   manipulation.
    /// - Else, zeroes are appended to the string.
    /// - Additionally, sometimes the exponent was wrongly computed and
    ///   while up-rounding we find that we need an extra digit. When this
    ///   happens, we return true so that the calling context can adjust
    ///   the exponent. The string is kept at an exact length of precision.
    ///
    /// When this procedure returns, digits is exactly precision long.
    ///
    /// Copied from Boa JS engine. Source https://github.com/boa-dev/boa/blob/6f1d7d11ce49040eafe54e5ff2da379be4d998c2/core/engine/src/builtins/number/mod.rs#L351
    ///
    /// Copyright (c) 2019 Jason Williams
    fn round_to_precision(digits: &mut std::string::String, precision: usize) -> bool {
        if digits.len() > precision {
            let to_round = digits.split_off(precision);
            let mut digit = digits
                .pop()
                .expect("already checked that length is bigger than precision")
                as u8;
            if let Some(first) = to_round.chars().next()
                && first > '4'
            {
                digit += 1;
            }

            if digit as char == ':' {
                // ':' is '9' + 1
                // need to propagate the increment backward
                let mut replacement = std::string::String::from("0");
                let mut propagated = false;
                for c in digits.chars().rev() {
                    let d = match (c, propagated) {
                        ('0'..='8', false) => (c as u8 + 1) as char,
                        (_, false) => '0',
                        (_, true) => c,
                    };
                    replacement.push(d);
                    if d != '0' {
                        propagated = true;
                    }
                }
                digits.clear();
                let replacement = if propagated {
                    replacement.as_str()
                } else {
                    digits.push('1');
                    &replacement.as_str()[1..]
                };
                for c in replacement.chars().rev() {
                    digits.push(c);
                }
                !propagated
            } else {
                digits.push(digit as char);
                false
            }
        } else {
            digits.push_str(&"0".repeat(precision - digits.len()));
            false
        }
    }

    /// Copied from Boa JS engine. Source https://github.com/boa-dev/boa/blob/6f1d7d11ce49040eafe54e5ff2da379be4d998c2/core/engine/src/builtins/number/mod.rs#L318
    ///
    /// Copyright (c) 2019 Jason Williams
    fn flt_str_to_exp(flt: &str) -> i32 {
        let mut non_zero_encountered = false;
        let mut dot_encountered = false;
        for (i, c) in flt.chars().enumerate() {
            if c == '.' {
                if non_zero_encountered {
                    return (i as i32) - 1;
                }
                dot_encountered = true;
            } else if c != '0' {
                if dot_encountered {
                    return 1 - (i as i32);
                }
                non_zero_encountered = true;
            }
        }
        (flt.len() as i32) - 1
    }

    /// ### [21.1.3.6 Number.prototype.toString ( \[ radix \] )](https://tc39.es/ecma262/#sec-number.prototype.tostring)
    ///
    /// > NOTE: The optional radix should be an integral Number value in the
    /// > inclusive interval from 2𝔽 to 36𝔽. If radix is undefined then 10𝔽 is
    /// > used as the value of radix.
    fn to_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let x be ? ThisNumberValue(this value).
        let x = this_number_value(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        let radix = arguments.get(0).bind(gc.nogc());
        // 2. If radix is undefined, let radixMV be 10.
        if radix.is_undefined() || radix == Value::from(10u8) {
            // 5. Return Number::toString(x, 10).
            Ok(Number::to_string_radix_10(agent, x.unbind(), gc.nogc())
                .unbind()
                .into())
        } else {
            let x = x.scope(agent, gc.nogc());
            // 3. Else, let radixMV be ? ToIntegerOrInfinity(radix).
            let radix = to_integer_or_infinity(agent, radix.unbind(), gc.reborrow()).unbind()?;
            let gc = gc.into_nogc();
            // 4. If radixMV is not in the inclusive interval from 2 to 36, throw a RangeError exception.
            if !(2..=36).contains(&radix) {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "radix must be an integer at least 2 and no greater than 36",
                    gc,
                ));
            }
            let radix = radix.into_i64() as u32;
            // 5. Return Number::toString(x, radixMV).
            Ok(Number::to_string_radix_n(agent, x.get(agent), radix, gc).into())
        }
    }

    /// ### [21.1.3.7 Number.prototype.valueOf ( )](https://tc39.es/ecma262/#sec-number.prototype.valueof)
    fn value_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Return ? ThisNumberValue(this value).
        this_number_value(agent, this_value, gc.into_nogc()).map(|result| result.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.number_prototype();
        let this_base_object = intrinsics.number_prototype_backing_object();
        let number_constructor = intrinsics.number();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this_base_object)
            .with_property_capacity(7)
            .with_prototype(object_prototype)
            .with_constructor_property(number_constructor)
            .with_builtin_function_property::<NumberPrototypeToExponential>()
            .with_builtin_function_property::<NumberPrototypeToFixed>()
            .with_builtin_function_property::<NumberPrototypeToLocaleString>()
            .with_builtin_function_property::<NumberPrototypeToPrecision>()
            .with_builtin_function_property::<NumberPrototypeToString>()
            .with_builtin_function_property::<NumberPrototypeValueOf>()
            .build();

        let slot = agent
            .heap
            .primitive_objects
            .get_mut(this.get_index())
            .unwrap();
        *slot = PrimitiveObjectRecord {
            object_index: Some(this_base_object),
            data: PrimitiveObjectData::Integer(SmallInteger::zero()),
        };
    }
}

fn f64_to_exponential<'a>(agent: &mut Agent, x: f64, gc: NoGcScope<'a, '_>) -> String<'a> {
    match x.abs() {
        x if x >= 1.0 || x == 0.0 => {
            String::from_string(agent, format!("{x:e}").replace('e', "e+"), gc)
        }
        _ => String::from_string(agent, format!("{x:e}"), gc),
    }
}

fn f64_to_exponential_with_precision<'a>(
    agent: &mut Agent,
    x: f64,
    f: usize,
    gc: NoGcScope<'a, '_>,
) -> String<'a> {
    let mut res = format!("{x:.f$e}");
    let idx = res.find('e').unwrap();
    if res.as_bytes()[idx + 1] != b'-' {
        res.insert(idx + 1, '+');
    }
    String::from_string(agent, res, gc)
}

/// ### [21.1.3.7.1 ThisNumberValue ( value )](https://tc39.es/ecma262/#sec-thisnumbervalue)
///
/// The abstract operation ThisNumberValue takes argument value (an ECMAScript
/// language value) and returns either a normal completion containing a Number
/// or a throw completion.
#[inline(always)]
fn this_number_value<'gc>(
    agent: &mut Agent,
    value: Value,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, Number<'gc>> {
    // 1. If value is a Number, return value.
    if let Ok(value) = Number::try_from(value) {
        return Ok(value.bind(gc));
    }
    // 2. If value is an Object and value has a [[NumberData]] internal slot, then
    if let Ok(value) = PrimitiveObject::try_from(value)
        && value.is_number_object(agent)
    {
        // a. Let n be value.[[NumberData]].
        // b. Assert: n is a Number.
        let n: Number = value.get(agent).data.try_into().unwrap();
        // c. Return n.
        return Ok(n.bind(gc));
    }
    // 3. Throw a TypeError exception.
    Err(agent.throw_exception_with_static_message(ExceptionType::TypeError, "Not a Number", gc))
}
