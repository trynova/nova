// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::time::SystemTime;

use crate::{
    SmallInteger,
    ecmascript::types::{OrdinaryObject, Value},
    engine::context::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

/// ### [21.4.1.1 Time Values and Time Range](https://tc39.es/ecma262/#sec-time-values-and-time-range)
///
/// A Number can exactly represent all integers from -9,007,199,254,740,992
/// to 9,007,199,254,740,992 (21.1.2.8 and 21.1.2.6). A time value supports
/// a slightly smaller range of -8,640,000,000,000,000 to 8,640,000,000,000,000 milliseconds.
/// This yields a supported time value range of exactly -100,000,000 days
/// to 100,000,000 days relative to midnight at the beginning of 1 January 1970 UTC.
///
/// In that case, the time value can be either:
///
/// - Invalid, which is presented as `i64::MAX`
/// - An integer in the range of -8,640,000,000,000,000 to 8,640,000,000,000,000,
///   which is represented as a non-max `i64`, and can also fit in `SmallInteger`
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(crate) struct DateValue(i64);

impl DateValue {
    pub(crate) const NAN: Self = Self(i64::MAX);

    pub(crate) fn get_i64(self) -> Option<i64> {
        if self.0 == i64::MAX {
            None
        } else {
            Some(self.0)
        }
    }

    pub(crate) fn get_f64(self) -> Option<f64> {
        self.get_i64().map(|v| v as f64)
    }

    pub(crate) fn now() -> Self {
        let now = SystemTime::now();
        let now = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();
        Self(now as i64)
    }

    /// ### [21.4.1.31 TimeClip ( time )](https://tc39.es/ecma262/#sec-timeclip)
    ///
    /// The abstract operation TimeClip takes argument time (a Number) and returns
    /// a Number. It calculates a number of milliseconds.
    pub(crate) fn time_clip(time: f64) -> DateValue {
        // 1. If time is not finite, return NaN.
        if !time.is_finite() {
            return DateValue::NAN;
        }

        // 2. If abs(ℝ(time)) > 8.64 × 10**15, return NaN.
        if time.abs() > 8.64e15 {
            return DateValue::NAN;
        }

        // 3. Return 𝔽(! ToIntegerOrInfinity(time)).
        DateValue(time.trunc() as i64)
    }
}

impl<'a> From<DateValue> for Value<'a> {
    fn from(value: DateValue) -> Self {
        if let Some(value) = value.get_f64() {
            // SAFETY: `value` is guaranteed to be in the range of `SmallInteger`.
            Value::Integer(SmallInteger::try_from(value).unwrap())
        } else {
            Value::nan()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DateHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) date: DateValue,
}

impl DateHeapData<'_> {
    pub(crate) fn new_invalid() -> Self {
        Self {
            object_index: None,
            date: DateValue::NAN,
        }
    }
}

bindable_handle!(DateHeapData);

impl HeapMarkAndSweep for DateHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            date: _,
        } = self;
        object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            date: _,
        } = self;
        object_index.sweep_values(compactions);
    }
}
