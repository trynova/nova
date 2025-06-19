// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast;

use crate::engine::Instruction;

use super::{CompileContext, CompileEvaluation, is_reference};

impl<'s> CompileEvaluation<'s> for ast::WithStatement<'s> {
    /// ### [14.11.2 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-with-statement-runtime-semantics-evaluation)
    ///
    /// ```text
    /// WithStatement : with ( Expression ) Statement
    /// ```
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // 1. Let val be ? Evaluation of Expression.
        self.object.compile(ctx);
        // 2. Let obj be ? ToObject(? GetValue(val)).
        if is_reference(&self.object) {
            ctx.add_instruction(Instruction::GetValue);
        }
        ctx.add_instruction(Instruction::ToObject);
        // 3. Let oldEnv be the running execution context's LexicalEnvironment.
        // 4. Let newEnv be NewObjectEnvironment(obj, true, oldEnv).
        // 5. Set the running execution context's LexicalEnvironment to newEnv.
        // 6. Let C be Completion(Evaluation of Statement).
        self.body.compile(ctx);
        // 7. Set the running execution context's LexicalEnvironment to oldEnv.
        // 8. Return ? UpdateEmpty(C, undefined).
    }
}
