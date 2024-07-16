// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{cell::RefCell, collections::VecDeque, fmt::Debug};

use clap::{Parser as ClapParser, Subcommand};
use nova_vm::ecmascript::{
    execution::{
        agent::{HostHooks, Job, Options},
        initialize_host_defined_realm, Agent, Realm,
    },
    scripts_and_modules::script::{parse_script, script_evaluation},
    types::{Object, Value},
};
use oxc_diagnostics::{GraphicalTheme, OxcDiagnostic};
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

        #[arg(short, long)]
        no_strict: bool,

        /// The files to evaluate
        #[arg(required = true)]
        paths: Vec<String>,
    },
}

#[derive(Default)]
struct CliHostHooks {
    promise_job_queue: RefCell<VecDeque<Job>>,
}

// RefCell doesn't implement Debug
impl Debug for CliHostHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliHostHooks")
            //.field("promise_job_queue", &*self.promise_job_queue.borrow())
            .finish()
    }
}

impl CliHostHooks {
    fn pop_promise_job(&self) -> Option<Job> {
        self.promise_job_queue.borrow_mut().pop_front()
    }
}

impl HostHooks for CliHostHooks {
    fn enqueue_promise_job(&self, job: Job) {
        self.promise_job_queue.borrow_mut().push_back(job);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Command::Parse { path } => {
            let file = std::fs::read_to_string(&path)?;
            let allocator = Default::default();
            let source_type: SourceType = Default::default();
            let parser = Parser::new(&allocator, &file, source_type.with_typescript(false));
            let result = parser.parse();

            if !result.errors.is_empty() {
                exit_with_parse_errors(result.errors, &path, &file);
            }
            println!("{:?}", result.program);
        }
        Command::Eval {
            verbose,
            no_strict,
            paths,
        } => {
            let allocator = Default::default();

            let host_hooks: &CliHostHooks = &*Box::leak(Box::default());
            let mut agent = Agent::new(
                Options {
                    disable_gc: false,
                    print_internals: verbose,
                },
                host_hooks,
            );
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

            // `final_result` will always be overwritten in the paths loop, but
            // we populate it with a dummy value here so rustc won't complain.
            let mut final_result = Ok(Value::Undefined);

            assert!(!paths.is_empty());
            for path in paths {
                let file = std::fs::read_to_string(&path)?;
                let script = match parse_script(&allocator, file.into(), realm, None, !no_strict) {
                    Ok(script) => script,
                    Err((file, errors)) => exit_with_parse_errors(errors, &path, &file),
                };
                final_result = script_evaluation(&mut agent, script);
                if final_result.is_err() {
                    break;
                }
            }

            if final_result.is_ok() {
                while let Some(job) = host_hooks.pop_promise_job() {
                    if let Err(err) = job.run(&mut agent) {
                        final_result = Err(err);
                        break;
                    }
                }
            }

            match final_result {
                Ok(result) => {
                    if verbose {
                        println!("{:?}", result);
                    }
                }
                Err(error) => {
                    eprintln!(
                        "Uncaught exception: {}",
                        error.value().string_repr(&mut agent).as_str(&agent)
                    );
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

fn exit_with_parse_errors(errors: Vec<OxcDiagnostic>, source_path: &str, source: &str) -> ! {
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
