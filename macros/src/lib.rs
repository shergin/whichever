//! Procedural macros of `whichever`.
//!
//! `#[dispatchable]` on a trait re-emits the trait and exports a hidden
//! `macro_rules!` that carries the trait's tokens; the macro is re-exported
//! under the trait's own name, so importing the trait imports the macro.
//! `#[dispatch(Trait)]` on an enum invokes that macro, which forwards the
//! trait tokens, the enum tokens and the options into the hidden
//! `implement!` macro that writes `impl Trait for Enum` with one `match` per
//! method. No state is shared between expansions, so the trait and the enum
//! may live in different crates.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use proc_macro::TokenStream;
use proc_macro2::{Group, Punct, Spacing, Span, TokenStream as TokenStream2, TokenTree};
use proc_macro_crate::FoundCrate;
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::visit_mut::{self, VisitMut};
use syn::{
    braced, parenthesized, parse_macro_input, Attribute, Error, Expr, Fields, FnArg,
    GenericArgument, GenericParam, Ident, ItemEnum, ItemTrait, Lifetime, LitBool, Meta, Pat,
    PatIdent, Path, PathArguments, PathSegment, QSelf, Result, ReturnType, Signature, Token,
    TraitItem, TraitItemFn, Type, TypeParamBound, TypePath, Variant,
};

/// The name the facade crate is published under; a renamed dependency is
/// resolved through `proc-macro-crate`.
const FACADE: &str = "whichever";

/// Returns whether `attribute` is this crate's `dispatch` attribute, written
/// bare or as `whichever::dispatch`.
fn is_dispatch_attribute(attribute: &Attribute) -> bool {
    attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "dispatch")
}

// ---------------------------------------------------------------------------
// Trait side
// ---------------------------------------------------------------------------

/// Makes a trait available to `#[dispatch]`, from this crate or any other.
///
/// The trait is emitted unchanged, except that `#[dispatch(..)]` attributes
/// on its methods are consumed. The attribute takes no arguments. The only
/// method option is `mismatch`, which gives the result of a method taking
/// `Self` arguments when the arguments hold a different variant than `self`:
///
/// ```rust,ignore
/// #[whichever::dispatchable]
/// pub trait Swap {
///     fn swap(&self, amount: u64) -> u64;
///     #[dispatch(mismatch = false)]
///     fn pricing_eq(&self, other: &Self) -> bool;
/// }
/// ```
///
/// The trait's signature types are later written into the dispatching
/// enum's module, so they must be nameable there; `crate::` paths are
/// rewritten to this crate, `super::` and `self::` paths are not and must
/// not be used.
#[proc_macro_attribute]
pub fn dispatchable(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return Error::new(Span::call_site(), "`#[dispatchable]` takes no arguments")
            .to_compile_error()
            .into();
    }
    let item_trait = parse_macro_input!(item as ItemTrait);
    let site = proc_macro::Span::call_site();
    expand_dispatchable(item_trait, site.line(), site.column())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

