// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::time::SystemTime;

use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::builtins::date::Date;
use crate::ecmascript::builtins::ordinary::ordinary_create_from_constructor;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::ProtoIntrinsics;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::ecmascript::types::Function;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Number;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::{String, Value};
use crate::ecmascript::{
    abstract_operations::type_conversion::to_number,
    numbers_and_dates::date_objects::date_prototype::{
        make_date, make_day, make_full_year, make_time, utc,
    },
};
use crate::engine::context::Bindable;
use crate::engine::context::GcScope;
use crate::heap::IntrinsicConstructorIndexes;
use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::type_conversion::to_primitive,
        numbers_and_dates::date_objects::date_prototype::time_clip,
    },
};

use super::date_prototype::{MS_PER_MINUTE, to_date_string};

pub struct DateConstructor;

impl Builtin for DateConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
    const LENGTH: u8 = 7;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Date;
}
impl BuiltinIntrinsicConstructor for DateConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Date;
}

struct DateNow;
impl Builtin for DateNow {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DateConstructor::now);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.now;
}
struct DateParse;
impl Builtin for DateParse {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DateConstructor::parse);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.parse;
}
struct DateUTC;
impl Builtin for DateUTC {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DateConstructor::utc);
    const LENGTH: u8 = 7;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.UTC;
}
impl DateConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. If NewTarget is undefined, then
        let Some(new_target) = new_target else {
            // a. Let now be the time value (UTC) identifying the current time.
            let now = SystemTime::now();
            let now = now
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;
            // b. Return ToDateString(now).
            return Ok(Value::from_string(
                agent,
                to_date_string(agent, now),
                gc.into_nogc(),
            ));
        };
        // 2. Let numberOfArgs be the number of elements in values.
        let number_of_args = arguments.len() as u32;
        let dv = match number_of_args {
            // 3. If numberOfArgs = 0, then
            0 => {
                // a. Let dv be the time value (UTC) identifying the current time.
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_millis() as f64
            }
            // 4. Else if numberOfArgs = 1, then
            1 => {
                // a. Let value be values[0].
                let value = arguments.get(0);
                // b. If value is an Object and value has a [[DateValue]] internal slot, then
                let tv = if let Value::Date(date) = value {
                    // i. Let tv be value.[[DateValue]].
                    date.date(agent)
                }
                // c. Else,
                else {
                    // i. Let v be ? ToPrimitive(value).
                    let v = to_primitive(agent, value, None, gc.reborrow())?.unbind();
                    // ii. If v is a String, then
                    if v.is_string() {
                        // 1. Assert: The next step never returns an abrupt completion because v is a String.
                        // 2. Let tv be the result of parsing v as a date, in exactly the same manner as for the parse method (21.4.3.2).
                        let v = v.into_value().to_string(agent, gc.reborrow()).unwrap();
                        parse_date::parse(agent, v.as_str(agent))
                    }
                    // iii. Else,
                    else {
                        // 1. Let tv be ? ToNumber(v).
                        to_number(agent, v, gc.reborrow())?.to_real(agent)
                    }
                };
                // d. Let dv be TimeClip(tv).
                time_clip(tv)
            }
            // 5. Else,
            _ => {
                // a. Assert: numberOfArgs ≥ 2.
                assert!(number_of_args >= 2);
                // b. Let y be ? ToNumber(values[0]).
                let y = to_number(agent, arguments.get(0), gc.reborrow())?.to_real(agent);
                // c. Let m be ? ToNumber(values[1]).
                let m = to_number(agent, arguments.get(1), gc.reborrow())?.to_real(agent);
                // d. If numberOfArgs > 2, let dt be ? ToNumber(values[2]); else let dt be 1𝔽.
                let dt = if number_of_args > 2 {
                    to_number(agent, arguments.get(2), gc.reborrow())?.to_real(agent)
                } else {
                    1.0
                };
                // e. If numberOfArgs > 3, let h be ? ToNumber(values[3]); else let h be +0𝔽.
                let h = if number_of_args > 3 {
                    to_number(agent, arguments.get(3), gc.reborrow())?.to_real(agent)
                } else {
                    0.0
                };
                // f. If numberOfArgs > 4, let min be ? ToNumber(values[4]); else let min be +0𝔽.
                let min = if number_of_args > 4 {
                    to_number(agent, arguments.get(4), gc.reborrow())?.to_real(agent)
                } else {
                    0.0
                };
                // g. If numberOfArgs > 5, let s be ? ToNumber(values[5]); else let s be +0𝔽.
                let s = if number_of_args > 5 {
                    to_number(agent, arguments.get(5), gc.reborrow())?.to_real(agent)
                } else {
                    0.0
                };
                // h. If numberOfArgs > 6, let milli be ? ToNumber(values[6]); else let milli be +0𝔽.
                let milli = if number_of_args > 6 {
                    to_number(agent, arguments.get(6), gc.reborrow())?.to_real(agent)
                } else {
                    0.0
                };
                // i. Let yr be MakeFullYear(y).
                let yr = make_full_year(y);
                // j. Let finalDate be MakeDate(MakeDay(yr, m, dt), MakeTime(h, min, s, milli)).
                let final_date = make_date(make_day(yr, m, dt), make_time(h, min, s, milli));
                // k. Let dv be TimeClip(UTC(finalDate)).
                time_clip(utc(agent, final_date))
            }
        };

        // 6. Let O be ? OrdinaryCreateFromConstructor(NewTarget, "%Date.prototype%", « [[DateValue]] »).
        let o = ordinary_create_from_constructor(
            agent,
            Function::try_from(new_target).unwrap(),
            ProtoIntrinsics::Date,
            gc.reborrow(),
        )?;
        // 7. Set O.[[DateValue]] to dv.
        agent[Date::try_from(o).unwrap()].date = dv;
        // 8. Return O.
        Ok(o.unbind().into_value())
    }

    /// ### [21.4.3.1 Date.now ( )](https://tc39.es/ecma262/#sec-date.now1)
    ///
    /// This function returns the time value designating the UTC date and time of the occurrence of the call to it.
    fn now<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let time_value = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        assert_eq!(time_value as u64 as u128, time_value);
        let time_value = time_value as u64;
        Ok(
            Number::from(SmallInteger::try_from(time_value).expect("SystemTime is beyond range"))
                .into_value(),
        )
    }

    /// ### [21.4.3.2 Date.parse ( string )](https://tc39.es/ecma262/#sec-date.parse)
    ///
    /// This function applies the ToString operator to its argument. If ToString results in
    /// an abrupt completion the Completion Record is immediately returned. Otherwise,
    /// this function interprets the resulting String as a date and time; it returns a Number,
    /// the UTC time value corresponding to the date and time. The String may be interpreted as a
    /// local time, a UTC time, or a time in some other time zone, depending on the contents of
    /// the String. The function first attempts to parse the String according to the format described
    /// in Date Time String Format (21.4.1.32), including expanded years. If the String does not
    /// conform to that format the function may fall back to any implementation-specific heuristics
    /// or implementation-specific date formats. Strings that are unrecognizable or contain
    /// out-of-bounds format element values shall cause this function to return NaN.
    ///
    /// If the String conforms to the Date Time String Format, substitute values take
    /// the place of absent format elements. When the MM or DD elements are absent,
    /// "01" is used. When the HH, mm, or ss elements are absent, "00" is used.
    /// When the sss element is absent, "000" is used. When the UTC offset representation is absent,
    /// date-only forms are interpreted as a UTC time and date-time forms are interpreted as a local time.
    ///
    /// If x is any Date whose milliseconds amount is zero within a particular implementation of ECMAScript,
    /// then all of the following expressions should produce the same numeric value in that implementation,
    /// if all the properties referenced have their initial values:
    ///
    /// ```js
    /// x.valueOf()
    /// Date.parse(x.toString())
    /// Date.parse(x.toUTCString())
    /// Date.parse(x.toISOString())
    /// ```
    ///
    /// However, the expression
    ///
    /// ```js
    /// Date.parse(x.toLocaleString())
    /// ```
    ///
    /// is not required to produce the same Number value as the preceding three expressions and,
    /// in general, the value produced by this function is implementation-defined when given any
    /// Stringvalue that does not conform to the Date Time String Format (21.4.1.32) and that
    /// could not be produced in that implementation by the toString or toUTCString method.
    fn parse<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let input = arguments.get(0).to_string(agent, gc.reborrow())?;
        let parsed = parse_date::parse(agent, input.as_str(agent));
        Ok(Value::from_f64(agent, parsed, gc.into_nogc()))
    }

    /// ### [21.4.3.4 Date.UTC ( year \[ , month \[ , date \[ , hours \[ , minutes \[ , seconds \[ , ms \] \] \] \] \] \] )](https://tc39.es/ecma262/#sec-date.utc)
    /// > #### Note
    /// >
    /// > This function differs from the Date constructor in two ways: it
    /// > returns a time value as a Number, rather than creating a Date,
    /// > and it interprets the arguments in UTC rather than as local time.
    fn utc<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let y be ? ToNumber(year).
        let y = to_number(agent, arguments.get(0), gc.reborrow())?.to_real(agent);
        // 2. If month is present, let m be ? ToNumber(month); else let m be +0𝔽.
        let m = if arguments.len() > 1 {
            to_number(agent, arguments.get(1), gc.reborrow())?.to_real(agent)
        } else {
            0.0
        };
        // 3. If date is present, let dt be ? ToNumber(date); else let dt be 1𝔽.
        let dt = if arguments.len() > 2 {
            to_number(agent, arguments.get(2), gc.reborrow())?.to_real(agent)
        } else {
            1.0
        };
        // 4. If hours is present, let h be ? ToNumber(hours); else let h be +0𝔽.
        let h = if arguments.len() > 3 {
            to_number(agent, arguments.get(3), gc.reborrow())?.to_real(agent)
        } else {
            0.0
        };
        // 5. If minutes is present, let min be ? ToNumber(minutes); else let min be +0𝔽.
        let min = if arguments.len() > 4 {
            to_number(agent, arguments.get(4), gc.reborrow())?.to_real(agent)
        } else {
            0.0
        };
        // 6. If seconds is present, let s be ? ToNumber(seconds); else let s be +0𝔽.
        let s = if arguments.len() > 5 {
            to_number(agent, arguments.get(5), gc.reborrow())?.to_real(agent)
        } else {
            0.0
        };
        // 7. If ms is present, let milli be ? ToNumber(ms); else let milli be +0𝔽.
        let milli = if arguments.len() > 6 {
            to_number(agent, arguments.get(6), gc.reborrow())?.to_real(agent)
        } else {
            0.0
        };
        // 8. Let yr be MakeFullYear(y).
        let yr = make_full_year(y);
        // 9. Return TimeClip(MakeDate(MakeDay(yr, m, dt), MakeTime(h, min, s, milli))).
        Ok(Value::from_f64(
            agent,
            time_clip(make_date(make_day(yr, m, dt), make_time(h, min, s, milli))),
            gc.into_nogc(),
        ))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let date_prototype = intrinsics.date_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<DateConstructor>(agent, realm)
            .with_property_capacity(4)
            .with_builtin_function_property::<DateNow>()
            .with_builtin_function_property::<DateParse>()
            .with_prototype_property(date_prototype.into_object())
            .with_builtin_function_property::<DateUTC>()
            .build();
    }
}

