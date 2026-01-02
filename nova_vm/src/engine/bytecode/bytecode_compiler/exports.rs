// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### [16.2.3.7 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-exports-runtime-semantics-evaluation)

use std::ops::ControlFlow;

use oxc_ast::ast;

use crate::{
    ecmascript::types::BUILTIN_STRING_MEMORY,
    engine::{
        Instruction, NamedEvaluationParameter,
        bytecode::bytecode_compiler::{
            StatementBreak, is_anonymous_function_definition, value_result_to_statement_result,
        },
    },
};

use super::CompileEvaluation;

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
    type Output = ControlFlow<StatementBreak>;
    /// ### ExportDeclaration :
    /// ```text
    /// export default HoistableDeclaration
    /// export default ClassDeclaration
    /// export default AssignmentExpression ;
    /// ```
    fn compile(&'s self, ctx: &mut super::CompileContext<'_, 's, '_, '_>) -> Self::Output {
        match &self.declaration {
            //  ExportDeclaration : export default HoistableDeclaration
            // 1. Return ? Evaluation of HoistableDeclaration.
            ast::ExportDefaultDeclarationKind::FunctionDeclaration(decl) => {
                if decl.id.is_none() {
                    ctx.add_instruction_with_constant(
                        Instruction::StoreConstant,
                        BUILTIN_STRING_MEMORY.default,
                    );
                    ctx.name_identifier = Some(NamedEvaluationParameter::Result);
                }
                decl.compile(ctx);
            }
            // ExportDeclaration : export default ClassDeclaration
            ast::ExportDefaultDeclarationKind::ClassDeclaration(decl) => {
                // 1. Let value be ? BindingClassDeclarationEvaluation of ClassDeclaration.
                if decl.id.is_none() {
                    ctx.add_instruction_with_constant(
                        Instruction::StoreConstant,
                        BUILTIN_STRING_MEMORY.default,
                    );
                    ctx.name_identifier = Some(NamedEvaluationParameter::Result);
                }
                if let Err(err) = decl.compile(ctx) {
                    return ControlFlow::Break(err.into());
                };
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
                value_result_to_statement_result(expr.compile(ctx).and_then(|r| r.get_value(ctx)))?;
            }
        }
        ctx.add_instruction_with_identifier(
            Instruction::ResolveBinding,
            BUILTIN_STRING_MEMORY._default_.to_property_key(),
        );
        ctx.add_instruction(Instruction::InitializeReferencedBinding);
        ControlFlow::Continue(())
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::ExportNamedDeclaration<'s>
{
    type Output = ControlFlow<StatementBreak>;
    /// ### ExportDeclaration :
    /// ```text
    /// export NamedExports ;
    /// export VariableStatement
    /// export Declaration
    /// ```
    fn compile(&'s self, ctx: &mut super::CompileContext<'_, 's, '_, '_>) -> Self::Output {
        let Some(decl) = &self.declaration else {
            // export NamedExports ;
            // 1. Return empty.
            return ControlFlow::Continue(());
        };
        match decl {
            // export VariableStatement
            // 1. Return ? Evaluation of VariableStatement.
            ast::Declaration::VariableDeclaration(decl) => {
                if let Err(err) = decl.compile(ctx) {
                    ControlFlow::Break(err.into())
                } else {
                    // 6. Return EMPTY.
                    ControlFlow::Continue(())
                }
            }
            // ExportDeclaration : export Declaration
            // 1. Return ? Evaluation of Declaration.
            ast::Declaration::FunctionDeclaration(decl) => {
                decl.compile(ctx);
                ControlFlow::Continue(())
            }
            ast::Declaration::ClassDeclaration(decl) => {
                if let Err(err) = decl.compile(ctx) {
                    ControlFlow::Break(err.into())
                } else {
                    ControlFlow::Continue(())
                }
            }
            ast::Declaration::TSTypeAliasDeclaration(_)
            | ast::Declaration::TSInterfaceDeclaration(_) => ControlFlow::Continue(()),
            #[cfg(feature = "typescript")]
            ast::Declaration::TSEnumDeclaration(decl) => {
                decl.compile(ctx);
                ControlFlow::Continue(())
            }
            #[cfg(feature = "typescript")]
            ast::Declaration::TSModuleDeclaration(_) => {
                // TODO: implement when module declarations are supported
                ControlFlow::Continue(())
            }
            #[cfg(feature = "typescript")]
            ast::Declaration::TSImportEqualsDeclaration(_) => {
                // TODO: implement when import equals declarations are supported
                ControlFlow::Continue(())
            }
            #[cfg(feature = "typescript")]
            ast::Declaration::TSGlobalDeclaration(_) => {
                // Global declarations don't generate runtime code
                ControlFlow::Continue(())
            }
            #[cfg(not(feature = "typescript"))]
            ast::Declaration::TSEnumDeclaration(_)
            | ast::Declaration::TSModuleDeclaration(_)
            | ast::Declaration::TSImportEqualsDeclaration(_)
            | ast::Declaration::TSGlobalDeclaration(_) => unreachable!(),
        }
    }
}
