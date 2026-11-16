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

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::*;

    /// The text of a predicate or an attribute, spelled as `quote!` spells
    /// the tokens a hand writes.
    fn spelled(tokens: TokenStream2) -> String {
        tokens.to_string()
    }

    fn written(attributes: &[Attribute]) -> Option<String> {
        Builds::of(attributes)
            .attribute(Span::call_site())
            .map(spelled)
    }

    #[test]
    fn the_cfg_attributes_of_an_item_are_one_predicate_or_all_of_several() {
        let none: Vec<Attribute> = vec![parse_quote!(#[inline])];
        assert!(Predicate::of(&none).is_none());
        let one: Vec<Attribute> = vec![parse_quote!(#[inline]), parse_quote!(#[cfg(unix)])];
        assert_eq!(
            Predicate::of(&one).map(|predicate| predicate.key()),
            Some(spelled(quote!(unix)))
        );
        let several: Vec<Attribute> = vec![
            parse_quote!(#[cfg(unix)]),
            parse_quote!(#[allow(dead_code)]),
            parse_quote!(#[cfg(feature = "named")]),
        ];
        assert_eq!(
            Predicate::of(&several).map(|predicate| predicate.key()),
            Some(spelled(quote!(all(unix, feature = "named"))))
        );
    }

    #[test]
    fn an_item_without_a_cfg_exists_in_every_build() {
        let plain: Vec<Attribute> = vec![parse_quote!(#[inline])];
        assert!(Builds::of(&plain).is_every());
        assert!(!Builds::of(&plain).is_never());
        assert_eq!(
            written(&plain),
            None,
            "an item of every build takes no `#[cfg]`"
        );
        let conditional: Vec<Attribute> = vec![parse_quote!(#[cfg(unix)])];
        assert!(!Builds::of(&conditional).is_every());
        assert!(!Builds::of(&conditional).is_never());
        assert_eq!(written(&conditional), Some(spelled(quote!(#[cfg(unix)]))));
        assert_eq!(
            Builds::Never.attribute(Span::call_site()).map(spelled),
            Some(spelled(quote!(#[cfg(any())]))),
            "no build is the predicate that never holds"
        );
    }

    #[test]
    fn every_build_and_no_build_are_the_ends_of_a_union() {
        let unix = || Builds::of(&[parse_quote!(#[cfg(unix)])]);
        assert!(unix().or(Builds::Every).is_every());
        assert!(Builds::Every.or(unix()).is_every());
        let attribute = |builds: Builds| builds.attribute(Span::call_site()).map(spelled);
        assert_eq!(
            attribute(Builds::Never.or(unix())),
            Some(spelled(quote!(#[cfg(unix)])))
        );
        assert_eq!(
            attribute(unix().or(Builds::Never)),
            Some(spelled(quote!(#[cfg(unix)])))
        );
        assert!(Builds::Never.or(Builds::Never).is_never());
        let three = unix()
            .or(Builds::of(&[parse_quote!(#[cfg(test)])]))
            .or(Builds::of(&[parse_quote!(#[cfg(windows)])]));
        assert_eq!(
            attribute(three),
            Some(spelled(quote!(#[cfg(any(unix, test, windows))]))),
            "a union grows one `any` rather than nesting one in another"
        );
    }

    #[test]
    fn the_complement_swaps_the_ends_and_negates_a_predicate() {
        assert!(Builds::Never.complement().is_every());
        assert!(Builds::Every.complement().is_never());
        let unix = Builds::of(&[parse_quote!(#[cfg(unix)])]);
        assert_eq!(
            unix.complement().attribute(Span::call_site()).map(spelled),
            Some(spelled(quote!(#[cfg(not(unix))]))),
            "a composition of one predicate is that predicate, not `not(any(all(unix)))`"
        );
    }

    #[test]
    fn the_builds_two_conditional_items_leave_read_as_a_hand_writes_them() {
        let first: Vec<Attribute> = vec![
            parse_quote!(#[cfg(feature = "named")]),
            parse_quote!(#[cfg(not(test))]),
        ];
        let second: Vec<Attribute> = vec![parse_quote!(#[cfg(unix)])];
        let left = Builds::of(&first).or(Builds::of(&second)).complement();
        assert_eq!(
            left.attribute(Span::call_site()).map(spelled),
            Some(spelled(
                quote!(#[cfg(not(any(all(feature = "named", not(test)), unix)))])
            ))
        );
    }
}
