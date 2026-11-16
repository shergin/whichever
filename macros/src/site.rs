//! Where a generated token is written.
//!
//! Inside `implement!` the proc macro's call site is the carrier's
//! expansion, which `#[dispatchable]` wrote, so a token written there puts
//! rustc's errors on the trait's attribute, in another crate's file when
//! the trait is foreign. Every token the engine writes is therefore
//! written at a [`Site`]: a span located at a token the user wrote, the
//! subject's, a method's or a payload's. A function that writes generated
//! code takes a `Site`, which only [`Site::at`] makes, so a span that was
//! never located cannot reach it.
use std::ops::Deref;

use proc_macro2::Span;

/// Returns the span a generated token carries: located at `span`, a token
/// the user wrote, so that an error rustc finds in the generated code lands
/// there, and resolved at the expansion's own site, so that lints treat the
/// token as the expansion's and not as the user's.
pub(crate) fn at(span: Span) -> Span {
    Span::mixed_site().located_at(span)
}

/// The span the tokens of one generated item, arm or check carry.
#[derive(Clone, Copy)]
pub(crate) struct Site(Span);

impl Site {
    /// The site located at `span`, a token the user wrote.
    pub(crate) fn at(span: Span) -> Self {
        Site(at(span))
    }
}

impl Deref for Site {
    type Target = Span;

    fn deref(&self) -> &Span {
        &self.0
    }
}
