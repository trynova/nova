use nova_vm::{
    ecmascript::{
        builtins::{ArgumentsList, Behaviour, BuiltinFunctionArgs, create_builtin_function},
        execution::{
            Agent, DefaultHostHooks, JsResult,
            agent::{GcAgent, Options, RealmRoot},
        },
        scripts_and_modules::script::{Script, parse_script, script_evaluation},
        types::{
            InternalMethods, IntoValue, Object, PropertyDescriptor, PropertyKey,
            String as JsString, Value,
        },
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::{HeapRootData, Rootable, Scopable},
    },
};

fn initialize_global(agent: &mut Agent, global: Object, mut gc: GcScope) {
    let global = global.scope(agent, gc.nogc());

    // `print` function, but for benchmarks make it a noop
    fn print<'gc>(
        _agent: &mut Agent,
        _this: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let _args = args.bind(gc.nogc());
        Ok(Value::Undefined)
    }

    let function = create_builtin_function(
        agent,
        Behaviour::Regular(print),
        BuiltinFunctionArgs::new(1, "print"),
        gc.nogc(),
    );
    let property_key = PropertyKey::from_static_str(agent, "print", gc.nogc());
    global
        .get(agent)
        .internal_define_own_property(
            agent,
            property_key.unbind(),
            PropertyDescriptor {
                value: Some(function.into_value().unbind()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            },
            gc.reborrow(),
        )
        .unwrap();
}

pub struct ParsedScript {
    agent: GcAgent,
    realm: RealmRoot,
    script: HeapRootData,
}

impl ParsedScript {
    pub fn new(source_str: &str, gc: bool) -> Self {
        let mut agent = GcAgent::new(
            Options {
                disable_gc: !gc,
                print_internals: false,
            },
            &DefaultHostHooks,
        );
        let create_global_object: Option<for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>> =
            None;
        let create_global_this_value: Option<
            for<'a> fn(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
        > = None;
        let realm = agent.create_realm(
            create_global_object,
            create_global_this_value,
            Some(initialize_global),
        );
        let script = agent.run_in_realm(&realm, |agent, gc| -> HeapRootData {
            let source_text = JsString::from_str(agent, source_str, gc.nogc());
            let realm = agent.current_realm(gc.nogc());
            let script = parse_script(agent, source_text, realm, true, None, gc.nogc())
                .expect("parse error");
            Rootable::to_root_repr(script).unwrap_err()
        });
        ParsedScript {
            agent,
            realm,
            script,
        }
    }

    pub fn run(self) {
        let ParsedScript {
            mut agent,
            realm,
            script,
        } = self;

        let script = Script::from_heap_data(script).unwrap();
        agent.run_in_realm(&realm, |agent, gc| {
            script_evaluation(agent, script, gc).expect("execution error");
        });
    }
}
