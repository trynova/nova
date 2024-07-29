// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Function;
use crate::ecmascript::types::language::IntoObject;

pub trait IntoFunction<'gen>
where
    Self: Sized + Copy + IntoObject<'gen>,
{
    fn into_function(self) -> Function<'gen>;
}
