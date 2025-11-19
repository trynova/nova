// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    cell::RefCell,
    collections::VecDeque,
    ops::Deref,
    sync::{LazyLock, atomic::AtomicBool, mpsc},
    thread,
    time::Duration,
};

// Record the start time of the program.
// To be used for the `now` function for time measurement.
static START_TIME: LazyLock<std::time::Instant> = LazyLock::new(std::time::Instant::now);

use nova_vm::{
    ecmascript::{
        builtins::{
            ArgumentsList, Behaviour, BuiltinFunctionArgs, RegularFn, SharedArrayBuffer,
            create_builtin_function,
        },
        execution::{
            Agent, JsResult,
            agent::{ExceptionType, GcAgent, HostHooks, Job, Options, unwrap_try},
        },
        scripts_and_modules::script::{parse_script, script_evaluation},
        types::{
            BigInt, Function, InternalMethods, IntoValue, Object, OrdinaryObject,
            PropertyDescriptor, PropertyKey, String, Value,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
};
use oxc_diagnostics::OxcDiagnostic;

use crate::{ChildToHostMessage, CliHostHooks, HostToChildMessage};

struct CliChildHooks {
    promise_job_queue: RefCell<VecDeque<Job>>,
    macrotask_queue: RefCell<Vec<Job>>,
    receiver: mpsc::Receiver<HostToChildMessage>,
    host_sender: mpsc::SyncSender<ChildToHostMessage>,
    ready_to_leave: AtomicBool,
}

// RefCell doesn't implement Debug
impl std::fmt::Debug for CliChildHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliHostHooks")
            //.field("promise_job_queue", &*self.promise_job_queue.borrow())
            .finish()
    }
}

impl CliChildHooks {
    fn new(
        host_sender: mpsc::SyncSender<ChildToHostMessage>,
    ) -> (Self, mpsc::SyncSender<HostToChildMessage>) {
        let (sender, receiver) = mpsc::sync_channel(1);
        (
            Self {
                promise_job_queue: Default::default(),
                macrotask_queue: Default::default(),
                receiver,
                host_sender,
                ready_to_leave: Default::default(),
            },
            sender,
        )
    }

    fn is_ready_to_leave(&self) -> bool {
        self.ready_to_leave
            .load(std::sync::atomic::Ordering::Relaxed)
    }

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

impl HostHooks for CliChildHooks {
    fn enqueue_generic_job(&self, job: Job) {
        self.macrotask_queue.borrow_mut().push(job);
    }

    fn enqueue_promise_job(&self, job: Job) {
        self.promise_job_queue.borrow_mut().push_back(job);
    }

    fn enqueue_timeout_job(&self, _timeout_job: Job, _milliseconds: u64) {}

    fn get_host_data(&self) -> &dyn std::any::Any {
        self
    }
}

/// Initialize the global object with the built-in functions.
pub fn initialize_global_object(agent: &mut Agent, global: Object, gc: GcScope) {
    let gc = gc.into_nogc();
    let global = global.scope(agent, gc);
    // `print` function
    fn print<'gc>(
        agent: &mut Agent,
        _this: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let args = args.bind(gc.nogc());
        if args.is_empty() {
            println!();
        } else {
            println!(
                "{}",
                args[0]
                    .unbind()
                    .to_string(agent, gc)?
                    .as_wtf8(agent)
                    .to_string_lossy()
            );
        }
        Ok(Value::Undefined)
    }

