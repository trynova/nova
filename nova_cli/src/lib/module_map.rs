// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A datastructure for keeping track of the loaded modules.

use std::{cell::RefCell, collections::HashMap, path::PathBuf};

use nova_vm::{
    ecmascript::{
        execution::Agent,
        scripts_and_modules::module::module_semantics::abstract_module_records::AbstractModule,
    },
    engine::{Global, context::NoGcScope},
};

#[derive(Default)]
pub struct ModuleMap {
    map: RefCell<HashMap<PathBuf, Global<AbstractModule<'static>>>>,
}

impl ModuleMap {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&self, path: PathBuf, module: Global<AbstractModule<'static>>) {
        self.map.borrow_mut().insert(path, module);
    }

    pub fn get<'a>(
        &self,
        agent: &Agent,
        path: &PathBuf,
        gc: NoGcScope<'a, '_>,
    ) -> Option<AbstractModule<'a>> {
        self.map.borrow().get(path).map(|g| g.get(agent, gc))
    }
}
