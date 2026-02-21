// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod duration;
mod error;
mod instant;
mod options;
mod plain_time;

pub use duration::*;
pub(crate) use error::*;
pub use instant::*;
pub(crate) use options::*;
pub use plain_time::*;

use temporal_rs::{
    options::{DifferenceSettings, RoundingIncrement, RoundingMode, Unit, UnitGroup},
    parsers::Precision,
};

use crate::{
    ecmascript::{
        Agent, BUILTIN_STRING_MEMORY, ExceptionType, JsResult, Number, Object, Realm, Value,
        builders::OrdinaryObjectBuilder, get,
    },
    engine::{Bindable, GcScope, NoGcScope, Scopable, trivially_bindable},
    heap::WellKnownSymbols,
};

pub(crate) struct TemporalObject;

impl TemporalObject {
    pub fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.temporal();

        let instant_constructor = intrinsics.temporal_instant();
        let duration_constructor = intrinsics.temporal_duration();
        let plain_time_constructor = intrinsics.temporal_plain_time();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(4)
            .with_prototype(object_prototype)
            // 1.2.1 Temporal.Instant ( . . . )
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.Instant.into())
                    .with_value(instant_constructor.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            // 1.2.2 Temporal.PlainDateTime ( . . . )
            // 1.2.3 Temporal.PlainDate ( . . . )
            // 1.2.4 Temporal.PlainTime ( . . . )
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.PlainTime.into())
                    .with_value(plain_time_constructor.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            // 1.2.5 Temporal.PlainYearMonth ( . . . )
            // 1.2.6 Temporal.PlainMonthDay ( . . . )
            // 1.2.7 Temporal.Duration ( . . . )
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.Duration.into())
                    .with_value(duration_constructor.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            // 1.2.8 Temporal.ZonedDateTime ( . . . )
            // 1.3.1 Temporal.Now
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbols::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Temporal.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

trivially_bindable!(DifferenceSettings);
trivially_bindable!(UnitGroup);
trivially_bindable!(Unit);
trivially_bindable!(RoundingMode);
trivially_bindable!(RoundingIncrement);
trivially_bindable!(Precision);

/// [13.15 GetTemporalFractionalSecondDigitsOption ( options )](https://tc39.es/proposal-temporal/#sec-temporal-gettemporalfractionalseconddigitsoption)
/// The abstract operation GetTemporalFractionalSecondDigitsOption takes argument
/// options (an Object) and returns either a normal completion containing
/// either auto or an integer in the inclusive interval from 0 to 9,
/// or a throw completion. It fetches and validates the "fractionalSecondDigits"
/// property from options, returning a default if absent.
pub(crate) fn get_temporal_fractional_second_digits_option<'gc>(
    agent: &mut Agent,
    options: Object<'gc>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, temporal_rs::parsers::Precision> {
    let options = options.bind(gc.nogc());
    // 1. Let digitsValue be ? Get(options, "fractionalSecondDigits").
    let digits_value = get(
        agent,
        options.unbind(),
        BUILTIN_STRING_MEMORY
            .fractionalSecondDigits
            .to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 2. If digitsValue is undefined, return auto.
    if digits_value.is_undefined() {
        return Ok(temporal_rs::parsers::Precision::Auto);
    }
    // If already a valid single digit in the range 0-9, return early.
    if let Value::Integer(digits_value) = digits_value
        && (0..=9).contains(&digits_value.into_i64())
    {
        return Ok(temporal_rs::parsers::Precision::Digit(
            digits_value.into_i64() as u8,
        ));
    }
    // 3. If digitsValue is not a Number, then
    let Ok(digits_number) = Number::try_from(digits_value) else {
        // a. If ? ToString(digitsValue) is not "auto", throw a RangeError exception.
        if digits_value
            .unbind()
            .to_string(agent, gc.reborrow())
            .unbind()?
            .as_bytes(agent)
            != b"auto"
        {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "fractionalSecondDigits must be a finite number or \"auto\"",
                gc.into_nogc(),
            ));
        }
        // b. Return auto.
        return Ok(temporal_rs::parsers::Precision::Auto);
    };

    // 4. If digitsValue is NaN, +∞𝔽, or -∞𝔽, throw a RangeError exception.
    if digits_number.is_nan(agent)
        || digits_number.is_pos_infinity(agent)
        || digits_number.is_neg_infinity(agent)
    {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "fractionalSecondDigits must be a finite number or \"auto\"",
            gc.into_nogc(),
        ));
    }
    // 5. Let digitCount be floor(ℝ(digitsValue)).
    let digit_count = digits_number.into_f64(agent).floor();
    // 6. If digitCount < 0 or digitCount > 9, throw a RangeError exception.
    if !(0.0..=9.0).contains(&digit_count) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "fractionalSecondDigits must be between 0 and 9",
            gc.into_nogc(),
        ));
    }
    // 7. Return digitCount.
    Ok(temporal_rs::parsers::Precision::Digit(digit_count as u8))
}