    // 'readTextFile' function
    fn read_text_file<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let args = args.bind(gc);
        if args.len() != 1 {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::Error,
                "Expected 1 argument",
                gc,
            ));
        }
        let Ok(path) = String::try_from(args.get(0)) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::Error,
                "Expected a string argument",
                gc,
            ));
        };

        let file = match std::fs::read_to_string(path.to_string_lossy(agent).deref()) {
            Ok(file) => file,
            Err(e) => {
                return Err(agent.throw_exception(ExceptionType::Error, e.to_string(), gc));
            }
        };
        Ok(String::from_string(agent, file, gc).into_value())
    }

    // 'now' function
    fn now<'gc>(
        agent: &mut Agent,
        _this: Value,
        _args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nanos = START_TIME.elapsed().as_nanos();
        let bigint = BigInt::from_u128(agent, nanos, gc.into_nogc());
        Ok(bigint.into_value())
    }

    let function = create_builtin_function(
        agent,
        Behaviour::Regular(print),
        BuiltinFunctionArgs::new(1, "print"),
        gc,
    );
    let property_key = PropertyKey::from_static_str(agent, "print", gc);
    unwrap_try(global.get(agent).try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_prototype_method_descriptor(function),
        None,
        gc,
    ));

    let function = create_builtin_function(
        agent,
        Behaviour::Regular(read_text_file),
        BuiltinFunctionArgs::new(1, "readTextFile"),
        gc,
    );
    let property_key = PropertyKey::from_static_str(agent, "readTextFile", gc);
    unwrap_try(global.get(agent).try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_prototype_method_descriptor(function),
        None,
        gc,
    ));

    let function = create_builtin_function(
        agent,
        Behaviour::Regular(now),
        BuiltinFunctionArgs::new(0, "now"),
        gc,
    );
    let property_key = PropertyKey::from_static_str(agent, "now", gc);
    unwrap_try(global.get(agent).try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_prototype_method_descriptor(function),
        None,
        gc,
    ));
}

/// # sleep
///
/// A function that takes a millisecond argument and sleeps the execution
/// for approximately that duration.
fn sleep<'gc>(
    agent: &mut Agent,
    _: Value,
    args: ArgumentsList,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let gc = gc.into_nogc();
    let Value::Integer(duration) = args.get(0).bind(gc) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "expected first argument to be an integer",
            gc,
        ));
    };
    let duration = duration.into_i64().max(0).unsigned_abs();
    thread::sleep(Duration::from_millis(duration));
    Ok(Value::Undefined)
}

fn create_obj_func(
    agent: &mut Agent,
    obj: OrdinaryObject,
    str: &'static str,
    func: RegularFn,
    len: u32,
    gc: NoGcScope,
) {
    let function = create_builtin_function(
        agent,
        Behaviour::Regular(func),
        BuiltinFunctionArgs::new(len, str),
        gc,
    );
    let property_key = PropertyKey::from_static_str(agent, str, gc);
    unwrap_try(obj.try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_data_descriptor(function),
        None,
        gc,
    ));
}