fn expand_dispatchable(item_trait: ItemTrait, line: usize, column: usize) -> Result<TokenStream2> {
    let captured = item_trait.clone();
    let mut emitted = item_trait;
    for item in &mut emitted.items {
        if let TraitItem::Fn(function) = item {
            function
                .attrs
                .retain(|attribute| !is_dispatch_attribute(attribute));
        }
    }
    let trait_ident = emitted.ident.clone();
    let vis = emitted.vis.clone();
    // The exported macro lives at the crate root, so its name carries the
    // trait's tokens and position to keep same-named traits apart.
    let mut hasher = DefaultHasher::new();
    captured.to_token_stream().to_string().hash(&mut hasher);
    let macro_name = format_ident!(
        "__whichever_{}_{:016x}_{}_{}",
        trait_ident,
        hasher.finish(),
        line,
        column
    );
    // The captured tokens expand inside the enum's crate, where `crate::`
    // would name the wrong crate; `$crate` resolves to this crate instead.
    let captured = rewrite_crate_paths(captured.to_token_stream());
    Ok(quote! {
        #emitted

        #[doc(hidden)]
        #[macro_export]
        macro_rules! #macro_name {
            (@v1 $facade:ident $($rest:tt)*) => {
                ::$facade::__private::implement! { @v1 trait { #captured } $($rest)* }
            };
        }

        #[doc(hidden)]
        #[allow(unused_imports)]
        #vis use #macro_name as #trait_ident;
    })
}

/// Replaces every `crate::` path prefix in `tokens` with `$crate::`.
fn rewrite_crate_paths(tokens: TokenStream2) -> TokenStream2 {
    let mut output = TokenStream2::new();
    let mut iter = tokens.into_iter().peekable();
    while let Some(token) = iter.next() {
        match token {
            TokenTree::Group(group) => {
                let mut rewritten =
                    Group::new(group.delimiter(), rewrite_crate_paths(group.stream()));
                rewritten.set_span(group.span());
                output.extend([TokenTree::Group(rewritten)]);
            }
            TokenTree::Ident(ident)
                if ident == "crate"
                    && matches!(iter.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == ':') =>
            {
                output.extend([
                    TokenTree::Punct(Punct::new('$', Spacing::Alone)),
                    TokenTree::Ident(ident),
                ]);
            }
            other => output.extend([other]),
        }
    }
    output
}

// ---------------------------------------------------------------------------
// Enum side
// ---------------------------------------------------------------------------

/// One `#[dispatch(Trait<Args>, key = value, ...)]` attribute.
struct TraitSpec {
    path: Path,
    assoc_overrides: Vec<(Ident, Type)>,
    from: bool,
    mismatch: Vec<(Ident, Expr)>,
}

impl TraitSpec {
    fn assoc_override(&self, name: &Ident) -> Option<&Type> {
        self.assoc_overrides
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, ty)| ty)
    }

    fn mismatch_for(&self, method: &Ident) -> Option<&Expr> {
        self.mismatch
            .iter()
            .find(|(key, _)| key == method)
            .map(|(_, expr)| expr)
    }
}

struct KeyedExpr {
    key: Ident,
    value: Expr,
}

impl Parse for KeyedExpr {
    fn parse(input: ParseStream) -> Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;
        Ok(Self { key, value })
    }
}

impl Parse for TraitSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let path: Path = input.parse()?;
        let mut spec = TraitSpec {
            path,
            assoc_overrides: vec![],
            from: true,
            mismatch: vec![],
        };
        while input.parse::<Option<Token![,]>>()?.is_some() {
            if input.is_empty() {
                break;
            }
            let key: Ident = input.parse()?;
            if key == "mismatch" {
                let content;
                parenthesized!(content in input);
                let pairs = Punctuated::<KeyedExpr, Token![,]>::parse_terminated(&content)?;
                spec.mismatch
                    .extend(pairs.into_iter().map(|pair| (pair.key, pair.value)));
                continue;
            }
            if !input.peek(Token![=]) {
                return Err(Error::new(
                    key.span(),
                    format!("unknown option `{key}`; the options are `Name = Type` for an associated type, `mismatch(method = expr)` and `from = false`"),
                ));
            }
            input.parse::<Token![=]>()?;
            if key == "from" {
                spec.from = input.parse::<LitBool>()?.value;
                continue;
            }
            let ty: Type = input.parse()?;
            spec.assoc_overrides.push((key, ty));
        }
        Ok(spec)
    }
}

