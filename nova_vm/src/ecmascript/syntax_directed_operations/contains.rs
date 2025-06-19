// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [8.5 Contains](https://tc39.es/ecma262/#sec-static-semantics-contains)

use oxc_ast::ast;

pub(crate) trait Contains {
    fn contains(&self, check: fn(()) -> bool) -> bool;
}

impl Contains for Vec<ast::Statement<'_>> {
    fn contains(&self, check: fn(()) -> bool) -> bool {
        self.iter().any(|st| st.contains(check))
    }
}

impl Contains for ast::Function<'_> {
    fn contains(&self, _check: fn(()) -> bool) -> bool {
        false
    }
}

impl Contains for ast::Class<'_> {
    /// ClassTail : ClassHeritageopt { ClassBody }
    fn contains(&self, check: fn(()) -> bool) -> bool {
        // 1. If symbol is ClassBody, return true.
        // 2. If symbol is ClassHeritage, then
        if let Some(heritage) = &self.super_class {
            // a. If ClassHeritage is present, return true; otherwise return false.
            // 3. If ClassHeritage is present, then
            // a. If ClassHeritage Contains symbol is true, return true.
            if heritage.contains(check) {
                return true;
            }
        }
        // 4. Return the result of ComputedPropertyContains of ClassBody with argument symbol.

        // Note 2

        // Static semantic rules that depend upon substructure generally do not look into class bodies except for PropertyNames.
        false
    }
}
