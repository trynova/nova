// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Primitive;

pub trait IntoPrimitive<'a>
where
    Self: Sized + Copy,
{
    fn into_primitive(self) -> Primitive<'a>;
}

impl<'a, T> IntoPrimitive<'a> for T
where
    T: Into<Primitive<'a>> + 'a + Sized + Copy,
{
    #[inline]
    fn into_primitive(self) -> Primitive<'a> {
        self.into()
    }
}

impl IntoPrimitive<'static> for bool {
    fn into_primitive(self) -> Primitive<'static> {
        Primitive::Boolean(self)
    }
}