/// Implements a `#[dispatchable]` trait for an enum by forwarding every
/// method to whichever variant the value holds.
///
/// Each attribute names one trait, with its generic arguments when it has
/// any, followed by options; several attributes dispatch several traits.
/// Every variant must hold exactly one unnamed field whose type implements
/// the trait. The enum also gets a `From<Payload>` impl for every payload
/// type that appears in exactly one variant.
///
/// ```rust,ignore
/// #[whichever::dispatch(Swap)]
/// #[whichever::dispatch(Settler<Solaris>, Error = SettlerError)]
/// pub enum SwapKind {
///     Uniswap(UniswapSwap),
///     Binance(BinanceSwap),
/// }
/// ```
///
/// Options, after the trait path:
/// - `Name = Type`: the associated type `Name` of the enum's impl, instead
///   of the first variant's. Methods returning `Self::Name` are converted
///   with `Into`, methods returning `Result<_, Self::Name>` with
///   `map_err(Into::into)`.
/// - `mismatch(method = expr, ..)`: the result of `method` when its `Self`
///   arguments hold a different variant than `self`.
/// - `from = false`: no `From<Payload>` impls.
#[proc_macro_attribute]
pub fn dispatch(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_enum = parse_macro_input!(item as ItemEnum);
    expand_dispatch(attr.into(), item_enum)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

fn expand_dispatch(first: TokenStream2, mut item_enum: ItemEnum) -> Result<TokenStream2> {
    let mut specs = vec![first];
    let mut kept = vec![];
    for attribute in std::mem::take(&mut item_enum.attrs) {
        if !is_dispatch_attribute(&attribute) {
            kept.push(attribute);
            continue;
        }
        let Meta::List(list) = &attribute.meta else {
            return Err(Error::new_spanned(
                attribute,
                "expected `#[dispatch(Trait, ...)]`",
            ));
        };
        specs.push(list.tokens.clone());
    }
    item_enum.attrs = kept;
    for variant in &item_enum.variants {
        let option = variant
            .attrs
            .iter()
            .find(|attribute| is_dispatch_attribute(attribute));
        if let Some(attribute) = option {
            return Err(Error::new_spanned(
                attribute,
                "options on variants are not supported yet",
            ));
        }
    }
    let captured = item_enum.clone();
    let emitted = item_enum;
    let facade = facade_ident()?;
    let parsed_specs = specs
        .iter()
        .map(|tokens| syn::parse2::<TraitSpec>(tokens.clone()))
        .collect::<Result<Vec<_>>>()?;
    let from_impls = if parsed_specs.iter().all(|spec| spec.from) {
        from_impls(&emitted)?
    } else {
        TokenStream2::new()
    };
    let mut invocations = TokenStream2::new();
    for (spec, tokens) in parsed_specs.iter().zip(&specs) {
        let mut macro_path = spec.path.clone();
        if let Some(last) = macro_path.segments.last_mut() {
            last.arguments = PathArguments::None;
        }
        invocations.extend(quote! {
            #macro_path! { @v1 #facade enum { #captured } spec { #tokens } }
        });
    }
    Ok(quote! {
        #emitted
        #from_impls
        #invocations
    })
}

fn facade_ident() -> Result<Ident> {
    match proc_macro_crate::crate_name(FACADE) {
        Ok(FoundCrate::Itself) => Ok(format_ident!("{FACADE}")),
        Ok(FoundCrate::Name(name)) => Ok(Ident::new(&name, Span::call_site())),
        Err(error) => Err(Error::new(
            Span::call_site(),
            format!("`{FACADE}` must be a dependency of this crate: {error}"),
        )),
    }
}

struct VariantInfo {
    ident: Ident,
    payload: Type,
    cfgs: Vec<Attribute>,
}

fn variant_info(variant: &Variant) -> Result<VariantInfo> {
    let Fields::Unnamed(fields) = &variant.fields else {
        return Err(Error::new_spanned(
            variant,
            "a dispatch variant must be `Name(Payload)`",
        ));
    };
    if fields.unnamed.len() != 1 {
        return Err(Error::new_spanned(
            variant,
            "a dispatch variant must hold exactly one payload",
        ));
    }
    Ok(VariantInfo {
        ident: variant.ident.clone(),
        payload: fields.unnamed[0].ty.clone(),
        cfgs: cfg_attributes(&variant.attrs),
    })
}

fn cfg_attributes(attributes: &[Attribute]) -> Vec<Attribute> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .cloned()
        .collect()
}

