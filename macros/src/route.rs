//! How an arm reaches its target from the bound payload: the route.
//!
//! A plain variant's arm calls the trait on the payload itself. A route
//! names the steps from the payload to the value the trait is called on:
//! `*` for what the payload dereferences to, `.field` or `.0` for a field,
//! `.method()` for what a method returns. `deref` is the route `*`.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Index, LitFloat, LitInt, Member, Result, Token};

use crate::spelling;

/// One step of a route.
#[derive(Clone)]
pub(crate) enum Step {
    /// `*`: what the value dereferences to.
    Deref,
    /// `.name` or `.0`: a field of the value.
    Field(Member),
    /// `.name()`: what a method of the value returns.
    Method(Ident),
}

/// The steps from the bound payload to the target; empty for the payload
/// itself.
#[derive(Clone, Default)]
pub(crate) struct Route {
    pub(crate) steps: Vec<Step>,
}

impl Route {
    /// The route `deref` spells.
    pub(crate) fn deref() -> Self {
        Route {
            steps: vec![Step::Deref],
        }
    }

    /// Returns whether the target is the payload itself.
    pub(crate) fn is_direct(&self) -> bool {
        self.steps.is_empty()
    }

    /// Returns whether the route is `*` alone.
    pub(crate) fn is_deref(&self) -> bool {
        matches!(self.steps.as_slice(), [Step::Deref])
    }

    /// Returns whether the first step dereferences the payload, so that a
    /// value reached through the route is moved out of what the payload
    /// points at.
    pub(crate) fn starts_with_deref(&self) -> bool {
        matches!(self.steps.first(), Some(Step::Deref))
    }

    /// Returns whether the target's type can be named from the payload's:
    /// the payload itself, or what it dereferences to. A field's or a
    /// method's type cannot be, and the call is qualified by the trait
    /// alone, `<_ as Trait>::method(..)`.
    pub(crate) fn names_the_target(&self) -> bool {
        self.is_direct() || self.is_deref()
    }

    /// Returns whether the last step is a method call, whose result is a
    /// value and not a place.
    pub(crate) fn ends_in_a_method(&self) -> bool {
        matches!(self.steps.last(), Some(Step::Method(_)))
    }

    /// Returns whether the last step dereferences, so that the expression
    /// `reach` writes ends in a prefix `*`.
    pub(crate) fn ends_in_a_deref(&self) -> bool {
        matches!(self.steps.last(), Some(Step::Deref))
    }

