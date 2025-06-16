// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
mod helper;
mod theme;

use std::{
    cell::RefCell, collections::VecDeque, fmt::Debug, path::PathBuf, ptr::NonNull, rc::Rc,
    str::FromStr,
};

use clap::{Parser as ClapParser, Subcommand};
use cliclack::{input, intro, set_theme};
use helper::{
    exit_with_parse_errors, initialize_global_object, initialize_global_object_with_internals,
};
use nova_vm::{
    ecmascript::{
        execution::{
            Agent, JsResult,
            agent::{ExceptionType, GcAgent, HostHooks, Job, Options},
        },
        scripts_and_modules::{
            module::module_semantics::{
                ModuleRequest, Referrer, cyclic_module_records::GraphLoadingStateRecord,
                finish_loading_imported_module, source_text_module_records::parse_module,
            },
            script::{HostDefined, parse_script, script_evaluation},
        },
        types::{Object, String as JsString, Value},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
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
        #[arg(long)]
        expose_internals: bool,

        #[arg(short, long)]
        module: bool,

        #[arg(long)]
        nogc: bool,

        #[arg(short, long)]
        no_strict: bool,

        /// The files to evaluate
        #[arg(required = true)]
        paths: Vec<String>,

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
    fn has_promise_jobs(&self) -> bool {
        !self.promise_job_queue.borrow().is_empty()
    }

    fn pop_promise_job(&self) -> Option<Job> {
        self.promise_job_queue.borrow_mut().pop_front()
    }
}

impl HostHooks for CliHostHooks {
    fn enqueue_promise_job(&self, job: Job) {
        self.promise_job_queue.borrow_mut().push_back(job);
    }

    fn load_imported_module<'gc>(
        &self,
        agent: &mut Agent,
        referrer: Referrer<'gc>,
        module_request: ModuleRequest<'gc>,
        _host_defined: Option<HostDefined>,
        payload: &mut GraphLoadingStateRecord<'gc>,
        gc: NoGcScope<'gc, '_>,
    ) {
        let specifier = module_request.specifier(agent);
        let specifier = specifier.as_str(agent);
        let specifier_target = if specifier.starts_with("./") {
            let referrer_path = referrer
                .host_defined(agent)
                .unwrap()
                .downcast::<PathBuf>()
                .unwrap();
            let parent = referrer_path
                .parent()
                .expect("Attempted to get sibling file of root");
            parent.join(&specifier[2..])
        } else if specifier.starts_with("../") {
            let referrer_path = referrer
                .host_defined(agent)
                .unwrap()
                .downcast::<PathBuf>()
                .unwrap();
            referrer_path
                .join(specifier)
                .canonicalize()
                .expect("Failed to canonicalize target path")
        } else {
            PathBuf::from_str(specifier).expect("Failed to parse target path into PathBuf")
        };
        let file = match std::fs::read_to_string(&specifier_target) {
            Ok(file) => file,
            Err(err) => {
                let result = Err(agent.throw_exception(ExceptionType::Error, err.to_string(), gc));
                finish_loading_imported_module(
                    agent,
                    referrer,
                    module_request,
                    payload,
                    result,
                    gc,
                );
                return;
            }
        };
        let source_text = JsString::from_string(agent, file, gc);
        let result = parse_module(
            agent,
            source_text,
            referrer.realm(agent, gc),
            Some(Rc::new(specifier_target)),
            gc,
        )
        .map(|m| m.into())
        .map_err(|err| {
            agent.throw_exception(ExceptionType::Error, err.first().unwrap().to_string(), gc)
        });
        finish_loading_imported_module(agent, referrer, module_request, payload, result, gc);
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

            let SemanticBuilderReturn { errors, .. } = SemanticBuilder::new()
                .with_check_syntax_error(true)
                .build(&result.program);

            if !errors.is_empty() {
                exit_with_parse_errors(result.errors, &path, &file);
            }

            println!("{:?}", result.program);
        }
        Command::Eval {
            verbose,
            module,
            no_strict,
            nogc,
            expose_internals,
            paths,
        } => {
            let host_hooks: NonNull<CliHostHooks> = NonNull::from(Box::leak(Box::default()));
            let mut agent = GcAgent::new(
                Options {
                    disable_gc: nogc,
                    print_internals: verbose,
                },
                // SAFETY: Host hooks is a valid pointer.
                unsafe { host_hooks.as_ref() },
            );
            assert!(!paths.is_empty());
            let create_global_object: Option<
                for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
            > = None;
            let create_global_this_value: Option<
                for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
            > = None;
            let initialize_global: Option<fn(&mut Agent, Object, GcScope)> = if expose_internals {
                Some(initialize_global_object_with_internals)
            } else {
                Some(initialize_global_object)
            };
            let realm = agent.create_realm(
                create_global_object,
                create_global_this_value,
                initialize_global,
            );
            let last_index = paths.len() - 1;
            for (index, path) in paths.into_iter().enumerate() {
                agent.run_in_realm(
                    &realm,
                    |agent, mut gc| -> Result<(), Box<dyn std::error::Error>> {
                        let absolute_path = std::fs::canonicalize(&path)?;
                        let file = std::fs::read_to_string(&absolute_path)?;
                        let source_text = JsString::from_string(agent, file, gc.nogc());
                        let host_defined = Rc::new(absolute_path);
                        let result = if module && last_index == index {
                            agent.run_module_script(
                                source_text.unbind(),
                                Some(host_defined),
                                gc.reborrow(),
                            )
                        } else {
                            let realm = agent.current_realm(gc.nogc());
                            let script = match parse_script(
                                agent,
                                source_text,
                                realm,
                                !no_strict,
                                Some(host_defined),
                                gc.nogc(),
                            ) {
                                Ok(script) => script,
                                Err(errors) => {
                                    // Borrow the string data from the Agent
                                    let source_text = source_text.as_str(agent);
                                    exit_with_parse_errors(errors, &path, source_text)
                                }
                            };
                            script_evaluation(agent, script.unbind(), gc.reborrow())
                        };

                        fn run_microtask_queue<'gc>(
                            agent: &mut Agent,
                            host_hooks: &CliHostHooks,
                            result: JsResult<Value>,
                            mut gc: GcScope<'gc, '_>,
                        ) -> JsResult<'gc, Value<'gc>> {
                            match result.bind(gc.nogc()) {
                                Ok(result) => {
                                    let ok_result = result.unbind().scope(agent, gc.nogc());
                                    while let Some(job) = host_hooks.pop_promise_job() {
                                        job.run(agent, gc.reborrow()).unbind()?.bind(gc.nogc());
                                    }
                                    Ok(ok_result.get(agent).bind(gc.into_nogc()))
                                }
                                Err(_) => result.unbind(),
                            }
                        }

                        // SAFETY: Still valid.
                        let host_hooks = unsafe { host_hooks.as_ref() };
                        let result = if host_hooks.has_promise_jobs() {
                            run_microtask_queue(agent, host_hooks, result.unbind(), gc.reborrow())
                                .unbind()
                                .bind(gc.nogc())
                        } else {
                            result
                        };

                        match result {
                            Ok(result) => {
                                if verbose {
                                    println!("{result:?}");
                                }
                            }
                            Err(error) => {
                                eprintln!(
                                    "Uncaught exception: {}",
                                    error
                                        .value()
                                        .unbind()
                                        .string_repr(agent, gc.reborrow())
                                        .as_str(agent)
                                );
                                std::process::exit(1);
                            }
                        }
                        Ok(())
                    },
                )?;
            }
            agent.remove_realm(realm);
            // SAFETY: Host hooks are no longer used as agent is dropped.
            let _ = unsafe { Box::from_raw(host_hooks.as_ptr()) };
        }
        Command::Repl {
            expose_internals,
            print_internals,
            disable_gc,
        } => {
            let host_hooks: &CliHostHooks = &*Box::leak(Box::default());
            let mut agent = GcAgent::new(
                Options {
                    disable_gc,
                    print_internals,
                },
                host_hooks,
            );
            let create_global_object: Option<
                for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
            > = None;
            let create_global_this_value: Option<
                for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
            > = None;
            let initialize_global: Option<fn(&mut Agent, Object, GcScope)> = if expose_internals {
                Some(initialize_global_object_with_internals)
            } else {
                Some(initialize_global_object)
            };
            let realm = agent.create_realm(
                create_global_object,
                create_global_this_value,
                initialize_global,
            );

            set_theme(DefaultTheme);
            println!("\n");
            let mut placeholder = "Enter a line of Javascript".to_string();

            // Register a signal handler for Ctrl+C
            let _ = ctrlc::set_handler(|| {
                std::process::exit(0);
            });
            loop {
                intro("Nova Repl")?;
                let input: String = input("").placeholder(&placeholder).interact()?;

                if input.matches("exit").count() == 1 {
                    std::process::exit(0);
                } else if input.matches("gc").count() == 1 {
                    agent.gc();
                    continue;
                }
                placeholder = input.to_string();
                agent.run_in_realm(&realm, |agent, mut gc| {
                    let realm = agent.current_realm(gc.nogc());
                    let source_text = JsString::from_string(agent, input, gc.nogc());
                    let script =
                        match parse_script(agent, source_text, realm, true, None, gc.nogc()) {
                            Ok(script) => script,
                            Err(errors) => {
                                exit_with_parse_errors(errors, "<stdin>", &placeholder);
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
                                error.value().unbind().string_repr(agent, gc).as_str(agent)
                            );
                        }
                    }
                });
            }
        }
    }
    Ok(())
}