fn from_impls(item_enum: &ItemEnum) -> Result<TokenStream2> {
    let (impl_generics, ty_generics, where_clause) = item_enum.generics.split_for_impl();
    let enum_ident = &item_enum.ident;
    // A payload type held by two variants gets no `From`: the impls would
    // conflict, and neither variant is the obvious target.
    let mut payload_counts = std::collections::HashMap::<String, usize>::new();
    for variant in &item_enum.variants {
        let payload = variant_info(variant)?.payload.to_token_stream().to_string();
        *payload_counts.entry(payload).or_default() += 1;
    }
    let mut output = TokenStream2::new();
    for variant in &item_enum.variants {
        let VariantInfo {
            ident,
            payload,
            cfgs,
        } = variant_info(variant)?;
        if payload_counts[&payload.to_token_stream().to_string()] > 1 {
            continue;
        }
        output.extend(quote! {
            #(#cfgs)*
            #[automatically_derived]
            impl #impl_generics ::core::convert::From<#payload> for #enum_ident #ty_generics #where_clause {
                #[inline]
                fn from(value: #payload) -> Self {
                    Self::#ident(value)
                }
            }
        });
    }
    Ok(output)
}

// ---------------------------------------------------------------------------
// The hidden implementation macro
// ---------------------------------------------------------------------------

struct ImplementInput {
    item_trait: ItemTrait,
    item_enum: ItemEnum,
    spec: TraitSpec,
}

impl Parse for ImplementInput {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![@]>()?;
        let version: Ident = input.parse()?;
        if version != "v1" {
            return Err(Error::new_spanned(
                version,
                "unsupported `whichever` protocol version",
            ));
        }
        input.parse::<Token![trait]>()?;
        let trait_tokens;
        braced!(trait_tokens in input);
        let item_trait: ItemTrait = trait_tokens.parse()?;
        input.parse::<Token![enum]>()?;
        let enum_tokens;
        braced!(enum_tokens in input);
        let item_enum: ItemEnum = enum_tokens.parse()?;
        let tag: Ident = input.parse()?;
        if tag != "spec" {
            return Err(Error::new_spanned(tag, "expected `spec`"));
        }
        let spec_tokens;
        braced!(spec_tokens in input);
        let spec: TraitSpec = spec_tokens.parse()?;
        Ok(Self {
            item_trait,
            item_enum,
            spec,
        })
    }
}

/// Writes the trait impl; called by the macro that `#[dispatchable]` exports.
#[doc(hidden)]
#[proc_macro]
pub fn implement(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ImplementInput);
    expand_implement(input)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Replaces the trait's generic parameters with the arguments the enum names.
struct Substitution {
    types: Vec<(Ident, Type, Option<Path>)>,
    lifetimes: Vec<(Lifetime, Lifetime)>,
    consts: Vec<(Ident, Expr)>,
}

impl Substitution {
    fn new(item_trait: &ItemTrait, trait_path: &Path) -> Result<Self> {
        let params: Vec<&GenericParam> = item_trait.generics.params.iter().collect();
        let last = trait_path
            .segments
            .last()
            .expect("a path has at least one segment");
        let args: Vec<&GenericArgument> = match &last.arguments {
            PathArguments::AngleBracketed(arguments) => arguments
                .args
                .iter()
                .filter(|argument| {
                    !matches!(
                        argument,
                        GenericArgument::AssocType(_)
                            | GenericArgument::AssocConst(_)
                            | GenericArgument::Constraint(_)
                    )
                })
                .collect(),
            _ => vec![],
        };
        if args.len() != params.len() {
            return Err(Error::new_spanned(
                trait_path,
                format!(
                    "`{}` takes {} generic argument(s); name them, e.g. `Trait<Arg>`",
                    item_trait.ident,
                    params.len()
                ),
            ));
        }
        let mut substitution = Self {
            types: vec![],
            lifetimes: vec![],
            consts: vec![],
        };
        for (param, argument) in params.into_iter().zip(args) {
            match (param, argument) {
                (GenericParam::Type(param), GenericArgument::Type(ty)) => {
                    let single_bound = match param.bounds.iter().collect::<Vec<_>>().as_slice() {
                        [TypeParamBound::Trait(bound)] => Some(bound.path.clone()),
                        _ => None,
                    };
                    substitution
                        .types
                        .push((param.ident.clone(), ty.clone(), single_bound));
                }
                (GenericParam::Lifetime(param), GenericArgument::Lifetime(lifetime)) => {
                    substitution
                        .lifetimes
                        .push((param.lifetime.clone(), lifetime.clone()));
                }
                (GenericParam::Const(param), GenericArgument::Const(expr)) => {
                    substitution
                        .consts
                        .push((param.ident.clone(), expr.clone()));
                }
                // A bare identifier given for a const parameter parses as a type.
                (GenericParam::Const(param), GenericArgument::Type(Type::Path(path)))
                    if path.qself.is_none() =>
                {
                    let expr = Expr::Path(syn::ExprPath {
                        attrs: vec![],
                        qself: None,
                        path: path.path.clone(),
                    });
                    substitution.consts.push((param.ident.clone(), expr));
                }
                _ => {
                    return Err(Error::new_spanned(
                        argument,
                        "generic argument does not match the trait's parameter",
                    ))
                }
            }
        }
        Ok(substitution)
    }

