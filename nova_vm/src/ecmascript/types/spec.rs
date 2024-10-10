// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data_block;
mod property_descriptor;
mod reference;
#[cfg(feature = "array-buffer")]
pub(crate) use data_block::DataBlock;
pub use property_descriptor::PropertyDescriptor;
pub(crate) use reference::*;
