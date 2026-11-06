//! The grammars of `#[dispatch(..)]` and `#[dispatchable(..)]`, one typed
//! struct per position the attribute is written in: a subject
//! ([`DispatchAttribute`]), a partial impl block ([`TraitSpec`]), a variant
//! or a field ([`TargetOptions`]), a method of an inherent block
//! ([`ForwardingOptions`]), a trait method ([`method_mismatch`]) and a trait
//! ([`DispatchableOptions`]).
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    parenthesized, Attribute, Error, Expr, Ident, LitBool, LitInt, Member, Meta, Path, Result,
    Token, TraitItemFn, Type,
};

use crate::route::Route;

/// Returns the candidate closest to `name` when it is within one edit, or
/// within two when both names are long enough for two edits to leave them
/// recognisable, so that a message can say what was probably meant.
pub(crate) fn closest<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    candidates
        .into_iter()
        .filter_map(|candidate| {
            let distance = edit_distance(name, candidate);
            let close = distance == 1 || (distance == 2 && name.len() >= 5 && candidate.len() >= 5);
            close.then_some((distance, candidate))
        })
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

/// Returns the number of insertions, deletions, substitutions and
/// transpositions of adjacent characters that turn `a` into `b`.
fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut rows: Vec<Vec<usize>> = (0..=a.len()).map(|i| vec![i; b.len() + 1]).collect();
    for (j, cell) in rows[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (rows[i - 1][j] + 1)
                .min(rows[i][j - 1] + 1)
                .min(rows[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(rows[i - 2][j - 2] + 1);
            }
            rows[i][j] = best;
        }
    }
    rows[a.len()][b.len()]
}

/// Spells `names` out as a list, `` `a` ``, `` `a` and `b` `` or
/// `` `a`, `b` and `c` ``.
pub(crate) fn listed(names: &[String]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    match quoted.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// Returns whether `attribute` is this crate's `dispatch` attribute, written
/// bare or as `whichever::dispatch`.
pub(crate) fn is_dispatch_attribute(attribute: &Attribute) -> bool {
    attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "dispatch")
}

