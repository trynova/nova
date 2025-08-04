// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [16.2.3.7 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-exports-runtime-semantics-evaluation)

use oxc_ast::ast;

use crate::{
    ecmascript::types::BUILTIN_STRING_MEMORY,
    engine::{
        Instruction, NamedEvaluationParameter,
        bytecode::bytecode_compiler::is_anonymous_function_definition,
    },
};

use super::{CompileEvaluation, compile_expression_get_value};

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ExportAllDeclaration<'s> {
    type Output = ();
    /// ### ExportDeclaration :
    /// ```text
    /// export ExportFromClause FromClause WithClause_opt ;
    /// ```
    fn compile(&'s self, _: &mut super::CompileContext<'_, 's, '_, '_>) {
        // 1. Return empty.
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::ExportDefaultDeclaration<'s>
{
    type Output = ();
    /// ### ExportDeclaration :
    /// ```text
    /// export default HoistableDeclaration
    /// export default ClassDeclaration
    /// export default AssignmentExpression ;
    /// ```
    fn compile(&'s self, ctx: &mut super::CompileContext<'_, 's, '_, '_>) {
        match &self.declaration {
            //  ExportDeclaration : export default HoistableDeclaration
            // 1. Return ? Evaluation of HoistableDeclaration.
            ast::ExportDefaultDeclarationKind::FunctionDeclaration(decl) => decl.compile(ctx),
            // ExportDeclaration : export default ClassDeclaration
            ast::ExportDefaultDeclarationKind::ClassDeclaration(decl) => {
                // 1. Let value be ? BindingClassDeclarationEvaluation of ClassDeclaration.
                ctx.add_instruction_with_constant(
                    Instruction::StoreConstant,
                    BUILTIN_STRING_MEMORY.default,
                );
                ctx.name_identifier = Some(NamedEvaluationParameter::Result);
                decl.compile(ctx);
                // 2. Let className be the sole element of the BoundNames of ClassDeclaration.
                // 3. If className is "*default*", then
                // a. Let env be the running execution context's LexicalEnvironment.
                // b. Perform ? InitializeBoundName("*default*", value, env).
                ctx.add_instruction_with_identifier(
                    Instruction::ResolveBinding,
                    BUILTIN_STRING_MEMORY._default_,
                );
                ctx.add_instruction(Instruction::InitializeReferencedBinding);
                // 4. Return empty.
            }
            ast::ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => unreachable!(),
            _ => {
                // ExportDeclaration : export default AssignmentExpression ;
                let expr = self.declaration.as_expression().unwrap();
                // 1. If IsAnonymousFunctionDefinition(AssignmentExpression) is
                //    true, then
                if is_anonymous_function_definition(expr) {
                    // a. Let value be ? NamedEvaluation of
                    //    AssignmentExpression with argument "default".
                    ctx.name_identifier = Some(NamedEvaluationParameter::Result);
                    ctx.add_instruction_with_constant(
                        Instruction::StoreConstant,
                        BUILTIN_STRING_MEMORY.default,
                    );
                }
                // 2. Else,
                // a. Let rhs be ? Evaluation of AssignmentExpression.
                // b. Let value be ? GetValue(rhs).
                compile_expression_get_value(expr, ctx);

                // 3. Let env be the running execution context's LexicalEnvironment.
                // 4. Perform ? InitializeBoundName("*default*", value, env).
                ctx.add_instruction_with_identifier(
                    Instruction::ResolveBinding,
                    BUILTIN_STRING_MEMORY._default_,
                );
                ctx.add_instruction(Instruction::InitializeReferencedBinding);
                // 5. Return empty.
            }
        }
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::ExportNamedDeclaration<'s>
{
    type Output = ();
    /// ### ExportDeclaration :
    /// ```text
    /// export NamedExports ;
    /// export VariableStatement
    /// export Declaration
    /// ```
    fn compile(&'s self, ctx: &mut super::CompileContext<'_, 's, '_, '_>) {
        let Some(decl) = &self.declaration else {
            // export NamedExports ;
            // 1. Return empty.
            return;
        };
        match decl {
            // export VariableStatement
            // 1. Return ? Evaluation of VariableStatement.
            ast::Declaration::VariableDeclaration(decl) => decl.compile(ctx),
            // ExportDeclaration : export Declaration
            // 1. Return ? Evaluation of Declaration.
            ast::Declaration::FunctionDeclaration(decl) => decl.compile(ctx),
            ast::Declaration::ClassDeclaration(decl) => decl.compile(ctx),
            ast::Declaration::TSTypeAliasDeclaration(_)
            | ast::Declaration::TSInterfaceDeclaration(_) => {}
            #[cfg(feature = "typescript")]
            ast::Declaration::TSEnumDeclaration(decl) => decl.compile(ctx),
            #[cfg(feature = "typescript")]
            ast::Declaration::TSModuleDeclaration(_) => {
                // TODO: implement when module declarations are supported
            }
            #[cfg(feature = "typescript")]
            ast::Declaration::TSImportEqualsDeclaration(_) => {
                // TODO: implement when import equals declarations are supported
            }
            #[cfg(not(feature = "typescript"))]
            ast::Declaration::TSEnumDeclaration(_)
            | ast::Declaration::TSModuleDeclaration(_)
            | ast::Declaration::TSImportEqualsDeclaration(_) => unreachable!(),
        }
    }
}