/// [13.42 GetDifferenceSettings ( operation, options, unitGroup, disallowedUnits, fallbackSmallestUnit, smallestLargestDefaultUnit )](https://tc39.es/proposal-temporal/#sec-temporal-getdifferencesettings)
/// The abstract operation GetDifferenceSettings takes arguments operation (since or until),
/// options (an Object), unitGroup (date, time, or datetime), disallowedUnits (a List of Temporal units),
/// fallbackSmallestUnit (a Temporal unit), and smallestLargestDefaultUnit (a Temporal unit) and returns either
/// a normal completion containing a Record with fields [[SmallestUnit]] (a Temporal unit),
/// [[LargestUnit]] (a Temporal unit), [[RoundingMode]] (a rounding mode),
/// and [[RoundingIncrement]] (an integer in the inclusive interval from 1 to 10**9),
/// or a throw completion. It reads unit and rounding options needed by difference operations.
pub(crate) fn get_difference_settings<'gc, const IS_UNTIL: bool>(
    agent: &mut Agent,
    options: Option<Object>,              // options (an Object)
    _unit_group: UnitGroup,               // unitGroup (date, time, or datetime)
    _disallowed_units: &[Unit],           // disallowedUnits (todo:a List of Temporal units)
    _fallback_smallest_unit: Unit,        // fallbackSmallestUnit (a Temporal unit)
    _smallest_largest_default_unit: Unit, // smallestLargestDefaultUnit (a Temporal unit)
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, DifferenceSettings> {
    let Some(options) = options else {
        let mut diff_settings = temporal_rs::options::DifferenceSettings::default();
        diff_settings.largest_unit = None;
        diff_settings.smallest_unit = None;
        diff_settings.rounding_mode = Some(RoundingMode::Trunc);
        diff_settings.increment = Some(Default::default());
        return Ok(diff_settings);
    };
    let options = options.scope(agent, gc.nogc());
    // 1. NOTE: The following steps read options and perform independent validation in alphabetical order.
    // 2. Let largestUnit be ? GetTemporalUnitValuedOption(options, "largestUnit", unset).
    let largest_unit = get_temporal_unit_valued_option(
        agent,
        options.get(agent),
        BUILTIN_STRING_MEMORY.largestUnit.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 3. Let roundingIncrement be ? GetRoundingIncrementOption(options).
    let rounding_increment =
        get_rounding_increment_option(agent, options.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
    // 4. Let roundingMode be ? GetRoundingModeOption(options, trunc).
    let rounding_mode = get_rounding_mode_option(
        agent,
        options.get(agent),
        RoundingMode::Trunc,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 5. Let smallestUnit be ? GetTemporalUnitValuedOption(options, "smallestUnit", unset).
    let smallest_unit = get_temporal_unit_valued_option(
        agent,
        options.get(agent),
        BUILTIN_STRING_MEMORY.smallestUnit.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // TODO(jesper): good starting point! check all data and if the values make sense :)
    // 6. Perform ? ValidateTemporalUnitValue(largestUnit, unitGroup, « auto »).
    // 7. If largestUnit is unset, then
    //    a. Set largestUnit to auto.
    // 8. If disallowedUnits contains largestUnit, throw a RangeError exception.
    // 9. If operation is since, then
    //    a. Set roundingMode to NegateRoundingMode(roundingMode).
    // 10. Perform ? ValidateTemporalUnitValue(smallestUnit, unitGroup).
    // 11. If smallestUnit is unset, then
    //     a. Set smallestUnit to fallbackSmallestUnit.
    // 12. If disallowedUnits contains smallestUnit, throw a RangeError exception.
    // 13. Let defaultLargestUnit be LargerOfTwoTemporalUnits(smallestLargestDefaultUnit, smallestUnit).
    // 14. If largestUnit is auto, set largestUnit to defaultLargestUnit.
    // 15. If LargerOfTwoTemporalUnits(largestUnit, smallestUnit) is not largestUnit, throw a RangeError exception.
    // 16. Let maximum be MaximumTemporalDurationRoundingIncrement(smallestUnit).
    // 17. If maximum is not unset, perform ? ValidateTemporalRoundingIncrement(roundingIncrement, maximum, false).
    // 18. Return the Record { [[SmallestUnit]]: smallestUnit, [[LargestUnit]]: largestUnit, [[RoundingMode]]: roundingMode, [[RoundingIncrement]]: roundingIncrement,  }.
    let mut diff_settings = temporal_rs::options::DifferenceSettings::default();
    diff_settings.largest_unit = largest_unit;
    diff_settings.smallest_unit = smallest_unit;
    diff_settings.rounding_mode = Some(rounding_mode);
    diff_settings.increment = Some(rounding_increment);
    Ok(diff_settings)
}