/// Parses options separated by commas, a trailing comma allowed, handing
/// each key to `option` with the input positioned right after it.
fn parse_options(
    input: ParseStream,
    mut option: impl FnMut(Ident, ParseStream) -> Result<()>,
) -> Result<()> {
    while !input.is_empty() {
        let key: Ident = input.parse()?;
        option(key, input)?;
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(())
}

/// A trait's options: the path written in `#[dispatch(Trait, ..)]` when
/// there is one, the `Name = ..` overrides and the `mismatch` results. This
/// is what travels to the implementation macro and what a partial impl
/// block's attribute holds.
pub(crate) struct TraitSpec {
    pub(crate) path: Option<Path>,
    pub(crate) overrides: Vec<Override>,
    pub(crate) mismatch: Vec<(Ident, Expr)>,
    /// The routes this trait takes through named variants, `via(Variant =
    /// .0, ..)`, over the variants' own.
    pub(crate) routes: Vec<(Ident, Route)>,
}

impl TraitSpec {
    pub(crate) fn override_for(&self, name: &Ident) -> Option<&Override> {
        self.overrides.iter().find(|fixed| &fixed.name == name)
    }

    /// Returns whether the associated type or const `name` is fixed on the
    /// subject.
    pub(crate) fn is_overridden(&self, name: &Ident) -> bool {
        self.override_for(name).is_some()
    }

    pub(crate) fn mismatch_for(&self, method: &Ident) -> Option<&Expr> {
        self.mismatch
            .iter()
            .find(|(key, _)| key == method)
            .map(|(_, expr)| expr)
    }

    /// Rejects a trait of `std`, `core` or `alloc`, which is never
    /// `#[dispatchable]`; rustc would only say that the macro of that name
    /// is missing.
    pub(crate) fn check_path(&self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let first = &path.segments[0].ident;
        if path.segments.len() > 1 && (first == "std" || first == "core" || first == "alloc") {
            let spelled = path.to_token_stream().to_string().replace(' ', "");
            let name = &path.segments.last().expect("checked above").ident;
            return Err(Error::new_spanned(
                path,
                format!("`{spelled}` is not `#[dispatchable]`; dispatch `whichever::foreign::{name}` if it is one of the traits `whichever` declares, or restate the trait with `#[whichever::dispatchable(remote = {spelled})]`"),
            ));
        }
        Ok(())
    }
}

impl Parse for TraitSpec {
    /// The trait's options alone; a conversion switch belongs on the
    /// subject.
    fn parse(input: ParseStream) -> Result<Self> {
        let DispatchAttribute { spec, switches } = input.parse()?;
        if let Some(span) = switches.first_span() {
            return Err(Error::new(
                span,
                "`from`, `try_from` and `into` are options of the enum; put them on its `#[dispatch]` attribute",
            ));
        }
        Ok(spec)
    }
}

impl ToTokens for TraitSpec {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let mut parts: Vec<TokenStream2> =
            self.path.iter().map(ToTokens::to_token_stream).collect();
        parts.extend(self.overrides.iter().map(ToTokens::to_token_stream));
        if !self.mismatch.is_empty() {
            let pairs = self
                .mismatch
                .iter()
                .map(|(method, expr)| quote!(#method = #expr));
            parts.push(quote!(mismatch(#(#pairs),*)));
        }
        if !self.routes.is_empty() {
            let pairs = self
                .routes
                .iter()
                .map(|(variant, route)| quote!(#variant = #route));
            parts.push(quote!(via(#(#pairs),*)));
        }
        tokens.extend(quote!(#(#parts),*));
    }
}

/// `#[dispatch(..)]` on a subject: a trait's options, and the subject's
/// conversion switches, which only `#[dispatch]` on the subject reads.
pub(crate) struct DispatchAttribute {
    pub(crate) spec: TraitSpec,
    pub(crate) switches: Switches,
}

impl Parse for DispatchAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        let starts_with_option =
            input.peek(Ident) && (input.peek2(Token![=]) || input.peek2(syn::token::Paren));
        let path = if input.is_empty() || starts_with_option {
            None
        } else {
            Some(input.parse::<Path>()?)
        };
        let mut spec = TraitSpec {
            path,
            overrides: vec![],
            mismatch: vec![],
            routes: vec![],
        };
        let mut switches = Switches::default();
        if let Some(path) = &spec.path {
            // `Item<'a> = ..` as the first thing is read as a trait path.
            if input.peek(Token![=]) {
                if let [segment] = path.segments.iter().collect::<Vec<_>>().as_slice() {
                    return Err(gat_override(&segment.ident, None));
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        parse_options(input, |key, input| {
            if key == "mismatch" {
                let content;
                parenthesized!(content in input);
                let pairs = Punctuated::<KeyedExpr, Token![,]>::parse_terminated(&content)?;
                for pair in pairs {
                    if spec.mismatch_for(&pair.key).is_some() {
                        return Err(given_twice(&pair.key));
                    }
                    spec.mismatch.push((pair.key, pair.value));
                }
                return Ok(());
            }
            if key == "via" && input.peek(syn::token::Paren) {
                let content;
                parenthesized!(content in input);
                let pairs = Punctuated::<KeyedRoute, Token![,]>::parse_terminated(&content)?;
                for pair in pairs {
                    if spec.routes.iter().any(|(variant, _)| *variant == pair.key) {
                        return Err(given_twice(&pair.key));
                    }
                    spec.routes.push((pair.key, pair.route));
                }
                return Ok(());
            }
            if input.peek(Token![<]) {
                return Err(gat_override(&key, spec.path.as_ref()));
            }
            if !input.peek(Token![=]) {
                return Err(Error::new(
                    key.span(),
                    format!("unknown option `{key}`; the options are `Name = Type` for an associated type, `Name = value` for an associated const, `mismatch(method = expr)`, `via(Variant = route)`, `from = false`, `try_from = true` and `into = Target`"),
                ));
            }
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "from" if switches.from.is_some() => return Err(given_twice(&key)),
                "try_from" if switches.try_from.is_some() => return Err(given_twice(&key)),
                "from" => switches.from = Some(Switch::parse(key, input)?),
                "try_from" => switches.try_from = Some(Switch::parse(key, input)?),
                "into" => switches.into.push(IntoTarget {
                    key,
                    target: input.parse()?,
                }),
                _ if spec.is_overridden(&key) => return Err(given_twice(&key)),
                _ => spec.overrides.push(Override::parse(key, input)?),
            }
            Ok(())
        })?;
        Ok(Self { spec, switches })
    }
}

fn given_twice(key: &Ident) -> Error {
    Error::new(key.span(), format!("`{key}` is given twice"))
}

/// The error for `Name<..> = Type`, an attempt to fix a generic associated
/// type with an option: only a partial impl block declares one.
fn gat_override(name: &Ident, trait_path: Option<&Path>) -> Error {
    let trait_name = trait_path
        .map(|path| path.to_token_stream().to_string().replace(' ', ""))
        .unwrap_or_else(|| "Trait".to_owned());
    Error::new(
        name.span(),
        format!("`{name}<..>` is a generic associated type, which only the enum can declare; write `type {name}<..> = ..;` in a `#[whichever::dispatch] impl {trait_name} for ..` block"),
    )
}

/// The subject's say on its conversions.
#[derive(Default)]
pub(crate) struct Switches {
    pub(crate) from: Option<Switch>,
    pub(crate) try_from: Option<Switch>,
    /// The types the enum converts into, `into = Target`, each through
    /// every payload's own `Into<Target>`.
    pub(crate) into: Vec<IntoTarget>,
}

impl Switches {
    /// Returns where the first switch was written, for a message about any
    /// of them.
    pub(crate) fn first_span(&self) -> Option<Span> {
        self.from
            .as_ref()
            .map(Switch::span)
            .or(self.try_from.as_ref().map(Switch::span))
            .or(self.into.first().map(|into| into.key.span()))
    }
}

/// One `into = Target`.
pub(crate) struct IntoTarget {
    pub(crate) key: Ident,
    pub(crate) target: Type,
}

/// One `from = ..` or `try_from = ..`, with where it was written.
pub(crate) struct Switch {
    key: Ident,
    value: TryFromForms,
}

impl Switch {
    /// `from` takes `true` or `false`; `try_from` takes those, `ref` and
    /// `ref_mut`.
    fn parse(key: Ident, input: ParseStream) -> Result<Self> {
        let value = TryFromForms::parse(input)?;
        if key == "from" && (value.by_ref || value.by_mut) {
            return Err(Error::new(
                key.span(),
                "`from` is `true` or `false`; the by-reference forms are `try_from`'s",
            ));
        }
        Ok(Self { key, value })
    }

    /// Returns whether the switch is on at all.
    pub(crate) fn value(&self) -> bool {
        self.value.owned
    }

    pub(crate) fn forms(&self) -> TryFromForms {
        self.value
    }

    pub(crate) fn span(&self) -> Span {
        self.key.span()
    }
}

/// Which `TryFrom` impls a payload gets: `true` is `TryFrom<Enum> for
/// Payload`; `ref` adds `TryFrom<&Enum> for &Payload`; `ref_mut` adds
/// `TryFrom<&mut Enum> for &mut Payload` to those; `false` is none. The
/// words are `derive_more`'s.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TryFromForms {
    pub(crate) owned: bool,
    pub(crate) by_ref: bool,
    pub(crate) by_mut: bool,
}

impl TryFromForms {
    pub(crate) const NONE: Self = Self {
        owned: false,
        by_ref: false,
        by_mut: false,
    };

    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitBool) {
            let on = input.parse::<LitBool>()?.value;
            return Ok(Self {
                owned: on,
                by_ref: false,
                by_mut: false,
            });
        }
        if input.peek(Token![ref]) {
            input.parse::<Token![ref]>()?;
            return Ok(Self {
                owned: true,
                by_ref: true,
                by_mut: false,
            });
        }
        let word: Ident = input
            .parse()
            .map_err(|_| input.error("expected `true`, `false`, `ref` or `ref_mut`"))?;
        if word == "ref_mut" {
            return Ok(Self {
                owned: true,
                by_ref: true,
                by_mut: true,
            });
        }
        Err(Error::new(
            word.span(),
            "expected `true`, `false`, `ref` or `ref_mut`",
        ))
    }

    /// The forms either of two switches asks for.
    pub(crate) fn or(self, other: Self) -> Self {
        Self {
            owned: self.owned || other.owned,
            by_ref: self.by_ref || other.by_ref,
            by_mut: self.by_mut || other.by_mut,
        }
    }
}

/// A `Name = ..` option, kept as the tokens written until the trait says
/// whether `Name` is an associated type or an associated const. The tokens
/// reach as far as a type does when a type is what is written, since a
/// value parser would stop at the comma in `Result<A, B>`, and as far as an
/// expression does otherwise.
pub(crate) struct Override {
    pub(crate) name: Ident,
    tokens: TokenStream2,
}

impl Override {
    fn parse(name: Ident, input: ParseStream) -> Result<Self> {
        let fork = input.fork();
        if let Ok(ty) = fork.parse::<Type>() {
            if fork.is_empty() || fork.peek(Token![,]) {
                input.advance_to(&fork);
                return Ok(Self {
                    name,
                    tokens: ty.into_token_stream(),
                });
            }
        }
        let expr: Expr = input.parse()?;
        Ok(Self {
            name,
            tokens: expr.into_token_stream(),
        })
    }

    /// An override written as an associated type in a partial impl block.
    pub(crate) fn of_type(name: Ident, ty: &Type) -> Self {
        Self {
            name,
            tokens: ty.to_token_stream(),
        }
    }

    /// Reads the override as the type of the associated type `name`.
    pub(crate) fn as_type(&self) -> Result<Type> {
        syn::parse2(self.tokens.clone()).map_err(|_| {
            Error::new_spanned(
                &self.tokens,
                format!("`{}` is an associated type; give it a type", self.name),
            )
        })
    }

    /// Reads the override as the value of the associated const `name`.
    pub(crate) fn as_value(&self) -> Result<Expr> {
        syn::parse2(self.tokens.clone()).map_err(|_| {
            Error::new_spanned(
                &self.tokens,
                format!("`{}` is an associated const; give it a value", self.name),
            )
        })
    }
}

impl ToTokens for Override {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Override {
            name,
            tokens: value,
        } = self;
        tokens.extend(quote!(#name = #value));
    }
}

pub(crate) struct KeyedExpr {
    pub(crate) key: Ident,
    pub(crate) value: Expr,
}

/// One `Variant = route` of a `via(..)` option.
struct KeyedRoute {
    key: Ident,
    route: Route,
}

impl Parse for KeyedRoute {
    fn parse(input: ParseStream) -> Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=]>()?;
        let route = input.parse()?;
        Ok(Self { key, route })
    }
}

impl Parse for KeyedExpr {
    fn parse(input: ParseStream) -> Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;
        Ok(Self { key, value })
    }
}

/// Returns the `mismatch` expression given on the trait method itself.
pub(crate) fn method_mismatch(function: &TraitItemFn) -> Result<Option<Expr>> {
    for attribute in &function.attrs {
        if !is_dispatch_attribute(attribute) {
            continue;
        }
        let pair: KeyedExpr = attribute.parse_args()?;
        if pair.key != "mismatch" {
            return Err(Error::new_spanned(
                pair.key,
                "expected `#[dispatch(mismatch = <expr>)]`",
            ));
        }
        return Ok(Some(pair.value));
    }
    Ok(None)
}

/// The `#[dispatch(..)]` options of a variant or a field.
pub(crate) struct TargetOptions {
    /// How an arm reaches the target from the payload; `deref` is `*`.
    pub(crate) route: Option<Route>,
    pub(crate) from: bool,
    pub(crate) try_from: Option<TryFromForms>,
}

impl TargetOptions {
    /// Reads every `#[dispatch(..)]` attribute of a variant, or of a field
    /// when `on_variant` is false: `deref` and `via = route` on either,
    /// `from = false` and `try_from = true` on a variant. A bare
    /// `#[dispatch]` marks the field to forward to and carries nothing.
    pub(crate) fn read(attributes: &[Attribute], on_variant: bool) -> Result<Self> {
        let mut options = Self {
            route: None,
            from: true,
            try_from: None,
        };
        for attribute in attributes
            .iter()
            .filter(|attribute| is_dispatch_attribute(attribute))
        {
            if matches!(attribute.meta, Meta::Path(_)) {
                continue;
            }
            attribute.parse_args_with(|input: ParseStream| {
                parse_options(input, |key, input| {
                    if key == "deref" || key == "via" {
                        let route = if key == "deref" {
                            Route::deref()
                        } else {
                            input.parse::<Token![=]>()?;
                            input.parse()?
                        };
                        if options.route.is_some() {
                            return Err(Error::new(
                                key.span(),
                                "`deref` is the route `*`; give one route, `deref` or `via = ..`",
                            ));
                        }
                        options.route = Some(route);
                    } else if key == "from" && on_variant {
                        input.parse::<Token![=]>()?;
                        options.from = input.parse::<LitBool>()?.value;
                    } else if key == "try_from" && on_variant {
                        input.parse::<Token![=]>()?;
                        options.try_from = Some(TryFromForms::parse(input)?);
                    } else {
                        let known = if on_variant {
                            "the options are `deref`, `via = route`, `from = false` and `try_from = true`"
                        } else {
                            "the options are `deref` and `via = route`"
                        };
                        return Err(Error::new(
                            key.span(),
                            format!("unknown option `{key}`; {known}"),
                        ));
                    }
                    Ok(())
                })
            })?;
        }
        Ok(options)
    }
}

/// Which variants an inherent method forwards to, and how.
pub(crate) struct ForwardingOptions {
    pub(crate) all: bool,
    pub(crate) variants: Vec<Ident>,
    pub(crate) call: Option<Ident>,
    pub(crate) with: Option<Path>,
    /// The route to the target for this method, over the variants' own.
    pub(crate) via: Option<Route>,
    /// `field = name`: the arm reads this field of the target instead of
    /// calling a method.
    pub(crate) field: Option<Member>,
    /// `value`: the arm is the target itself, for `with` to wrap or an
    /// unsized coercion to widen.
    pub(crate) value: bool,
}

impl Parse for ForwardingOptions {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut forwarding = ForwardingOptions {
            all: false,
            variants: vec![],
            call: None,
            with: None,
            via: None,
            field: None,
            value: false,
        };
        while !input.is_empty() {
            if input.peek(Token![*]) {
                input.parse::<Token![*]>()?;
                forwarding.all = true;
            } else {
                let ident: Ident = input.parse()?;
                if input.peek(Token![=]) {
                    input.parse::<Token![=]>()?;
                    match ident.to_string().as_str() {
                        "call" => forwarding.call = Some(input.parse()?),
                        "with" if input.peek(Token![|]) || input.peek(Token![move]) => {
                            return Err(Error::new(
                                ident.span(),
                                "`with` takes the path of a function, called on each arm's result; write a helper function instead of a closure, generic over the result when the payloads' results differ",
                            ))
                        }
                        "with" => forwarding.with = Some(input.parse()?),
                        "via" => forwarding.via = Some(input.parse()?),
                        "field" => {
                            forwarding.field = Some(if input.peek(LitInt) {
                                let index: LitInt = input.parse()?;
                                Member::Unnamed(syn::Index {
                                    index: index.base10_parse()?,
                                    span: index.span(),
                                })
                            } else {
                                Member::Named(input.parse()?)
                            });
                        }
                        other => {
                            return Err(Error::new(
                                ident.span(),
                                format!("unknown option `{other}`; the options are `call = method`, `with = path`, `via = route` and `field = name`"),
                            ))
                        }
                    }
                } else if ident == "value" {
                    forwarding.value = true;
                } else {
                    if forwarding.variants.contains(&ident) {
                        return Err(Error::new(
                            ident.span(),
                            format!("`{ident}` is named twice"),
                        ));
                    }
                    forwarding.variants.push(ident);
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        if forwarding.all && !forwarding.variants.is_empty() {
            return Err(Error::new(
                forwarding.variants[0].span(),
                "`*` already names every variant",
            ));
        }
        if !forwarding.all && forwarding.variants.is_empty() {
            return Err(input.error("name the variants to forward to, or `*` for all of them"));
        }
        let reads = match (&forwarding.field, forwarding.value) {
            (Some(_), true) => {
                return Err(
                    input.error("`field` reads a field and `value` is the target itself; give one")
                )
            }
            (Some(field), false) => Some(format!("`field = {}`", field.to_token_stream())),
            (None, true) => Some("`value`".to_owned()),
            (None, false) => None,
        };
        if let (Some(reads), Some(call)) = (reads, &forwarding.call) {
            return Err(Error::new(
                call.span(),
                format!("{reads} calls no method, so `call` has nothing to name"),
            ));
        }
        Ok(forwarding)
    }
}

/// The options of `#[dispatchable(..)]`.
pub(crate) struct DispatchableOptions {
    /// The real path of a trait restated on behalf of another crate.
    pub(crate) remote: Option<Path>,
    /// A name for the carrier macro, in place of the trait's text and
    /// position.
    pub(crate) name: Option<Ident>,
}

impl Parse for DispatchableOptions {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut options = Self {
            remote: None,
            name: None,
        };
        parse_options(input, |key, input| {
            if !input.peek(Token![=]) || !(key == "remote" || key == "name") {
                return Err(Error::new(
                    key.span(),
                    format!("unknown option `{key}`; the options are `remote = path` and `name = ident`"),
                ));
            }
            input.parse::<Token![=]>()?;
            if key == "remote" {
                options.remote = Some(input.parse()?);
            } else {
                options.name = Some(input.parse()?);
            }
            Ok(())
        })?;
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    fn attribute(tokens: &str) -> DispatchAttribute {
        syn::parse_str(tokens).expect("a valid attribute")
    }

    #[test]
    fn an_override_reaches_as_far_as_a_type_or_a_value_does() {
        let attribute = attribute(
            "T, N = -1, S = \"s\", R = (A, B), E = Result<A, B>, F = Foo::<T>::bar(), L = [u8; 4]",
        );
        let fixed = |name: &str| {
            let name: Ident = syn::parse_str(name).unwrap();
            attribute.spec.override_for(&name).expect("an override")
        };
        assert_eq!(attribute.spec.overrides.len(), 6);
        assert!(fixed("N").as_value().is_ok());
        assert!(fixed("N").as_type().is_err());
        assert!(fixed("S").as_value().is_ok());
        assert!(
            fixed("R").as_value().is_ok(),
            "a tuple of paths reads as a value"
        );
        assert!(fixed("R").as_type().is_ok());
        assert!(
            fixed("E").as_type().is_ok(),
            "the comma inside `Result<A, B>` is the type's"
        );
        assert!(fixed("F").as_value().is_ok());
        assert!(fixed("L").as_type().is_ok());
    }

    #[test]
    fn the_path_is_optional_and_the_switches_are_the_subjects() {
        let switches_only = attribute("from = false, try_from = true");
        assert!(switches_only.spec.path.is_none());
        assert_eq!(
            switches_only.switches.from.as_ref().map(Switch::value),
            Some(false)
        );
        assert_eq!(
            switches_only.switches.try_from.as_ref().map(Switch::value),
            Some(true)
        );
        let by_ref = attribute("try_from = ref, into = Target<A, B>, into = Other");
        let forms = by_ref.switches.try_from.as_ref().unwrap().forms();
        assert!(forms.owned && forms.by_ref && !forms.by_mut);
        assert_eq!(by_ref.switches.into.len(), 2);
        assert!(syn::parse_str::<DispatchAttribute>("from = ref").is_err());
        assert!(syn::parse_str::<DispatchAttribute>("try_from = sometimes").is_err());
        let with_path = attribute("a::Trait<X>, mismatch(same = false, other = 1)");
        assert!(with_path.spec.path.is_some());
        assert_eq!(with_path.spec.mismatch.len(), 2);
        assert!(
            syn::parse_str::<TraitSpec>("Trait, from = false").is_err(),
            "a trait's options alone take no switch"
        );
        assert!(syn::parse_str::<DispatchAttribute>("Trait, Error = A, Error = B").is_err());
        assert!(syn::parse_str::<DispatchAttribute>("Trait, from = false, from = true").is_err());
        assert!(syn::parse_str::<DispatchAttribute>("Trait, bogus").is_err());
        let gat = |tokens: &str| {
            syn::parse_str::<DispatchAttribute>(tokens)
                .err()
                .expect("an error")
                .to_string()
        };
        assert!(gat("Lend, Item<'a> = &'a str").contains("`Item<..>` is a generic associated type"));
        assert!(gat("Item<'a> = &'a str").contains("impl Trait for .."));
    }

    #[test]
    fn forwarding_names_variants_or_everything() {
        let options: ForwardingOptions =
            syn::parse_str("Circle, Square, with = Some, call = r").unwrap();
        assert_eq!(options.variants.len(), 2);
        assert!(options.with.is_some() && options.call.is_some() && !options.all);
        assert!(syn::parse_str::<ForwardingOptions>("*").unwrap().all);
        assert!(syn::parse_str::<ForwardingOptions>("*, Circle").is_err());
        assert!(syn::parse_str::<ForwardingOptions>("Circle, Circle").is_err());
        assert!(syn::parse_str::<ForwardingOptions>("with = Some").is_err());
        let closure = syn::parse_str::<ForwardingOptions>("*, with = |s| Some(s)")
            .err()
            .expect("an error")
            .to_string();
        assert!(
            closure.starts_with("`with` takes the path of a function"),
            "{closure}"
        );
    }

    #[test]
    fn the_closest_name_is_within_one_edit_or_two_for_long_names() {
        let names = ["Unit", "Error", "from", "try_from"];
        assert_eq!(closest("Unti", names), Some("Unit"));
        assert_eq!(closest("form", names), Some("from"));
        assert_eq!(closest("Eror", names), Some("Error"));
        assert_eq!(closest("try_form", names), Some("try_from"));
        assert_eq!(closest("tri_form", names), Some("try_from"));
        assert_eq!(closest("Unix", names), Some("Unit"));
        assert_eq!(
            closest("Un", names),
            None,
            "two edits on a short name say nothing"
        );
        assert_eq!(closest("target", names), None);
        assert_eq!(edit_distance("eqq", "eq"), 1);
        assert_eq!(edit_distance("same", "sane"), 1);
        assert_eq!(listed(&[]), "");
        assert_eq!(listed(&["a".into()]), "`a`");
        assert_eq!(
            listed(&["a".into(), "b".into(), "c".into()]),
            "`a`, `b` and `c`"
        );
    }

    #[test]
    fn target_options_differ_between_a_variant_and_a_field() {
        let attributes: Vec<Attribute> = vec![
            parse_quote!(#[dispatch(deref, from = false)]),
            parse_quote!(#[dispatch(try_from = true)]),
        ];
        let variant = TargetOptions::read(&attributes, true).unwrap();
        assert!(variant.route.as_ref().is_some_and(Route::is_deref));
        assert!(
            !variant.from
                && variant
                    .try_from
                    .is_some_and(|forms| forms.owned && !forms.by_ref)
        );
        let by_ref: Vec<Attribute> = vec![parse_quote!(#[dispatch(try_from = ref_mut)])];
        let forms = TargetOptions::read(&by_ref, true)
            .unwrap()
            .try_from
            .unwrap();
        assert!(forms.owned && forms.by_ref && forms.by_mut);
        assert!(
            TargetOptions::read(&attributes, false).is_err(),
            "a field takes `deref` and `via` only"
        );
        let bare: Vec<Attribute> = vec![parse_quote!(#[dispatch])];
        assert!(TargetOptions::read(&bare, false).unwrap().from);
        let routed: Vec<Attribute> = vec![parse_quote!(#[dispatch(via = .base.get_ref())])];
        let route = TargetOptions::read(&routed, false).unwrap().route.unwrap();
        assert_eq!(route.spelled(), ".base.get_ref()");
        let both: Vec<Attribute> = vec![parse_quote!(#[dispatch(deref, via = .0)])];
        assert!(TargetOptions::read(&both, true).is_err());
        let spec: TraitSpec = syn::parse_str("Trait, via(VarChar = .1, Decimal = .2)").unwrap();
        assert_eq!(spec.routes.len(), 2);
        assert_eq!(
            spec.to_token_stream().to_string().replace(' ', ""),
            "Trait,via(VarChar=.1,Decimal=.2)"
        );
        let forwarding: ForwardingOptions = syn::parse_str("*, field = line_no").unwrap();
        assert!(forwarding.field.is_some() && !forwarding.value);
        let forwarding: ForwardingOptions = syn::parse_str("A, B, value, with = Some").unwrap();
        assert!(forwarding.value && forwarding.with.is_some());
        assert!(syn::parse_str::<ForwardingOptions>("*, field = a, value").is_err());
        assert!(syn::parse_str::<ForwardingOptions>("*, value, call = m").is_err());
        let forwarding: ForwardingOptions =
            syn::parse_str("*, via = .content(), call = refresh").unwrap();
        assert_eq!(forwarding.via.unwrap().spelled(), ".content()");
    }
}
