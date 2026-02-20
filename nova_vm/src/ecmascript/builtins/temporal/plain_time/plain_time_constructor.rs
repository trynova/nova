// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinIntrinsicConstructor, String},
    heap::IntrinsicConstructorIndexes,
};

/// Constructor function object for %Temporal.PlainTime%.
pub(crate) struct TemporalPlainTimeConstructor;

impl Builtin for TemporalPlainTimeConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.PlainTime;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(TemporalPlainTimeConstructor::constructor);
}
impl BuiltinIntrinsicConstructor for TemporalPlainTimeConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TemporalPlainTime;
}
