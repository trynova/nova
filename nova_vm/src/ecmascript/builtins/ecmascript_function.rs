use oxc_ast::ast::{FormalParameters, FunctionBody};

use crate::{
    ecmascript::{
        execution::{
            agent::{get_active_script_or_module, ExceptionType::SyntaxError},
            Agent, EnvironmentIndex, JsResult, PrivateEnvironmentIndex, RealmIdentifier,
        },
        scripts_and_modules::ScriptOrModule,
        types::{BuiltinFunctionHeapData, Function, Object, Value},
    },
    heap::indexes::BuiltinFunctionIndex,
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
pub(crate) struct ECMAScriptFunction<'ctx, 'host> {
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

/// ## [10.2.3 OrdinaryFunctionCreate ( functionPrototype, sourceText, ParameterList, Body, thisMode, env, privateEnv )](https://tc39.es/ecma262/#sec-ordinaryfunctioncreate)
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
pub(crate) fn ordinary_function_create<'ctx, 'host>(
    agent: &mut Agent<'ctx, 'host>,
    function_prototype: Option<Object>,
    source_text: &'host str,
    parameters_list: &'host FormalParameters<'host>,
    body: &'host FunctionBody<'host>,
    this_mode: ThisMode,
    env: EnvironmentIndex,
    private_env: Option<PrivateEnvironmentIndex>,
) -> Function {
    // 1. Let internalSlotsList be the internal slots listed in Table 30.
    // 2. Let F be OrdinaryObjectCreate(functionPrototype, internalSlotsList).
    // 3. Set F.[[Call]] to the definition specified in 10.2.1.
    let mut function = BuiltinFunctionHeapData {
        object_index: None,
        length: 0,
        initial_name: Value::Undefined,
    };
    if function_prototype.is_some()
        && function_prototype != Some(agent.current_realm().intrinsics().function_prototype())
    {
        function.object_index = Some(
            agent
                .heap
                .create_object_with_prototype(function_prototype.unwrap()),
        );
    }

    let _ecmascript_function = ECMAScriptFunction {
        // 13. Set F.[[Environment]] to env.
        environment: env,
        // 14. Set F.[[PrivateEnvironment]] to privateEnv.
        private_environment: private_env,
        // 5. Set F.[[FormalParameters]] to ParameterList.
        formal_parameters: parameters_list,
        // 6. Set F.[[ECMAScriptCode]] to Body.
        ecmascript_code: body,
        constructor_kind: ConstructorKind::Base,
        // 16. Set F.[[Realm]] to the current Realm Record.
        realm: agent.current_realm_id(),
        // 15. Set F.[[ScriptOrModule]] to GetActiveScriptOrModule().
        script_or_module: get_active_script_or_module(agent).unwrap(),
        // 9. If thisMode is LEXICAL-THIS, set F.[[ThisMode]] to LEXICAL.
        // 10. Else if Strict is true, set F.[[ThisMode]] to STRICT.
        // 11. Else, set F.[[ThisMode]] to GLOBAL.
        this_mode,
        // 7. If the source text matched by Body is strict mode code, let Strict be true; else let Strict be false.
        // 8. Set F.[[Strict]] to Strict.
        strict: true,
        // 17. Set F.[[HomeObject]] to undefined.
        home_object: None,
        // 4. Set F.[[SourceText]] to sourceText.
        source_text,
        // 12. Set F.[[IsClassConstructor]] to false.
        is_class_constructor: false,
    };
    // 18. Set F.[[Fields]] to a new empty List.
    // 19. Set F.[[PrivateMethods]] to a new empty List.
    // 20. Set F.[[ClassFieldInitializerName]] to EMPTY.
    // 21. Let len be the ExpectedArgumentCount of ParameterList.
    // 22. Perform SetFunctionLength(F, len).
    set_function_length(agent, &mut function, parameters_list.parameters_count()).unwrap();
    // 23. Return F.
    agent.heap.builtin_functions.push(Some(function));
    Function::from(BuiltinFunctionIndex::from_usize(
        agent.heap.builtin_functions.len(),
    ))
}

/// 10.2.10 SetFunctionLength ( F, length )
/// https://tc39.es/ecma262/#sec-setfunctionlength
fn set_function_length(
    agent: &mut Agent,
    function: &mut BuiltinFunctionHeapData,
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
