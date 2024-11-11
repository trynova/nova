use nova_vm::ecmascript::{
    builtins::{create_builtin_function, ArgumentsList, Behaviour, BuiltinFunctionArgs},
    execution::{agent::ExceptionType, Agent, JsResult},
    types::{InternalMethods, IntoValue, Object, PropertyDescriptor, PropertyKey, String, Value},
};
use nova_vm::engine::context::GcScope;
use oxc_diagnostics::OxcDiagnostic;

/// Initialize the global object with the built-in functions.
pub fn initialize_global_object(agent: &mut Agent, mut gc: GcScope<'_, '_>, global: Object) {
    // `print` function
    fn print(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,

        _this: Value,
        args: ArgumentsList,
    ) -> JsResult<Value> {
        if args.len() == 0 {
            println!();
        } else {
            println!("{}", args[0].to_string(agent, gc)?.as_str(agent));
        }
        Ok(Value::Undefined)
    }
    // 'readTextFile' function
    fn read_text_file(
        agent: &mut Agent,
        _gc: GcScope<'_, '_>,

        _: Value,
        args: ArgumentsList,
    ) -> JsResult<Value> {
        if args.len() != 1 {
            return Err(agent
                .throw_exception_with_static_message(ExceptionType::Error, "Expected 1 argument"));
        }
        let Ok(path) = String::try_from(args.get(0)) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::Error,
                "Expected a string argument",
            ));
        };

        let file = std::fs::read_to_string(path.as_str(agent))
            .map_err(|e| agent.throw_exception(ExceptionType::Error, e.to_string()))?;
        Ok(String::from_string(agent, file).into_value())
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
            gc.reborrow(),
            property_key,
            PropertyDescriptor {
                value: Some(function.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            },
        )
        .unwrap();

    let function = create_builtin_function(
        agent,
        Behaviour::Regular(read_text_file),
        BuiltinFunctionArgs::new(1, "readTextFile", agent.current_realm_id()),
    );
    let property_key = PropertyKey::from_static_str(agent, "readTextFile");
    global
        .internal_define_own_property(
            agent,
            gc.reborrow(),
            property_key,
            PropertyDescriptor {
                value: Some(function.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
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
        Box::new(oxc_diagnostics::GraphicalReportHandler::new())
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
