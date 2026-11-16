//! The builds something exists in.
//!
//! An item under `#[cfg]` exists in some builds and not in others, and the
//! impl follows: an arm sits under its variant's attributes, a forwarded
//! method takes the builds a partial block's conditional items leave, and a
//! variant no method can forward to is reported where some method forwards
//! to every variant. A [`Predicate`] is a `cfg` condition, as written or
//! composed, and [`Builds`] is the set of builds one names, with the two
//! ends a predicate cannot say: every build and none.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote_spanned;
use syn::{Attribute, Meta};

/// A `cfg` predicate: what one `#[cfg(..)]` attribute holds, or a
/// composition of several. A composition of one predicate is that
/// predicate, as a hand writes it.
#[derive(Clone)]
pub(crate) enum Predicate {
    /// The tokens between the parentheses of one `#[cfg(..)]`, as written.
    Written(TokenStream2),
    /// `all(..)`: every one of them holds.
    All(Vec<Predicate>),
    /// `any(..)`: at least one of them holds.
    Any(Vec<Predicate>),
    /// `not(..)`.
    Not(Box<Predicate>),
}

impl Predicate {
    /// Returns what the `#[cfg]` attributes among `attributes` amount to:
    /// the one predicate, `all` of several, or nothing for an item that
    /// has none.
    pub(crate) fn of(attributes: &[Attribute]) -> Option<Self> {
        let written: Vec<Predicate> = attributes
            .iter()
            .filter(|attribute| attribute.path().is_ident("cfg"))
            .map(|attribute| match &attribute.meta {
                Meta::List(list) => Predicate::Written(list.tokens.clone()),
                _ => Predicate::Written(TokenStream2::new()),
            })
            .collect();
        if written.is_empty() {
            return None;
        }
        Some(Self::all(written))
    }

    /// Returns the predicate that holds where every one of `predicates`
    /// does.
    pub(crate) fn all(mut predicates: Vec<Predicate>) -> Self {
        if predicates.len() == 1 {
            return predicates.remove(0);
        }
        Predicate::All(predicates)
    }

    /// Returns the predicate that holds where this one or `other` does.
    fn or(self, other: Predicate) -> Self {
        match self {
            Predicate::Any(mut predicates) => {
                predicates.push(other);
                Predicate::Any(predicates)
            }
            first => Predicate::Any(vec![first, other]),
        }
    }

    /// Returns the predicate that holds where this one does not.
    pub(crate) fn not(self) -> Self {
        Predicate::Not(Box::new(self))
    }

    /// Writes the predicate; the tokens a composition adds carry `span`.
    pub(crate) fn tokens(&self, span: Span) -> TokenStream2 {
        match self {
            Predicate::Written(tokens) => tokens.clone(),
            Predicate::All(predicates) => {
                let predicates = predicates.iter().map(|predicate| predicate.tokens(span));
                quote_spanned!(span => all(#(#predicates),*))
            }
            Predicate::Any(predicates) => {
                let predicates = predicates.iter().map(|predicate| predicate.tokens(span));
                quote_spanned!(span => any(#(#predicates),*))
            }
            Predicate::Not(predicate) => {
                let predicate = predicate.tokens(span);
                quote_spanned!(span => not(#predicate))
            }
        }
    }

    /// Returns the text that tells two predicates apart.
    pub(crate) fn key(&self) -> String {
        self.tokens(Span::call_site()).to_string()
    }
}

/// The builds in which something exists.
#[derive(Clone)]
pub(crate) enum Builds {
    /// In no build.
    Never,
    /// Where the predicate holds.
    Under(Predicate),
    /// In every build.
    Every,
}

impl Builds {
    /// Returns the builds an item carrying `attributes` exists in: every
    /// build without a `#[cfg]` among them, and otherwise where all of
    /// them hold.
    pub(crate) fn of(attributes: &[Attribute]) -> Self {
        match Predicate::of(attributes) {
            Some(predicate) => Builds::Under(predicate),
            None => Builds::Every,
        }
    }

    /// Returns whether these are all the builds there are.
    pub(crate) fn is_every(&self) -> bool {
        matches!(self, Builds::Every)
    }

    /// Returns whether these are no builds at all.
    pub(crate) fn is_never(&self) -> bool {
        matches!(self, Builds::Never)
    }

    /// Returns these builds and those of `other` together.
    pub(crate) fn or(self, other: Builds) -> Self {
        match (self, other) {
            (Builds::Every, _) | (_, Builds::Every) => Builds::Every,
            (Builds::Never, builds) | (builds, Builds::Never) => builds,
            (Builds::Under(first), Builds::Under(second)) => Builds::Under(first.or(second)),
        }
    }

    /// Returns the builds these leave.
    pub(crate) fn complement(&self) -> Self {
        match self {
            Builds::Never => Builds::Every,
            Builds::Under(predicate) => Builds::Under(predicate.clone().not()),
            Builds::Every => Builds::Never,
        }
    }

    /// Returns the `#[cfg(..)]` that puts an item in these builds, its own
    /// tokens at `span`; an item of every build takes none.
    pub(crate) fn attribute(&self, span: Span) -> Option<TokenStream2> {
        match self {
            Builds::Every => None,
            Builds::Under(predicate) => {
                let predicate = predicate.tokens(span);
                Some(quote_spanned!(span => #[cfg(#predicate)]))
            }
            Builds::Never => Some(quote_spanned!(span => #[cfg(any())])),
        }
    }
}
