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
    parenthesized, Attribute, Error, Expr, Ident, LitBool, Meta, Path, Result, Token, TraitItemFn,
    Type,
};

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
        if let Some(switch) = switches.first() {
            return Err(Error::new(
                switch.span(),
                "`from` and `try_from` are options of the enum; put them on its `#[dispatch]` attribute",
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
        };
        let mut switches = Switches::default();
        if spec.path.is_some() && !input.is_empty() {
            input.parse::<Token![,]>()?;
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
            if !input.peek(Token![=]) {
                return Err(Error::new(
                    key.span(),
                    format!("unknown option `{key}`; the options are `Name = Type` for an associated type, `Name = value` for an associated const, `mismatch(method = expr)`, `from = false` and `try_from = true`"),
                ));
            }
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "from" if switches.from.is_some() => return Err(given_twice(&key)),
                "try_from" if switches.try_from.is_some() => return Err(given_twice(&key)),
                "from" => switches.from = Some(Switch::parse(key, input)?),
                "try_from" => switches.try_from = Some(Switch::parse(key, input)?),
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

/// The subject's say on its conversions.
#[derive(Default)]
pub(crate) struct Switches {
    pub(crate) from: Option<Switch>,
    pub(crate) try_from: Option<Switch>,
}

impl Switches {
    /// Returns the first switch written, for a message about any of them.
    pub(crate) fn first(&self) -> Option<&Switch> {
        self.from.as_ref().or(self.try_from.as_ref())
    }
}

/// One `from = ..` or `try_from = ..`, with where it was written.
pub(crate) struct Switch {
    key: Ident,
    value: LitBool,
}

impl Switch {
    fn parse(key: Ident, input: ParseStream) -> Result<Self> {
        Ok(Self {
            key,
            value: input.parse()?,
        })
    }

    pub(crate) fn value(&self) -> bool {
        self.value.value
    }

    pub(crate) fn span(&self) -> Span {
        self.key.span()
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
    pub(crate) deref: bool,
    pub(crate) from: bool,
    pub(crate) try_from: Option<bool>,
}

impl TargetOptions {
    /// Reads every `#[dispatch(..)]` attribute of a variant, or of a field
    /// when `on_variant` is false: `deref` on either, `from = false` and
    /// `try_from = true` on a variant. A bare `#[dispatch]` marks the field
    /// to forward to and carries nothing.
    pub(crate) fn read(attributes: &[Attribute], on_variant: bool) -> Result<Self> {
        let mut options = Self {
            deref: false,
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
                    if key == "deref" {
                        options.deref = true;
                    } else if key == "from" && on_variant {
                        input.parse::<Token![=]>()?;
                        options.from = input.parse::<LitBool>()?.value;
                    } else if key == "try_from" && on_variant {
                        input.parse::<Token![=]>()?;
                        options.try_from = Some(input.parse::<LitBool>()?.value);
                    } else {
                        let known = if on_variant {
                            "the options are `deref`, `from = false` and `try_from = true`"
                        } else {
                            "the only one is `deref`"
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
}

impl Parse for ForwardingOptions {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut forwarding = ForwardingOptions {
            all: false,
            variants: vec![],
            call: None,
            with: None,
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
                        "with" => forwarding.with = Some(input.parse()?),
                        other => {
                            return Err(Error::new(
                                ident.span(),
                                format!("unknown option `{other}`; the options are `call = method` and `with = path`"),
                            ))
                        }
                    }
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
    }

    #[test]
    fn target_options_differ_between_a_variant_and_a_field() {
        let attributes: Vec<Attribute> = vec![
            parse_quote!(#[dispatch(deref, from = false)]),
            parse_quote!(#[dispatch(try_from = true)]),
        ];
        let variant = TargetOptions::read(&attributes, true).unwrap();
        assert!(variant.deref && !variant.from && variant.try_from == Some(true));
        assert!(
            TargetOptions::read(&attributes, false).is_err(),
            "a field takes `deref` only"
        );
        let bare: Vec<Attribute> = vec![parse_quote!(#[dispatch])];
        assert!(TargetOptions::read(&bare, false).unwrap().from);
    }
}
