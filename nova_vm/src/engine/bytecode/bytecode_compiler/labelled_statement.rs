// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{CompileContext, CompileEvaluation, CompileLabelledEvaluation};

impl<'s> CompileLabelledEvaluation<'s> for oxc_ast::ast::LabeledStatement<'s> {
    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s oxc_ast::ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) {
        ctx.enter_label(&self.label);
        let mut local_label_set: Vec<&'s oxc_ast::ast::LabelIdentifier<'s>>;
        let label_set = if let Some(label_set) = label_set {
            label_set.push(&self.label);
            Some(label_set)
        } else {
            local_label_set = vec![&self.label];
            Some(&mut local_label_set)
        };
        match &self.body {
            oxc_ast::ast::Statement::DoWhileStatement(st) => st.compile_labelled(label_set, ctx),
            oxc_ast::ast::Statement::ForInStatement(st) => st.compile_labelled(label_set, ctx),
            oxc_ast::ast::Statement::ForOfStatement(st) => st.compile_labelled(label_set, ctx),
            oxc_ast::ast::Statement::ForStatement(st) => st.compile_labelled(label_set, ctx),
            oxc_ast::ast::Statement::LabeledStatement(st) => st.compile_labelled(label_set, ctx),
            oxc_ast::ast::Statement::SwitchStatement(st) => st.compile_labelled(label_set, ctx),
            oxc_ast::ast::Statement::WhileStatement(st) => st.compile_labelled(label_set, ctx),
            _ => self.body.compile(ctx),
        }
        ctx.exit_label();
    }
}
