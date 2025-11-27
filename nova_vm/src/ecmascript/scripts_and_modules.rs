// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! # [16 ECMAScript Language: Scripts and Modules](https://tc39.es/ecma262/#sec-ecmascript-language-scripts-and-modules)

use module::module_semantics::source_text_module_records::SourceTextModule;

use crate::{
    engine::context::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use self::script::Script;

pub mod module;
pub mod script;
pub mod source_code;

#[derive(Clone, Copy)]
pub(crate) enum ScriptOrModule<'a> {
    /// ## [16.1 Scripts](https://tc39.es/ecma262/#sec-scripts)
    Script(Script<'a>),
    /// ### [16.2.1.7 Source Text Module Records](https://tc39.es/ecma262/#sec-source-text-module-records)
    SourceTextModule(SourceTextModule<'a>),
}

impl core::fmt::Debug for ScriptOrModule<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ScriptOrModule::Script(script) => script.fmt(f),
            ScriptOrModule::SourceTextModule(module) => module.fmt(f),
        }
    }
}

bindable_handle!(ScriptOrModule);

impl HeapMarkAndSweep for ScriptOrModule<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            ScriptOrModule::Script(s) => s.mark_values(queues),
            ScriptOrModule::SourceTextModule(m) => m.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            ScriptOrModule::Script(s) => s.sweep_values(compactions),
            ScriptOrModule::SourceTextModule(m) => m.sweep_values(compactions),
        }
    }
}
