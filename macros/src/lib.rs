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
use proc_macro2::{Delimiter, Group, Punct, Spacing, Span, TokenStream as TokenStream2, TokenTree};
use proc_macro_crate::FoundCrate;
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::visit_mut::{self, VisitMut};
use syn::{
    braced, parenthesized, parse_macro_input, Attribute, Block, Error, Expr, Fields, FnArg,
    GenericArgument, GenericParam, Ident, ImplItem, Item, ItemEnum, ItemImpl, ItemTrait, Lifetime,
    LitBool, Meta, Pat, PatIdent, Path, PathArguments, PathSegment, QSelf, Receiver, Result,
    ReturnType, Signature, Token, TraitItem, TraitItemFn, Type, TypeParamBound, TypePath, Variant,
    Visibility,
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
/// pub trait Shape {
///     fn area(&self) -> u32;
///     #[dispatch(mismatch = false)]
///     fn congruent(&self, other: &Self) -> bool;
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
    // A `pub` trait may be dispatched from another crate, which reaches a
    // `macro_rules!` only through `#[macro_export]`; the macro then lives at
    // the crate root, so its name carries the trait's text, the position of
    // every token and the call site to keep same-named traits apart. A trait
    // of any other visibility cannot leave the crate, so its macro stays a
    // plain `macro_rules!` in the trait's module, reachable through the
    // re-export wherever the trait is, and adds nothing to the crate root.
    let export = if matches!(vis, Visibility::Public(_)) {
        quote!(#[macro_export])
    } else {
        quote!(#[allow(unused_macros)])
    };
    let mut hasher = DefaultHasher::new();
    captured.to_token_stream().to_string().hash(&mut hasher);
    hash_token_positions(captured.to_token_stream(), &mut hasher);
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
        #export
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

/// Hashes the file and position of every token in `tokens`, groups included.
///
/// The text alone does not tell apart the same trait emitted twice by one
/// `macro_rules!` in two modules, and the call site of `#[dispatchable]`
/// then lies inside the macro's body for both expansions. The tokens the
/// invocation supplied, usually the trait's name, keep the file and position
/// they were written at, so hashing those keeps the two apart. A trait
/// written out in full in the macro's body is the one case this leaves.
fn hash_token_positions(tokens: TokenStream2, hasher: &mut DefaultHasher) {
    for tree in tokens {
        let span = tree.span().unwrap();
        (file_key(&span.file()), span.line(), span.column()).hash(hasher);
        if let TokenTree::Group(group) = tree {
            hash_token_positions(group.stream(), hasher);
        }
    }
}

/// Returns the last two components of a source file's path. They tell
/// `tokio/mod.rs` from `smol/mod.rs` without depending on where the checkout
/// lives, so the hidden names come out the same on every machine.
fn file_key(path: &str) -> String {
    let mut components = path.rsplit(['/', '\\']);
    let file = components.next().unwrap_or_default();
    match components.next() {
        Some(directory) => format!("{directory}/{file}"),
        None => file.to_owned(),
    }
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

/// One `#[dispatch(..)]` attribute on an enum: a trait path with its
/// arguments, options, or both. The path is absent when the attribute only
/// carries enum-level options.
struct TraitSpec {
    path: Option<Path>,
    assoc_overrides: Vec<(Ident, Type)>,
    from: Option<bool>,
    try_from: Option<bool>,
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
        let starts_with_option =
            input.peek(Ident) && (input.peek2(Token![=]) || input.peek2(syn::token::Paren));
        let path = if input.is_empty() || starts_with_option {
            None
        } else {
            Some(input.parse::<Path>()?)
        };
        let mut spec = TraitSpec {
            path,
            assoc_overrides: vec![],
            from: None,
            try_from: None,
            mismatch: vec![],
        };
        let mut expect_comma = spec.path.is_some();
        while !input.is_empty() {
            if expect_comma {
                input.parse::<Token![,]>()?;
                if input.is_empty() {
                    break;
                }
            }
            expect_comma = true;
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
                    format!("unknown option `{key}`; the options are `Name = Type` for an associated type, `mismatch(method = expr)`, `from = false` and `try_from = true`"),
                ));
            }
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "from" => spec.from = Some(input.parse::<LitBool>()?.value),
                "try_from" => spec.try_from = Some(input.parse::<LitBool>()?.value),
                _ => spec.assoc_overrides.push((key, input.parse()?)),
            }
        }
        Ok(spec)
    }
}

