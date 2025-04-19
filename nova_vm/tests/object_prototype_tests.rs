// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{fs, path::PathBuf};

use nova_vm::{
    ecmascript::{
        execution::{
            DefaultHostHooks,
            agent::{GcAgent, Options},
        },
        scripts_and_modules::script::{parse_script, script_evaluation},
        types::String,
    },
    engine::context::Bindable,
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

    let mut agent = GcAgent::new(Options::default(), &DefaultHostHooks);
    let realm = agent.create_default_realm();
    agent.run_in_realm(&realm, |agent, mut gc| {
        let realm = agent.current_realm(gc.nogc());
        let source_text = String::from_string(agent, contents, gc.nogc());
        let script = parse_script(agent, source_text, realm, false, None, gc.nogc()).unwrap();
        if let Err(err) = script_evaluation(agent, script.unbind(), gc.reborrow()) {
            panic!(
                "Test '{}' failed: {:?}",
                d.display(),
                err.unbind().to_string(agent, gc).as_str(agent)
            )
        }
    });
}
