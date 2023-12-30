use std::ptr::NonNull;

use oxc_ast::ast::{FormalParameters, FunctionBody};
use oxc_span::Span;

use crate::{
    ecmascript::{
        execution::{
            agent::{get_active_script_or_module, ExceptionType::SyntaxError},
            Agent, EnvironmentIndex, JsResult, PrivateEnvironmentIndex, RealmIdentifier,
        },
        scripts_and_modules::ScriptOrModule,
        types::{ECMAScriptFunctionHeapData, Function, Object, Value},
    },
    heap::CreateHeapData,
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

/// ### [10.2 ECMAScript Function Objects](https://tc39.es/ecma262/#sec-ecmascript-function-objects)
#[derive(Debug)]
pub(crate) struct ECMAScriptFunction {
    /// \[\[Environment]]
    pub environment: EnvironmentIndex,

    /// \[\[PrivateEnvironment]]
    pub private_environment: Option<PrivateEnvironmentIndex>,

    /// \[\[FormalParameters]]
    ///
    /// SAFETY: ScriptOrModule owns the Program which this refers to.
    /// Our GC algorithm keeps it alive as long as this function is alive.
    pub formal_parameters: NonNull<FormalParameters<'static>>,

    /// \[\[ECMAScriptCode]]
    ///
    /// SAFETY: ScriptOrModule owns the Program which this refers to.
    /// Our GC algorithm keeps it alive as long as this function is alive.
    pub ecmascript_code: NonNull<FunctionBody<'static>>,

    /// \[\[ConstructorKind]]
    pub constructor_kind: ConstructorKind,

    /// \[\[Realm]]
    pub realm: RealmIdentifier,

    /// \[\[ScriptOrModule]]
    pub script_or_module: ScriptOrModule,

    /// \[\[ThisMode]]
    pub this_mode: ThisMode,

    /// \[\[Strict]]
    pub strict: bool,

    /// \[\[HomeObject]]
    pub home_object: Option<Object>,

    ///  [[SourceText]]
    pub source_text: Span,

    // TODO: [[Fields]],  [[PrivateMethods]], [[ClassFieldInitializerName]]
    /// \[\[IsClassConstructor]]
    pub is_class_constructor: bool,
}

pub(crate) struct OrdinaryFunctionCreateParams<'program> {
    pub function_prototype: Option<Object>,
    pub source_text: Span,
    pub parameters_list: &'program FormalParameters<'program>,
    pub body: &'program FunctionBody<'program>,
    pub this_mode: ThisMode,
    pub env: EnvironmentIndex,
    pub private_env: Option<PrivateEnvironmentIndex>,
}

