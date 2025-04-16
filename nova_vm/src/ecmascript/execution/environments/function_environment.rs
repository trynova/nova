// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{
    DeclarativeEnvironment, DeclarativeEnvironmentRecord, Environment, FunctionEnvironment,
};
use crate::ecmascript::types::OrdinaryObject;
use crate::engine::context::{Bindable, NoGcScope};
use crate::engine::unwrap_try;
use crate::{
    ecmascript::{
        builtins::{ECMAScriptFunction, ThisMode},
        execution::{Agent, JsResult, agent::ExceptionType},
        types::{Function, InternalMethods, IntoFunction, IntoValue, Object, String, Value},
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ThisBindingStatus {
    /// Function is an ArrowFunction and does not have a local `this` value.
    Lexical,
    /// Function is a normal function and does not have a bound `this` value.
    Initialized,
    /// Function is a normal function and has a bound `this` value.
    Uninitialized,
}

/// ### [9.1.1.3 Function Environment Records](https://tc39.es/ecma262/#sec-function-environment-records)
///
/// A Function Environment Record is a Declarative Environment Record that is
/// used to represent the top-level scope of a function and, if the function is
/// not an ArrowFunction, provides a this binding. If a function is not an
/// ArrowFunction function and references super, its Function Environment
/// Record also contains the state that is used to perform super method
/// invocations from within the function.
#[derive(Debug)]
pub struct FunctionEnvironmentRecord {
    /// ### \[\[ThisValue\]\]
    ///
    /// This is the this value used for this invocation of the function.
    pub(crate) this_value: Option<Value<'static>>,

    /// ### \[\[ThisBindingStatus\]\]
    ///
    /// If the value is LEXICAL, this is an ArrowFunction and does not have a
    /// local this value.
    pub(crate) this_binding_status: ThisBindingStatus,

    /// ### \[\[FunctionObject\]\]
    ///
    /// The function object whose invocation caused this Environment Record to
    /// be created.
    pub(crate) function_object: Function<'static>,

    /// ### \[\[NewTarget\]\]
    ///
    /// If this Environment Record was created by the \[\[Construct\]\]
    /// internal method, \[\[NewTarget\]\] is the value of the
    /// \[\[Construct\]\] newTarget parameter. Otherwise, its value is
    /// undefined.
    pub(crate) new_target: Option<Object<'static>>,

    /// Function Environment Records support all of the Declarative Environment
    /// Record methods listed in Table 16 and share the same specifications for
    /// all of those methods except for HasThisBinding and HasSuperBinding.
    ///
    /// TODO: Use Struct of Arrays to keep the DeclarativeEnvironment alignside
    /// FunctionEnvironment
    pub(crate) declarative_environment: DeclarativeEnvironment<'static>,
}

impl HeapMarkAndSweep for FunctionEnvironmentRecord {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            this_value,
            this_binding_status: _,
            function_object,
            new_target,
            declarative_environment,
        } = self;
        declarative_environment.mark_values(queues);
        function_object.mark_values(queues);
        new_target.mark_values(queues);
        this_value.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            this_value,
            this_binding_status: _,
            function_object,
            new_target,
            declarative_environment,
        } = self;
        declarative_environment.sweep_values(compactions);
        function_object.sweep_values(compactions);
        new_target.sweep_values(compactions);
        this_value.sweep_values(compactions);
    }
}

/// ### [9.1.2.4 NewFunctionEnvironment ( F, newTarget )](https://tc39.es/ecma262/#sec-newfunctionenvironment)
///
/// The abstract operation NewFunctionEnvironment takes arguments F (an
/// ECMAScript function object) and newTarget (an Object or undefined) and
/// returns a Function Environment Record.
pub(crate) fn new_function_environment<'a>(
    agent: &mut Agent,
    f: ECMAScriptFunction,
    new_target: Option<Object>,
    gc: NoGcScope<'a, '_>,
) -> FunctionEnvironment<'a> {
    let ecmascript_function_object = &agent[f].ecmascript_function;
    let this_mode = ecmascript_function_object.this_mode;
    // 1. Let env be a new Function Environment Record containing no bindings.
    let dcl_env = DeclarativeEnvironmentRecord::new(Some(ecmascript_function_object.environment));
    agent.heap.environments.declarative.push(Some(dcl_env));
    let declarative_environment =
        DeclarativeEnvironment::last(&agent.heap.environments.declarative);
    // 2. Set env.[[FunctionObject]] to F.
    let function_object = f.into_function().unbind();
    // 3. If F.[[ThisMode]] is LEXICAL, set env.[[ThisBindingStatus]] to LEXICAL.
    let this_binding_status = if this_mode == ThisMode::Lexical {
        ThisBindingStatus::Lexical
    } else {
        // 4. Else, set env.[[ThisBindingStatus]] to UNINITIALIZED.
        ThisBindingStatus::Uninitialized
    };
    let env = FunctionEnvironmentRecord {
        this_value: None,

        function_object,

        this_binding_status,

        // 5. Set env.[[NewTarget]] to newTarget.
        new_target: new_target.unbind(),

        // 6. Set env.[[OuterEnv]] to F.[[Environment]].
        declarative_environment: declarative_environment.unbind(),
    };
    // 7. Return env.
    agent.heap.environments.push_function_environment(env, gc)
}

