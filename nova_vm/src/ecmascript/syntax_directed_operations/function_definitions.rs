use oxc_ast::ast;

use crate::ecmascript::{
    builtins::{ordinary_function_create, OrdinaryFunctionCreateParams},
    execution::{Agent, EnvironmentIndex, PrivateEnvironmentIndex},
    types::Function,
};

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
            parameters_list: unsafe { std::mem::transmute(&function.params) },
            body: unsafe { std::mem::transmute(&function.body.as_ref().unwrap()) },
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
            parameters_list: unsafe { std::mem::transmute(&function.params) },
            body: unsafe { std::mem::transmute(&function.body.as_ref().unwrap()) },
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
