// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
mod helper;
mod theme;

use std::{cell::RefCell, collections::VecDeque, fmt::Debug};

use clap::{Parser as ClapParser, Subcommand};
use cliclack::{input, intro, set_theme};
use helper::{exit_with_parse_errors, initialize_global_object};
use nova_vm::ecmascript::{
    execution::{
        agent::{GcAgent, HostHooks, Job, Options},
        Agent,
    },
    scripts_and_modules::script::{parse_script, script_evaluation},
    types::Object,
};
use oxc_parser::Parser;
use oxc_semantic::{SemanticBuilder, SemanticBuilderReturn};
use oxc_span::SourceType;
use theme::DefaultTheme;

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
        nogc: bool,

        #[arg(short, long)]
        no_strict: bool,

        /// The files to evaluate
        #[arg(required = true)]
        paths: Vec<String>,
    },

    /// Runs the REPL
    Repl {},
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

            let SemanticBuilderReturn { errors, .. } = SemanticBuilder::new(&file, source_type)
                .with_check_syntax_error(true)
                .build(&result.program);

            if !errors.is_empty() {
                exit_with_parse_errors(result.errors, &path, &file);
            }

            println!("{:?}", result.program);
        }
        Command::Eval {
            verbose,
            no_strict,
            nogc,
            paths,
        } => {
            let allocator = Default::default();

            let host_hooks: &CliHostHooks = &*Box::leak(Box::default());
            let mut agent = GcAgent::new(
                Options {
                    disable_gc: nogc,
                    print_internals: verbose,
                },
                host_hooks,
            );
            assert!(!paths.is_empty());
            let create_global_object: Option<fn(&mut Agent) -> Object> = None;
            let create_global_this_value: Option<fn(&mut Agent) -> Object> = None;
            let realm = agent.create_realm(
                create_global_object,
                create_global_this_value,
                Some(initialize_global_object),
            );
            let mut is_first = true;
            for path in paths {
                if is_first {
                    is_first = false;
                } else {
                    agent.gc();
                }
                agent.run_in_realm(&realm, |agent| -> Result<(), Box<dyn std::error::Error>> {
                    let realm = agent.current_realm_id();
                    let file = std::fs::read_to_string(&path)?;
                    let script =
                        match parse_script(&allocator, file.into(), realm, !no_strict, None) {
                            Ok(script) => script,
                            Err((file, errors)) => exit_with_parse_errors(errors, &path, &file),
                        };
                    let mut result = script_evaluation(agent, script);

                    if result.is_ok() {
                        while let Some(job) = host_hooks.pop_promise_job() {
                            if let Err(err) = job.run(agent) {
                                result = Err(err);
                                break;
                            }
                        }
                    }

                    match result {
                        Ok(result) => {
                            if verbose {
                                println!("{:?}", result);
                            }
                        }
                        Err(error) => {
                            eprintln!(
                                "Uncaught exception: {}",
                                error.value().string_repr(agent).as_str(agent)
                            );
                            std::process::exit(1);
                        }
                    }
                    Ok(())
                })?;
            }
            agent.remove_realm(realm);
        }
        Command::Repl {} => {
            let allocator = Default::default();
            let host_hooks: &CliHostHooks = &*Box::leak(Box::default());
            let mut agent = GcAgent::new(
                Options {
                    disable_gc: false,
                    print_internals: true,
                },
                host_hooks,
            );
            let create_global_object: Option<fn(&mut Agent) -> Object> = None;
            let create_global_this_value: Option<fn(&mut Agent) -> Object> = None;
            let realm = agent.create_realm(
                create_global_object,
                create_global_this_value,
                Some(initialize_global_object),
            );

            set_theme(DefaultTheme);
            println!("\n\n");
            let mut placeholder = "Enter a line of Javascript".to_string();

            loop {
                intro("Nova Repl (type exit or ctrl+c to exit)")?;
                let input: String = input("").placeholder(&placeholder).interact()?;

                if input.matches("exit").count() == 1 {
                    std::process::exit(0);
                } else if input.matches("gc").count() == 1 {
                    agent.gc();
                    continue;
                }
                placeholder = input.to_string();
                agent.run_in_realm(&realm, |agent| {
                    let script = match parse_script(
                        &allocator,
                        input.into(),
                        agent.current_realm_id(),
                        true,
                        None,
                    ) {
                        Ok(script) => script,
                        Err((file, errors)) => {
                            exit_with_parse_errors(errors, "<stdin>", &file);
                        }
                    };
                    let result = script_evaluation(agent, script);
                    match result {
                        Ok(result) => {
                            println!("{:?}\n", result);
                        }
                        Err(error) => {
                            eprintln!(
                                "Uncaught exception: {}",
                                error.value().string_repr(agent).as_str(agent)
                            );
                        }
                    }
                });
            }
        }
    }
    Ok(())
}
