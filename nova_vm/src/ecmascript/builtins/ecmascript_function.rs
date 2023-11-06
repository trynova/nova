use oxc_ast::ast::{FormalParameters, FunctionBody};

use crate::ecmascript::{
    execution::{Agent, EnvironmentIndex, JsResult, PrivateEnvironmentIndex, RealmIdentifier},
    scripts_and_modules::ScriptOrModule,
    types::{Number, Object, PropertyDescriptor, PropertyKey, Value},
};

#[derive(Debug, Clone, Copy)]
pub enum ConstructorKind {
    Base,
    Derived,
}

#[derive(Debug, Clone, Copy)]
pub enum ThisMode {
    Lexical,
    Strict,
    Global,
}

/// 10.2 ECMAScript Function Objects
/// https://tc39.es/ecma262/#sec-ecmascript-function-objects
#[derive(Debug)]
pub struct ECMAScriptFunction<'ctx, 'host> {
    /// [[Environment]]
    pub environment: EnvironmentIndex,

    /// [[PrivateEnvironment]]
    pub private_environment: Option<PrivateEnvironmentIndex>,

    /// [[FormalParameters]]
    pub formal_parameters: &'host FormalParameters<'host>,

    /// [[ECMAScriptCode]]
    pub ecmascript_code: &'host FunctionBody<'host>,

    /// [[ConstructorKind]]
    pub constructor_kind: ConstructorKind,

    /// [[Realm]]
    pub realm: RealmIdentifier<'ctx, 'host>,

    /// [[ScriptOrModule]]
    pub script_or_module: ScriptOrModule<'ctx, 'host>,

    /// [[ThisMode]]
    pub this_mode: ThisMode,

    /// [[Strict]]
    pub strict: bool,

    /// [[HomeObject]]
    pub home_object: Option<Object>,

    ///  [[SourceText]]
    pub source_text: &'host str,

    // TODO: [[Fields]],  [[PrivateMethods]], [[ClassFieldInitializerName]]
    /// [[IsClassConstructor]]
    pub is_class_constructor: bool,
}

impl Object {
    /// 10.2.10 SetFunctionLength ( F, length )
    /// https://tc39.es/ecma262/#sec-setfunctionlength
    pub fn set_function_length(self, agent: &mut Agent, length: i64) -> JsResult<()> {
        let function = self;

        // TODO: 1. Assert: F is an extensible object that does not have a "length" own property.

        // 2. Perform ! DefinePropertyOrThrow(F, "length", PropertyDescriptor { [[Value]]: 𝔽(length), [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }).
        function.define_property_or_throw(
            agent,
            PropertyKey::try_from(Value::try_from("length").unwrap()).unwrap(),
            PropertyDescriptor {
                value: Some(Number::try_from(length).unwrap().into_value()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            },
        )?;

        // 3. Return unused.
        Ok(())
    }
}