pub fn initialize_global_object_with_internals(agent: &mut Agent, global: Object, mut gc: GcScope) {
    /// `createRealm` function
    fn create_realm<'gc>(
        agent: &mut Agent,
        _this: Value,
        _args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let create_global_object: Option<for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>> =
            None;
        let create_global_this_value: Option<
            for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
        > = None;
        let realm = agent.create_realm(
            create_global_object,
            create_global_this_value,
            Some(initialize_global_object_with_internals),
            gc,
        );
        Ok(realm.global_object(agent).into_value().unbind())
    }

    /// `detachArrayBuffer` function
    fn detach_array_buffer<'gc>(
        agent: &mut Agent,
        _this: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let args = args.bind(gc.nogc());
        let Value::ArrayBuffer(array_buffer) = args.get(0) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::Error,
                "Cannot detach non ArrayBuffer argument",
                gc.into_nogc(),
            ));
        };
        array_buffer.detach(agent, None, gc.nogc()).unbind()?;
        Ok(Value::Undefined)
    }

    /// `gc` function
    fn run_gc<'gc>(
        agent: &mut Agent,
        _this: Value,
        _args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        agent.gc(gc);
        Ok(Value::Undefined)
    }

    initialize_global_object(agent, global, gc.reborrow());

    let gc = gc.into_nogc();

    let nova_obj = OrdinaryObject::create_empty_object(agent, gc).bind(gc);
    let property_key = PropertyKey::from_static_str(agent, "__nova__", gc);
    unwrap_try(global.try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_data_descriptor(nova_obj),
        None,
        gc,
    ));

    create_obj_func(agent, nova_obj, "createRealm", create_realm, 0, gc);
    create_obj_func(
        agent,
        nova_obj,
        "detachArrayBuffer",
        detach_array_buffer,
        1,
        gc,
    );
    create_obj_func(agent, nova_obj, "gc", run_gc, 0, gc);

    /// `start` function
    fn start<'gc>(
        agent: &mut Agent,
        _this: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let script_src = args.get(0).bind(gc.nogc());

        let Ok(script_src) = String::try_from(script_src) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "expected script source to be a string",
                gc.into_nogc(),
            ));
        };

        let source_text = script_src.as_str(agent).unwrap().to_owned();
        let host_hooks = agent
            .get_host_data()
            .downcast_ref::<CliHostHooks>()
            .unwrap();
        let host_sender = host_hooks.own_sender.clone();
        let (child_hooks, child_sender) = CliChildHooks::new(host_sender);
        host_hooks.add_child(child_sender);
        let _ = thread::spawn(|| {
            let child_hooks = &*Box::leak(Box::new(child_hooks));
            let mut child_agent = GcAgent::new(
                Options {
                    disable_gc: false,
                    print_internals: false,
                },
                child_hooks,
            );
            let create_global_object: Option<
                for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
            > = None;
            let create_global_this_value: Option<
                for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
            > = None;
            let initialize_global: Option<fn(&mut Agent, Object, GcScope)> =
                Some(initialize_child_global_object);
            let realm = child_agent.create_realm(
                create_global_object,
                create_global_this_value,
                initialize_global,
            );

            let is_ok = child_agent.run_in_realm(&realm, |child_agent, mut gc| -> bool {
                let source_text = String::from_string(child_agent, source_text, gc.nogc());
                let realm = child_agent.current_realm(gc.nogc());
                let script = match parse_script(
                    child_agent,
                    source_text,
                    realm,
                    // Probably shouldn't run in strict mode; bite me!
                    true,
                    None,
                    gc.nogc(),
                ) {
                    Ok(script) => script,
                    Err(_) => {
                        return false;
                    }
                };

                child_hooks
                    .host_sender
                    .send(ChildToHostMessage::Joined)
                    .unwrap();

                let result = script_evaluation(child_agent, script.unbind(), gc.reborrow())
                    .unbind()
                    .bind(gc.nogc());

                if let Err(error) = result {
                    eprintln!(
                        "Uncaught exception: {}",
                        error
                            .value()
                            .unbind()
                            .string_repr(child_agent, gc)
                            .to_string_lossy(child_agent)
                    );
                    return false;
                }
                true
            });
            if !is_ok || child_hooks.is_ready_to_leave() {
                return;
            }
            fn run_microtask_queue<'gc>(
                agent: &mut GcAgent,
                host_hooks: &CliChildHooks,
            ) -> Option<()> {
                while let Some(job) = host_hooks.pop_promise_job() {
                    if host_hooks.is_ready_to_leave() {
                        return None;
                    }
                    agent.run_job(job, |_, result, _| result.ok())?;
                }
                Some(())
            }
            while (child_hooks.has_promise_jobs() || child_hooks.has_macrotasks())
                && !child_hooks.is_ready_to_leave()
            {
                let microtask_result = run_microtask_queue(&mut child_agent, child_hooks);
                if microtask_result.is_none() {
                    return;
                }
                if let Some(job) = child_hooks.pop_macrotask() {
                    if child_hooks.is_ready_to_leave() {
                        return;
                    }
                    let is_ok = child_agent.run_job(job, |_, result, _| result.is_ok());
                    if !is_ok {
                        return;
                    }
                }
            }
        });

        let message = host_hooks.receiver.recv().unwrap();

        let ChildToHostMessage::Joined = message else {
            unreachable!()
        };

        Ok(Value::Undefined)
    }

    /// # broadcast
    ///
    /// A function that takes a SharedArrayBuffer and an Int32 or BigInt and
    /// broadcasts the two values to all concurrent agents. The function blocks
    /// until all agents have retrieved the message. Note, this assumes that
    /// all agents that were started are still running.
    fn broadcast<'gc>(
        agent: &mut Agent,
        _this: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let sab = args.get(0).bind(gc);

        let Ok(sab) = SharedArrayBuffer::try_from(sab) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "expected first argument to be a SharedArrayBuffer",
                gc,
            ));
        };

        let hooks = agent
            .get_host_data()
            .downcast_ref::<CliHostHooks>()
            .unwrap();

        let sdb = sab.get_data_block(agent);
        for child in hooks.child_senders.borrow().iter() {
            while child
                .send(HostToChildMessage::Broadcast(sdb.clone()))
                .is_err()
            {}
        }

        Ok(Value::Undefined)
    }
    /// # getReport
    ///
    /// A function that reads an incoming string from any agent, and returns it
    /// if it exists, or returns `null` otherwise.
    fn get_report<'gc>(
        agent: &mut Agent,
        _: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let hooks = agent
            .get_host_data()
            .downcast_ref::<CliHostHooks>()
            .unwrap();

        let Ok(message) = hooks.receiver.try_recv() else {
            return Ok(Value::Null);
        };

        let ChildToHostMessage::Report(message) = message else {
            unreachable!()
        };

        Ok(Value::from_string(agent, message, gc.into_nogc()))
    }

    let agent_obj = OrdinaryObject::create_empty_object(agent, gc);
    let property_key = PropertyKey::from_static_str(agent, "agent", gc);
    unwrap_try(nova_obj.try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_data_descriptor(agent_obj),
        None,
        gc,
    ));

    create_obj_func(agent, agent_obj, "start", start, 1, gc);
    create_obj_func(agent, agent_obj, "broadcast", broadcast, 2, gc);
    create_obj_func(agent, agent_obj, "getReport", get_report, 0, gc);
    create_obj_func(agent, agent_obj, "sleep", sleep, 1, gc);
    let property_key = PropertyKey::from_static_str(agent, "now", gc);
    let function = Function::try_from(
        unwrap_try(global.try_get_own_property(agent, property_key, None, gc))
            .unwrap()
            .value
            .unwrap(),
    )
    .unwrap();
    let property_key = PropertyKey::from_static_str(agent, "monotonicNow", gc);
    unwrap_try(agent_obj.try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_data_descriptor(function),
        None,
        gc,
    ));
}

