// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashMap;

use super::{CompileContext, CompileLabelledEvaluation};

impl<'s> CompileLabelledEvaluation<'s> for oxc_ast::ast::LabeledStatement<'s> {
    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s oxc_ast::ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) {
        let mut local_label_set: Vec<&'s oxc_ast::ast::LabelIdentifier<'s>>;
        let label_set = if let Some(label_set) = label_set {
            label_set.push(&self.label);
            label_set
        } else {
            local_label_set = vec![&self.label];
            &mut local_label_set
        };
        if ctx.labelled_statements.is_none() {
            ctx.labelled_statements = Some(Box::new(AHashMap::with_capacity(1)));
        }
        self.body.compile_labelled(Some(label_set), ctx);
    }
}
