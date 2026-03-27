// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A datastructure for keeping track of the loaded modules.

use std::{borrow::Cow, cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

use nova_vm::{
    ecmascript::{AbstractModule, Agent},
    engine::{Global, NoGcScope},
};

pub(crate) fn specifier_target<F: FnOnce() -> Rc<PathBuf>>(
    specifier: Cow<'_, str>,
    get_referrer_path: F,
) -> PathBuf {
    if let Some(specifier) = specifier.strip_prefix("./") {
        let referrer_path = get_referrer_path();
        let parent = referrer_path
            .parent()
            .expect("Attempted to get sibling file of root");
        parent.join(specifier)
    } else if specifier.starts_with("../") {
        get_referrer_path()
            .join(&*specifier)
            .canonicalize()
            .expect("Failed to canonicalize target path")
    } else {
        match specifier {
            std::borrow::Cow::Borrowed(str) => PathBuf::from(str),
            std::borrow::Cow::Owned(string) => PathBuf::from(string),
        }
    }
}

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
