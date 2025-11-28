use std::sync::LazyLock;

use clippy_utils::diagnostics::span_lint_and_then;
use regex::{Regex, RegexBuilder};
use rustc_ast::Attribute;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_span::{BytePos, Span};

dylint_linting::declare_early_lint! {
    /// ### What it does
    ///
    /// This lint disallows doc comments that contain the phrase
    /// "It performs the following steps when called", or any variation of it
    /// like: "This method performs the following steps when called".
    ///
    /// ### Why is this bad?
    ///
    /// This phrase is a leftover from copy-pasting the TC-39 specification and
    /// does not add value to the documentation.
    ///
    /// ### Example
    ///
    /// ```rust
    /// /// 7.1.2 ToBoolean ( argument )
    /// ///
    /// /// The abstract operation ToBoolean takes argument argument (an ECMAScript
    /// /// language value) and returns a Boolean. It converts argument to a value of
    /// /// type Boolean. It performs the following steps when called:
    /// fn to_boolean() {
    ///     unimplemented!()
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// /// 7.1.2 ToBoolean ( argument )
    /// ///
    /// /// The abstract operation ToBoolean takes argument argument (an ECMAScript
    /// /// language value) and returns a Boolean. It converts argument to a value of
    /// /// type Boolean.
    /// fn to_boolean() {
    ///     unimplemented!()
    /// }
    /// ```
    pub NO_IT_PERFORMS_THE_FOLLOWING,
    Warn,
    "you should omit \"It performs the following steps when called\" from spec comments"
}

impl EarlyLintPass for NoItPerformsTheFollowing {
    // TODO: This should check for multiline comments too, probably using the
    // `check_attributes` method instead because each doc comment line is it's
    // own attribute.
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            RegexBuilder::new(r"(This method|It) performs the following steps when called:?")
                .case_insensitive(true)
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

        span_lint_and_then(
            cx,
            NO_IT_PERFORMS_THE_FOLLOWING,
            span,
            format!(
                "this comment contains \"{}\", a leftover from copy-pasting the TC-39 specification",
                matched.as_str()
            ),
            |diag| {
                diag.span_suggestion_hidden(
                    span,
                    "consider removing this phrase",
                    "",
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}
