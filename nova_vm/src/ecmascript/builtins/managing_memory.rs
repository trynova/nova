// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod finalization_registry_objects;
#[cfg(feature = "weak-refs")]
mod weak_ref_objects;

pub(crate) use finalization_registry_objects::*;
#[cfg(feature = "weak-refs")]
pub(crate) use weak_ref_objects::*;
