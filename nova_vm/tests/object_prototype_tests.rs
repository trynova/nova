// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{fs, path::PathBuf};

use nova_vm::ecmascript::{
    execution::{agent::Options, initialize_default_realm, Agent, DefaultHostHooks},
    scripts_and_modules::script::{parse_script, script_evaluation},
    types::String as JsString,
};

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

    let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
    initialize_default_realm(&mut agent);
    let realm = agent.current_realm_id();
    let source_text = JsString::from_string(&mut agent, contents);
    let script = parse_script(&mut agent, source_text, realm, false, None).unwrap();
    let _ = script_evaluation(&mut agent, script).unwrap_or_else(|err| {
        panic!(
            "Test '{}' failed: {:?}",
            d.display(),
            err.to_string(&mut agent).as_str(&agent)
        )
    });
}
