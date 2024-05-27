use clap::{Parser as ClapParser, Subcommand};
use nova_vm::ecmascript::{
    execution::{agent::Options, initialize_host_defined_realm, Agent, DefaultHostHooks, Realm},
    scripts_and_modules::script::{parse_script, script_evaluation},
    types::Object,
};
use oxc_parser::Parser;
use oxc_span::SourceType;

/// A JavaScript engine
#[derive(Debug, ClapParser)] // requires `derive` feature
#[command(name = "nova")]
#[command(about = "A JavaScript engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Parses a file and logs out the AST
    Parse {
        /// The path of the file to parse
        path: String,
    },

    /// Evaluates a file
    Eval {
        #[arg(short, long)]
        verbose: bool,

        /// The file to evaluate
        path: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Command::Parse { path } => {
            let file = std::fs::read_to_string(path)?;
            let allocator = Default::default();
            let source_type: SourceType = Default::default();
            let parser = Parser::new(&allocator, &file, source_type.with_typescript(false));
            let result = parser.parse();

            println!("{:?}", result.program);
        }
        Command::Eval { verbose, path } => {
            let file = std::fs::read_to_string(path)?;
            let allocator = Default::default();

            let mut agent = Agent::new(Options::default(), &DefaultHostHooks);
            {
                let create_global_object: Option<fn(&mut Realm) -> Object> = None;
                let create_global_this_value: Option<fn(&mut Realm) -> Object> = None;
                initialize_host_defined_realm(
                    &mut agent,
                    create_global_object,
                    create_global_this_value,
                    Some(initialize_global_object),
                );
            }
            let realm = agent.current_realm_id();

            let script = parse_script(&allocator, file.into(), realm, None).unwrap();
            let result = script_evaluation(&mut agent, script).unwrap();

            if verbose {
                println!("{:?}", result);
            }
        }
    }

    Ok(())
}

fn initialize_global_object(agent: &mut Agent, global: Object) {
    use nova_vm::ecmascript::{
        builtins::{create_builtin_function, ArgumentsList, Behaviour, BuiltinFunctionArgs},
        execution::JsResult,
        types::{InternalMethods, IntoValue, PropertyDescriptor, PropertyKey, Value},
    };

    // `print` function
    fn print(agent: &mut Agent, _this: Value, args: ArgumentsList) -> JsResult<Value> {
        if args.len() == 0 || args[0].is_undefined() {
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
