// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{fs, path::PathBuf};

use nova_vm::ecmascript::{
    execution::{
        agent::{BoxedAgent, Options},
        initialize_default_realm, DefaultHostHooks,
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
    let mut agent = BoxedAgent::new(Options::default(), &DefaultHostHooks);
    agent.with(|agent, _r| {
        initialize_default_realm(agent);
        let realm = agent.current_realm_id();
        let script = parse_script(&allocator, contents.into_boxed_str(), realm, false, None).unwrap();
        let _ = script_evaluation(agent, script).unwrap_or_else(|err| {
            panic!(
                "Test '{}' failed: {:?}",
                d.display(),
                err.to_string(agent).as_str(agent)
            )
        });
    });
}