/// ### NewClassStaticElementEnvironment ( classConstructor )
///
/// This is a non-standard abstract operation that performs the same steps as
/// NewFunctionEnvironment, but for a class static element's evaluation
/// function. These functions are never visible to ECMAScript code and thus we
/// avoid creating them entirely. The only parameter is the class constructor,
/// which is used as both the this value and the \[\[FunctionObject]] of the
/// new function environment.
pub(crate) fn new_class_static_element_environment<'a>(
    agent: &mut Agent,
    class_constructor: Function,
    gc: NoGcScope<'a, '_>,
) -> FunctionEnvironment<'a> {
    // 1. Let env be a new Function Environment Record containing no bindings.
    let dcl_env = DeclarativeEnvironmentRecord::new(Some(agent.current_lexical_environment(gc)));
    agent.heap.environments.declarative.push(Some(dcl_env));
    let declarative_environment =
        DeclarativeEnvironment::last(&agent.heap.environments.declarative);

    let env = FunctionEnvironmentRecord {
        this_value: Some(class_constructor.into_value().unbind()),

        function_object: class_constructor.unbind(),

        this_binding_status: ThisBindingStatus::Initialized,

        // 5. Set env.[[NewTarget]] to newTarget.
        new_target: None,

        // 6. Set env.[[OuterEnv]] to F.[[Environment]].
        declarative_environment,
    };
    // 7. Return env.
    agent.heap.environments.push_function_environment(env, gc)
}

pub(crate) fn new_class_field_initializer_environment<'a>(
    agent: &mut Agent,
    class_constructor: Function,
    class_instance: Object,
    outer_env: Environment,
    gc: NoGcScope<'a, '_>,
) -> FunctionEnvironment<'a> {
    agent
        .heap
        .environments
        .declarative
        .push(Some(DeclarativeEnvironmentRecord::new(Some(outer_env))));
    let declarative_environment =
        DeclarativeEnvironment::last(&agent.heap.environments.declarative);
    agent.heap.environments.push_function_environment(
        FunctionEnvironmentRecord {
            this_value: Some(class_instance.into_value().unbind()),
            this_binding_status: ThisBindingStatus::Initialized,
            function_object: class_constructor.unbind(),
            new_target: None,
            declarative_environment,
        },
        gc,
    )
}

