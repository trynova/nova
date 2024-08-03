// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_integer_or_infinity,
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            primitive_objects::{PrimitiveObject, PrimitiveObjectData, PrimitiveObjectHeapData},
            ArgumentsList, Builtin,
        },
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{IntoValue, Number, String, Value, BUILTIN_STRING_MEMORY},
    },
    SmallInteger,
};

pub(crate) struct NumberPrototype;

struct NumberPrototypeToExponential;
impl Builtin for NumberPrototypeToExponential {
    const NAME: String = BUILTIN_STRING_MEMORY.toExponential;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(NumberPrototype::to_exponential);
}

struct NumberPrototypeToFixed;
impl Builtin for NumberPrototypeToFixed {
    const NAME: String = BUILTIN_STRING_MEMORY.toFixed;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(NumberPrototype::to_fixed);
}

struct NumberPrototypeToLocaleString;
impl Builtin for NumberPrototypeToLocaleString {
    const NAME: String = BUILTIN_STRING_MEMORY.toLocaleString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(NumberPrototype::to_locale_string);
}

struct NumberPrototypeToPrecision;
impl Builtin for NumberPrototypeToPrecision {
    const NAME: String = BUILTIN_STRING_MEMORY.toPrecision;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(NumberPrototype::to_precision);
}

struct NumberPrototypeToString;
impl Builtin for NumberPrototypeToString {
    const NAME: String = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(NumberPrototype::to_string);
}

struct NumberPrototypeValueOf;
impl Builtin for NumberPrototypeValueOf {
    const NAME: String = BUILTIN_STRING_MEMORY.valueOf;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(NumberPrototype::value_of);
}

