use crate::ecmascript::{
    abstract_operations::type_conversion::to_integer_or_infinity,
    builders::ordinary_object_builder::OrdinaryObjectBuilder,
    builtins::{ArgumentsList, Builtin},
    execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
    types::{IntoValue, Number, String, Value, BUILTIN_STRING_MEMORY},
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
            return Err(agent.throw_exception(
                ExceptionType::RangeError,
                "Fraction digits count out of range",
            ));
        }
        let f = f as usize;

        // 6. Set x to ℝ(x).
        let x = x.into_f64(agent);
        // This gets rid of -0.0
        if x == 0.0 {
            0.0
        } else {
            x
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
            return Err(agent.throw_exception(
                ExceptionType::RangeError,
                "Fraction digits count out of range",
            ));
        }
        let f = f.into_i64(agent);
        // 5. If f < 0 or f > 100, throw a RangeError exception.
        if !(0..=100).contains(&f) {
            return Err(agent.throw_exception(
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

    fn to_precision(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
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
        let number_constructor = intrinsics.number();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
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
    // a. Let n be value.[[NumberData]].
    // b. Assert: n is a Number.
    // c. Return n.
    // 3. Throw a TypeError exception.
    Err(agent.throw_exception(ExceptionType::TypeError, "Not a Number"))
}
