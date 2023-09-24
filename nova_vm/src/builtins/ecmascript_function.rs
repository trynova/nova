use std::{cell::RefCell, rc::Rc};

use oxc_ast::ast::{FormalParameters, FunctionBody};

use crate::{
    execution::{Environment, PrivateEnvironment, Realm, ScriptOrModule},
    types::Object,
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
#[derive(Debug, Clone)]
pub struct ECMAScriptFunction<'ctx, 'host> {
    /// [[Environment]]
    pub environment: Environment,

    /// [[PrivateEnvironment]]
    pub private_environment: Option<Rc<RefCell<PrivateEnvironment>>>,

    /// [[FormalParameters]]
    pub formal_parameters: &'host FormalParameters<'host>,

    /// [[ECMAScriptCode]]
    pub ecmascript_code: &'host FunctionBody<'host>,

    /// [[ConstructorKind]]
    pub constructor_kind: ConstructorKind,

    /// [[Realm]]
    pub realm: Rc<RefCell<Realm<'ctx, 'host>>>,

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
