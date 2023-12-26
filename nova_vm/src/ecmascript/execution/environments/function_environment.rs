use super::DeclarativeEnvironment;
use crate::{
    ecmascript::{
        execution::{agent::ExceptionType, Agent, JsResult},
        types::{Function, Object, Value},
    },
    heap::GetHeapData,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThisBindingStatus {
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
/// ArrowFunction function and references super, its Function Environment Record
/// also contains the state that is used to perform super method invocations
/// from within the function.
#[derive(Debug)]
pub struct FunctionEnvironment {
    /// ### \[\[ThisValue\]\]
    ///
    /// This is the this value used for this invocation of the function.
    this_value: Option<Value>,

    /// ### \[\[ThisBindingStatus\]\]
    ///
    /// If the value is LEXICAL, this is an ArrowFunction and does not have a
    /// local this value.
    this_binding_status: ThisBindingStatus,

    /// ### \[\[FunctionObject\]\]
    ///
    /// The function object whose invocation caused this Environment Record to
    /// be created.
    function_object: Function,

    /// ### \[\[NewTarget\]\]
    ///
    /// If this Environment Record was created by the \[\[Construct\]\] internal
    /// method, \[\[NewTarget\]\] is the value of the \[\[Construct\]\]
    /// newTarget parameter. Otherwise, its value is undefined.
    new_target: Option<Object>,

    /// Function Environment Records support all of the Declarative Environment
    /// Record methods listed in Table 16 and share the same specifications for
    /// all of those methods except for HasThisBinding and HasSuperBinding.
    declarative_environment: DeclarativeEnvironment,
}

impl std::ops::Deref for FunctionEnvironment {
    type Target = DeclarativeEnvironment;
    fn deref(&self) -> &Self::Target {
        &self.declarative_environment
    }
}

impl FunctionEnvironment {
    /// ### [9.1.2.4 NewFunctionEnvironment ( F, newTarget )](https://tc39.es/ecma262/#sec-newfunctionenvironment)
    ///
    /// The abstract operation NewFunctionEnvironment takes arguments F (an
    /// ECMAScript function object) and newTarget (an Object or undefined) and
    /// returns a Function Environment Record.
    pub(crate) fn new(
        agent: &Agent,
        function_object: Function,
        new_target: Option<Object>,
    ) -> FunctionEnvironment {
        let ecmascript_function = match function_object {
            Function::ECMAScriptFunction(d) => &agent.heap.get(d).ecmascript_function,
            _ => unreachable!(),
        };
        // 1. Let env be a new Function Environment Record containing no bindings.
        FunctionEnvironment {
            this_value: None,

            // 2. Set env.[[FunctionObject]] to F.
            function_object,

            // 3. If F.[[ThisMode]] is LEXICAL, set env.[[ThisBindingStatus]] to LEXICAL.
            // 4. Else, set env.[[ThisBindingStatus]] to UNINITIALIZED.
            this_binding_status: ThisBindingStatus::Uninitialized,

            // 5. Set env.[[NewTarget]] to newTarget.
            new_target,

            // 6. Set env.[[OuterEnv]] to F.[[Environment]].
            declarative_environment: DeclarativeEnvironment::new(Some(
                ecmascript_function.environment,
            )),
        }
        // 7. Return env.
    }

    /// ### [9.1.1.3.1 BindThisValue ( V )](https://tc39.es/ecma262/#sec-bindthisvalue)
    ///
    /// The BindThisValue concrete method of a Function Environment Record envRec takes argument V (an ECMAScript language value) and returns either a normal completion containing an ECMAScript language value or a throw completion. It performs the following steps when called:
    pub(crate) fn bind_this_value(&mut self, agent: &mut Agent, value: Value) -> JsResult<Value> {
        // 1. Assert: envRec.[[ThisBindingStatus]] is not LEXICAL.
        debug_assert!(self.this_binding_status != ThisBindingStatus::Lexical);

        // 2. If envRec.[[ThisBindingStatus]] is INITIALIZED, throw a
        // ReferenceError exception.
        if self.this_binding_status == ThisBindingStatus::Initialized {
            return Err(agent.throw_exception(
                ExceptionType::ReferenceError,
                "Identifier is already initialized.",
            ));
        }

        // 3. Set envRec.[[ThisValue]] to V.
        self.this_value = Some(value);

        // 4. Set envRec.[[ThisBindingStatus]] to INITIALIZED.
        self.this_binding_status = ThisBindingStatus::Initialized;

        // 5. Return V.
        Ok(value)
    }

    /// ### [9.1.1.3.2 HasThisBinding ( )](https://tc39.es/ecma262/#sec-function-environment-records-hasthisbinding)
    ///
    /// The HasThisBinding concrete method of a Function Environment Record
    /// envRec takes no arguments and returns a Boolean.
    pub(crate) fn has_this_binding(&self) -> bool {
        // 1. If envRec.[[ThisBindingStatus]] is LEXICAL, return false;
        // otherwise, return true.
        self.this_binding_status != ThisBindingStatus::Lexical
    }

    /// ### [9.1.1.3.3 HasSuperBinding ( )](https://tc39.es/ecma262/#sec-function-environment-records-hassuperbinding)
    ///
    /// The HasSuperBinding concrete method of a Function Environment Record
    /// envRec takes no arguments and returns a Boolean.
    pub(crate) fn has_super_binding(&self, _agent: &Agent) -> bool {
        // 1. If envRec.[[ThisBindingStatus]] is LEXICAL, return false.
        if self.this_binding_status == ThisBindingStatus::Lexical {
            return false;
        }

        // TODO: 2. If envRec.[[FunctionObject]].[[HomeObject]] is undefined, return false; otherwise, return true.
        todo!("Finish this")
    }

    /// ### [9.1.1.3.5 GetSuperBase ( )](https://tc39.es/ecma262/#sec-getsuperbase)
    ///
    /// The GetSuperBase concrete method of a Function Environment Record envRec
    /// takes no arguments and returns either a normal completion containing
    /// either an Object, null, or undefined, or a throw completion.
    pub(crate) fn get_super_base(&self) -> Value {
        todo!("Finish this")

        // 1. Let home be envRec.[[FunctionObject]].[[HomeObject]].
        // 2. If home is undefined, return undefined.
        // 3. Assert: home is an Object.
        // 4. Return ? home.[[GetPrototypeOf]]().
    }
}
