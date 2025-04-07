// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    engine::context::Bindable,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use self::script::Script;

use super::builtins::module::Module;

pub mod module;
pub mod script;
pub mod source_code;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScriptOrModule<'a> {
    Script(Script<'a>),
    Module(Module<'a>),
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ScriptOrModule<'_> {
    type Of<'a> = ScriptOrModule<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _: crate::engine::context::NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for ScriptOrModule<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            ScriptOrModule::Script(idx) => idx.mark_values(queues),
            ScriptOrModule::Module(idx) => idx.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            ScriptOrModule::Script(idx) => idx.sweep_values(compactions),
            ScriptOrModule::Module(idx) => idx.sweep_values(compactions),
        }
    }
}