    /// Returns the expression reaching the target from `binding`, which is
    /// a reference to the payload when `through_reference` and the payload
    /// itself otherwise. Field and method steps go through the reference by
    /// themselves; a deref step is written out, with the payload behind the
    /// reference dereferenced first.
    pub(crate) fn reach(
        &self,
        binding: &Ident,
        through_reference: bool,
        site: Span,
    ) -> TokenStream2 {
        let mut expression = quote_spanned!(site => #binding);
        let mut derefs = 0usize;
        for (index, step) in self.steps.iter().enumerate() {
            match step {
                Step::Deref => {
                    if index == 0 && through_reference {
                        // The reference first, then the payload.
                        expression = quote_spanned!(site => **#expression);
                        derefs = 2;
                    } else {
                        expression = quote_spanned!(site => *#expression);
                        derefs += 1;
                    }
                }
                Step::Field(member) => {
                    if derefs > 0 {
                        expression = quote_spanned!(site => (#expression).#member);
                    } else {
                        expression = quote_spanned!(site => #expression.#member);
                    }
                    derefs = 0;
                }
                Step::Method(method) => {
                    if derefs > 0 {
                        expression = quote_spanned!(site => (#expression).#method());
                    } else {
                        expression = quote_spanned!(site => #expression.#method());
                    }
                    derefs = 0;
                }
            }
        }
        expression
    }

    /// Returns the expression `reach` writes, ready for a field or a method
    /// call to follow it. A prefix `*` binds looser than both, so a route
    /// ending in a deref is parenthesised: `(**inner).radius` reads the
    /// field of the target, where `**inner.radius` would dereference the
    /// field.
    pub(crate) fn reach_for_postfix(
        &self,
        binding: &Ident,
        through_reference: bool,
        site: Span,
    ) -> TokenStream2 {
        let expression = self.reach(binding, through_reference, site);
        if self.ends_in_a_deref() {
            return quote_spanned!(site => (#expression));
        }
        expression
    }

    /// Spells the route as it is written, for a message.
    pub(crate) fn spelled(&self) -> String {
        spelling::spelled(self)
    }
}

impl Parse for Route {
    /// `*`, `.name`, `.0` and `.name()` in any sequence, up to a comma or
    /// the end of the input; at least one step.
    fn parse(input: ParseStream) -> Result<Self> {
        let mut steps = vec![];
        while !input.is_empty() && !input.peek(Token![,]) {
            if input.peek(Token![*]) {
                input.parse::<Token![*]>()?;
                steps.push(Step::Deref);
                continue;
            }
            if input.peek(Token![.]) {
                input.parse::<Token![.]>()?;
                if input.peek(Ident) {
                    let ident: Ident = input.parse()?;
                    if input.peek(syn::token::Paren) {
                        let content;
                        syn::parenthesized!(content in input);
                        if !content.is_empty() {
                            return Err(content.error("a method on a route takes no arguments"));
                        }
                        steps.push(Step::Method(ident));
                    } else {
                        steps.push(Step::Field(Member::Named(ident)));
                    }
                    continue;
                }
                if input.peek(LitInt) {
                    let index: LitInt = input.parse()?;
                    steps.push(Step::Field(Member::Unnamed(Index {
                        index: index.base10_parse()?,
                        span: index.span(),
                    })));
                    continue;
                }
                // `.0.1` is lexed as the float `0.1`.
                if input.peek(LitFloat) {
                    let float: LitFloat = input.parse()?;
                    for part in float.base10_digits().split('.') {
                        steps.push(Step::Field(Member::Unnamed(Index {
                            index: part.parse().map_err(|_| {
                                syn::Error::new(float.span(), "expected a field index")
                            })?,
                            span: float.span(),
                        })));
                    }
                    continue;
                }
                return Err(
                    input.error("expected a field, `.name` or `.0`, or a method, `.name()`")
                );
            }
            return Err(input.error(
                "a route is `*` for what the payload dereferences to, `.name` or `.0` for a field, or `.name()` for what a method returns, in any sequence",
            ));
        }
        if steps.is_empty() {
            return Err(
                input.error("a route needs at least one step: `*`, `.name`, `.0` or `.name()`")
            );
        }
        Ok(Route { steps })
    }
}

impl ToTokens for Route {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        for step in &self.steps {
            match step {
                Step::Deref => tokens.extend(quote!(*)),
                Step::Field(member) => tokens.extend(quote!(.#member)),
                Step::Method(method) => tokens.extend(quote!(.#method())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(tokens: &str) -> Route {
        syn::parse_str(tokens).expect("a valid route")
    }

    fn spelled(tokens: TokenStream2) -> String {
        tokens.to_string().replace(' ', "")
    }

    #[test]
    fn a_route_is_read_and_written_back() {
        for text in [
            "*",
            ".base",
            ".1",
            ".get_ref()",
            "*.inner",
            ".0.1",
            ".a.b()",
            "*.a*",
        ] {
            assert_eq!(route(text).spelled(), text, "{text}");
        }
        assert!(syn::parse_str::<Route>("").is_err());
        assert!(syn::parse_str::<Route>("base").is_err());
        assert!(syn::parse_str::<Route>(".get(1)").is_err());
        assert!(route("*").is_deref() && route("*").names_the_target());
        assert!(!route(".a").names_the_target());
        assert!(route(".a.b()").ends_in_a_method());
    }

    #[test]
    fn the_target_is_reached_through_the_reference_or_from_the_payload() {
        let binding = Ident::new("inner", Span::call_site());
        let reach = |text: &str, through_reference: bool| {
            spelled(route(text).reach(&binding, through_reference, Span::call_site()))
        };
        assert_eq!(reach("*", true), "**inner");
        assert_eq!(reach("*", false), "*inner");
        assert_eq!(reach(".base", true), "inner.base");
        assert_eq!(reach(".base", false), "inner.base");
        assert_eq!(reach(".1", true), "inner.1");
        assert_eq!(reach(".get_ref()", true), "inner.get_ref()");
        assert_eq!(reach("*.inner", true), "(**inner).inner");
        assert_eq!(reach("*.inner", false), "(*inner).inner");
        assert_eq!(reach(".a*", true), "*inner.a");
    }

    #[test]
    fn a_route_ends_in_a_deref_when_its_last_step_is_one() {
        for text in ["*", ".boxed*", "*.a*", ".get_ref()*"] {
            assert!(route(text).ends_in_a_deref(), "{text}");
        }
        for text in [".base", "*.inner", ".a.b()", ".0.1"] {
            assert!(!route(text).ends_in_a_deref(), "{text}");
        }
        assert!(!Route::default().ends_in_a_deref());
    }

    #[test]
    fn a_route_ending_in_a_deref_is_parenthesised_for_a_postfix() {
        let binding = Ident::new("inner", Span::call_site());
        let reach = |text: &str, through_reference: bool| {
            spelled(route(text).reach_for_postfix(&binding, through_reference, Span::call_site()))
        };
        assert_eq!(reach("*", true), "(**inner)");
        assert_eq!(reach("*", false), "(*inner)");
        assert_eq!(reach(".boxed*", true), "(*inner.boxed)");
        assert_eq!(reach(".boxed*", false), "(*inner.boxed)");
        assert_eq!(reach("*.a*", true), "(*(**inner).a)");
        assert_eq!(reach(".get_ref()*", true), "(*inner.get_ref())");
        // A route ending in a field or a method is already a postfix
        // expression, and is written as `reach` writes it.
        assert_eq!(reach(".base", true), "inner.base");
        assert_eq!(reach("*.inner", true), "(**inner).inner");
        assert_eq!(reach(".a.b()", false), "inner.a.b()");
    }
}
