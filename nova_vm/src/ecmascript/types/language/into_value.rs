// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::types::{Object, Primitive};

use super::Value;

pub trait IntoValue<'a>
where
    Self: 'a + Sized + Copy,
{
    fn into_value(self) -> Value<'a>;
}

impl<'a, T> IntoValue<'a> for T
where
    T: Into<Value<'a>> + 'a + Sized + Copy,
{
    #[inline]
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a, T: Into<Primitive<'a>>> From<T> for Value<'a> {
    #[inline]
    fn from(value: T) -> Self {
        let value: Primitive = value.into();
        value.into()
    }
}
