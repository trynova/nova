// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ecmascript::BoundNames;

use crate::ecmascript::types::String;

use super::{
    CompileContext, CompileEvaluation, Instruction, LexicallyScopedDeclaration,
    LexicallyScopedDeclarations,
};

/// ### [14.2.3 BlockDeclarationInstantiation ( code, env )](https://tc39.es/ecma262/#sec-blockdeclarationinstantiation)
///
/// The abstract operation BlockDeclarationInstantiation takes arguments code
/// (a Parse Node) and env (a Declarative Environment Record) and returns
/// unused. code is the Parse Node corresponding to the body of the block. env
/// is the Environment Record in which bindings are to be created.
///
/// > Note
/// >
/// > When a Block or CaseBlock is evaluated a new Declarative Environment
/// > Record is created and bindings for each block scoped variable, constant,
/// > function, or class declared in the block are instantiated in the
/// > Environment Record.
pub(super) fn instantiation<'s>(
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    code: &'s impl LexicallyScopedDeclarations<'s>,
) -> bool {
    let mut did_enter_declarative_environment = false;
    // 1. Let declarations be the LexicallyScopedDeclarations of code.
    // 2. Let privateEnv be the running execution context's PrivateEnvironment.
    // 3. For each element d of declarations, do
    code.lexically_scoped_declarations(&mut |d| {
        if !did_enter_declarative_environment {
            did_enter_declarative_environment = true;
            ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
            ctx.current_lexical_depth += 1;
        }
        handle_block_lexically_scoped_declaration(ctx, d);
    });

    // 4. Return unused.
    did_enter_declarative_environment
}

pub fn handle_block_lexically_scoped_declaration<'s>(
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    d: LexicallyScopedDeclaration<'s>,
) {
    match d {
        // a. For each element dn of the BoundNames of d, do
        LexicallyScopedDeclaration::Variable(decl) if decl.kind.is_const() => {
            // i. If IsConstantDeclaration of d is true, then
            decl.id.bound_names(&mut |identifier| {
                let dn = String::from_str(ctx.agent, &identifier.name, ctx.gc);
                // 1. Perform ! env.CreateImmutableBinding(dn, true).
                ctx.add_instruction_with_identifier(Instruction::CreateImmutableBinding, dn);
            })
        }
        // ii. Else,
        LexicallyScopedDeclaration::Variable(decl) => decl.id.bound_names(&mut |identifier| {
            // 1. Perform ! env.CreateMutableBinding(dn, false).
            // NOTE: This step is replaced in section B.3.2.6.
            let dn = String::from_str(ctx.agent, &identifier.name, ctx.gc);
            ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, dn);
        }),
        LexicallyScopedDeclaration::Function(decl) => {
            // b. If d is either a FunctionDeclaration,
            // a GeneratorDeclaration, an AsyncFunctionDeclaration,
            // or an AsyncGeneratorDeclaration, then
            // i. Let fn be the sole element of the BoundNames of d.
            let Some(r#fn) = &decl.id else { unreachable!() };
            let dn = String::from_str(ctx.agent, &r#fn.name, ctx.gc);
            // 1. Perform ! env.CreateMutableBinding(dn, false).
            // NOTE: This step is replaced in section B.3.2.6.
            ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, dn);
            // ii. Let fo be InstantiateFunctionObject of d with arguments env and privateEnv.
            decl.compile(ctx);
            // iii. Perform ! env.InitializeBinding(fn, fo).
            ctx.add_instruction_with_identifier(Instruction::ResolveBinding, dn);
            ctx.add_instruction(Instruction::InitializeReferencedBinding);
            // NOTE: This step is replaced in section B.3.2.6.
        }
        LexicallyScopedDeclaration::Class(decl) => {
            decl.bound_names(&mut |identifier| {
                // 1. Perform ! env.CreateMutableBinding(dn, false).
                // NOTE: This step is replaced in section B.3.2.6.
                let dn = String::from_str(ctx.agent, &identifier.name, ctx.gc);
                ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, dn);
            });
        }
        LexicallyScopedDeclaration::DefaultExport => unreachable!(),
    }
}