impl FunctionEnvironment<'_> {
    pub(crate) fn get_this_binding_status(self, agent: &Agent) -> ThisBindingStatus {
        agent[self].this_binding_status
    }

    /// ### [9.1.1.3.4 GetThisBinding ( )](https://tc39.es/ecma262/#sec-function-environment-records-getthisbinding)
    /// The GetThisBinding concrete method of a Function Environment Record
    /// envRec takes no arguments and returns either a normal completion
    /// containing an ECMAScript language value or a throw completion.
    pub(crate) fn get_this_binding<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Assert: envRec.[[ThisBindingStatus]] is not lexical.
        // 2. If envRec.[[ThisBindingStatus]] is uninitialized, throw a ReferenceError exception.
        // 3. Return envRec.[[ThisValue]].
        let env_rec = &agent[self];
        match env_rec.this_binding_status {
            ThisBindingStatus::Lexical => unreachable!(),
            ThisBindingStatus::Initialized => Ok(env_rec.this_value.unwrap()),
            ThisBindingStatus::Uninitialized => Err(agent
                .throw_exception_with_static_message(
                    ExceptionType::ReferenceError,
                    "Uninitialized this binding",
                    gc,
                )
                .unbind()),
        }
    }

    /// ### [9.1.1.1.1 HasBinding ( N )](https://tc39.es/ecma262/#sec-declarative-environment-records-hasbinding-n)
    pub(crate) fn has_binding(self, agent: &Agent, name: String) -> bool {
        agent[self].declarative_environment.has_binding(agent, name)
    }

    /// ### [9.1.1.1.2 CreateMutableBinding ( N, D )](https://tc39.es/ecma262/#sec-declarative-environment-records-createmutablebinding-n-d)
    pub(crate) fn create_mutable_binding(
        self,
        agent: &mut Agent,
        name: String,
        is_deletable: bool,
    ) {
        agent[self]
            .declarative_environment
            .create_mutable_binding(agent, name, is_deletable)
    }

    /// ### [9.1.1.1.3 CreateImmutableBinding ( N, S )](https://tc39.es/ecma262/#sec-declarative-environment-records-createimmutablebinding-n-s)
    pub(crate) fn create_immutable_binding(self, agent: &mut Agent, name: String, is_strict: bool) {
        agent[self]
            .declarative_environment
            .create_immutable_binding(agent, name, is_strict)
    }

    /// ### [9.1.1.1.4 InitializeBinding ( N, V )](https://tc39.es/ecma262/#sec-declarative-environment-records-initializebinding-n-v)
    pub(crate) fn initialize_binding(self, agent: &mut Agent, name: String, value: Value) {
        agent[self]
            .declarative_environment
            .initialize_binding(agent, name, value)
    }

    /// ### [9.1.1.1.5 SetMutableBinding ( N, V, S )](https://tc39.es/ecma262/#sec-declarative-environment-records-setmutablebinding-n-v-s)
    pub(crate) fn set_mutable_binding(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        mut is_strict: bool,
        gc: NoGcScope,
    ) -> JsResult<()> {
        let env_rec = &agent[self];
        let dcl_rec = env_rec.declarative_environment;
        // 1. If envRec does not have a binding for N, then
        if !dcl_rec.has_binding(agent, name) {
            // a. If S is true, throw a ReferenceError exception.
            if is_strict {
                let error_message = format!("Identifier '{}' does not exist.", name.as_str(agent));
                return Err(agent
                    .throw_exception(ExceptionType::ReferenceError, error_message, gc)
                    .unbind());
            }

            // b. Perform ! envRec.CreateMutableBinding(N, true).
            dcl_rec.create_mutable_binding(agent, name, true);

            // c. Perform ! envRec.InitializeBinding(N, V).
            dcl_rec.initialize_binding(agent, name, value);

            // d. Return UNUSED.
            return Ok(());
        };

        let binding = agent[dcl_rec].bindings.get_mut(&name.unbind()).unwrap();

        // 2. If the binding for N in envRec is a strict binding, set S to true.
        if binding.strict {
            is_strict = true;
        }

        // 3. If the binding for N in envRec has not yet been initialized, then
        if binding.value.is_none() {
            // a. Throw a ReferenceError exception.
            let error_message = format!(
                "Identifier '{}' has not been initialized.",
                name.as_str(agent)
            );
            return Err(agent
                .throw_exception(ExceptionType::ReferenceError, error_message, gc)
                .unbind());
        }

        // 4. Else if the binding for N in envRec is a mutable binding, then
        if binding.mutable {
            // a. Change its bound value to V.
            binding.value = Some(value.unbind());
        }
        // 5. Else,
        else {
            // a. Assert: This is an attempt to change the value of an immutable binding.
            debug_assert!(!binding.mutable);

            // b. If S is true, throw a TypeError exception.
            if is_strict {
                let error_message = format!(
                    "Cannot assign to immutable identifier '{}' in strict mode.",
                    name.as_str(agent)
                );
                return Err(agent
                    .throw_exception(ExceptionType::TypeError, error_message, gc)
                    .unbind());
            }
        }

        // 6. Return UNUSED.
        Ok(())
    }

    /// ### [9.1.1.1.6 GetBindingValue ( N, S )](https://tc39.es/ecma262/#sec-declarative-environment-records-getbindingvalue-n-s)
    pub(crate) fn get_binding_value<'gc>(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        agent[self]
            .declarative_environment
            .get_binding_value(agent, name, is_strict, gc)
    }

    /// ### [9.1.1.1.7 DeleteBinding ( N )](https://tc39.es/ecma262/#sec-declarative-environment-records-deletebinding-n)
    pub(crate) fn delete_binding(self, agent: &mut Agent, name: String) -> bool {
        agent[self]
            .declarative_environment
            .delete_binding(agent, name)
    }

    /// ### [9.1.1.1.10 WithBaseObject ( )](https://tc39.es/ecma262/#sec-declarative-environment-records-withbaseobject)
    pub(crate) fn with_base_object(self) -> Option<Object<'static>> {
        // 1. Return undefined.
        None
    }

    /// ### [9.1.1.3.1 BindThisValue ( V )](https://tc39.es/ecma262/#sec-bindthisvalue)
    ///
    /// The BindThisValue concrete method of a Function Environment Record
    /// envRec takes argument V (an ECMAScript language value) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion.
    pub(crate) fn bind_this_value<'gc>(
        self,
        agent: &mut Agent,
        value: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let env_rec = &mut agent[self];
        // 1. Assert: envRec.[[ThisBindingStatus]] is not LEXICAL.
        debug_assert!(env_rec.this_binding_status != ThisBindingStatus::Lexical);

        // 2. If envRec.[[ThisBindingStatus]] is INITIALIZED, throw a
        // ReferenceError exception.
        if env_rec.this_binding_status == ThisBindingStatus::Initialized {
            return Err(agent
                .throw_exception_with_static_message(
                    ExceptionType::ReferenceError,
                    "[[ThisBindingStatus]] is INITIALIZED",
                    gc,
                )
                .unbind());
        }

        // 3. Set envRec.[[ThisValue]] to V.
        env_rec.this_value = Some(value.unbind());

        // 4. Set envRec.[[ThisBindingStatus]] to INITIALIZED.
        env_rec.this_binding_status = ThisBindingStatus::Initialized;

        // 5. Return V.
        Ok(value.bind(gc))
    }

    /// ### [9.1.1.3.2 HasThisBinding ( )](https://tc39.es/ecma262/#sec-function-environment-records-hasthisbinding)
    ///
    /// The HasThisBinding concrete method of a Function Environment Record
    /// envRec takes no arguments and returns a Boolean.
    pub(crate) fn has_this_binding(self, agent: &Agent) -> bool {
        let env_rec = &agent[self];
        // 1. If envRec.[[ThisBindingStatus]] is LEXICAL, return false;
        // otherwise, return true.
        env_rec.this_binding_status != ThisBindingStatus::Lexical
    }

    /// ### [9.1.1.3.3 HasSuperBinding ( )](https://tc39.es/ecma262/#sec-function-environment-records-hassuperbinding)
    ///
    /// The HasSuperBinding concrete method of a Function Environment Record
    /// envRec takes no arguments and returns a Boolean.
    pub(crate) fn has_super_binding(self, agent: &Agent) -> bool {
        let env_rec = &agent[self];
        // 1. If envRec.[[ThisBindingStatus]] is LEXICAL, return false.
        if env_rec.this_binding_status == ThisBindingStatus::Lexical {
            return false;
        }

        // 2. If envRec.[[FunctionObject]].[[HomeObject]] is undefined, return false; otherwise, return true.
        match env_rec.function_object {
            Function::BoundFunction(_) => todo!(),
            Function::BuiltinFunction(_) => unreachable!(),
            Function::ECMAScriptFunction(func) => {
                agent[func].ecmascript_function.home_object.is_some()
            }
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(_) => unreachable!(),
            Function::BuiltinPromiseResolvingFunction(_) => unreachable!(),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        }
    }

    /// ### [9.1.1.3.5 GetSuperBase ( )](https://tc39.es/ecma262/#sec-getsuperbase)
    ///
    /// The GetSuperBase concrete method of a Function Environment Record
    /// envRec takes no arguments and returns either a normal completion
    /// containing either an Object, null, or undefined.
    pub(crate) fn get_super_base<'a>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> Option<Option<Object<'a>>> {
        let env_rec: &FunctionEnvironmentRecord = &agent[self];

        // 1. Let home be envRec.[[FunctionObject]].[[HomeObject]].
        let home = match env_rec.function_object {
            Function::BoundFunction(_) => todo!(),
            Function::BuiltinFunction(_) => unreachable!(),
            // 2. If home is undefined, return undefined.
            Function::ECMAScriptFunction(func) => agent[func].ecmascript_function.home_object?,
            Function::BuiltinGeneratorFunction => todo!(),
            Function::BuiltinConstructorFunction(_) => unreachable!(),
            Function::BuiltinPromiseResolvingFunction(_) => unreachable!(),
            Function::BuiltinPromiseCollectorFunction => todo!(),
            Function::BuiltinProxyRevokerFunction => todo!(),
        };
        // 3. Assert: home is an ordinary object.
        let home = OrdinaryObject::try_from(home).unwrap();
        // 4. Return ! home.[[GetPrototypeOf]]().
        Some(unwrap_try(home.try_get_prototype_of(agent, gc)))
    }
}

impl HeapMarkAndSweep for FunctionEnvironment<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.function_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.into_u32_index();
        *self = Self::from_u32_index(
            self_index
                - compactions
                    .function_environments
                    .get_shift_for_index(self_index),
        );
    }
}
