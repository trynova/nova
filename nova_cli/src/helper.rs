use nova_vm::ecmascript::{
    builtins::{create_builtin_function, ArgumentsList, Behaviour, BuiltinFunctionArgs},
    execution::{
        agent::{HostHooks, Job, Options},
        initialize_host_defined_realm, Agent, JsResult, Realm, RealmIdentifier,
    },
    scripts_and_modules::script::{parse_script, script_evaluation},
    types::{InternalMethods, IntoValue, Object, PropertyDescriptor, PropertyKey, Value},
};
use oxc_diagnostics::OxcDiagnostic;
use std::{cell::RefCell, collections::VecDeque, fmt::Debug};

/// Initialize the global object with the built-in functions.
fn initialize_global_object(agent: &mut Agent, global: Object) {
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

pub struct CliRunner {
    allocator: oxc_allocator::Allocator,
    host_hooks: &'static CliHostHooks,
    agent: Option<Agent>,
    realm_id: RealmIdentifier,
}

impl CliRunner {
    pub fn new(print_internals: bool) -> Self {
        let host_hooks: &CliHostHooks = &*Box::leak(Box::default());
        let mut agent = Agent::new(
            Options {
                disable_gc: false,
                print_internals,
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

        let realm_id = agent.current_realm_id();
        Self {
            allocator: Default::default(),
            host_hooks,
            agent: Some(agent),
            realm_id,
        }
    }

    pub fn run_script_and_microtasks(
        &mut self,
        script: Box<str>,
        script_path: &str,
        allow_loose_mode: bool,
    ) -> JsResult<Value> {
        let script = match parse_script(
            &self.allocator,
            script,
            self.realm_id,
            !allow_loose_mode,
            None,
        ) {
            Ok(script) => script,
            Err((file, errors)) => exit_with_parse_errors(errors, script_path, &file),
        };

        let result = script_evaluation(self.agent(), script)?;

        while let Some(job) = self.host_hooks.pop_promise_job() {
            job.run(self.agent())?;
        }

        Ok(result)
    }

    pub fn agent(&mut self) -> &mut Agent {
        self.agent.as_mut().unwrap()
    }
}

impl Drop for CliRunner {
    fn drop(&mut self) {
        // The agent unsafely borrows the allocator and the host hooks, so it
        // has to be dropped first.
        self.agent.take();
    }
}
