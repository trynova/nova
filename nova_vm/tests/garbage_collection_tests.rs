use std::{fs, path::PathBuf};

use nova_vm::ecmascript::{
    execution::{
        agent::{GcAgent, Options},
        Agent, DefaultHostHooks,
    },
    scripts_and_modules::script::{parse_script, script_evaluation},
    types::{Object, Value},
};
use oxc_allocator::Allocator;

fn initialize_global_object(agent: &mut Agent, global: Object) {
    use nova_vm::ecmascript::{
        builtins::{create_builtin_function, ArgumentsList, Behaviour, BuiltinFunctionArgs},
        execution::JsResult,
        types::{InternalMethods, IntoValue, PropertyDescriptor, PropertyKey},
    };

    // `print` function
    fn print(agent: &mut Agent, _this: Value, args: ArgumentsList) -> JsResult<Value> {
        if args.len() == 0 {
            println!();
        } else {
            println!("{}", args[0].to_string(agent)?.as_str(agent));
        }
        Ok(Value::Undefined)
    }
    let function = create_builtin_function(
        agent,
        Behaviour::Regular(print),
        BuiltinFunctionArgs::new(1, "print", agent.current_realm_id()),
    );
    let property_key = PropertyKey::from_static_str(agent, "print");
    global
        .internal_define_own_property(
            agent,
            property_key,
            PropertyDescriptor {
                value: Some(function.into_value()),
                ..Default::default()
            },
        )
        .unwrap();
}

#[test]
fn garbage_collection_tests() {
    let d: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "sources",
        "garbageCollectionHeader.test.js",
    ]
    .iter()
    .collect();
    let header_contents =
        fs::read_to_string(d.clone()).expect("Should have been able to read the file");
    let d: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "sources",
        "garbageCollectionCall.test.js",
    ]
    .iter()
    .collect();
    let call_contents =
        fs::read_to_string(d.clone()).expect("Should have been able to read the file");

    let allocator = Allocator::default();
    let mut agent = GcAgent::new(Options::default(), &DefaultHostHooks);
    let create_global_object: Option<fn(&mut Agent) -> Object> = None;
    let create_global_this_value: Option<fn(&mut Agent) -> Object> = None;
    let realm = agent.create_realm(
        create_global_object,
        create_global_this_value,
        Some(initialize_global_object),
    );
    agent.run_in_realm(&realm, |agent| {
        let realm = agent.current_realm_id();
        let script = parse_script(
            &allocator,
            header_contents.into_boxed_str(),
            realm,
            false,
            None,
        )
        .unwrap();
        let _ = script_evaluation(agent, script).unwrap_or_else(|err| {
            panic!(
                "Header evaluation failed: '{}' failed: {:?}",
                d.display(),
                err.value().string_repr(agent).as_str(agent)
            )
        });
    });
    agent.gc();

    for i in 0..2 {
        agent.run_in_realm(&realm, |agent| {
            let realm = agent.current_realm_id();
            let script = parse_script(
                &allocator,
                call_contents.clone().into_boxed_str(),
                realm,
                false,
                None,
            )
            .unwrap();
            let _ = script_evaluation(agent, script).unwrap_or_else(|err| {
                println!("Error kind: {:?}", err.value());
                panic!(
                    "Loop index run {} '{}' failed: {:?}",
                    i,
                    d.display(),
                    err.value().string_repr(agent).as_str(agent)
                )
            });
        });
        agent.gc();
    }
}
