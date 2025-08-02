// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast;

use crate::engine::Instruction;

use super::{CompileContext, CompileEvaluation, compile_expression_get_value};

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::WithStatement<'s> {
    type Output = ();
    /// ### [14.11.2 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-with-statement-runtime-semantics-evaluation)
    ///
    /// ```text
    /// WithStatement : with ( Expression ) Statement
    /// ```
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) {
        // 1. Let val be ? Evaluation of Expression.
        // 2. Let obj be ? ToObject(? GetValue(val)).
        compile_expression_get_value(&self.object, ctx);
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
