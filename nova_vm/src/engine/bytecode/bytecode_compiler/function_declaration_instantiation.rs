// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::{AHashMap, AHashSet};
use oxc_ast::{
    ast::{FormalParameters, FunctionBody},
    syntax_directed_operations::BoundNames,
};
use oxc_span::Atom;

use crate::{
    ecmascript::{
        syntax_directed_operations::{
            function_definitions::ContainsExpression,
            scope_analysis::{
                function_body_lexically_declared_names, function_body_lexically_scoped_decarations,
                function_body_var_declared_names, function_body_var_scoped_declarations,
                LexicallyScopedDeclaration, VarScopedDeclaration,
            },
        },
        types::{String, Value, BUILTIN_STRING_MEMORY},
    },
    engine::{bytecode::bytecode_compiler::CompileContext, Instruction},
};

use super::{complex_array_pattern, simple_array_pattern, CompileEvaluation};

pub(crate) fn instantiation(
    ctx: &mut CompileContext,
    formals: &FormalParameters,
    body: &FunctionBody<'_>,
    strict: bool,
    is_lexical: bool,
) {
    // 5. Let parameterNames be the BoundNames of formals.
    // 6. If parameterNames has any duplicate entries, let hasDuplicates be true. Otherwise, let hasDuplicates be false.
    let mut parameter_names = AHashSet::with_capacity(formals.parameters_count());
    let mut has_duplicates = false;
    formals.bound_names(&mut |identifier| {
        if parameter_names.contains(&identifier.name) {
            has_duplicates = true;
        } else {
            parameter_names.insert(identifier.name.clone());
        }
    });

    // 8. Let hasParameterExpressions be ContainsExpression of formals.
    let has_parameter_expressions = formals
        .iter_bindings()
        .any(|binding| binding.contains_expression());

    // 12. Let functionNames be a new empty List.
    // 13. Let functionsToInitialize be a new empty List.
    // NOTE: the keys of `functions` will be `functionNames`, its values will be
    // `functionsToInitialize`.
    let mut functions = AHashMap::new();
    for d in function_body_var_scoped_declarations(body) {
        // a. If d is neither a VariableDeclaration nor a ForBinding nor a BindingIdentifier, then
        if let VarScopedDeclaration::Function(d) = d {
            // i. Assert: d is either a FunctionDeclaration, a GeneratorDeclaration, an AsyncFunctionDeclaration, or an AsyncGeneratorDeclaration.
            // ii. Let fn be the sole element of the BoundNames of d.
            let f_name = d.id.as_ref().unwrap().name.clone();
            // iii. If functionNames does not contain fn, then
            //   1. Insert fn as the first element of functionNames.
            //   2. NOTE: If there are multiple function declarations for the same name, the last declaration is used.
            //   3. Insert d as the first element of functionsToInitialize.
            functions.insert(f_name, d);
        }
    }

    // 15. Let argumentsObjectNeeded be true.
    // 16. If func.[[ThisMode]] is lexical, then
    //   a. NOTE: Arrow functions never have an arguments object.
    //   b. Set argumentsObjectNeeded to false.
    // 17. Else if parameterNames contains "arguments", then
    //   a. Set argumentsObjectNeeded to false.
    // 18. Else if hasParameterExpressions is false, then
    //   a. If functionNames contains "arguments" or lexicalNames contains "arguments", then
    //     i. Set argumentsObjectNeeded to false.
    let arguments_object_needed = !is_lexical
        && !parameter_names.contains("arguments")
        && (has_parameter_expressions
            || (!functions.contains_key("arguments")
                && !function_body_lexically_declared_names(body)
                    .contains(&Atom::from("arguments"))));

    // 19. If strict is true or hasParameterExpressions is false, then
    //   a. NOTE: Only a single Environment Record is needed for the parameters, since calls to eval in strict mode code cannot create new bindings which are visible outside of the eval.
    //   b. Let env be the LexicalEnvironment of calleeContext.
    // 20. Else,
    //   a. NOTE: A separate Environment Record is needed to ensure that bindings created by direct eval calls in the formal parameter list are outside the environment where parameters are declared.
    //   b. Let calleeEnv be the LexicalEnvironment of calleeContext.
    //   c. Let env be NewDeclarativeEnvironment(calleeEnv).
    //   d. Assert: The VariableEnvironment of calleeContext and calleeEnv are the same Environment Record.
    //   e. Set the LexicalEnvironment of calleeContext to env.
    if !strict && has_parameter_expressions {
        ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
    }

    // 21. For each String paramName of parameterNames, do
    // NOTE: The behavior should not depend on the order in which the parameter
    // names are iterated, so it's fine for `parameter_names` to be a set.
    for param_name in &parameter_names {
        // a. Let alreadyDeclared be ! env.HasBinding(paramName).
        // b. NOTE: Early errors ensure that duplicate parameter names can only occur in non-strict functions that do not have parameter default values or rest parameters.
        // c. If alreadyDeclared is false, then
        // NOTE: Since `parameter_names` is a set, `alreadyDeclared` here should always be false.

        // i. Perform ! env.CreateMutableBinding(paramName, false).
        let param_name = String::from_str(ctx.agent, param_name);
        ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, param_name);
        // ii. If hasDuplicates is true, then
        if has_duplicates {
            // 1. Perform ! env.InitializeBinding(paramName, undefined).
            ctx.add_instruction_with_identifier(Instruction::ResolveBinding, param_name);
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
            ctx.add_instruction(Instruction::InitializeReferencedBinding);
        }
    }

    // 22. If argumentsObjectNeeded is true, then
    if arguments_object_needed {
        // a. If strict is true or simpleParameterList is false, then
        //     i. Let ao be CreateUnmappedArgumentsObject(argumentsList).
        // b. Else,
        //     i. NOTE: A mapped argument object is only provided for non-strict functions that don't have a rest parameter, any parameter default value initializers, or any destructured parameters.
        //     ii. Let ao be CreateMappedArgumentsObject(func, formals, argumentsList, env).
        // TODO: Currently, we always create an unmapped arguments object, even
        // in non-strict mode.
        ctx.add_instruction(Instruction::CreateUnmappedArgumentsObject);

        // c. If strict is true, then
        if strict {
            // i. Perform ! env.CreateImmutableBinding("arguments", false).
            // ii. NOTE: In strict mode code early errors prevent attempting to assign to this binding, so its mutability is not observable.
            ctx.add_instruction_with_identifier(
                Instruction::CreateImmutableBinding,
                BUILTIN_STRING_MEMORY.arguments,
            );
        } else {
            // d. Else,
            //   i. Perform ! env.CreateMutableBinding("arguments", false).
            ctx.add_instruction_with_identifier(
                Instruction::CreateMutableBinding,
                BUILTIN_STRING_MEMORY.arguments,
            );
        }

        // e. Perform ! env.InitializeBinding("arguments", ao).
        ctx.add_instruction_with_identifier(
            Instruction::ResolveBinding,
            BUILTIN_STRING_MEMORY.arguments,
        );
        ctx.add_instruction(Instruction::InitializeReferencedBinding);

        // f. Let parameterBindings be the list-concatenation of parameterNames and « "arguments" ».
        // NOTE: reusing `parameter_names` for `parameterBindings`.
        parameter_names.insert("arguments".into());
    }

    // 24. Let iteratorRecord be CreateListIteratorRecord(argumentsList).
    // 25. If hasDuplicates is true, then
    //   a. Perform ? IteratorBindingInitialization of formals with arguments iteratorRecord and undefined.
    // 26. Else,
    //   a. Perform ? IteratorBindingInitialization of formals with arguments iteratorRecord and env.
    if !formals.has_parameter() {
        // Remove the arguments iterator from the iterator stack.
        ctx.add_instruction(Instruction::IteratorClose)
    } else if has_parameter_expressions {
        complex_array_pattern(
            ctx,
            formals.items.iter().map(|param| Some(&param.pattern)),
            formals.rest.as_deref(),
            !has_duplicates,
        );
    } else {
        simple_array_pattern(
            ctx,
            formals.items.iter().map(|param| Some(&param.pattern)),
            formals.rest.as_deref(),
            formals.items.len(),
            !has_duplicates,
        );
    }

    // 27. If hasParameterExpressions is false, then
    if !has_parameter_expressions {
        // a. NOTE: Only a single Environment Record is needed for the parameters and top-level vars.
        // b. Let instantiatedVarNames be a copy of the List parameterBindings.
        let mut instantiated_var_names = AHashSet::new();
        // c. For each element n of varNames, do
        for n in function_body_var_declared_names(body) {
            // i. If instantiatedVarNames does not contain n, then
            if instantiated_var_names.contains(&n) || parameter_names.contains(&n) {
                continue;
            }
            // 1. Append n to instantiatedVarNames.
            let n_string = String::from_str(ctx.agent, &n);
            instantiated_var_names.insert(n);
            // 2. Perform ! env.CreateMutableBinding(n, false).
            ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, n_string);
            // 3. Perform ! env.InitializeBinding(n, undefined).
            ctx.add_instruction_with_identifier(Instruction::ResolveBinding, n_string);
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
            ctx.add_instruction(Instruction::InitializeReferencedBinding);
        }

        // d. Let varEnv be env.
        // 30. If strict is false, then
        //   a. Let lexEnv be NewDeclarativeEnvironment(varEnv).
        // 31. Else,
        //   a. Let lexEnv be varEnv.
        // 32. Set the LexicalEnvironment of calleeContext to lexEnv.
        if !strict {
            ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
        }
    } else {
        // 28. Else,
        // a. NOTE: A separate Environment Record is needed to ensure that closures created by expressions in the formal parameter list do not have visibility of declarations in the function body.
        // b. Let varEnv be NewDeclarativeEnvironment(env).
        // c. Set the VariableEnvironment of calleeContext to varEnv.
        // NOTE: Since this step operates on a variable environment, rather than
        // the usual lexical environments, we implement this by pushing all
        // variable names and values into the stack, and then having a single
        // instruction that initializes all of them and sets the right
        // environment in one go.

        // d. Let instantiatedVarNames be a new empty List.
        let mut instantiated_var_names = AHashSet::new();
        // e. For each element n of varNames, do
        for n in function_body_var_declared_names(body) {
            // i. If instantiatedVarNames does not contain n, then
            if instantiated_var_names.contains(&n) {
                continue;
            }
            // 1. Append n to instantiatedVarNames.
            instantiated_var_names.insert(n.clone());
            // 3. If parameterBindings does not contain n, or if functionNames contains n, then
            let n_string = String::from_str(ctx.agent, &n);
            if !parameter_names.contains(&n) || functions.contains_key(&n) {
                // a. Let initialValue be undefined.
                ctx.add_instruction_with_constant(Instruction::LoadConstant, Value::Undefined);
            } else {
                // 4. Else,
                // a. Let initialValue be ! env.GetBindingValue(n, false).
                ctx.add_instruction_with_identifier(Instruction::ResolveBinding, n_string);
                ctx.add_instruction(Instruction::GetValue);
                ctx.add_instruction(Instruction::Load);
            }

            // 2. Perform ! varEnv.CreateMutableBinding(n, false).
            // 5. Perform ! varEnv.InitializeBinding(n, initialValue).
            // 6. NOTE: A var with the same name as a formal parameter initially has the same value as the corresponding initialized parameter.
            ctx.add_instruction_with_constant(Instruction::LoadConstant, n_string);
        }

        // 30. If strict is false, then
        //   a. Let lexEnv be NewDeclarativeEnvironment(varEnv).
        //   b. NOTE: Non-strict functions use a separate Environment Record for top-level lexical
        //      declarations so that a direct eval can determine whether any var scoped declarations
        //      introduced by the eval code conflict with pre-existing top-level lexically scoped
        //      declarations. This is not needed for strict functions because a strict direct eval
        //      always places all declarations into a new Environment Record.
        // 31. Else,
        //   a. Let lexEnv be varEnv.
        // 32. Set the LexicalEnvironment of calleeContext to lexEnv.
        ctx.add_instruction_with_immediate_and_immediate(
            Instruction::InitializeVariableEnvironment,
            instantiated_var_names.len(),
            strict.into(),
        );
    }

    // 33. Let lexDeclarations be the LexicallyScopedDeclarations of code.
    // 34. For each element d of lexDeclarations, do
    for d in function_body_lexically_scoped_decarations(body) {
        // a. NOTE: A lexically declared name cannot be the same as a function/generator declaration, formal parameter, or a var name. Lexically declared names are only instantiated here but not initialized.
        // b. For each element dn of the BoundNames of d, do
        match d {
            // i. If IsConstantDeclaration of d is true, then
            LexicallyScopedDeclaration::Variable(decl) if decl.kind.is_const() => {
                decl.id.bound_names(&mut |identifier| {
                    let dn = String::from_str(ctx.agent, &identifier.name);
                    // 1. Perform ! lexEnv.CreateImmutableBinding(dn, true).
                    ctx.add_instruction_with_identifier(Instruction::CreateImmutableBinding, dn);
                })
            }
            // ii. Else,
            //   1. Perform ! lexEnv.CreateMutableBinding(dn, false).
            LexicallyScopedDeclaration::Variable(decl) => decl.id.bound_names(&mut |identifier| {
                let dn = String::from_str(ctx.agent, &identifier.name);
                ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, dn);
            }),
            LexicallyScopedDeclaration::Function(decl) => {
                let dn = String::from_str(ctx.agent, &decl.id.as_ref().unwrap().name);
                ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, dn);
            }
            LexicallyScopedDeclaration::Class(decl) => {
                let dn = String::from_str(ctx.agent, &decl.id.as_ref().unwrap().name);
                ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, dn);
            }
            LexicallyScopedDeclaration::DefaultExport => {
                let dn = BUILTIN_STRING_MEMORY._default_;
                ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, dn);
            }
        }
    }

    // 36. For each Parse Node f of functionsToInitialize, do
    for f in functions.values() {
        // b. Let fo be InstantiateFunctionObject of f with arguments lexEnv and privateEnv.
        f.compile(ctx);
        // a. Let fn be the sole element of the BoundNames of f.
        let f_name = String::from_str(ctx.agent, &f.id.as_ref().unwrap().name);
        // c. Perform ! varEnv.SetMutableBinding(fn, fo, false).
        // TODO: This compilation is incorrect if !strict, when varEnv != lexEnv.
        ctx.add_instruction_with_identifier(Instruction::ResolveBinding, f_name);
        ctx.add_instruction(Instruction::PutValue);
    }
}
