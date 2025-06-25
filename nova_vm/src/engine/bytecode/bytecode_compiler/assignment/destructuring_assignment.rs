// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [13.15.5 Destructuring Assignment](https://tc39.es/ecma262/#sec-destructuring-assignment)

use crate::engine::CompileContext;

/// ### [13.15.5.2 Runtime Semantics: DestructuringAssignmentEvaluation](https://tc39.es/ecma262/#sec-runtime-semantics-destructuringassignmentevaluation)
trait CompileDestructuringAssignmentEvaluation<'s> {
    fn compile_destructuring_assignment(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>);
}
