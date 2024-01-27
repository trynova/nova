use std::ops::Deref;

use crate::{
    ecmascript::{
        execution::{Agent, JsResult, RealmIdentifier},
        types::{Function, Object, PropertyDescriptor, Value},
    },
    heap::CreateHeapData,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct ArgumentsList<'a>(pub &'a [Value]);

impl<'a> Deref for ArgumentsList<'a> {
    type Target = &'a [Value];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ArgumentsList<'_> {
    #[inline]
    pub fn get(&self, index: usize) -> Value {
        *self.0.get(index).unwrap_or(&Value::Undefined)
    }
}

pub type RegularFn = fn(&mut Agent, Value, ArgumentsList<'_>) -> JsResult<Value>;
pub type ConstructorFn =
    fn(&mut Agent, Value, ArgumentsList<'_>, Option<Object>) -> JsResult<Value>;

#[derive(Debug, Clone, Copy)]
pub enum Behaviour {
    Regular(RegularFn),
    Constructor(ConstructorFn),
}

pub trait Builtin {
    fn create(agent: &mut Agent) -> JsResult<Object>;
}

#[derive(Debug, Default)]
pub struct BuiltinFunctionArgs {
    pub length: u32,
    pub name: &'static str,
    pub realm: Option<RealmIdentifier>,
    pub prototype: Option<Object>,
    pub prefix: Option<Object>,
}

impl BuiltinFunctionArgs {
    pub fn new(length: u32, name: &'static str, realm: RealmIdentifier) -> Self {
        Self {
            length,
            name,
            realm: Some(realm),
            ..Default::default()
        }
    }
}

/// ### [10.3.3 CreateBuiltinFunction ( behaviour, length, name, additionalInternalSlotsList \[ , realm \[ , prototype \[ , prefix \] \] \] )](https://tc39.es/ecma262/#sec-createbuiltinfunction)
pub fn create_builtin_function(
    agent: &mut Agent,
    _behaviour: Behaviour,
    args: BuiltinFunctionArgs,
) -> Function {
    // 1. If realm is not present, set realm to the current Realm Record.
    let realm_id = args.realm.unwrap_or(agent.current_realm_id());

    // 2. If prototype is not present, set prototype to realm.[[Intrinsics]].[[%Function.prototype%]].
    let prototype = args
        .prototype
        .unwrap_or_else(|| agent.get_realm(realm_id).intrinsics().function_prototype());
    let heap = &mut agent.heap;

    // TODO: Steps 3-4
    // 3. Let internalSlotsList be a List containing the names of all the internal slots that 10.3
    //    requires for the built-in function object that is about to be created.
    // 4. Append to internalSlotsList the elements of additionalInternalSlotsList.

    // 5. Let func be a new built-in function object that, when called, performs the action
    //    described by behaviour using the provided arguments as the values of the corresponding
    //    parameters specified by behaviour. The new function object has internal slots whose names
    //    are the elements of internalSlotsList, and an [[InitialName]] internal slot.
    // 6. Set func.[[Prototype]] to prototype.
    // 7. Set func.[[Extensible]] to true.
    // 8. Set func.[[Realm]] to realm.
    // NOTE: Heap data is implicitly attached to the Realm so I don't think
    //       this matters.
    let _object = heap.create_object_with_prototype(prototype);

    // 9. Set func.[[InitialName]] to null.
    // TODO: This is non-standard.
    let initial_name = heap.create(args.name).into_value();
    // 10. Perform SetFunctionLength(func, length).
    let length = args.length as u8;
    // TODO: Actually set behaviour somewhere
    let func = heap.create_function(initial_name, length, false);

    // TODO: Steps 11-12
    // 11. If prefix is not present, then
    //     a. Perform SetFunctionName(func, name).
    // 12. Else,
    //     a. Perform SetFunctionName(func, name, prefix).

    // 13. Return func.
    Function::from(func)
}

pub fn define_builtin_function(
    agent: &mut Agent,
    _object: Object,
    name: &'static str,
    behaviour: RegularFn,
    length: u32,
    realm: RealmIdentifier,
) -> JsResult<()> {
    let _function = create_builtin_function(
        agent,
        Behaviour::Regular(behaviour),
        BuiltinFunctionArgs::new(length, name, realm),
    );

    Ok(())
}

pub fn define_builtin_property(
    _object: Object,
    _name: &'static str,
    _descriptor: PropertyDescriptor,
) -> JsResult<()> {
    Ok(())
}

pub fn todo_builtin(agent: &mut Agent, _: Value, _: ArgumentsList) -> JsResult<Value> {
    agent.throw_exception(
        crate::ecmascript::execution::agent::ExceptionType::SyntaxError,
        "TODO: Builtin not implemented.",
    );
    Err(Default::default())
}
