// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod theme;

use std::{fmt::Debug, rc::Rc};

use clap::{Parser as ClapParser, Subcommand};
use cliclack::{input, intro, set_theme};
use nova_cli::{self as lib, Instance, InstanceConfig, ModuleMap};
use nova_vm::{
    ecmascript::{String as JsString, parse_module, parse_script, script_evaluation},
    engine::{Bindable, Global, Scopable},
    register_probes,
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
        /// Exposes internal functions needed by Test262.
        #[arg(long)]
        expose_internals: bool,

        /// Evaluates the last file as an ECMAScript module.
        #[arg(short, long)]
        module: bool,

        /// Sets the \[\[CanBlock]] value of the Agent Record to false.
        #[arg(long)]
        no_block: bool,

        /// Disabled garbage collection.
        #[arg(long)]
        nogc: bool,

        /// Evaluates all scripts in sloppy mode.
        #[arg(short, long)]
        no_strict: bool,

        /// The files to evaluate.
        #[arg(required = true)]
        paths: Vec<String>,

        /// Prints all internal data during evaluation.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Runs the REPL
    Repl {
        #[arg(long)]
        expose_internals: bool,

        #[arg(long)]
        print_internals: bool,

        #[arg(long)]
        disable_gc: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    register_probes().unwrap();

    match args.command {
        Command::Parse { path } => {
            let file = std::fs::read_to_string(&path)?;
            let allocator = Default::default();
            let source_type: SourceType = Default::default();
            let parser = Parser::new(&allocator, &file, source_type.with_typescript(false));
            let result = parser.parse();

            if !result.errors.is_empty() {
                lib::exit_with_parse_errors(result.errors, &path, &file);
            }

            let SemanticBuilderReturn { errors, .. } = SemanticBuilder::new()
                .with_check_syntax_error(true)
                .build(&result.program);

            if !errors.is_empty() {
                lib::exit_with_parse_errors(result.errors, &path, &file);
            }

            println!("{:?}", result.program);
        }
        Command::Eval {
            verbose,
            module,
            no_block,
            no_strict,
            nogc,
            expose_internals,
            paths,
        } => {
            let config = InstanceConfig {
                block: !no_block,
                enable_gc: !nogc,
                verbose,
                expose_internals,
                strict: !no_strict,
            };
            let mut instance = Instance::new(config);

            assert!(!paths.is_empty());
            let module_map = ModuleMap::new();
            instance.initialize_module_map(module_map);
            let (config, host_hooks, realm) = instance.split_mut();
            let last_index = paths.len() - 1;
            for (index, path) in paths.into_iter().enumerate() {
                realm.run_in(|agent, mut gc| -> Result<(), Box<dyn std::error::Error>> {
                    let absolute_path = std::fs::canonicalize(&path)?;
                    let file = std::fs::read_to_string(&absolute_path)?;
                    let source_text = JsString::from_string(agent, file, gc.nogc());
                    let realm = agent.current_realm(gc.nogc());
                    let result = if module && last_index == index {
                        let module = match parse_module(
                            agent,
                            source_text.unbind(),
                            realm,
                            Some(Rc::new(absolute_path.clone())),
                            gc.nogc(),
                        ) {
                            Ok(module) => module,
                            Err(errors) => {
                                // Borrow the string data from the Agent
                                let source_text = source_text.to_string_lossy(agent);
                                lib::exit_with_parse_errors(errors, &path, &source_text)
                            }
                        };
                        let module_map: Rc<ModuleMap> = lib::get_module_map(agent, gc.nogc());
                        module_map.add(absolute_path, Global::new(agent, module.unbind().into()));
                        agent
                            .run_module(module.unbind(), Some(module_map.clone()), gc.reborrow())
                            .unbind()
                            .bind(gc.nogc())
                    } else {
                        let script = match parse_script(
                            agent,
                            source_text,
                            realm,
                            config.strict,
                            Some(Rc::new(absolute_path.clone())),
                            gc.nogc(),
                        ) {
                            Ok(script) => script,
                            Err(errors) => {
                                // Borrow the string data from the Agent
                                let source_text = source_text.to_string_lossy(agent);
                                lib::exit_with_parse_errors(errors, &path, &source_text)
                            }
                        };
                        script_evaluation(agent, script.unbind(), gc.reborrow())
                            .unbind()
                            .bind(gc.nogc())
                    };

                    let result = if let Ok(result) = result
                        && host_hooks.has_promise_jobs()
                    {
                        let result = result.scope(agent, gc.nogc());
                        let microtask_result =
                            lib::run_microtask_queue(agent, host_hooks, gc.reborrow())
                                .unbind()
                                .bind(gc.nogc());
                        // SAFETY: not shared.
                        microtask_result.map(|_| unsafe { result.take(agent) }.bind(gc.nogc()))
                    } else {
                        result
                    };

                    lib::print_result(agent, result.unbind(), verbose, gc);
                    Ok(())
                })?;
            }
            instance.run_tasks();
        }
        Command::Repl {
            expose_internals,
            print_internals,
            disable_gc,
        } => {
            let config = InstanceConfig {
                enable_gc: !disable_gc,
                verbose: print_internals,
                expose_internals,
                // Never allow blocking in the REPL.
                block: false,
                ..Default::default()
            };
            let mut instance = Instance::new(config);

            set_theme(DefaultTheme);
            println!("\n");
            let mut placeholder = "Enter a line of Javascript".to_string();

            // Register a signal handler for Ctrl+C
            let _ = ctrlc::set_handler(|| {
                std::process::exit(0);
            });

            let (_config, _host_hooks, realm) = instance.split_mut();
            loop {
                intro("Nova Repl")?;
                let input: String = input("").placeholder(&placeholder).interact()?;

                if input.matches("exit").count() == 1 {
                    std::process::exit(0);
                } else if input.matches("gc").count() == 1 {
                    realm.run_gc();
                    continue;
                }
                placeholder = input.to_string();
                realm.run_in(|agent, mut gc| {
                    let realm = agent.current_realm(gc.nogc());
                    let source_text = JsString::from_string(agent, input, gc.nogc());
                    let script =
                        match parse_script(agent, source_text, realm, true, None, gc.nogc()) {
                            Ok(script) => script,
                            Err(errors) => {
                                lib::exit_with_parse_errors(errors, "<stdin>", &placeholder);
                            }
                        };
                    let result = script_evaluation(agent, script.unbind(), gc.reborrow());
                    match result {
                        Ok(result) => {
                            println!("{result:?}\n");
                        }
                        Err(error) => {
                            eprintln!(
                                "Uncaught exception: {}",
                                error
                                    .value()
                                    .unbind()
                                    .string_repr(agent, gc)
                                    .to_string_lossy(agent)
                            );
                        }
                    }
                });
            }
        }
    }
    Ok(())
}
