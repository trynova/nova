// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};

use self::script::ScriptIdentifier;

use super::builtins::module::Module;

pub mod eval_source;
pub mod module;
pub mod script;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScriptOrModule {
    Script(ScriptIdentifier),
    Module(Module),
}

impl HeapMarkAndSweep for ScriptOrModule {
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