    fn const_for(&self, ident: &Ident) -> Option<&Expr> {
        self.consts
            .iter()
            .find(|(name, _)| name == ident)
            .map(|(_, expr)| expr)
    }
}

/// Returns the identifier of a bare single-segment path type.
fn bare_ident(ty: &Type) -> Option<&Ident> {
    match ty {
        Type::Path(TypePath { qself: None, path }) if path.segments.len() == 1 => {
            Some(&path.segments[0].ident)
        }
        _ => None,
    }
}

impl VisitMut for Substitution {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        if let Type::Path(type_path) = ty {
            if type_path.qself.is_none() {
                let first = &type_path.path.segments[0];
                let replacement = self
                    .types
                    .iter()
                    .find(|(ident, _, _)| *ident == first.ident);
                if let Some((_, replacement, bound)) = replacement {
                    if type_path.path.segments.len() == 1 {
                        *ty = replacement.clone();
                        return;
                    }
                    // `W::Assoc` becomes `<Arg as Bound>::Assoc` when the
                    // parameter has exactly one bound to qualify it with.
                    if let Some(bound) = bound {
                        let rest: Vec<PathSegment> =
                            type_path.path.segments.iter().skip(1).cloned().collect();
                        let mut path = bound.clone();
                        let position = path.segments.len();
                        path.segments.extend(rest);
                        *ty = Type::Path(TypePath {
                            qself: Some(QSelf {
                                lt_token: Default::default(),
                                ty: Box::new(replacement.clone()),
                                position,
                                as_token: Some(Default::default()),
                                gt_token: Default::default(),
                            }),
                            path,
                        });
                        return;
                    }
                }
            }
        }
        visit_mut::visit_type_mut(self, ty);
    }

    fn visit_lifetime_mut(&mut self, lifetime: &mut Lifetime) {
        if let Some((_, replacement)) = self.lifetimes.iter().find(|(from, _)| from == lifetime) {
            *lifetime = replacement.clone();
        }
    }

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Path(path) = expr {
            if path.qself.is_none() && path.path.segments.len() == 1 {
                if let Some(replacement) = self.const_for(&path.path.segments[0].ident) {
                    *expr = replacement.clone();
                    return;
                }
            }
        }
        visit_mut::visit_expr_mut(self, expr);
    }

    /// A const parameter in argument position parses as a type when it is a
    /// bare identifier; it becomes the const argument the enum named.
    fn visit_generic_argument_mut(&mut self, argument: &mut GenericArgument) {
        if let GenericArgument::Type(ty) = argument {
            if let Some(replacement) = bare_ident(ty).and_then(|ident| self.const_for(ident)) {
                *argument = GenericArgument::Const(replacement.clone());
                return;
            }
        }
        visit_mut::visit_generic_argument_mut(self, argument);
    }
}

