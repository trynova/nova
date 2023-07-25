use crate::{
    execution::{Agent, JsResult, Realm},
    types::{Function, Object, Value},
};

#[derive(Debug)]
pub struct ArgumentsList<'a>(&'a [Value]);

pub type RegularFn = fn(&mut Agent, Value, ArgumentsList<'_>) -> JsResult<Value>;
type ConstructorFn = fn(&mut Agent, Value, ArgumentsList<'_>, Option<Object>) -> JsResult<Value>;

#[derive(Debug)]
pub enum Behaviour {
    Regular(RegularFn),
    Constructor(ConstructorFn),
}

pub trait Builtin {
    fn create<'a>(realm: &'a mut Realm<'a, 'a>) -> JsResult<Object>;
}

#[derive(Debug, Default)]
pub struct BuiltinFunctionArgs<'a, 'ctx, 'host> {
    pub length: u32,
    pub name: &'static str,
    pub realm: Option<&'a mut Realm<'ctx, 'host>>,
    pub prototype: Option<Object>,
    pub prefix: Option<Object>,
}

impl<'a, 'ctx: 'a, 'host: 'ctx> BuiltinFunctionArgs<'a, 'ctx, 'host> {
    pub fn new(length: u32, name: &'static str, realm: &'a mut Realm<'ctx, 'host>) -> Self {
        Self {
            length,
            name,
            realm: Some(realm),
            ..Default::default()
        }
    }
}

/// 10.3.3 CreateBuiltinFunction ( behaviour, length, name, additionalInternalSlotsList [ , realm [ , prototype [ , prefix ] ] ] )
/// https://tc39.es/ecma262/#sec-createbuiltinfunction
pub fn create_builtin_function<'a, 'b: 'a>(
    behaviour: Behaviour,
    args: BuiltinFunctionArgs<'a, 'b, 'b>,
) -> Function {
    // 1. If realm is not present, set realm to the current Realm Record.
    let realm = args.realm.unwrap(); // TODO: load record

    // 2. If prototype is not present, set prototype to realm.[[Intrinsics]].[[%Function.prototype%]].
    let prototype = if let Some(prototype) = args.prototype {
        prototype
    } else {
        realm.intrinsics().function()
    };

    // 3. Let internalSlotsList be a List containing the names of all the internal slots that 10.3
    //    requires for the built-in function object that is about to be created.
    // 4. Append to internalSlotsList the elements of additionalInternalSlotsList.

    // 5. Let func be a new built-in function object that, when called, performs the action
    //    described by behaviour using the provided arguments as the values of the corresponding
    //    parameters specified by behaviour. The new function object has internal slots whose names
    //    are the elements of internalSlotsList, and an [[InitialName]] internal slot.

    // 10. Perform SetFunctionLength(func, length).

    // 11. If prefix is not present, then
    //     a. Perform SetFunctionName(func, name).
    // 12. Else,
    //     a. Perform SetFunctionName(func, name, prefix).

    // 13. Return func.
    todo!();
}

pub fn define_builtin_function<'a>(
    object: Object,
    name: &'static str,
    behaviour: RegularFn,
    length: u32,
    realm: &'a mut Realm<'a, 'a>,
) -> JsResult<()> {
    let function = create_builtin_function(
        Behaviour::Regular(behaviour),
        BuiltinFunctionArgs::new(length, name, realm),
    );

    Ok(())
}

pub fn define_builtin_property(object: Object, name: &'static str, value: Value) {}

pub fn todo_builtin(agent: &mut Agent, _: Value, _: ArgumentsList) -> JsResult<Value> {
    agent.throw_exception(
        crate::execution::agent::ExceptionType::SyntaxError,
        "TODO: Builtin not implemented.",
    );
    Err(())
}