fn initialize_child_global_object(agent: &mut Agent, global: Object, mut gc: GcScope) {
    initialize_global_object(agent, global, gc.reborrow());

    let gc = gc.into_nogc();

    // The agent script will be run in an environment that has an object `$262`
    // with a property `agent` with the following properties:

    /// # receiveBroadcast
    ///
    /// A function that takes a function and calls the function when it has
    /// received a broadcast from the parent, passing it the broadcast as two
    /// arguments, a SharedArrayBuffer and an Int32 or BigInt. This function
    /// may return before a broadcast is received (eg. to return to an event
    /// loop to await a message) and no code should follow the call to this
    /// function.
    fn receive_broadcast<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let cb = args.get(0).bind(gc.nogc());

        let Ok(cb) = Function::try_from(cb) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "expected first argument to be a Function",
                gc.into_nogc(),
            ));
        };

        let hooks = agent
            .get_host_data()
            .downcast_ref::<CliChildHooks>()
            .unwrap();

        let message = match hooks.receiver.recv() {
            Ok(m) => m,
            Err(err) => {
                return Err(agent.throw_exception(
                    ExceptionType::Error,
                    err.to_string(),
                    gc.into_nogc(),
                ));
            }
        };

        let HostToChildMessage::Broadcast(shared_block) = message;

        let sab = SharedArrayBuffer::new_from_data_block(agent, shared_block, gc.nogc());

        let _ = cb
            .unbind()
            .call(agent, Value::Null, &mut [sab.into_value().unbind()], gc);

        Ok(Value::Undefined)
    }

    /// # report
    ///
    /// A function that accepts a single "message" argument, which is converted
    /// to a string and placed in a transmit queue whence the parent will
    /// retrieve it. Messages should be short. (Note that string conversion has
    /// been implicit since the introduction of this host API, but is now
    /// explicit.)
    fn report<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let report = args.get(0).bind(gc.nogc());

        let report = report
            .unbind()
            .string_repr(agent, gc.reborrow())
            .unbind()
            .bind(gc.nogc());

        let hooks = agent
            .get_host_data()
            .downcast_ref::<CliChildHooks>()
            .unwrap();

        let report = report.to_string_lossy(agent).into_owned();
        match hooks.host_sender.send(ChildToHostMessage::Report(report)) {
            Ok(_) => Ok(Value::Undefined),
            Err(err) => {
                Err(agent.throw_exception(ExceptionType::Error, err.to_string(), gc.into_nogc()))
            }
        }
    }

    /// leaving
    ///
    /// A function that signals that the agent is done and may be terminated
    /// (if possible).
    fn leaving<'gc>(
        agent: &mut Agent,
        _: Value,
        _: ArgumentsList,
        _: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let hooks = agent
            .get_host_data()
            .downcast_ref::<CliChildHooks>()
            .unwrap();
        hooks
            .ready_to_leave
            .store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(Value::Undefined)
    }

    let property_key = PropertyKey::from_static_str(agent, "$262", gc);
    let test262_obj = OrdinaryObject::create_empty_object(agent, gc);
    unwrap_try(global.try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor {
            value: Some(test262_obj.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        },
        None,
        gc,
    ));

    let property_key = PropertyKey::from_static_str(agent, "agent", gc);
    let agent_obj = OrdinaryObject::create_empty_object(agent, gc);
    unwrap_try(test262_obj.try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor {
            value: Some(agent_obj.into_value()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        },
        None,
        gc,
    ));

    let property_key = PropertyKey::from_static_str(agent, "receiveBroadcast", gc);
    let function = create_builtin_function(
        agent,
        Behaviour::Regular(receive_broadcast),
        BuiltinFunctionArgs::new(1, "receiveBroadcast"),
        gc,
    );
    unwrap_try(agent_obj.try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_prototype_method_descriptor(function),
        None,
        gc,
    ));
    let property_key = PropertyKey::from_static_str(agent, "report", gc);
    let function = create_builtin_function(
        agent,
        Behaviour::Regular(report),
        BuiltinFunctionArgs::new(1, "report"),
        gc,
    );
    unwrap_try(agent_obj.try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_prototype_method_descriptor(function),
        None,
        gc,
    ));
    let property_key = PropertyKey::from_static_str(agent, "leaving", gc);
    let function = create_builtin_function(
        agent,
        Behaviour::Regular(leaving),
        BuiltinFunctionArgs::new(0, "leaving"),
        gc,
    );
    unwrap_try(agent_obj.try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_prototype_method_descriptor(function),
        None,
        gc,
    ));
    let property_key = PropertyKey::from_static_str(agent, "sleep", gc);
    let function = create_builtin_function(
        agent,
        Behaviour::Regular(sleep),
        BuiltinFunctionArgs::new(1, "sleep"),
        gc,
    );
    unwrap_try(agent_obj.try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_prototype_method_descriptor(function),
        None,
        gc,
    ));
    let property_key = PropertyKey::from_static_str(agent, "now", gc);
    let function = Function::try_from(
        unwrap_try(global.try_get_own_property(agent, property_key, None, gc))
            .unwrap()
            .value
            .unwrap(),
    )
    .unwrap();
    let property_key = PropertyKey::from_static_str(agent, "monotonicNow", gc);
    unwrap_try(agent_obj.try_define_own_property(
        agent,
        property_key,
        PropertyDescriptor::new_prototype_method_descriptor(function),
        None,
        gc,
    ));
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
        eprint!("{report:?}");
    }
    eprintln!();

    std::process::exit(1);
}
