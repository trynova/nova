// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::types::OrdinaryObject;

#[derive(Debug, Clone, Default)]
pub struct WeakSetHeapData<'gen> {
    pub(crate) object_index: Option<OrdinaryObject<'gen>>,
}
