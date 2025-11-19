// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
mod helper;
mod theme;

use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt::Debug,
    ops::Deref,
    path::PathBuf,
    ptr::NonNull,
    rc::Rc,
    thread,
    time::Duration,
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
                ModuleRequest, Referrer, abstract_module_records::AbstractModule,
                cyclic_module_records::GraphLoadingStateRecord, finish_loading_imported_module,
                source_text_module_records::parse_module,
            },
            script::{HostDefined, parse_script, script_evaluation},
        },
        types::{Object, String as JsString, Value},
    },
    engine::{
        Global,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
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
    macrotask_queue: RefCell<Vec<Job>>,
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

    fn has_macrotasks(&self) -> bool {
        !self.macrotask_queue.borrow().is_empty()
    }

    fn pop_macrotask(&self) -> Option<Job> {
        let mut off_thread_job_queue = self.macrotask_queue.borrow_mut();
        let mut counter = 0u8;
        while !off_thread_job_queue.is_empty() {
            counter = counter.wrapping_add(1);
            for (i, job) in off_thread_job_queue.iter().enumerate() {
                if job.is_finished() {
                    let job = off_thread_job_queue.swap_remove(i);
                    return Some(job);
                }
            }
            if counter == 0 {
                thread::sleep(Duration::from_millis(5));
            } else {
                core::hint::spin_loop();
            }
        }
        None
    }
}

impl HostHooks for CliHostHooks {
    fn enqueue_generic_job(&self, job: Job) {
        self.macrotask_queue.borrow_mut().push(job);
    }

    fn enqueue_promise_job(&self, job: Job) {
        self.promise_job_queue.borrow_mut().push_back(job);
    }

    fn enqueue_timeout_job(&self, _timeout_job: Job, _milliseconds: u64) {}

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
        let specifier = specifier.to_string_lossy(agent);
        let specifier_target = if let Some(specifier) = specifier.strip_prefix("./") {
            let referrer_path = referrer
                .host_defined(agent)
                .unwrap()
                .downcast::<PathBuf>()
                .unwrap();
            let parent = referrer_path
                .parent()
                .expect("Attempted to get sibling file of root");
            parent.join(specifier)
        } else if specifier.starts_with("../") {
            let referrer_path = referrer
                .host_defined(agent)
                .unwrap()
                .downcast::<PathBuf>()
                .unwrap();
            referrer_path
                .join(specifier.deref())
                .canonicalize()
                .expect("Failed to canonicalize target path")
        } else {
            match specifier {
                std::borrow::Cow::Borrowed(str) => PathBuf::from(str),
                std::borrow::Cow::Owned(string) => PathBuf::from(string),
            }
        };
        let realm = referrer.realm(agent, gc);
        let module_map = realm
            .host_defined(agent)
            .expect("No referrer realm [[HostDefined]] slot")
            .downcast::<ModuleMap>()
            .expect("No referrer realm ModuleMap");
        if let Some(module) = module_map.get(agent, &specifier_target, gc) {
            finish_loading_imported_module(
                agent,
                referrer,
                module_request,
                payload,
                Ok(module),
                gc,
            );
            return;
        }
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
            Some(Rc::new(specifier_target.clone())),
            gc,
        )
        .map(|m| {
            let global_m = Global::new(agent, m.unbind().into());
            module_map.add(specifier_target, global_m);
            m.into()
        })
        .map_err(|err| {
            agent.throw_exception(ExceptionType::Error, err.first().unwrap().to_string(), gc)
        });
        finish_loading_imported_module(agent, referrer, module_request, payload, result, gc);
    }
}

struct ModuleMap {
    map: RefCell<HashMap<PathBuf, Global<AbstractModule<'static>>>>,
}

impl ModuleMap {
    fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    fn add(&self, path: PathBuf, module: Global<AbstractModule<'static>>) {
        self.map.borrow_mut().insert(path, module);
    }

    fn get<'a>(
        &self,
        agent: &Agent,
        path: &PathBuf,
        gc: NoGcScope<'a, '_>,
    ) -> Option<AbstractModule<'a>> {
        self.map.borrow().get(path).map(|g| g.get(agent, gc))
    }
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
            fn run_microtask_queue<'gc>(
                agent: &mut Agent,
                host_hooks: &CliHostHooks,
                mut gc: GcScope<'gc, '_>,
            ) -> JsResult<'gc, ()> {
                while let Some(job) = host_hooks.pop_promise_job() {
                    job.run(agent, gc.reborrow()).unbind()?.bind(gc.nogc());
                }
                Ok(())
            }

            fn print_result(
                agent: &mut Agent,
                result: JsResult<Value>,
                verbose: bool,
                gc: GcScope,
            ) {
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
                                .string_repr(agent, gc)
                                .as_wtf8(agent)
                                .to_string_lossy()
                        );
                        std::process::exit(1);
                    }
                }
            }

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
            let module_map = Rc::new(ModuleMap::new());
            realm.initialize_host_defined(&mut agent, module_map.clone());
            let last_index = paths.len() - 1;
            for (index, path) in paths.into_iter().enumerate() {
                // SAFETY: Still valid.
                let host_hooks = unsafe { host_hooks.as_ref() };
                agent.run_in_realm(
                    &realm,
                    |agent, mut gc| -> Result<(), Box<dyn std::error::Error>> {
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
                                    exit_with_parse_errors(errors, &path, &source_text)
                                }
                            };
                            module_map
                                .add(absolute_path, Global::new(agent, module.unbind().into()));
                            agent
                                .run_parsed_module(
                                    module.unbind(),
                                    Some(module_map.clone()),
                                    gc.reborrow(),
                                )
                                .unbind()
                                .bind(gc.nogc())
                        } else {
                            let script = match parse_script(
                                agent,
                                source_text,
                                realm,
                                !no_strict,
                                Some(Rc::new(absolute_path.clone())),
                                gc.nogc(),
                            ) {
                                Ok(script) => script,
                                Err(errors) => {
                                    // Borrow the string data from the Agent
                                    let source_text = source_text.to_string_lossy(agent);
                                    exit_with_parse_errors(errors, &path, &source_text)
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
                                run_microtask_queue(agent, host_hooks, gc.reborrow())
                                    .unbind()
                                    .bind(gc.nogc());
                            // SAFETY: not shared.
                            microtask_result.map(|_| unsafe { result.take(agent) }.bind(gc.nogc()))
                        } else {
                            result
                        };

                        print_result(agent, result.unbind(), verbose, gc);
                        Ok(())
                    },
                )?;
            }
            {
                // SAFETY: Still valid.
                let host_hooks = unsafe { host_hooks.as_ref() };
                if host_hooks.has_macrotasks() {
                    while let Some(job) = host_hooks.pop_macrotask() {
                        agent.run_job(job, |agent, result, mut gc| {
                            let result = if result.is_ok() && host_hooks.has_promise_jobs() {
                                run_microtask_queue(agent, host_hooks, gc.reborrow())
                                    .unbind()
                                    .bind(gc.nogc())
                            } else {
                                result
                            };
                            print_result(
                                agent,
                                result.map(|_| Value::Undefined).unbind(),
                                false,
                                gc,
                            );
                        });
                    }
                }
            }
            agent.remove_realm(realm);
            drop(agent);
            // SAFETY: Host hooks are no longer used as agent is dropped.
            drop(unsafe { Box::from_raw(host_hooks.as_ptr()) });
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