impl NumberPrototype {
    fn to_exponential(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let fraction_digits = arguments.get(0);
        // Let x be ? ThisNumberValue(this value).
        let x = this_number_value(agent, this_value)?;
        // 2. Let f be ? ToIntegerOrInfinity(fractionDigits).
        let f = to_integer_or_infinity(agent, fraction_digits)?;
        // 3. Assert: If fractionDigits is undefined, then f is 0.
        debug_assert!(!fraction_digits.is_undefined() || f.is_pos_zero(agent));
        // 4. If x is not finite, return Number::toString(x, 10).
        if !x.is_finite(agent) {
            return Number::to_string_radix_10(agent, x).map(|result| result.into_value());
        }
        let f = f.into_i64(agent);
        // 5. If f < 0 or f > 100, throw a RangeError exception.
        if !(0..=100).contains(&f) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Fraction digits count out of range",
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
            Ok(f64_to_exponential(agent, x))
        } else {
            Ok(f64_to_exponential_with_precision(agent, x, f))
        }
    }

    fn to_fixed(agent: &mut Agent, this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let fraction_digits = arguments.get(0);
        // Let x be ? ThisNumberValue(this value).
        let x = this_number_value(agent, this_value)?;
        // 2. Let f be ? ToIntegerOrInfinity(fractionDigits).
        let f = to_integer_or_infinity(agent, fraction_digits)?;
        // 3. Assert: If fractionDigits is undefined, then f is 0.
        debug_assert!(!fraction_digits.is_undefined() || f.is_pos_zero(agent));
        // 4. If f is not finite, throw a RangeError exception.
        if !f.is_finite(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Fraction digits count out of range",
            ));
        }
        let f = f.into_i64(agent);
        // 5. If f < 0 or f > 100, throw a RangeError exception.
        if !(0..=100).contains(&f) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Fraction digits count out of range",
            ));
        }
        // 6. If x is not finite, return Number::toString(x, 10).
        if !x.is_finite(agent) {
            return Number::to_string_radix_10(agent, x).map(|result| result.into_value());
        }
        // 7. Set x to ℝ(x).
        let x = x.into_f64(agent);
        let mut buffer = ryu_js::Buffer::new();
        let string = buffer.format_to_fixed(x, f as u8);
        Ok(Value::from_str(agent, string))
    }

    fn to_locale_string(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Self::to_string(agent, this_value, arguments)
    }

    /// ### [21.1.3.5 Number.prototype.toPrecision ( )](https://tc39.es/ecma262/#sec-number.prototype.toprecision)
    /// Copied from Boa JS engine. Source https://github.com/boa-dev/boa/blob/6f1d7d11ce49040eafe54e5ff2da379be4d998c2/core/engine/src/builtins/number/mod.rs#L412
    ///
    /// Copyright (c) 2019 Jason Williams
    fn to_precision(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1.
        let x = this_number_value(agent, this_value)?;

        let precision = arguments.get(0);
        if precision.is_undefined() {
            // 2.
            return Self::to_string(agent, this_value, ArgumentsList(&[]));
        }

        // 3.
        let p = to_integer_or_infinity(agent, precision)?;

        if !x.is_finite(agent) {
            // 4.
            return Self::to_string(agent, this_value, ArgumentsList(&[]));
        }

        // 5.
        let precision = p.into_i64(agent) as i32;
        if !(1..=100).contains(&precision) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Precision out of range",
            ));
        }

        // 6.
        let mut x_f64 = x.into_f64(agent);

        // 7.
        let mut s = std::string::String::new();
        let mut m: std::string::String;
        let mut e: i32;

        if x_f64 < 0. {
            // 8.
            s.push('-');
            x_f64 = -x_f64;
        }

        if x_f64 == 0. {
            // 9.
            m = "0".repeat(precision as usize);
            e = 0;
        } else {
            // 10.

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

            // c: switching to scientific notation
            if e < -6 || e >= precision {
                assert_ne!(e, 0);

                // ii
                if precision > 1 {
                    m.insert(1, '.');
                }

                // vi
                m.push('e');

                // iii
                if e >= precision {
                    m.push('+');
                }

                // iv, v
                m.push_str(&e.to_string());

                return Ok(Value::from_string(agent, s + &m));
            }
        }

        // 11
        let e_inc = e + 1;
        if e_inc == precision as i32 {
            return Ok(String::from_string(agent, s + &m).into_value());
        }

        // 12
        if e >= 0 {
            m.insert(e_inc as usize, '.');
        // 13
        } else {
            s.push('0');
            s.push('.');
            s.push_str(&"0".repeat(-e_inc as usize));
        }

        Ok(String::from_string(agent, s + &m).into_value())
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
            if let Some(first) = to_round.chars().next() {
                if first > '4' {
                    digit += 1;
                }
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

    fn to_string(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let x = this_number_value(agent, this_value)?;
        let radix = arguments.get(0);
        if radix.is_undefined() || radix == Value::from(10u8) {
            Number::to_string_radix_10(agent, x).map(|result| result.into_value())
        } else {
            todo!();
        }
    }

    fn value_of(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        this_number_value(agent, this_value).map(|result| result.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.number_prototype();
        let this_base_object = intrinsics.number_prototype_base_object().into();
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
        assert!(slot.is_none());
        *slot = Some(PrimitiveObjectHeapData {
            object_index: Some(this_base_object),
            data: PrimitiveObjectData::Integer(SmallInteger::zero()),
        });
    }
}

fn f64_to_exponential(agent: &mut Agent, x: f64) -> Value {
    match x.abs() {
        x if x >= 1.0 || x == 0.0 => Value::from_string(agent, format!("{x:e}").replace('e', "e+")),
        _ => Value::from_string(agent, format!("{x:e}")),
    }
}

fn f64_to_exponential_with_precision(agent: &mut Agent, x: f64, f: usize) -> Value {
    let mut res = format!("{x:.f$e}");
    let idx = res.find('e').unwrap();
    if res.as_bytes()[idx + 1] != b'-' {
        res.insert(idx + 1, '+');
    }
    Value::from_string(agent, res)
}

/// ### [21.1.3.7.1 ThisNumberValue ( value )](https://tc39.es/ecma262/#sec-thisnumbervalue)
///
/// The abstract operation ThisNumberValue takes argument value (an ECMAScript language value) and returns either a normal completion containing a Number or a throw completion. It performs the following steps when called:
#[inline(always)]
fn this_number_value(agent: &mut Agent, value: Value) -> JsResult<Number> {
    // 1. If value is a Number, return value.
    if let Ok(value) = Number::try_from(value) {
        return Ok(value);
    }
    // 2. If value is an Object and value has a [[NumberData]] internal slot, then
    if let Ok(value) = PrimitiveObject::try_from(value) {
        if value.is_number_object(agent) {
            // a. Let n be value.[[NumberData]].
            // b. Assert: n is a Number.
            let n: Number = agent[value].data.try_into().unwrap();
            // c. Return n.
            return Ok(n);
        }
    }
    // 3. Throw a TypeError exception.
    Err(agent.throw_exception_with_static_message(ExceptionType::TypeError, "Not a Number"))
}
