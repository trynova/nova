// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::ECMAScriptFunction,
        execution::{Agent, EnvironmentIndex, PrivateEnvironmentIndex},
        syntax_directed_operations::function_definitions::instantiate_ordinary_function_object,
    },
    engine::context::GcScope,
};
use oxc_ast::ast;

/// ### [8.6.1 Runtime Semantics: InstantiateFunctionObject](https://tc39.es/ecma262/#sec-runtime-semantics-instantiatefunctionobject)
///
/// The syntax-directed operation InstantiateFunctionObject takes arguments env
/// (an Environment Record) and privateEnv (a PrivateEnvironment Record or
/// null) and returns an ECMAScript function object.
pub(crate) fn instantiate_function_object(
    agent: &mut Agent,
    gc: GcScope<'_, '_>,
    function: &ast::Function<'_>,
    env: EnvironmentIndex,
    private_env: Option<PrivateEnvironmentIndex>,
) -> ECMAScriptFunction {
    // FunctionDeclaration :
    // function BindingIdentifier ( FormalParameters ) { FunctionBody }
    // function ( FormalParameters ) { FunctionBody }
    if !function.r#async && !function.generator {
        // 1. Return InstantiateOrdinaryFunctionObject of FunctionDeclaration with arguments env and privateEnv.
        return instantiate_ordinary_function_object(agent, gc, function, env, private_env);
    }
    // GeneratorDeclaration :
    // function * BindingIdentifier ( FormalParameters ) { GeneratorBody }
    // function * ( FormalParameters ) { GeneratorBody }
    if !function.r#async && function.generator {
        // 1. Return InstantiateGeneratorFunctionObject of GeneratorDeclaration with arguments env and privateEnv.
        return instantiate_ordinary_function_object(agent, gc, function, env, private_env);
    }
    // AsyncGeneratorDeclaration :
    // async function * BindingIdentifier ( FormalParameters ) { AsyncGeneratorBody }
    // async function * ( FormalParameters ) { AsyncGeneratorBody }
    if function.r#async && function.generator {
        // 1. Return InstantiateAsyncGeneratorFunctionObject of AsyncGeneratorDeclaration with arguments env and privateEnv.
        todo!("InstantiateAsyncGeneratorFunctionObject")
    }
    // AsyncFunctionDeclaration :
    // async function BindingIdentifier ( FormalParameters ) { AsyncFunctionBody }
    // async function ( FormalParameters ) { AsyncFunctionBody }
    if function.r#async && !function.generator {
        // 1. Return InstantiateAsyncFunctionObject of AsyncFunctionDeclaration with arguments env and privateEnv.
        return instantiate_ordinary_function_object(agent, gc, function, env, private_env);
    }
    unreachable!();
}
