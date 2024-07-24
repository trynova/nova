// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{fs, path::PathBuf};

use nova_vm::ecmascript::{
    execution::{
        agent::{GcAgent, Options},
        DefaultHostHooks,
    },
    scripts_and_modules::script::{parse_script, script_evaluation},
};
use oxc_allocator::Allocator;

#[test]
fn object_prototype_tests() {
    let d: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "sources",
        "objectPrototype.test.js",
    ]
    .iter()
    .collect();
    let contents = fs::read_to_string(d.clone()).expect("Should have been able to read the file");

    let allocator = Allocator::default();
    let mut agent = GcAgent::new(Options::default(), &DefaultHostHooks);
    let realm = agent.create_default_realm();
    agent.run_in_realm(&realm, |agent| {
        let realm = agent.current_realm_id();
        let script =
            parse_script(&allocator, contents.into_boxed_str(), realm, false, None).unwrap();
        let _ = script_evaluation(agent, script).unwrap_or_else(|err| {
            panic!(
                "Test '{}' failed: {:?}",
                d.display(),
                err.to_string(agent).as_str(agent)
            )
        });
    });
}
