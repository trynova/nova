use std::{fs, path::PathBuf};

use nova_vm::{
    ecmascript::{
        execution::{
            Agent, DefaultHostHooks,
            agent::{GcAgent, Options},
        },
        scripts_and_modules::script::{parse_script, script_evaluation},
        types::{Object, String, Value},
    },
    engine::context::{Bindable, GcScope},
};

fn initialize_global_object(agent: &mut Agent, global: Object, gc: GcScope) {
    use nova_vm::ecmascript::{
        builtins::{ArgumentsList, Behaviour, BuiltinFunctionArgs, create_builtin_function},
        execution::JsResult,
        types::{InternalMethods, PropertyDescriptor, PropertyKey},
    };

    // `print` function
    fn print<'gc>(
        agent: &mut Agent,
        _this: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        if args.is_empty() {
            println!();
        } else {
            println!("{}", args[0].to_string(agent, gc)?.to_string_lossy(agent));
        }
        Ok(Value::Undefined)
    }
    let function = create_builtin_function(
        agent,
        Behaviour::Regular(print),
        BuiltinFunctionArgs::new(1, "print"),
        gc.nogc(),
    );
    let property_key = PropertyKey::from_static_str(agent, "print", gc.nogc());
    global
        .internal_define_own_property(
            agent,
            property_key.unbind(),
            PropertyDescriptor {
                value: Some(function.unbind().into()),
                ..Default::default()
            },
            gc,
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

    let mut agent = GcAgent::new(Options::default(), &DefaultHostHooks);
    let create_global_object: Option<for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>> = None;
    let create_global_this_value: Option<for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>> =
        None;
    let realm = agent.create_realm(
        create_global_object,
        create_global_this_value,
        Some(initialize_global_object),
    );
    agent.run_in_realm(&realm, |agent, mut gc| {
        let realm = agent.current_realm(gc.nogc());
        let source_text = String::from_string(agent, header_contents, gc.nogc());
        let script = parse_script(agent, source_text, realm, false, None, gc.nogc()).unwrap();
        if let Err(err) = script_evaluation(agent, script.unbind(), gc.reborrow()) {
            panic!(
                "Header evaluation failed: '{}' failed: {:?}",
                d.display(),
                err.value()
                    .unbind()
                    .string_repr(agent, gc)
                    .to_string_lossy(agent)
            )
        }
    });
    agent.gc();

    for i in 0..2 {
        agent.run_in_realm(&realm, |agent, mut gc| {
            let realm = agent.current_realm(gc.nogc());
            let source_text = String::from_string(agent, call_contents.clone(), gc.nogc());
            let script = parse_script(agent, source_text, realm, false, None, gc.nogc()).unwrap();
            if let Err(err) = script_evaluation(agent, script.unbind(), gc.reborrow()) {
                println!("Error kind: {:?}", err.value());
                panic!(
                    "Loop index run {} '{}' failed: {:?}",
                    i,
                    d.display(),
                    err.value()
                        .unbind()
                        .string_repr(agent, gc)
                        .to_string_lossy(agent)
                )
            }
        });
        agent.gc();
    }
}
