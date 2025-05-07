// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Numeric;

pub trait IntoNumeric<'a>
where
    Self: Sized + Copy,
{
    fn into_numeric(self) -> Numeric<'a>;
}

impl<'a, T> IntoNumeric<'a> for T
where
    T: Into<Numeric<'a>> + 'a + Sized + Copy,
{
    #[inline]
    fn into_numeric(self) -> Numeric<'a> {
        self.into()
    }
}
