// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod any_typed_array;
mod data;
mod normal_typed_array;
#[cfg(feature = "shared-array-buffer")]
mod shared_typed_array;

pub use any_typed_array::*;
pub(crate) use data::*;
pub use normal_typed_array::*;
#[cfg(feature = "shared-array-buffer")]
pub use shared_typed_array::*;

use crate::{
    ecmascript::{Agent, Number, PropertyKey, String, canonical_numeric_index_string},
    engine::context::NoGcScope,
};

/// Canonicalize the given property key if it is a numeric string key.
pub(crate) fn canonicalize_numeric_index_string(
    agent: &mut Agent,
    p: &mut PropertyKey,
    gc: NoGcScope,
) {
    let Ok(numeric_index) = String::try_from(unsafe { p.into_value_unchecked() }) else {
        return;
    };
    let numeric_index = canonical_numeric_index_string(agent, numeric_index, gc);
    let Some(numeric_index) = numeric_index else {
        return;
    };
    if let Number::Integer(numeric_index) = numeric_index {
        // Got proper integer index.
        *p = PropertyKey::Integer(numeric_index);
    } else {
        // Non-integer index: this should pass into the "!IsValidIntegerIndex"
        // code path. Negative indexes are always invalid so we use that.
        *p = PropertyKey::Integer((-1i32).into())
    };
}
