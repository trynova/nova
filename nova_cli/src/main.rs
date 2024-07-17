// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
mod helper;

use std::{cell::RefCell, collections::VecDeque, fmt::Debug};

use clap::{Parser as ClapParser, Subcommand};
use helper::{exit_with_parse_errors, initialize_global_object};
use nova_vm::ecmascript::{
    execution::{
        agent::{HostHooks, Job, Options},
        initialize_host_defined_realm, Agent, Realm,
    },
    scripts_and_modules::script::{parse_script, script_evaluation},
    types::{Object, Value},
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

        #[arg(short, long)]
        no_strict: bool,

        /// The files to evaluate
        #[arg(required = true)]
        paths: Vec<String>,
    },

    /// Runs the REPL
    Repl {
        #[arg(short, long)]
        verbose: bool,
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
                let script = match parse_script(&allocator, file.into(), realm, !no_strict, None) {
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
        Command::Repl { verbose } => {
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

            loop {
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                let script = match parse_script(&allocator, input.into(), realm, true, None) {
                    Ok(script) => script,
                    Err((file, errors)) => {
                        exit_with_parse_errors(errors, "<stdin>", &file);
                    }
                };
                let result = script_evaluation(&mut agent, script);
                match result {
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
                    }
                }
            }
        }
    }
    Ok(())
}
