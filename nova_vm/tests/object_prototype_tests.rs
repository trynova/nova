use std::{fs, path::PathBuf};

use nova_vm::ecmascript::{
    execution::{agent::Options, initialize_default_realm, Agent, DefaultHostHooks},
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
    let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
    initialize_default_realm(&mut agent);
    let realm = agent.current_realm_id();
    let script = parse_script(&allocator, contents.into_boxed_str(), realm, None).unwrap();
    let _ = script_evaluation(&mut agent, script).unwrap_or_else(|err| {
        panic!(
            "Test '{}' failed with error: {:?}",
            d.display(),
            err.to_string(&mut agent).as_str(&mut agent)
        )
    });
}
