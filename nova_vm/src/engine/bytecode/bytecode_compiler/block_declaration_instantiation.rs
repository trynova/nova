// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ecmascript::BoundNames;

use crate::engine::bytecode::bytecode_compiler::{
    StatementResult, compile_context::BlockEnvPrep, variable_escapes_scope,
};

use super::{
    CompileContext, CompileEvaluation, Instruction, LexicallyScopedDeclaration,
    LexicallyScopedDeclarations,
};

/// ### [14.2.3 BlockDeclarationInstantiation ( code, env )](https://tc39.es/ecma262/#sec-blockdeclarationinstantiation)
///
/// This can clobber the result register and can push additional items to the top of the stack.
///
/// The abstract operation BlockDeclarationInstantiation takes arguments code
/// (a Parse Node) and env (a Declarative Environment Record) and returns
/// unused. code is the Parse Node corresponding to the body of the block. env
/// is the Environment Record in which bindings are to be created.
///
/// > Note:
/// >
/// > When a Block or CaseBlock is evaluated a new Declarative Environment
/// > Record is created and bindings for each block scoped variable, constant,
/// > function, or class declared in the block are instantiated in the
/// > Environment Record.
pub(super) fn instantiation<'s, 'gc>(
    ctx: &mut CompileContext<'_, 's, 'gc, '_>,
    code: &'s impl LexicallyScopedDeclarations<'s>,
    cb: impl FnOnce(&mut CompileContext<'_, 's, 'gc, '_>) -> StatementResult<'gc>,
) -> StatementResult<'gc> {
    let mut block_prep = Vec::new();
    // 1. Let declarations be the LexicallyScopedDeclarations of code.
    // 2. Let privateEnv be the running execution context's PrivateEnvironment.
    // 3. For each element d of declarations, do
    code.lexically_scoped_declarations(&mut |d| {
        handle_block_lexically_scoped_declaration(ctx, &mut block_prep, d);
    });

    // 4. Return unused.
    let result = cb(ctx);

    for prop in block_prep.into_iter().rev() {
        prop.exit(ctx);
    }
    result
}

fn handle_block_lexically_scoped_declaration<'s>(
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    block_prop: &mut Vec<BlockEnvPrep>,
    d: LexicallyScopedDeclaration<'s>,
) {
    match d {
        // a. For each element dn of the BoundNames of d, do
        LexicallyScopedDeclaration::Variable(decl) if decl.kind.is_const() => {
            // i. If IsConstantDeclaration of d is true, then
            decl.id.bound_names(&mut |identifier| {
                if handle_lexical_variable(ctx, identifier, block_prop, None) {
                    let dn = ctx.create_string(&identifier.name);
                    // 1. Perform ! env.CreateImmutableBinding(dn, true).
                    ctx.add_instruction_with_identifier(
                        Instruction::CreateImmutableBinding,
                        dn.to_property_key(),
                    );
                }
            })
        }
        // ii. Else,
        LexicallyScopedDeclaration::Variable(decl) => decl.id.bound_names(&mut |identifier| {
            if handle_lexical_variable(ctx, identifier, block_prop, None) {
                // 1. Perform ! env.CreateMutableBinding(dn, false).
                // NOTE: This step is replaced in section B.3.2.6.
                let dn = ctx.create_string(&identifier.name);
                ctx.add_instruction_with_identifier(
                    Instruction::CreateMutableBinding,
                    dn.to_property_key(),
                );
            }
        }),
        LexicallyScopedDeclaration::Function(decl) => {
            // b. If d is either a FunctionDeclaration,
            // a GeneratorDeclaration, an AsyncFunctionDeclaration,
            // or an AsyncGeneratorDeclaration, then
            // i. Let fn be the sole element of the BoundNames of d.
            let Some(identifier) = &decl.id else {
                unreachable!()
            };
            if handle_lexical_variable(ctx, identifier, block_prop, Some(decl)) {
                let dn = ctx.create_string(&identifier.name);
                // 1. Perform ! env.CreateMutableBinding(dn, false).
                // NOTE: This step is replaced in section B.3.2.6.
                ctx.add_instruction_with_identifier(
                    Instruction::CreateMutableBinding,
                    dn.to_property_key(),
                );
                // ii. Let fo be InstantiateFunctionObject of d with arguments env and privateEnv.
                decl.compile(ctx);
                // iii. Perform ! env.InitializeBinding(fn, fo).
                ctx.add_instruction_with_identifier(
                    Instruction::ResolveBinding,
                    dn.to_property_key(),
                );
                ctx.add_instruction(Instruction::InitializeReferencedBinding);
                // NOTE: This step is replaced in section B.3.2.6.
            }
        }
        LexicallyScopedDeclaration::Class(decl) => {
            decl.bound_names(&mut |identifier| {
                if handle_lexical_variable(ctx, identifier, block_prop, None) {
                    // 1. Perform ! env.CreateMutableBinding(dn, false).
                    // NOTE: This step is replaced in section B.3.2.6.
                    let dn = ctx.create_string(&identifier.name);
                    ctx.add_instruction_with_identifier(
                        Instruction::CreateMutableBinding,
                        dn.to_property_key(),
                    );
                }
            });
        }
        LexicallyScopedDeclaration::DefaultExport => unreachable!(),
        #[cfg(feature = "typescript")]
        LexicallyScopedDeclaration::TSEnum(decl) => {
            if handle_lexical_variable(ctx, &decl.id, decl_env, block_prop, None) {
                let dn = ctx.create_string(&decl.id.name);
                // Create mutable binding for the enum
                ctx.add_instruction_with_identifier(
                    Instruction::CreateMutableBinding,
                    dn.to_property_key(),
                );
            }
        }
    }
}

fn handle_lexical_variable<'s>(
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    identifier: &oxc_ast::ast::BindingIdentifier,
    block_prop: &mut Vec<BlockEnvPrep>,
    f: Option<&'s oxc_ast::ast::Function<'s>>,
) -> bool {
    if variable_escapes_scope(ctx, identifier) {
        if !block_prop.iter().any(|p| p.is_env()) {
            block_prop.push(BlockEnvPrep::Env(ctx.enter_lexical_scope()));
        }
        true
    } else {
        let var = if let Some(f) = f {
            f.compile(ctx);
            ctx.push_stack_variable(identifier.symbol_id(), true)
        } else {
            ctx.push_stack_variable(identifier.symbol_id(), false)
        };
        block_prop.push(BlockEnvPrep::Var(var));
        false
    }
}
