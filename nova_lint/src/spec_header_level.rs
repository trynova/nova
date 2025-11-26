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
    ///
    /// ### Why is this bad?
    ///
    ///
    ///
    /// ### Example
    ///
    /// ```rust
    /// /// ## [7.1.2 ToBoolean ( argument )](https://tc39.es/ecma262/#sec-toboolean)
    /// fn to_boolean() {
    ///     unimplemented!()
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// /// ### [7.1.2 ToBoolean ( argument )](https://tc39.es/ecma262/#sec-toboolean)
    /// fn to_boolean() {
    ///     unimplemented!()
    /// }
    /// ```
    pub SPEC_HEADER_LEVEL,
    Warn,
    "you should match the header level of the TC-39 spec in your documentation comments"
}

impl EarlyLintPass for SpecHeaderLevel {
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            RegexBuilder::new(r"(#*)\s\[([0-9.]*)[^]]*]\(https?://tc39\.es/ecma262/.*\)")
                .build()
                .unwrap()
        });

        let Some(doc) = attr.doc_str().map(|sym| sym.to_string()) else {
            return;
        };
        let Some(captures) = RE.captures(&doc) else {
            return;
        };

        let header_level = captures
            .get(1)
            .map(|hashes| hashes.len() as u32)
            .unwrap_or(0);
        let expected_level = captures
            .get(2)
            .map(|numbering| numbering.as_str().chars().filter(|&c| c == '.').count() as u32 + 1)
            .unwrap_or(0);

        if header_level == expected_level {
            return;
        }

        let span = Span::new(
            attr.span.lo() + BytePos(header_level + 3),
            attr.span.lo() + BytePos(header_level + 4),
            attr.span.ctxt(),
            attr.span.parent(),
        );

        span_lint_and_sugg(
            cx,
            SPEC_HEADER_LEVEL,
            span,
            "the header level of your comment and the TC-39 specification does not match",
            "use the correct header level",
            "#".repeat(expected_level as usize),
            Applicability::MachineApplicable,
        );
    }
}
