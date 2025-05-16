// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### [6.2.12 Private Names](https://tc39.es/ecma262/#sec-private-names)
//!
//! The Private Name specification type is used to describe a globally unique
//! value (one which differs from any other Private Name, even if they are
//! otherwise indistinguishable) which represents the key of a private class
//! element (field, method, or accessor). Each Private Name has an associated
//! immutable \[\[Description\]\] which is a String value. A Private Name may
//! be installed on any ECMAScript object with PrivateFieldAdd or
//! PrivateMethodOrAccessorAdd, and then read or written using PrivateGet and
//! PrivateSet.

use crate::{
    ecmascript::{
        execution::Agent,
        types::{PropertyKey, String},
    },
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

/// ### [6.2.12 Private Names](https://tc39.es/ecma262/#sec-private-names)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PrivateName(u32);

impl PrivateName {
    /// Get the inner u32 value from a PrivateName.
    pub(crate) const fn into_u32(self) -> u32 {
        self.0
    }

    /// Create a PrivateName from a u32.
    pub(crate) const fn from_u32(data: u32) -> Self {
        Self(data)
    }

    /// Try to get PrivateName's \[\[Description]] field.
    ///
    /// This method only succeeds if it is being asked in a JavaScript
    /// execution scope where the PrivateName is accessible in the current
    /// private environment.
    pub(crate) fn get_description<'a>(
        self,
        agent: &Agent,
        gc: NoGcScope<'a, '_>,
    ) -> Option<String<'a>> {
        let env = agent.current_private_environment(gc)?;
        env.resolve_description(agent, self, gc)
    }
}

impl From<PrivateName> for PropertyKey<'static> {
    #[inline(always)]
    fn from(value: PrivateName) -> Self {
        Self::PrivateName(value)
    }
}

// SAFETY: Trivially safe.
unsafe impl Bindable for PrivateName {
    type Of<'a> = PrivateName;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        self
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        self
    }
}

// Private names are never garbage collected; only PrivateEnvironments are.
impl HeapMarkAndSweep for PrivateName {
    fn mark_values(&self, _: &mut WorkQueues) {}
    fn sweep_values(&mut self, _: &CompactionLists) {}
}
