// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use eval_source::EvalSource;

use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};

use self::script::ScriptIdentifier;

use super::builtins::module::Module;

pub mod eval_source;
pub mod module;
pub mod script;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScriptOrModule {
    EvalSource(EvalSource),
    Module(Module),
    Script(ScriptIdentifier),
}

impl HeapMarkAndSweep for ScriptOrModule {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            ScriptOrModule::EvalSource(data) => data.mark_values(queues),
            ScriptOrModule::Script(data) => data.mark_values(queues),
            ScriptOrModule::Module(data) => data.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            ScriptOrModule::EvalSource(data) => data.sweep_values(compactions),
            ScriptOrModule::Script(data) => data.sweep_values(compactions),
            ScriptOrModule::Module(data) => data.sweep_values(compactions),
        }
    }
}
