#![feature(rustc_private)]
#![allow(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_arena;
extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_hir_analysis;
extern crate rustc_hir_pretty;
extern crate rustc_hir_typeck;
extern crate rustc_index;
extern crate rustc_infer;
extern crate rustc_lexer;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_mir_dataflow;
extern crate rustc_parse;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_trait_selection;

mod agent_comes_first;
mod can_use_no_gc_scope;
mod gc_scope_comes_last;
mod gc_scope_is_only_passed_by_value;
mod no_it_performs_the_following;
mod no_multipage_spec;
mod spec_header_level;
mod utils;

pub(crate) use utils::*;

#[unsafe(no_mangle)]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    agent_comes_first::register_lints(sess, lint_store);
    can_use_no_gc_scope::register_lints(sess, lint_store);
    gc_scope_comes_last::register_lints(sess, lint_store);
    gc_scope_is_only_passed_by_value::register_lints(sess, lint_store);
    no_it_performs_the_following::register_lints(sess, lint_store);
    no_multipage_spec::register_lints(sess, lint_store);
    spec_header_level::register_lints(sess, lint_store);
}

#[test]
fn ui_examples() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
