// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast;

use crate::engine::{
    Instruction,
    bytecode::bytecode_compiler::{ExpressionError, ExpressionOutput},
};

use super::{CompileContext, CompileEvaluation};

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::WithStatement<'s> {
    type Output = Result<ExpressionOutput<'s, 'gc>, ExpressionError>;
    /// ### [14.11.2 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-with-statement-runtime-semantics-evaluation)
    ///
    /// ```text
    /// WithStatement : with ( Expression ) Statement
    /// ```
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // 1. Let val be ? Evaluation of Expression.
        let val = self.object.compile(ctx)?;
        // 2. Let obj be ? ToObject(? GetValue(val)).
        let _obj = val.get_value(ctx)?;
        ctx.add_instruction(Instruction::ToObject);
        // 3. Let oldEnv be the running execution context's LexicalEnvironment.
        // 4. Let newEnv be NewObjectEnvironment(obj, true, oldEnv).
        // 5. Set the running execution context's LexicalEnvironment to newEnv.
        // 6. Let C be Completion(Evaluation of Statement).
        let _c = self.body.compile(ctx);
        // 7. Set the running execution context's LexicalEnvironment to oldEnv.
        // 8. Return ? UpdateEmpty(C, undefined).
        Ok(ExpressionOutput::Value)
    }
}