/// Ported from Boa JS engine. Source https://github.com/boa-dev/boa/blob/13a030a0aa452e6f78e4a7e8bbc0e11b878bbd58/core/engine/src/builtins/date/utils.rs#L745
mod parse_date {
    use super::*;

    /// Parse a date string according to the steps specified in [`Date.parse`][spec].
    ///
    /// We parse three different formats:
    /// - The [`Date Time String Format`][spec-format] specified in the spec: `YYYY-MM-DDTHH:mm:ss.sssZ`
    /// - The `toString` format: `Thu Jan 01 1970 00:00:00 GMT+0000`
    /// - The `toUTCString` format: `Thu, 01 Jan 1970 00:00:00 GMT`
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-date.parse
    /// [spec-format]: https://tc39.es/ecma262/#sec-date-time-string-format
    pub fn parse(agent: &Agent, date: &str) -> f64 {
        // Date Time String Format: 'YYYY-MM-DDTHH:mm:ss.sssZ'
        if let Some(dt) = DateParser::new(agent, date).parse() {
            return dt as f64;
        }

        // `toString` format: `Thu Jan 01 1970 00:00:00 GMT+0000`
        // TODO:
        // if let Ok(t) = OffsetDateTime::parse(&date, &format_description!("[weekday repr:short] [month repr:short] [day] [year] [hour]:[minute]:[second] GMT[offset_hour sign:mandatory][offset_minute][end]")) {
        //     return Some(t.unix_timestamp() * 1000 + i64::from(t.millisecond()));
        // }

        // `toUTCString` format: `Thu, 01 Jan 1970 00:00:00 GMT`
        // TODO:
        // if let Ok(t) = PrimitiveDateTime::parse(&date, &format_description!("[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT[end]")) {
        //     let t = t.assume_utc();
        //     return Some(t.unix_timestamp() * 1000 + i64::from(t.millisecond()));
        // }

        f64::NAN
    }

