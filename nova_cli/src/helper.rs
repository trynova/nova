use nova_vm::ecmascript::{
    builtins::{create_builtin_function, ArgumentsList, Behaviour, BuiltinFunctionArgs},
    execution::{Agent, JsResult},
    types::{InternalMethods, IntoValue, Object, PropertyDescriptor, PropertyKey, Value},
};
use oxc_diagnostics::{GraphicalTheme, OxcDiagnostic};

/// Initialize the global object with the built-in functions.
pub fn initialize_global_object(agent: &mut Agent, global: Object) {
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

/// Exit the program with parse errors.
pub fn exit_with_parse_errors(errors: Vec<OxcDiagnostic>, source_path: &str, source: &str) -> ! {
    assert!(!errors.is_empty());

    // This seems to be needed for color and Unicode output.
    miette::set_hook(Box::new(|_| {
        use std::io::IsTerminal;
        let mut handler = oxc_diagnostics::GraphicalReportHandler::new();
        if !std::io::stdout().is_terminal() || std::io::stderr().is_terminal() {
            // Fix for https://github.com/oxc-project/oxc/issues/3539
            handler = handler.with_theme(GraphicalTheme::none());
        }
        Box::new(handler)
    }))
    .unwrap();

    eprintln!("Parse errors:");

    // SAFETY: This function never returns, so `source`'s lifetime must last for
    // the duration of the program.
    let source: &'static str = unsafe { std::mem::transmute(source) };
    let named_source = miette::NamedSource::new(source_path, source);

    for error in errors {
        let report = error.with_source_code(named_source.clone());
        eprint!("{:?}", report);
    }
    eprintln!();

    std::process::exit(1);
}