/// Implements a `#[dispatchable]` trait for an enum by forwarding every
/// method to whichever variant the value holds, or forwards the methods of
/// an inherent `impl` block of such an enum.
///
/// On an enum, each attribute names one trait, with its generic arguments
/// when it has any, followed by options; several attributes dispatch several
/// traits, and a bare `#[dispatch]` only registers the enum. Every variant
/// must hold exactly one unnamed field whose type implements the trait. The
/// enum also gets a `From<Payload>` impl for every payload type that appears
/// in exactly one variant.
///
/// ```rust,ignore
/// #[whichever::dispatch(Shape)]
/// #[whichever::dispatch(Encoder<Json>, Error = EncodeError)]
/// pub enum AnyShape {
///     Circle(Circle),
///     #[dispatch(deref)]
///     Square(Box<Square>),
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
/// - `try_from = true`: a `TryFrom<Enum> for Payload` impl per payload type,
///   returning the enum unchanged when it holds another variant.
///
/// The one variant option, `#[dispatch(deref)]`, calls the trait on what the
/// payload dereferences to, for `Box<T>`, `Rc<T>`, `Arc<T>`, `&T` and
/// `&mut T` payloads; such a variant cannot take part in a method that
/// returns `Self`.
///
/// On an inherent `impl` block of a dispatch enum, in the same crate, every
/// method carrying `#[dispatch(..)]` is forwarded to the named variants'
/// own method of the same name, and the body is what the other variants
/// return. `*` names every variant; the method then has no fallback and is
/// written without a body:
///
/// ```rust,ignore
/// #[whichever::dispatch]
/// impl AnyShape {
///     /// Only circles have a radius.
///     #[dispatch(Circle, with = Some)]
///     pub fn radius(&self) -> Option<f64> {
///         None
///     }
///
///     #[dispatch(*)]
///     pub fn perimeter(&self) -> f64;
/// }
/// ```
///
/// Method options: `with = path` wraps each forwarded result in a call to
/// `path`, and `call = name` forwards to a payload method of another name.
/// Methods without the attribute are left as they are.
#[proc_macro_attribute]
pub fn dispatch(attr: TokenStream, item: TokenStream) -> TokenStream {
    let site = proc_macro::Span::call_site();
    let (line, column) = (site.line(), site.column());
    let item = parse_macro_input!(item as Item);
    let expanded = match item {
        Item::Enum(item_enum) => {
            // A rejected enum is still emitted, with its macro, so that its
            // derives, its uses and its impl blocks resolve and the one real
            // error is not buried under theirs.
            let emitted = without_dispatch_attributes(item_enum.clone());
            let enum_macro = enum_macro(&emitted, true, line, column);
            expand_dispatch(attr.into(), item_enum, line, column).unwrap_or_else(|error| {
                let error = error.to_compile_error();
                quote! { #emitted #enum_macro #error }
            })
        }
        Item::Impl(item_impl) => expand_inherent_invocation(attr.into(), item_impl)
            .unwrap_or_else(|error| error.to_compile_error()),
        other => {
            let error = Error::new_spanned(
                &other,
                "`#[dispatch]` goes on an enum, or on an inherent impl block of a dispatch enum",
            )
            .to_compile_error();
            quote! { #other #error }
        }
    };
    expanded.into()
}

/// Returns `item_enum` without the `#[dispatch]` attributes on the enum and
/// on its variants.
fn without_dispatch_attributes(mut item_enum: ItemEnum) -> ItemEnum {
    item_enum
        .attrs
        .retain(|attribute| !is_dispatch_attribute(attribute));
    for variant in &mut item_enum.variants {
        variant
            .attrs
            .retain(|attribute| !is_dispatch_attribute(attribute));
    }
    item_enum
}

/// Writes the enum's own macro, which carries its definition to `#[dispatch]`
/// on an impl block. Impl blocks live in the enum's crate, so the macro stays
/// crate-local and is reached through the alias. `rejected` tells the impl
/// side that the enum itself was rejected, so it must not report the same
/// error again.
fn enum_macro(emitted: &ItemEnum, rejected: bool, line: usize, column: usize) -> TokenStream2 {
    let enum_ident = &emitted.ident;
    let mut hasher = DefaultHasher::new();
    emitted.to_token_stream().to_string().hash(&mut hasher);
    let macro_name = format_ident!(
        "__whichever_enum_{}_{:016x}_{}_{}",
        enum_ident,
        hasher.finish(),
        line,
        column
    );
    let rejected = rejected.then(|| quote!(rejected));
    quote! {
        #[allow(unused_macros)]
        macro_rules! #macro_name {
            (@v1 $facade:ident $($rest:tt)*) => {
                ::$facade::__private::implement_inherent! { @v1 #rejected enum { #emitted } $($rest)* }
            };
        }

        #[doc(hidden)]
        #[allow(unused_imports)]
        pub(crate) use #macro_name as #enum_ident;
    }
}

fn expand_dispatch(
    first: TokenStream2,
    mut item_enum: ItemEnum,
    line: usize,
    column: usize,
) -> Result<TokenStream2> {
    let mut specs = Vec::new();
    if !first.is_empty() {
        specs.push(first);
    }
    let mut kept = vec![];
    for attribute in std::mem::take(&mut item_enum.attrs) {
        if !is_dispatch_attribute(&attribute) {
            kept.push(attribute);
            continue;
        }
        match &attribute.meta {
            Meta::List(list) => specs.push(list.tokens.clone()),
            // A bare `#[dispatch]` registers the enum without a trait.
            Meta::Path(_) => {}
            Meta::NameValue(_) => {
                return Err(Error::new_spanned(
                    attribute,
                    "expected `#[dispatch(Trait, ...)]`",
                ));
            }
        }
    }
    item_enum.attrs = kept;
    let variants = item_enum
        .variants
        .iter()
        .map(variant_info)
        .collect::<Result<Vec<_>>>()?;
    let captured = item_enum.clone();
    let emitted = without_dispatch_attributes(item_enum);
    let facade = facade_ident()?;
    let parsed_specs = specs
        .iter()
        .map(|tokens| syn::parse2::<TraitSpec>(tokens.clone()))
        .collect::<Result<Vec<_>>>()?;
    let generate_from = !parsed_specs.iter().any(|spec| spec.from == Some(false));
    let generate_try_from = parsed_specs.iter().any(|spec| spec.try_from == Some(true));
    let conversions = conversion_impls(&emitted, &variants, generate_from, generate_try_from);
    let mut invocations = TokenStream2::new();
    for (spec, tokens) in parsed_specs.iter().zip(&specs) {
        let Some(path) = &spec.path else { continue };
        let mut macro_path = path.clone();
        if let Some(last) = macro_path.segments.last_mut() {
            last.arguments = PathArguments::None;
        }
        invocations.extend(quote! {
            #macro_path! { @v1 #facade enum { #captured } spec { #tokens } }
        });
    }
    let enum_macro = enum_macro(&emitted, false, line, column);
    Ok(quote! {
        #emitted
        #conversions
        #enum_macro
        #invocations
    })
}

/// Turns `#[dispatch]` on an inherent impl block into a call of the enum's
/// macro, which supplies the enum's definition.
fn expand_inherent_invocation(attr: TokenStream2, item_impl: ItemImpl) -> Result<TokenStream2> {
    if !attr.is_empty() {
        return Err(Error::new_spanned(
            attr,
            "`#[dispatch]` on an impl block takes no arguments; the variants are named on each method",
        ));
    }
    if let Some((_, trait_path, _)) = &item_impl.trait_ {
        return Err(Error::new_spanned(
            trait_path,
            "`#[dispatch]` forwards inherent methods; a trait is dispatched with `#[dispatch(Trait)]` on the enum",
        ));
    }
    let Type::Path(self_type) = &*item_impl.self_ty else {
        return Err(Error::new_spanned(
            &item_impl.self_ty,
            "the impl must name a dispatch enum by path",
        ));
    };
    let mut macro_path = self_type.path.clone();
    if let Some(last) = macro_path.segments.last_mut() {
        last.arguments = PathArguments::None;
    }
    let facade = facade_ident()?;
    Ok(quote! {
        #macro_path! { @v1 #facade impl { #item_impl } }
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
    /// Whether the trait is called on what the payload dereferences to.
    deref: bool,
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
    let mut deref = false;
    for attribute in variant
        .attrs
        .iter()
        .filter(|attribute| is_dispatch_attribute(attribute))
    {
        let option: Ident = attribute.parse_args()?;
        if option != "deref" {
            return Err(Error::new_spanned(
                &option,
                format!("unknown variant option `{option}`; the only one is `deref`"),
            ));
        }
        deref = true;
    }
    Ok(VariantInfo {
        ident: variant.ident.clone(),
        payload: fields.unnamed[0].ty.clone(),
        cfgs: cfg_attributes(&variant.attrs),
        deref,
    })
}

/// Returns the type the trait is called on for `variant`: its payload, or
/// what the payload dereferences to.
fn target_type(variant: &VariantInfo) -> TokenStream2 {
    let payload = &variant.payload;
    if variant.deref {
        quote!(<#payload as ::core::ops::Deref>::Target)
    } else {
        quote!(#payload)
    }
}

fn cfg_attributes(attributes: &[Attribute]) -> Vec<Attribute> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .cloned()
        .collect()
}

/// Writes the `From<Payload>` and `TryFrom<Enum>` impls that were asked for.
fn conversion_impls(
    item_enum: &ItemEnum,
    variants: &[VariantInfo],
    from: bool,
    try_from: bool,
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = item_enum.generics.split_for_impl();
    let enum_ident = &item_enum.ident;
    // A payload type held by two variants gets no conversion: the impls
    // would conflict, and neither variant is the obvious target.
    let mut payload_counts = std::collections::HashMap::<String, usize>::new();
    for variant in variants {
        *payload_counts
            .entry(variant.payload.to_token_stream().to_string())
            .or_default() += 1;
    }
    let value = Ident::new("__value", Span::mixed_site());
    let other = Ident::new("__other", Span::mixed_site());
    let mut output = TokenStream2::new();
    for variant in variants {
        let VariantInfo {
            ident,
            payload,
            cfgs,
            ..
        } = variant;
        if payload_counts[&payload.to_token_stream().to_string()] > 1 {
            continue;
        }
        if from {
            output.extend(quote! {
                #(#cfgs)*
                #[automatically_derived]
                impl #impl_generics ::core::convert::From<#payload> for #enum_ident #ty_generics #where_clause {
                    #[inline]
                    fn from(#value: #payload) -> Self {
                        Self::#ident(#value)
                    }
                }
            });
        }
        if try_from {
            output.extend(quote! {
                #(#cfgs)*
                #[automatically_derived]
                impl #impl_generics ::core::convert::TryFrom<#enum_ident #ty_generics> for #payload #where_clause {
                    type Error = #enum_ident #ty_generics;
                    #[inline]
                    #[allow(unreachable_patterns)]
                    fn try_from(#value: #enum_ident #ty_generics) -> ::core::result::Result<Self, Self::Error> {
                        match #value {
                            #enum_ident::#ident(#value) => ::core::result::Result::Ok(#value),
                            #other => ::core::result::Result::Err(#other),
                        }
                    }
                }
            });
        }
    }
    output
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
    let Some(trait_path) = &spec.path else {
        return Err(Error::new(
            Span::call_site(),
            "the trait to implement is missing",
        ));
    };
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
    let first_target = target_type(first);
    let first_payload = &first_target;
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
///
/// The helper carries the enum's own generics, since a nested function
/// could not name them; the outer function passes them on to each call.
fn associated_type_check(
    names: &[&Ident],
    variants: &[VariantInfo],
    trait_path: &Path,
    first_payload: &TokenStream2,
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
    let mut helper_generics = generics.clone();
    helper_generics
        .params
        .push(syn::parse_quote!(__Variant: ?Sized + #bound));
    let (helper_impl_generics, _, helper_where_clause) = helper_generics.split_for_impl();
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let enum_arguments: Vec<TokenStream2> = generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Lifetime(param) => param.lifetime.to_token_stream(),
            GenericParam::Type(param) => param.ident.to_token_stream(),
            GenericParam::Const(param) => param.ident.to_token_stream(),
        })
        .collect();
    let checks = variants.iter().skip(1).map(|variant| {
        let payload = target_type(variant);
        let cfgs = &variant.cfgs;
        quote! { #(#cfgs)* __same_as_first_variant::<#(#enum_arguments,)* #payload>(); }
    });
    quote! {
        const _: () = {
            #[allow(dead_code, non_snake_case)]
            fn __same_as_first_variant #helper_impl_generics () #helper_where_clause {}
            #[allow(dead_code, non_snake_case)]
            fn __whichever_associated_types_agree #impl_generics () #where_clause {
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

/// Returns the last segment of a path type without a qualified self, so
/// that `Result<Self, E>`, `io::Result<Self>` and `core::option::Option<Self>`
/// are all seen by the name they end in.
fn last_segment(ty: &Type) -> Option<&PathSegment> {
    match ty {
        Type::Path(TypePath { qself: None, path }) => path.segments.last(),
        _ => None,
    }
}

/// Returns the `Output` of a return-position `impl Future`: `Some(Some(T))`
/// for `impl Future<Output = T>` with any further bounds, `Some(None)` for
/// an `impl Future` without an `Output` binding, and `None` for any other
/// type.
fn future_output(ty: &Type) -> Option<Option<&Type>> {
    let Type::ImplTrait(impl_trait) = ty else {
        return None;
    };
    impl_trait.bounds.iter().find_map(|bound| {
        let TypeParamBound::Trait(bound) = bound else {
            return None;
        };
        let last = bound.path.segments.last()?;
        if last.ident != "Future" {
            return None;
        }
        let PathArguments::AngleBracketed(arguments) = &last.arguments else {
            return Some(None);
        };
        Some(arguments.args.iter().find_map(|argument| match argument {
            GenericArgument::AssocType(assoc) if assoc.ident == "Output" => Some(&assoc.ty),
            _ => None,
        }))
    })
}

/// Classifies what the arms of a method must do with the payload's result,
/// and whether the method returns `impl Future`, in which case the arms are
/// awaited inside one `async` block and the classification applies to the
/// future's `Output`. `spec` carries the associated type overrides of a
/// trait; inherent methods have none.
fn classify_return(output: &ReturnType, spec: Option<&TraitSpec>) -> Result<(ReturnShape, bool)> {
    let ReturnType::Type(_, ty) = output else {
        return Ok((ReturnShape::Plain, false));
    };
    let (ty, returns_future) = match future_output(ty) {
        Some(Some(output)) => (output, true),
        Some(None) => return Ok((ReturnShape::Plain, true)),
        None => (&**ty, false),
    };
    if let Type::ImplTrait(_) = ty {
        return Err(Error::new_spanned(
            ty,
            "a return-position `impl Trait` cannot be dispatched by a `match` because every variant returns a different opaque type; return a concrete type or a boxed trait object, or `impl Future<Output = T>`, which is awaited like an `async fn`",
        ));
    }
    Ok((return_shape(ty, spec)?, returns_future))
}

fn return_shape(ty: &Type, spec: Option<&TraitSpec>) -> Result<ReturnShape> {
    let overridden = |name: &Ident| spec.is_some_and(|spec| spec.assoc_override(name).is_some());
    if is_self(ty) {
        return Ok(ReturnShape::RewrapSelf);
    }
    if let Some(name) = self_assoc(ty) {
        return Ok(if overridden(name) {
            ReturnShape::IntoOverridden
        } else {
            ReturnShape::Plain
        });
    }
    if let Some(segment) = last_segment(ty) {
        let args = generic_type_args(segment);
        let wraps_self = args.first().is_some_and(|first| is_self(first));
        if (segment.ident == "Option" || segment.ident == "Result") && wraps_self {
            return Ok(ReturnShape::MapSelf);
        }
        if segment.ident == "Result" && args.len() == 2 {
            if let Some(name) = self_assoc(args[1]) {
                if overridden(name) {
                    return Ok(ReturnShape::MapErrOverridden);
                }
            }
        }
    }
    if mentions_self(ty) {
        return Err(Error::new_spanned(
            ty,
            "this return type mentions `Self` where `whichever` cannot wrap the payload's result back into the enum; return `Self`, `Option<Self>` or `Result<Self, E>`, spelled with those names, or implement the method by hand",
        ));
    }
    Ok(ReturnShape::Plain)
}

/// Returns whether `ty` names the implementing type itself anywhere; the
/// `Self` of a projection such as `Self::Item` or `<Self as Trait>::Item`
/// names an associated type instead and does not count.
fn mentions_self(ty: &Type) -> bool {
    let mut finder = SelfFinder(false);
    finder.visit_type(ty);
    finder.0
}

struct SelfFinder(bool);

impl<'ast> Visit<'ast> for SelfFinder {
    fn visit_type_path(&mut self, type_path: &'ast TypePath) {
        if let Some(qself) = &type_path.qself {
            if !is_self(&qself.ty) {
                self.visit_type(&qself.ty);
            }
            self.visit_path(&type_path.path);
            return;
        }
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Self" {
            self.0 = true;
        }
        syn::visit::visit_type_path(self, type_path);
    }
}

/// How a method reaches its receiver or a `Self` argument.
#[derive(Clone, Copy)]
enum Access {
    Shared,
    Exclusive,
    Owned,
}

enum ArgumentKind {
    Plain,
    SameVariant(Access),
}

/// Returns how a receiver reaches the value, from its type: `self` and
/// `self: Self` own it, `&self` and `self: &Self` share it, `&mut self` and
/// `self: &mut Self` have it exclusively. Any other receiver type is an
/// error.
fn receiver_access(receiver: &Receiver) -> Result<Access> {
    match &*receiver.ty {
        ty if is_self(ty) => Ok(Access::Owned),
        Type::Reference(reference) if is_self(&reference.elem) => {
            Ok(if reference.mutability.is_some() { Access::Exclusive } else { Access::Shared })
        }
        other => Err(Error::new_spanned(
            other,
            "this receiver type is not supported; a receiver is `self`, `&self` or `&mut self`, also written `self: Self`, `self: &Self` or `self: &mut Self`",
        )),
    }
}

fn argument_kind(ty: &Type) -> Result<ArgumentKind> {
    let (inner, access) = match ty {
        Type::Reference(reference) if reference.mutability.is_some() => {
            (&*reference.elem, Access::Exclusive)
        }
        Type::Reference(reference) => (&*reference.elem, Access::Shared),
        other => (other, Access::Owned),
    };
    if is_self(inner) {
        return Ok(ArgumentKind::SameVariant(access));
    }
    if mentions_self(ty) {
        return Err(Error::new_spanned(
            ty,
            "only `Self`, `&Self` and `&mut Self` arguments can be dispatched; this type mentions `Self` elsewhere",
        ));
    }
    Ok(ArgumentKind::Plain)
}

/// Returns the expression that hands `binding`, bound to a variant's
/// payload, to the trait method: the payload itself, or what it
/// dereferences to for a `#[dispatch(deref)]` variant.
fn payload_expression(deref: bool, access: Access, binding: &Ident) -> TokenStream2 {
    if !deref {
        return quote!(#binding);
    }
    match access {
        Access::Shared => quote!(&**#binding),
        Access::Exclusive => quote!(&mut **#binding),
        Access::Owned => quote!(*#binding),
    }
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
    let receiver_access = receiver_access(receiver)?;
    // `self` is resolved like a local variable, so the generated `match`
    // must use the receiver's own token to share its hygiene.
    let self_token = receiver.self_token;
    // Generated bindings get `macro_rules`-style hygiene, so they can never
    // collide with identifiers written by the user.
    let hygienic = |name: &str| Ident::new(name, Span::mixed_site());
    let inner = hygienic("__inner");
    let mut argument_idents = vec![];
    let mut same_variant: Vec<(usize, Access)> = vec![];
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
        if let ArgumentKind::SameVariant(access) = argument_kind(&pat_type.ty)? {
            same_variant.push((index, access));
        }
        argument_idents.push(ident);
    }
    let (return_shape, returns_future) = classify_return(&signature.output, Some(spec))?;
    let is_async = signature.asyncness.is_some() || returns_future;
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
            deref,
        } = variant;
        if *deref && matches!(return_shape, ReturnShape::RewrapSelf | ReturnShape::MapSelf) {
            return Err(Error::new_spanned(
                payload,
                format!("`{method}` returns `Self`, which cannot be wrapped back into the dereferenced payload of variant `{variant_ident}`; drop `deref` on it or implement the method by hand"),
            ));
        }
        let target = target_type(variant);
        let receiver_expression = payload_expression(*deref, receiver_access, &inner);
        let call_arguments = argument_idents.iter().enumerate().map(|(index, ident)| {
            match same_variant
                .iter()
                .find(|(same_index, _)| *same_index == index)
            {
                Some((_, access)) => {
                    payload_expression(*deref, *access, &hygienic(&format!("__same{index}")))
                }
                None => quote!(#ident),
            }
        });
        let mut call = quote! {
            <#target as #trait_path>::#method(#receiver_expression #(, #call_arguments)*)
        };
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
            let same_patterns = same_variant.iter().map(|(index, _)| {
                let same = hygienic(&format!("__same{index}"));
                quote! { #enum_ident::#variant_ident(#same) }
            });
            quote! { (#enum_ident::#variant_ident(#inner), #(#same_patterns),*) }
        };
        arms.extend(quote! { #(#cfgs)* #pattern => #call, });
    }
    let scrutinee = if same_variant.is_empty() {
        quote!(#self_token)
    } else {
        let arguments = same_variant
            .iter()
            .map(|(index, _)| &argument_idents[*index]);
        quote! { (#self_token, #(#arguments),*) }
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
    let body = method_body(
        quote! { match #scrutinee { #arms #fallback } },
        returns_future,
        method.span(),
    );
    Ok(quote! {
        #(#cfgs)*
        #inline
        #allow_unreachable
        #signature #body
    })
}

/// Wraps a generated `match` into a method body: inside an `async move`
/// block when the method returns `impl Future`, so that every arm is awaited
/// where an `async fn` would await it, and in braces carrying the method's
/// own span, so that the item rustc points at as "defined here" is the
/// method and not the attribute that generated it.
fn method_body(body: TokenStream2, returns_future: bool, span: Span) -> TokenStream2 {
    let body = if returns_future {
        quote! { async move { #body } }
    } else {
        body
    };
    let mut group = Group::new(Delimiter::Brace, body);
    group.set_span(span);
    TokenTree::Group(group).into()
}

// ---------------------------------------------------------------------------
// Inherent methods
// ---------------------------------------------------------------------------

struct InherentInput {
    /// Whether `#[dispatch]` rejected the enum itself, whose error is then
    /// already reported.
    rejected: bool,
    item_enum: ItemEnum,
    item_impl: ItemImpl,
}

impl Parse for InherentInput {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![@]>()?;
        let version: Ident = input.parse()?;
        if version != "v1" {
            return Err(Error::new_spanned(
                version,
                "unsupported `whichever` protocol version",
            ));
        }
        let rejected = if input.peek(Ident) {
            let flag: Ident = input.parse()?;
            if flag != "rejected" {
                return Err(Error::new_spanned(flag, "expected `rejected`"));
            }
            true
        } else {
            false
        };
        input.parse::<Token![enum]>()?;
        let enum_tokens;
        braced!(enum_tokens in input);
        let item_enum: ItemEnum = enum_tokens.parse()?;
        input.parse::<Token![impl]>()?;
        let impl_tokens;
        braced!(impl_tokens in input);
        let item_impl: ItemImpl = impl_tokens.parse()?;
        Ok(Self {
            rejected,
            item_enum,
            item_impl,
        })
    }
}

/// Rewrites an inherent impl block; called by the macro that `#[dispatch]`
/// defines for the enum.
#[doc(hidden)]
#[proc_macro]
pub fn implement_inherent(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as InherentInput);
    expand_inherent(input)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Which variants an inherent method forwards to, and how.
struct Forwarding {
    all: bool,
    variants: Vec<Ident>,
    call: Option<Ident>,
    with: Option<Path>,
}

impl Parse for Forwarding {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut forwarding = Forwarding {
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

/// A method declared without a body, which `syn` leaves as verbatim tokens.
struct BodylessMethod {
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
}

impl Parse for BodylessMethod {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let sig = input.parse()?;
        input.parse::<Token![;]>()?;
        Ok(Self { attrs, vis, sig })
    }
}

/// Rewrites every marked method of the block. A method that cannot be
/// forwarded is reported and kept as written, without the attribute, or
/// dropped when it has no body, so that the rest of the block still
/// compiles and the real error stands alone. When the enum itself was
/// rejected nothing is reported here.
fn expand_inherent(input: InherentInput) -> Result<TokenStream2> {
    let InherentInput {
        rejected,
        item_enum,
        mut item_impl,
    } = input;
    let variants = if rejected {
        None
    } else {
        Some(
            item_enum
                .variants
                .iter()
                .map(variant_info)
                .collect::<Result<Vec<_>>>()?,
        )
    };
    let mut items = Vec::with_capacity(item_impl.items.len());
    let mut errors: Vec<Error> = vec![];
    for item in std::mem::take(&mut item_impl.items) {
        match item {
            ImplItem::Fn(mut function) => {
                let Some(position) = function.attrs.iter().position(is_dispatch_attribute) else {
                    items.push(ImplItem::Fn(function));
                    continue;
                };
                let attribute = function.attrs.remove(position);
                let Some(variants) = &variants else {
                    items.push(ImplItem::Fn(function));
                    continue;
                };
                let forwarded = attribute.parse_args().and_then(|forwarding| {
                    forward_method(
                        &function.attrs,
                        &function.vis,
                        &function.sig,
                        Some(&function.block),
                        forwarding,
                        variants,
                        &item_enum.ident,
                    )
                });
                match forwarded {
                    Ok(generated) => items.push(ImplItem::Verbatim(generated)),
                    Err(error) => {
                        errors.push(error);
                        items.push(ImplItem::Fn(function));
                    }
                }
            }
            ImplItem::Verbatim(tokens) => {
                let Ok(method) = syn::parse2::<BodylessMethod>(tokens.clone()) else {
                    errors.push(Error::new_spanned(
                        &tokens,
                        "only methods, with or without a body, are forwarded in a `#[dispatch]` impl block",
                    ));
                    continue;
                };
                let BodylessMethod {
                    mut attrs,
                    vis,
                    sig,
                } = method;
                let Some(position) = attrs.iter().position(is_dispatch_attribute) else {
                    errors.push(Error::new_spanned(
                        &sig,
                        "a method without a body needs `#[dispatch(*)]` to be forwarded to every variant",
                    ));
                    continue;
                };
                let attribute = attrs.remove(position);
                let Some(variants) = &variants else {
                    continue;
                };
                let forwarded = attribute.parse_args().and_then(|forwarding| {
                    forward_method(
                        &attrs,
                        &vis,
                        &sig,
                        None,
                        forwarding,
                        variants,
                        &item_enum.ident,
                    )
                });
                match forwarded {
                    Ok(generated) => items.push(ImplItem::Verbatim(generated)),
                    Err(error) => errors.push(error),
                }
            }
            other => items.push(other),
        }
    }
    item_impl.items = items;
    let errors = errors.into_iter().map(|error| error.to_compile_error());
    Ok(quote! { #item_impl #(#errors)* })
}

/// Writes one forwarded inherent method: a `match` with one arm per selected
/// variant calling the payload's own method, and the body as the arm for
/// every other variant.
fn forward_method(
    attrs: &[Attribute],
    vis: &Visibility,
    sig: &Signature,
    body: Option<&Block>,
    forwarding: Forwarding,
    variants: &[VariantInfo],
    enum_ident: &Ident,
) -> Result<TokenStream2> {
    let method = &sig.ident;
    let Some(FnArg::Receiver(receiver)) = sig.inputs.first() else {
        return Err(Error::new_spanned(
            sig,
            format!("`{method}` has no receiver, so there is no variant to forward to"),
        ));
    };
    receiver_access(receiver)?;
    let self_token = receiver.self_token;
    let mut arguments = Vec::new();
    for input in sig.inputs.iter().skip(1) {
        let FnArg::Typed(pat_type) = input else {
            unreachable!("receiver handled above")
        };
        let Pat::Ident(pattern) = &*pat_type.pat else {
            return Err(Error::new_spanned(
                &pat_type.pat,
                "a forwarded parameter must be a plain identifier",
            ));
        };
        arguments.push(pattern.ident.clone());
    }
    let selected: Vec<&VariantInfo> = if forwarding.all {
        variants.iter().collect()
    } else {
        forwarding
            .variants
            .iter()
            .map(|name| {
                variants
                    .iter()
                    .find(|variant| &variant.ident == name)
                    .ok_or_else(|| {
                        Error::new(
                            name.span(),
                            format!("`{enum_ident}` has no variant `{name}`"),
                        )
                    })
            })
            .collect::<Result<_>>()?
    };
    let covers_all = forwarding.all
        || variants
            .iter()
            .all(|variant| forwarding.variants.contains(&variant.ident));
    match (covers_all, body) {
        (true, Some(body)) => {
            return Err(Error::new_spanned(
                body,
                format!("`{method}` is forwarded to every variant, so its body would never run; end the signature with `;` instead"),
            ))
        }
        (false, None) => {
            return Err(Error::new_spanned(
                sig,
                format!("`{method}` is forwarded to some variants only; give it a body, which is what the other variants return"),
            ))
        }
        _ => {}
    }
    let inner = Ident::new("__inner", Span::mixed_site());
    let call_name = forwarding.call.as_ref().unwrap_or(method);
    let (return_shape, returns_future) = classify_return(&sig.output, None)?;
    let awaited = sig.asyncness.is_some() || returns_future;
    let arms = selected.iter().map(|variant| {
        let VariantInfo { ident, cfgs, .. } = variant;
        let mut call = quote! { #inner.#call_name(#(#arguments),*) };
        if awaited {
            call = quote! { #call.await };
        }
        // A payload's `Self` comes back as the variant. Without `with`, the
        // payload's own `Option` or `Result` is mapped; with it, the payload
        // returns the plain value and `with` lifts the variant into the
        // declared `Option` or `Result`.
        call = match (&return_shape, &forwarding.with) {
            (ReturnShape::MapSelf, None) => quote! { #call.map(Self::#ident) },
            (ReturnShape::RewrapSelf | ReturnShape::MapSelf, _) => quote! { Self::#ident(#call) },
            _ => call,
        };
        if let Some(with) = &forwarding.with {
            call = quote! { #with(#call) };
        }
        quote! { #(#cfgs)* Self::#ident(#inner) => #call, }
    });
    let fallback = body.map(|body| quote! { _ => #body, });
    let body = method_body(
        quote! { match #self_token { #(#arms)* #fallback } },
        returns_future,
        method.span(),
    );
    Ok(quote! { #(#attrs)* #vis #sig #body })
}
