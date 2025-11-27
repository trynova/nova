use std::sync::LazyLock;

use clippy_utils::diagnostics::span_lint_and_sugg;
use regex::{Regex, RegexBuilder};
use rustc_ast::Attribute;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::{BytePos, Span};

dylint_linting::declare_early_lint! {
    /// ### What it does
    ///
    /// This lint disallows linking to the multi-page TC-39 specification in documentation comments.
    ///
    /// ### Why is this bad?
    ///
    /// For nova the general practice is to link to the single-page version of the TC-39 specification.
    ///
    /// ### Example
    ///
    /// ```rust
    /// /// [7.1.2 ToBoolean ( argument )](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-toboolean)
    /// fn to_boolean() {
    ///     unimplemented!()
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// /// [7.1.2 ToBoolean ( argument )](https://tc39.es/ecma262/#sec-toboolean)
    /// fn to_boolean() {
    ///     unimplemented!()
    /// }
    /// ```
    pub NO_MULTIPAGE_SPEC,
    Warn,
    "you should link to the single-page spec instead of the multi-page spec"
}

impl EarlyLintPass for NoMultipageSpec {
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            RegexBuilder::new(r"https?://tc39.es/ecma262/multipage/?[^#]*")
                .build()
                .unwrap()
        });

        let Some(doc) = attr.doc_str().map(|sym| sym.to_string()) else {
            return;
        };
        let Some(matched) = RE.find(&doc) else { return };
        if matched.is_empty() {
            return;
        }

        let span = Span::new(
            attr.span.lo() + BytePos(matched.start() as u32 + 3),
            attr.span.lo() + BytePos(matched.end() as u32 + 3),
            attr.span.ctxt(),
            attr.span.parent(),
        );

        span_lint_and_sugg(
            cx,
            NO_MULTIPAGE_SPEC,
            span,
            "linking to the multi-page TC-39 specification",
            "use the single-page version instead",
            "https://tc39.es/ecma262/".to_owned(),
            Applicability::MachineApplicable,
        );
    }
}
