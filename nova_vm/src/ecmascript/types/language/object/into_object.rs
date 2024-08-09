// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::types::language::into_value::IntoValue;

use super::Object;

pub trait IntoObject<'gen>
where
    Self: Sized + Copy + IntoValue<'gen>,
{
    fn into_object(self) -> Object;
}
