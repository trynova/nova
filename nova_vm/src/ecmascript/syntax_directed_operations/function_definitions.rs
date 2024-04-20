use crate::{
    ecmascript::{
        builtins::{
            function_declaration_instantiation, ordinary_function_create, set_function_name,
            ArgumentsList, ECMAScriptFunction, OrdinaryFunctionCreateParams, ThisMode,
        },
        execution::{
            Agent, ECMAScriptCodeEvaluationState, EnvironmentIndex, JsResult,
            PrivateEnvironmentIndex,
        },
        types::{Function, PropertyKey, String, Value},
    },
    engine::{Executable, FunctionExpression, Vm},
    heap::GetHeapData,
};
use oxc_ast::ast::{self};

/// ### [15.2.4 Runtime Semantics: InstantiateOrdinaryFunctionObject](https://tc39.es/ecma262/#sec-runtime-semantics-instantiateordinaryfunctionobject)
///
/// The syntax-directed operation InstantiateOrdinaryFunctionObject takes
/// arguments env (an Environment Record) and privateEnv (a PrivateEnvironment
/// Record or null) and returns an ECMAScript function object.
pub(crate) fn instantiate_ordinary_function_object(
    agent: &mut Agent,
    function: &ast::Function<'_>,
    env: EnvironmentIndex,
    private_env: Option<PrivateEnvironmentIndex>,
) -> Function {
    // FunctionDeclaration : function BindingIdentifier ( FormalParameters ) { FunctionBody }
    if let Some(id) = &function.id {
        // 1. Let name be StringValue of BindingIdentifier.
        let _name = id.name.clone();
        // 2. Let sourceText be the source text matched by FunctionDeclaration.
        let source_text = function.body.as_ref().unwrap().span;
        // 3. Let F be OrdinaryFunctionCreate(%Function.prototype%, sourceText, FormalParameters, FunctionBody, NON-LEXICAL-THIS, env, privateEnv).
        let params = OrdinaryFunctionCreateParams {
            function_prototype: None,
            source_text,
            parameters_list: &function.params,
            body: function.body.as_deref().unwrap(),
            this_mode: crate::ecmascript::builtins::ThisMode::Global,
            env,
            private_env,
        };

        // 4. Perform SetFunctionName(F, name).
        // set_function_name(f, name);
        // 5. Perform MakeConstructor(F).
        // make_constructor(f);
        // 6. Return F.
        ordinary_function_create(agent, params)
    } else {
        // FunctionDeclaration : function ( FormalParameters ) { FunctionBody }
        // 1. Let sourceText be the source text matched by FunctionDeclaration.
        let source_text = function.body.as_ref().unwrap().span;
        // 2. Let F be OrdinaryFunctionCreate(%Function.prototype%, sourceText, FormalParameters, FunctionBody, NON-LEXICAL-THIS, env, privateEnv).
        let params = OrdinaryFunctionCreateParams {
            function_prototype: None,
            source_text,
            parameters_list: &function.params,
            body: function.body.as_ref().unwrap(),
            this_mode: crate::ecmascript::builtins::ThisMode::Global,
            env,
            private_env,
        };

        // 3. Perform SetFunctionName(F, "default").
        // set_function_name(f, "default");
        // 4. Perform MakeConstructor(F).
        // make_constructor(f);
        // 5. Return F.
        ordinary_function_create(agent, params)
    }
    // NOTE
    // An anonymous FunctionDeclaration can only occur as part of an export default declaration, and its function code is therefore always strict mode code.
}

// 15.2.5 Runtime Semantics: InstantiateOrdinaryFunctionExpression
// The syntax-directed operation InstantiateOrdinaryFunctionExpression takes optional argument name (a property key or a Private Name) and returns an ECMAScript function object. It is defined piecewise over the following productions:

pub(crate) fn instantiate_ordinary_function_expression(
    agent: &mut Agent,
    function: &FunctionExpression,
    name: Option<String>,
) -> Function {
    if let Some(_identifier) = function.identifier {
        todo!();
    } else {
        // 1. If name is not present, set name to "".
        let name = name.map_or_else(|| String::EMPTY_STRING, |name| name);
        // 2. Let env be the LexicalEnvironment of the running execution context.
        // 3. Let privateEnv be the running execution context's PrivateEnvironment.
        let ECMAScriptCodeEvaluationState {
            lexical_environment,
            private_environment,
            ..
        } = *agent
            .running_execution_context()
            .ecmascript_code
            .as_ref()
            .unwrap();
        // 4. Let sourceText be the source text matched by FunctionExpression.
        let source_text = function.expression.span;
        // 5. Let closure be OrdinaryFunctionCreate(%Function.prototype%, sourceText, FormalParameters, FunctionBody, NON-LEXICAL-THIS, env, privateEnv).
        let params = OrdinaryFunctionCreateParams {
            function_prototype: None,
            source_text,
            parameters_list: &function.expression.params,
            body: function.expression.body.as_ref().unwrap(),
            this_mode: ThisMode::Global,
            env: lexical_environment,
            private_env: private_environment,
        };
        let closure = ordinary_function_create(agent, params);
        // 6. Perform SetFunctionName(closure, name).
        let name = PropertyKey::from(name);
        set_function_name(agent, closure, name, None);
        // 7. Perform MakeConstructor(closure).
        // 8. Return closure.
        closure
    }
}

/// ### [15.2.3 Runtime Semantics: EvaluateFunctionBody](https://tc39.es/ecma262/#sec-runtime-semantics-evaluatefunctionbody)
/// The syntax-directed operation EvaluateFunctionBody takes arguments
/// functionObject (an ECMAScript function object) and argumentsList (a List of
/// ECMAScript language values) and returns either a normal completion
/// containing an ECMAScript language value or an abrupt completion.
pub(crate) fn evaluate_function_body(
    agent: &mut Agent,
    function_object: ECMAScriptFunction,
    arguments_list: ArgumentsList,
) -> JsResult<Value> {
    // 1. Perform ? FunctionDeclarationInstantiation(functionObject, argumentsList).
    function_declaration_instantiation(agent, function_object, arguments_list)?;
    // 2. Return ? Evaluation of FunctionStatementList.
    let body = agent
        .heap
        .get(function_object.into())
        .ecmascript_function
        .ecmascript_code;
    let exe = Executable::compile_function_body(agent, body);
    Ok(Vm::execute(agent, &exe)?.unwrap_or(Value::Undefined))
}