    /// Parses a date string according to the [`Date Time String Format`][spec].
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-date-time-string-format
    struct DateParser<'a> {
        agent: &'a Agent,
        input: std::iter::Peekable<std::slice::Iter<'a, u8>>,
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        millisecond: u32,
        offset: i64,
    }

    // Copied from https://github.com/RoDmitry/atoi_simd/blob/master/src/fallback.rs,
    // which is based on https://rust-malaysia.github.io/code/2020/07/11/faster-integer-parsing.html.
    #[doc(hidden)]
    #[allow(clippy::inline_always)]
    mod fast_atoi {
        #[inline(always)]
        pub const fn process_8(mut val: u64, len: usize) -> u64 {
            val <<= 64_usize.saturating_sub(len << 3); // << 3 - same as mult by 8
            val = (val & 0x0F0F_0F0F_0F0F_0F0F).wrapping_mul(0xA01) >> 8;
            val = (val & 0x00FF_00FF_00FF_00FF).wrapping_mul(0x64_0001) >> 16;
            (val & 0x0000_FFFF_0000_FFFF).wrapping_mul(0x2710_0000_0001) >> 32
        }

        #[inline(always)]
        pub const fn process_4(mut val: u32, len: usize) -> u32 {
            val <<= 32_usize.saturating_sub(len << 3); // << 3 - same as mult by 8
            val = (val & 0x0F0F_0F0F).wrapping_mul(0xA01) >> 8;
            (val & 0x00FF_00FF).wrapping_mul(0x64_0001) >> 16
        }
    }

    impl<'a> DateParser<'a> {
        fn new(agent: &'a Agent, s: &'a str) -> Self {
            Self {
                agent,
                input: s.as_bytes().iter().peekable(),
                year: 0,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
                millisecond: 0,
                offset: 0,
            }
        }

        fn next_expect(&mut self, expect: u8) -> Option<()> {
            self.input
                .next()
                .and_then(|c| if *c == expect { Some(()) } else { None })
        }

        fn next_ascii_digit(&mut self) -> Option<u8> {
            self.input
                .next()
                .and_then(|c| if c.is_ascii_digit() { Some(*c) } else { None })
        }

        fn next_n_ascii_digits<const N: usize>(&mut self) -> Option<[u8; N]> {
            let mut digits = [0; N];
            for digit in &mut digits {
                *digit = self.next_ascii_digit()?;
            }
            Some(digits)
        }

        fn parse_n_ascii_digits<const N: usize>(&mut self) -> Option<u64> {
            assert!(N <= 8, "parse_n_ascii_digits parses no more than 8 digits");
            if N == 0 {
                return None;
            }
            let ascii_digits = self.next_n_ascii_digits::<N>()?;
            match N {
                1..4 => {
                    // When N is small, process digits naively.
                    let mut res = 0;
                    for digit in ascii_digits {
                        res = res * 10 + u64::from(digit & 0xF);
                    }
                    Some(res)
                }
                4 => {
                    // Process digits as an u32 block.
                    let mut src = [0; 4];
                    src[..N].copy_from_slice(&ascii_digits);
                    let val = u32::from_le_bytes(src);
                    Some(u64::from(fast_atoi::process_4(val, N)))
                }
                _ => {
                    // Process digits as an u64 block.
                    let mut src = [0; 8];
                    src[..N].copy_from_slice(&ascii_digits);
                    let val = u64::from_le_bytes(src);
                    Some(fast_atoi::process_8(val, N))
                }
            }
        }

        fn finish(&mut self) -> Option<i64> {
            if self.input.peek().is_some() {
                return None;
            }

            let date = make_date(
                make_day(self.year.into(), (self.month - 1).into(), self.day.into()),
                make_time(
                    self.hour.into(),
                    self.minute.into(),
                    self.second.into(),
                    self.millisecond.into(),
                ),
            );

            let date = date + (self.offset as f64) * MS_PER_MINUTE;

            let t = time_clip(date);
            if t.is_finite() { Some(t as i64) } else { None }
        }

        fn finish_local(&mut self) -> Option<i64> {
            if self.input.peek().is_some() {
                return None;
            }

            let date = make_date(
                make_day(self.year.into(), (self.month - 1).into(), self.day.into()),
                make_time(
                    self.hour.into(),
                    self.minute.into(),
                    self.second.into(),
                    self.millisecond.into(),
                ),
            );

            let t = time_clip(utc(self.agent, date));
            if t.is_finite() { Some(t as i64) } else { None }
        }

        #[allow(clippy::as_conversions)]
        fn parse(&mut self) -> Option<i64> {
            self.parse_year()?;
            match self.input.peek() {
                Some(b'T') => return self.parse_time(),
                None => return self.finish(),
                _ => {}
            }
            self.next_expect(b'-')?;
            self.month = self.parse_n_ascii_digits::<2>()? as u32;
            if self.month < 1 || self.month > 12 {
                return None;
            }
            match self.input.peek() {
                Some(b'T') => return self.parse_time(),
                None => return self.finish(),
                _ => {}
            }
            self.next_expect(b'-')?;
            self.day = self.parse_n_ascii_digits::<2>()? as u32;
            if self.day < 1 || self.day > 31 {
                return None;
            }
            match self.input.peek() {
                Some(b'T') => self.parse_time(),
                _ => self.finish(),
            }
        }

        #[allow(clippy::as_conversions)]
        fn parse_year(&mut self) -> Option<()> {
            if let &&sign @ (b'+' | b'-') = self.input.peek()? {
                // Consume the sign.
                self.input.next();
                let year = self.parse_n_ascii_digits::<6>()? as i32;
                let neg = sign == b'-';
                if neg && year == 0 {
                    return None;
                }
                self.year = if neg { -year } else { year };
            } else {
                self.year = self.parse_n_ascii_digits::<4>()? as i32;
            }
            Some(())
        }

        #[allow(clippy::as_conversions)]
        fn parse_time(&mut self) -> Option<i64> {
            self.next_expect(b'T')?;
            self.hour = self.parse_n_ascii_digits::<2>()? as u32;
            if self.hour > 24 {
                return None;
            }
            self.next_expect(b':')?;
            self.minute = self.parse_n_ascii_digits::<2>()? as u32;
            if self.minute > 59 {
                return None;
            }
            match self.input.peek() {
                Some(b':') => self.input.next(),
                None => return self.finish_local(),
                _ => {
                    self.parse_timezone()?;
                    return self.finish();
                }
            };
            self.second = self.parse_n_ascii_digits::<2>()? as u32;
            if self.second > 59 {
                return None;
            }
            match self.input.peek() {
                Some(b'.') => self.input.next(),
                None => return self.finish_local(),
                _ => {
                    self.parse_timezone()?;
                    return self.finish();
                }
            };
            self.millisecond = self.parse_n_ascii_digits::<3>()? as u32;
            if self.input.peek().is_some() {
                self.parse_timezone()?;
                self.finish()
            } else {
                self.finish_local()
            }
        }

        #[allow(clippy::as_conversions)]
        fn parse_timezone(&mut self) -> Option<()> {
            match self.input.next() {
                Some(b'Z') => return Some(()),
                Some(sign @ (b'+' | b'-')) => {
                    let neg = *sign == b'-';
                    let offset_hour = self.parse_n_ascii_digits::<2>()? as i64;
                    if offset_hour > 23 {
                        return None;
                    }
                    self.offset = if neg { offset_hour } else { -offset_hour } * 60;
                    if self.input.peek().is_none() {
                        return Some(());
                    }
                    self.next_expect(b':')?;
                    let offset_minute = self.parse_n_ascii_digits::<2>()? as i64;
                    if offset_minute > 59 {
                        return None;
                    }
                    self.offset += if neg { offset_minute } else { -offset_minute };
                }
                _ => return None,
            }
            Some(())
        }
    }
}
