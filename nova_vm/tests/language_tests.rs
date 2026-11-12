// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{fs, path::PathBuf};

use nova_vm::{
    ecmascript::{
        AgentOptions, DefaultHostHooks, GcAgent, String, parse_script, script_evaluation,
    },
    engine::Bindable,
};

fn run_test_file(file_name: &str) {
    let d: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "sources", file_name]
        .iter()
        .collect();
    let contents = fs::read_to_string(d.clone()).expect("Should have been able to read the file");

    let mut agent = GcAgent::new(AgentOptions::default(), &DefaultHostHooks);
    let realm = agent.create_default_realm();
    agent.run_in_realm(&realm, |agent, mut gc| {
        let realm = agent.current_realm(gc.nogc());
        let source_text = String::from_string(agent, contents, gc.nogc());
        let script = parse_script(agent, source_text, realm, false, None, gc.nogc()).unwrap();
        if let Err(err) = script_evaluation(agent, script.unbind(), gc.reborrow()) {
            panic!(
                "Test '{}' failed: {:?}",
                d.display(),
                err.unbind().to_string(agent, gc).to_string_lossy(agent)
            )
        }
    });
}

#[test]
fn destructuring_default_tests() {
    run_test_file("destructuringDefault.test.js");
}

#[test]
fn block_function_declaration_tests() {
    run_test_file("blockFunctionDeclaration.test.js");
}

#[test]
fn class_private_accessor_tests() {
    run_test_file("classPrivateAccessor.test.js");
}