/// ### [10.2.3 OrdinaryFunctionCreate ( functionPrototype, sourceText, ParameterList, Body, thisMode, env, privateEnv )](https://tc39.es/ecma262/#sec-ordinaryfunctioncreate)
///
/// The abstract operation OrdinaryFunctionCreate takes arguments
/// functionPrototype (an Object), sourceText (a sequence of Unicode code
/// points), ParameterList (a Parse Node), Body (a Parse Node), thisMode
/// (LEXICAL-THIS or NON-LEXICAL-THIS), env (an Environment Record), and
/// privateEnv (a PrivateEnvironment Record or null) and returns an ECMAScript
/// function object. It is used to specify the runtime creation of a new
/// function with a default \[\[Call\]\] internal method and no
/// \[\[Construct\]\] internal method (although one may be subsequently added
/// by an operation such as MakeConstructor). sourceText is the source text of
/// the syntactic definition of the function to be created.
pub(crate) fn ordinary_function_create<'program>(
    agent: &mut Agent,
    params: OrdinaryFunctionCreateParams<'program>,
) -> Function {
    // 1. Let internalSlotsList be the internal slots listed in Table 30.
    // 2. Let F be OrdinaryObjectCreate(functionPrototype, internalSlotsList).
    // 3. Set F.[[Call]] to the definition specified in 10.2.1.
    let ecmascript_function = ECMAScriptFunction {
        // 13. Set F.[[Environment]] to env.
        environment: params.env,
        // 14. Set F.[[PrivateEnvironment]] to privateEnv.
        private_environment: params.private_env,
        // 5. Set F.[[FormalParameters]] to ParameterList.
        // SAFETY: The reference to FormalParameters points to ScriptOrModule
        // and is valid until it gets dropped. Our GC keeps ScriptOrModule
        // alive until this ECMAScriptFunction gets dropped, hence the 'static
        // lifetime here is justified.
        formal_parameters: unsafe {
            std::mem::transmute::<
                NonNull<FormalParameters<'program>>,
                NonNull<FormalParameters<'static>>,
            >(params.parameters_list.into())
        },
        // 6. Set F.[[ECMAScriptCode]] to Body.
        // SAFETY: Same as above: Self-referential reference to ScriptOrModule.
        ecmascript_code: unsafe {
            std::mem::transmute::<NonNull<FunctionBody<'program>>, NonNull<FunctionBody<'static>>>(
                params.body.into(),
            )
        },
        constructor_kind: ConstructorKind::Base,
        // 16. Set F.[[Realm]] to the current Realm Record.
        realm: agent.current_realm_id(),
        // 15. Set F.[[ScriptOrModule]] to GetActiveScriptOrModule().
        script_or_module: get_active_script_or_module(agent).unwrap(),
        // 9. If thisMode is LEXICAL-THIS, set F.[[ThisMode]] to LEXICAL.
        // 10. Else if Strict is true, set F.[[ThisMode]] to STRICT.
        // 11. Else, set F.[[ThisMode]] to GLOBAL.
        this_mode: params.this_mode,
        // 7. If the source text matched by Body is strict mode code, let Strict be true; else let Strict be false.
        // 8. Set F.[[Strict]] to Strict.
        strict: true,
        // 17. Set F.[[HomeObject]] to undefined.
        home_object: None,
        // 4. Set F.[[SourceText]] to sourceText.
        source_text: params.source_text,
        // 12. Set F.[[IsClassConstructor]] to false.
        is_class_constructor: false,
    };

    let mut function = ECMAScriptFunctionHeapData {
        object_index: None,
        length: 0,
        initial_name: Value::Undefined,
        ecmascript_function,
    };
    if let Some(function_prototype) = params.function_prototype {
        if function_prototype != agent.current_realm().intrinsics().function_prototype() {
            function.object_index =
                Some(agent.heap.create_object_with_prototype(function_prototype));
        }
    }

    // 18. Set F.[[Fields]] to a new empty List.
    // 19. Set F.[[PrivateMethods]] to a new empty List.
    // 20. Set F.[[ClassFieldInitializerName]] to EMPTY.
    // 21. Let len be the ExpectedArgumentCount of ParameterList.
    // 22. Perform SetFunctionLength(F, len).
    set_ecmascript_function_length(
        agent,
        &mut function,
        params
            .parameters_list
            .items
            .iter()
            .filter(|par| !par.pattern.kind.is_binding_identifier())
            .count(),
    )
    .unwrap();
    // 23. Return F.
    agent.heap.create(function)
}

/// 10.2.10 SetFunctionLength ( F, length )
/// https://tc39.es/ecma262/#sec-setfunctionlength
fn set_ecmascript_function_length(
    agent: &mut Agent,
    function: &mut ECMAScriptFunctionHeapData,
    length: usize,
) -> JsResult<()> {
    // TODO: 1. Assert: F is an extensible object that does not have a "length" own property.

    // 2. Perform ! DefinePropertyOrThrow(F, "length", PropertyDescriptor { [[Value]]: 𝔽(length), [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }).
    if length > u8::MAX as usize {
        return Err(agent.throw_exception(
            SyntaxError,
            "Too many arguments in function call (only 255 allowed)",
        ));
    }
    function.length = length as u8;

    // 3. Return unused.
    Ok(())
}