fn expand_implement(input: ImplementInput) -> Result<TokenStream2> {
    let ImplementInput {
        item_trait,
        item_enum,
        spec,
    } = input;
    let enum_ident = &item_enum.ident;
    let (impl_generics, ty_generics, where_clause) = item_enum.generics.split_for_impl();
    let trait_path = &spec.path;
    let variants = item_enum
        .variants
        .iter()
        .map(variant_info)
        .collect::<Result<Vec<_>>>()?;
    let Some(first) = variants.first() else {
        return Err(Error::new_spanned(
            &item_enum,
            "a dispatch enum needs at least one variant",
        ));
    };
    let first_payload = &first.payload;
    let mut substitution = Substitution::new(&item_trait, trait_path)?;
    let mut items = TokenStream2::new();
    let mut agreed_types: Vec<&Ident> = vec![];
    for item in &item_trait.items {
        match item {
            TraitItem::Type(assoc) => {
                if !assoc.generics.params.is_empty() {
                    return Err(Error::new_spanned(
                        assoc,
                        "generic associated types are not supported",
                    ));
                }
                let name = &assoc.ident;
                let ty = match spec.assoc_override(name) {
                    Some(ty) => quote!(#ty),
                    None => {
                        agreed_types.push(name);
                        quote!(<#first_payload as #trait_path>::#name)
                    }
                };
                items.extend(quote! { type #name = #ty; });
            }
            TraitItem::Const(constant) => {
                let name = &constant.ident;
                let mut ty = constant.ty.clone();
                substitution.visit_type_mut(&mut ty);
                items.extend(quote! {
                    const #name: #ty = <#first_payload as #trait_path>::#name;
                });
            }
            TraitItem::Fn(function) => {
                items.extend(expand_method(
                    function,
                    &variants,
                    enum_ident,
                    trait_path,
                    &spec,
                    &mut substitution,
                )?);
            }
            TraitItem::Macro(item) => {
                return Err(Error::new_spanned(
                    item,
                    "macro invocations inside the trait are not supported",
                ));
            }
            _ => {}
        }
    }
    let agreement_check = associated_type_check(
        &agreed_types,
        &variants,
        trait_path,
        first_payload,
        &item_enum.generics,
    );
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #trait_path for #enum_ident #ty_generics #where_clause {
            #items
        }
        #agreement_check
    })
}

/// Checks at compile time that every variant's associated types equal the
/// first variant's, so a disagreement is reported on the variant instead of
/// inside a generated method body.
fn associated_type_check(
    names: &[&Ident],
    variants: &[VariantInfo],
    trait_path: &Path,
    first_payload: &Type,
    generics: &syn::Generics,
) -> TokenStream2 {
    if names.is_empty() || variants.len() < 2 {
        return TokenStream2::new();
    }
    let bindings: Vec<GenericArgument> = names
        .iter()
        .map(|name| syn::parse_quote!(#name = <#first_payload as #trait_path>::#name))
        .collect();
    let mut bound = trait_path.clone();
    let last = bound
        .segments
        .last_mut()
        .expect("a path has at least one segment");
    match &mut last.arguments {
        PathArguments::AngleBracketed(arguments) => arguments.args.extend(bindings),
        arguments => {
            *arguments = PathArguments::AngleBracketed(syn::parse_quote!(<#(#bindings),*>))
        }
    }
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let checks = variants.iter().skip(1).map(|variant| {
        let payload = &variant.payload;
        let cfgs = &variant.cfgs;
        quote! { #(#cfgs)* __same_as_first_variant::<#payload>(); }
    });
    quote! {
        const _: () = {
            #[allow(dead_code, non_snake_case)]
            fn __whichever_associated_types_agree #impl_generics () #where_clause {
                fn __same_as_first_variant<__Variant: ?Sized + #bound>() {}
                #(#checks)*
            }
        };
    }
}

enum ReturnShape {
    Plain,
    RewrapSelf,
    MapSelf,
    IntoOverridden,
    MapErrOverridden,
}

fn single_segment(ty: &Type) -> Option<&PathSegment> {
    match ty {
        Type::Path(TypePath { qself: None, path }) if path.segments.len() == 1 => {
            Some(&path.segments[0])
        }
        _ => None,
    }
}

fn is_self(ty: &Type) -> bool {
    single_segment(ty).is_some_and(|segment| segment.ident == "Self" && segment.arguments.is_none())
}

/// Returns the name of the associated type in a `Self::Name` type.
fn self_assoc(ty: &Type) -> Option<&Ident> {
    match ty {
        Type::Path(TypePath { qself: None, path })
            if path.segments.len() == 2 && path.segments[0].ident == "Self" =>
        {
            Some(&path.segments[1].ident)
        }
        _ => None,
    }
}

fn generic_type_args(segment: &PathSegment) -> Vec<&Type> {
    match &segment.arguments {
        PathArguments::AngleBracketed(arguments) => arguments
            .args
            .iter()
            .filter_map(|argument| match argument {
                GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

fn classify_return(output: &ReturnType, spec: &TraitSpec) -> Result<ReturnShape> {
    let ReturnType::Type(_, ty) = output else {
        return Ok(ReturnShape::Plain);
    };
    if let Type::ImplTrait(_) = **ty {
        return Err(Error::new_spanned(
            ty,
            "a return-position `impl Trait` cannot be dispatched by a `match` because every variant returns a different opaque type; return a concrete type or a boxed trait object",
        ));
    }
    if is_self(ty) {
        return Ok(ReturnShape::RewrapSelf);
    }
    if let Some(name) = self_assoc(ty) {
        return Ok(if spec.assoc_override(name).is_some() {
            ReturnShape::IntoOverridden
        } else {
            ReturnShape::Plain
        });
    }
    if let Some(segment) = single_segment(ty) {
        let args = generic_type_args(segment);
        let wraps_self = args.first().is_some_and(|first| is_self(first));
        if (segment.ident == "Option" || segment.ident == "Result") && wraps_self {
            return Ok(ReturnShape::MapSelf);
        }
        if segment.ident == "Result" && args.len() == 2 {
            if let Some(name) = self_assoc(args[1]) {
                if spec.assoc_override(name).is_some() {
                    return Ok(ReturnShape::MapErrOverridden);
                }
            }
        }
    }
    Ok(ReturnShape::Plain)
}

struct SelfFinder(bool);

impl<'ast> Visit<'ast> for SelfFinder {
    fn visit_type_path(&mut self, type_path: &'ast TypePath) {
        if type_path.qself.is_none()
            && type_path.path.segments.len() == 1
            && type_path.path.segments[0].ident == "Self"
        {
            self.0 = true;
        }
        syn::visit::visit_type_path(self, type_path);
    }
}

enum ArgumentKind {
    Plain,
    SameVariant,
}

fn argument_kind(ty: &Type) -> Result<ArgumentKind> {
    let inner = match ty {
        Type::Reference(reference) => &*reference.elem,
        other => other,
    };
    if is_self(inner) {
        return Ok(ArgumentKind::SameVariant);
    }
    let mut finder = SelfFinder(false);
    finder.visit_type(ty);
    if finder.0 {
        return Err(Error::new_spanned(
            ty,
            "only `Self`, `&Self` and `&mut Self` arguments can be dispatched; this type mentions `Self` elsewhere",
        ));
    }
    Ok(ArgumentKind::Plain)
}

/// Returns the `mismatch` expression given on the trait method itself.
fn method_mismatch(function: &TraitItemFn) -> Result<Option<Expr>> {
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

fn expand_method(
    function: &TraitItemFn,
    variants: &[VariantInfo],
    enum_ident: &Ident,
    trait_path: &Path,
    spec: &TraitSpec,
    substitution: &mut Substitution,
) -> Result<TokenStream2> {
    let mut signature: Signature = function.sig.clone();
    substitution.visit_signature_mut(&mut signature);
    let method = signature.ident.clone();
    let Some(FnArg::Receiver(receiver)) = signature.inputs.first() else {
        if function.default.is_some() {
            return Ok(TokenStream2::new());
        }
        return Err(Error::new_spanned(
            &function.sig,
            format!("`{method}` has no receiver, so there is no variant to dispatch on; give it a default body or implement it by hand"),
        ));
    };
    if receiver.colon_token.is_some() {
        return Err(Error::new_spanned(
            receiver,
            "explicit `self: Type` receivers are not supported",
        ));
    }
    // Generated bindings get `macro_rules`-style hygiene, so they can never
    // collide with identifiers written by the user.
    let hygienic = |name: &str| Ident::new(name, Span::mixed_site());
    let inner = hygienic("__inner");
    let mut argument_idents = vec![];
    let mut same_variant = vec![];
    for (index, input) in signature.inputs.iter_mut().skip(1).enumerate() {
        let FnArg::Typed(pat_type) = input else {
            unreachable!("receiver handled above")
        };
        let ident = hygienic(&format!("__argument{index}"));
        *pat_type.pat = Pat::Ident(PatIdent {
            attrs: vec![],
            by_ref: None,
            mutability: None,
            ident: ident.clone(),
            subpat: None,
        });
        if let ArgumentKind::SameVariant = argument_kind(&pat_type.ty)? {
            same_variant.push(index);
        }
        argument_idents.push(ident);
    }
    let return_shape = classify_return(&signature.output, spec)?;
    let is_async = signature.asyncness.is_some();
    let is_unsafe = signature.unsafety.is_some();
    let mismatch = match spec.mismatch_for(&method) {
        Some(expr) => Some(expr.clone()),
        None => method_mismatch(function)?,
    };
    let mut arms = TokenStream2::new();
    for variant in variants {
        let VariantInfo {
            ident: variant_ident,
            payload,
            cfgs,
        } = variant;
        let call_arguments = argument_idents.iter().enumerate().map(|(index, ident)| {
            if same_variant.contains(&index) {
                let same = hygienic(&format!("__same{index}"));
                quote!(#same)
            } else {
                quote!(#ident)
            }
        });
        let mut call = quote! { <#payload as #trait_path>::#method(#inner #(, #call_arguments)*) };
        if is_unsafe {
            call = quote! { unsafe { #call } };
        }
        if is_async {
            call = quote! { #call.await };
        }
        call = match return_shape {
            ReturnShape::Plain => call,
            ReturnShape::RewrapSelf => quote! { #enum_ident::#variant_ident(#call) },
            ReturnShape::MapSelf => quote! { #call.map(#enum_ident::#variant_ident) },
            ReturnShape::IntoOverridden => quote! { ::core::convert::Into::into(#call) },
            ReturnShape::MapErrOverridden => {
                quote! { #call.map_err(::core::convert::Into::into) }
            }
        };
        let pattern = if same_variant.is_empty() {
            quote! { #enum_ident::#variant_ident(#inner) }
        } else {
            let same_patterns = same_variant.iter().map(|index| {
                let same = hygienic(&format!("__same{index}"));
                quote! { #enum_ident::#variant_ident(#same) }
            });
            quote! { (#enum_ident::#variant_ident(#inner), #(#same_patterns),*) }
        };
        arms.extend(quote! { #(#cfgs)* #pattern => #call, });
    }
    let scrutinee = if same_variant.is_empty() {
        quote!(self)
    } else {
        let arguments = same_variant.iter().map(|index| &argument_idents[*index]);
        quote! { (self, #(#arguments),*) }
    };
    let (fallback, allow_unreachable) = if same_variant.is_empty() {
        (TokenStream2::new(), TokenStream2::new())
    } else {
        let Some(expr) = mismatch else {
            return Err(Error::new_spanned(
                &function.sig,
                format!("`{method}` takes `Self` arguments; give the result for two different variants with `#[dispatch(mismatch = <expr>)]` on the trait method or `mismatch({method} = <expr>)` on the enum"),
            ));
        };
        (
            quote! { _ => #expr, },
            quote! { #[allow(unreachable_patterns)] },
        )
    };
    let inline = if function
        .attrs
        .iter()
        .any(|attribute| attribute.path().is_ident("inline"))
    {
        TokenStream2::new()
    } else {
        quote! { #[inline] }
    };
    let cfgs = cfg_attributes(&function.attrs);
    Ok(quote! {
        #(#cfgs)*
        #inline
        #allow_unreachable
        #signature {
            match #scrutinee {
                #arms
                #fallback
            }
        }
    })
}
