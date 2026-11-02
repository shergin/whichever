//! Procedural macros of `whichever`.
//!
//! `#[dispatchable]` on a trait re-emits the trait and exports its carrier
//! macro, a hidden `macro_rules!` that carries the trait's tokens; the macro
//! is re-exported under the trait's own name, so importing the trait imports
//! the macro. `#[dispatch(Trait)]` on an enum invokes that macro, which
//! forwards the trait tokens, the enum tokens and the options into the
//! implementation macro, `implement!`, which writes `impl Trait for Enum`
//! with one `match` per method. No state is shared between expansions, so
//! the trait and the enum may live in different crates.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, Punct, Spacing, Span, TokenStream as TokenStream2, TokenTree};
use proc_macro_crate::FoundCrate;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::visit_mut::{self, VisitMut};
use syn::{
    braced, parenthesized, parse_macro_input, Attribute, Block, Error, Expr, Field, Fields, FnArg,
    GenericArgument, GenericParam, Ident, ImplItem, Index, Item, ItemEnum, ItemImpl, ItemStruct,
    ItemTrait, Lifetime, LitBool, Member, Meta, Pat, PatIdent, Path, PathArguments, PathSegment,
    QSelf, Receiver, Result, ReturnType, Signature, Token, TraitItem, TraitItemFn, Type,
    TypeParamBound, TypePath, Variant, Visibility,
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
/// on its methods are consumed. The only method option is `mismatch`, which
/// gives the result of a method taking `Self` arguments when the arguments
/// hold a different variant than `self`:
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
/// Options:
/// - `remote = path` declares a trait of another crate: the item is a
///   restatement of that trait's signatures, nothing is emitted for it but
///   the carrier macro, and `#[dispatch]` of the declaration implements the
///   trait at `path`. Only the methods restated are forwarded.
/// - `name = ident` names the carrier macro after `ident` instead of the
///   trait's text and position, for a trait that one `macro_rules!` emits
///   several times with identical tokens.
///
/// ```rust,ignore
/// #[whichever::dispatchable(remote = std::io::Write)]
/// trait Write {
///     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
///     fn flush(&mut self) -> std::io::Result<()>;
///     fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
/// }
/// ```
///
/// The trait's signature types are later written into the dispatching
/// enum's module, so they must be nameable there; `crate::` paths are
/// rewritten to this crate, `super::` and `self::` paths are not and must
/// not be used.
#[proc_macro_attribute]
pub fn dispatchable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let options = parse_macro_input!(attr as DispatchableOptions);
    let item_trait = parse_macro_input!(item as ItemTrait);
    let site = proc_macro::Span::call_site();
    expand_dispatchable(item_trait, options, site.line(), site.column())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// The options of `#[dispatchable(..)]`.
struct DispatchableOptions {
    /// The real path of a trait restated on behalf of another crate.
    remote: Option<Path>,
    /// A name for the carrier macro, in place of the trait's text and
    /// position.
    name: Option<Ident>,
}

impl Parse for DispatchableOptions {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut options = Self {
            remote: None,
            name: None,
        };
        while !input.is_empty() {
            let key: Ident = input.parse()?;
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
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(options)
    }
}

fn expand_dispatchable(
    item_trait: ItemTrait,
    options: DispatchableOptions,
    line: usize,
    column: usize,
) -> Result<TokenStream2> {
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
    let macro_name = match &options.name {
        Some(name) => format_ident!("__whichever_{}_{}", trait_ident, name),
        None => {
            let mut hasher = DefaultHasher::new();
            captured.to_token_stream().to_string().hash(&mut hasher);
            hash_token_positions(captured.to_token_stream(), &mut hasher);
            format_ident!(
                "__whichever_{}_{:016x}_{}_{}",
                trait_ident,
                hasher.finish(),
                line,
                column
            )
        }
    };
    // The captured tokens expand inside the enum's crate, where `crate::`
    // would name the wrong crate; `$crate` resolves to this crate instead.
    let captured = rewrite_crate_paths(captured.to_token_stream());
    // A restated trait exists elsewhere; only its macro is emitted here, and
    // the impl is written for the real path.
    let remote = options
        .remote
        .map(|path| rewrite_crate_paths(path.to_token_stream()))
        .map(|path| quote!(remote { #path }));
    let emitted = remote.is_none().then_some(emitted);
    Ok(quote! {
        #emitted

        #[doc(hidden)]
        #export
        macro_rules! #macro_name {
            (@v1 $facade:ident $($rest:tt)*) => {
                ::$facade::__private::implement! { @v1 trait { #captured } #remote $($rest)* }
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
/// lives, so the carrier macros' names come out the same on every machine.
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
    /// `Name = ..` options: a type for an associated type, a value for an
    /// associated const. Which one is meant is known once the trait is.
    assoc_overrides: Vec<(Ident, Override)>,
    from: Option<bool>,
    try_from: Option<bool>,
    mismatch: Vec<(Ident, Expr)>,
}

/// The right-hand side of a `Name = ..` option. A bare path parses as a
/// type and is read as a value when `Name` turns out to be a const.
enum Override {
    Type(Type),
    Expr(Expr),
}

impl Parse for Override {
    fn parse(input: ParseStream) -> Result<Self> {
        let fork = input.fork();
        if let Ok(ty) = fork.parse::<Type>() {
            if fork.is_empty() || fork.peek(Token![,]) {
                input.advance_to(&fork);
                return Ok(Self::Type(ty));
            }
        }
        Ok(Self::Expr(input.parse()?))
    }
}

impl TraitSpec {
    fn override_for(&self, name: &Ident) -> Option<&Override> {
        self.assoc_overrides
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    /// Returns the type given for the associated type `name`.
    fn assoc_override(&self, name: &Ident) -> Option<&Type> {
        match self.override_for(name) {
            Some(Override::Type(ty)) => Some(ty),
            _ => None,
        }
    }

    /// Returns the value given for the associated const `name`.
    fn const_override(&self, name: &Ident) -> Result<Option<Expr>> {
        match self.override_for(name) {
            None => Ok(None),
            Some(Override::Expr(expr)) => Ok(Some(expr.clone())),
            Some(Override::Type(Type::Path(path))) => Ok(Some(Expr::Path(syn::ExprPath {
                attrs: vec![],
                qself: path.qself.clone(),
                path: path.path.clone(),
            }))),
            Some(Override::Type(other)) => Err(Error::new_spanned(
                other,
                format!("`{name}` is an associated const; give it a value"),
            )),
        }
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
                    format!("unknown option `{key}`; the options are `Name = Type` for an associated type, `Name = value` for an associated const, `mismatch(method = expr)`, `from = false` and `try_from = true`"),
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
/// Two options go on a variant. `#[dispatch(deref)]` calls the trait on what
/// the payload dereferences to, for `Box<T>`, `Rc<T>`, `Arc<T>`, `&T` and
/// `&mut T` payloads; such a variant cannot take part in a method that
/// returns `Self`. `#[dispatch(from = false)]` leaves the variant out of the
/// conversions, and `#[dispatch(try_from = true)]` adds the `TryFrom` impl
/// for that payload alone.
///
/// On a struct, the trait is forwarded to its one field, or to the field
/// marked `#[dispatch]`, with `#[dispatch(deref)]` for a field behind a
/// pointer. A struct gets no conversions.
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
///
/// On a trait impl block of a dispatch enum, in the same crate, the block
/// holds the items the enum implements itself and the rest of the trait is
/// forwarded around them. The enum then does not name that trait in its own
/// attribute; the trait's options go on the block's attribute:
///
/// ```rust,ignore
/// #[whichever::dispatch]
/// pub enum Handler {
///     Text(TextHandler),
///     Map(MapHandler),
/// }
///
/// #[whichever::dispatch]
/// impl HandlerTrait for Handler {
///     fn from_handler(handler: Handler) -> Option<Self> {
///         Some(handler)
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn dispatch(attr: TokenStream, item: TokenStream) -> TokenStream {
    let site = proc_macro::Span::call_site();
    let (line, column) = (site.line(), site.column());
    let item = parse_macro_input!(item as Item);
    let expanded = match item {
        Item::Enum(item_enum) => {
            expand_subject(attr.into(), Subject::Enum(item_enum), line, column)
        }
        Item::Struct(item_struct) => {
            expand_subject(attr.into(), Subject::Struct(item_struct), line, column)
        }
        Item::Impl(item_impl) => expand_impl_invocation(attr.into(), item_impl)
            .unwrap_or_else(|error| error.to_compile_error()),
        other => {
            let error = Error::new_spanned(
                &other,
                "`#[dispatch]` goes on an enum or a struct, or on an impl block of one",
            )
            .to_compile_error();
            quote! { #other #error }
        }
    };
    expanded.into()
}

/// The item an impl is written for: an enum over implementors, or a struct
/// wrapping one.
#[derive(Clone)]
enum Subject {
    Enum(ItemEnum),
    Struct(ItemStruct),
}

impl Subject {
    fn ident(&self) -> &Ident {
        match self {
            Subject::Enum(item) => &item.ident,
            Subject::Struct(item) => &item.ident,
        }
    }

    fn generics(&self) -> &syn::Generics {
        match self {
            Subject::Enum(item) => &item.generics,
            Subject::Struct(item) => &item.generics,
        }
    }

    fn attrs_mut(&mut self) -> &mut Vec<Attribute> {
        match self {
            Subject::Enum(item) => &mut item.attrs,
            Subject::Struct(item) => &mut item.attrs,
        }
    }

    fn is_enum(&self) -> bool {
        matches!(self, Subject::Enum(_))
    }

    /// Returns the subject without the `#[dispatch]` attributes on the item
    /// itself; the variants or fields keep theirs.
    fn without_item_attributes(mut self) -> Self {
        self.attrs_mut()
            .retain(|attribute| !is_dispatch_attribute(attribute));
        self
    }

    /// Returns the subject without the `#[dispatch]` attributes on its
    /// variants or fields.
    fn without_inner_attributes(mut self) -> Self {
        match &mut self {
            Subject::Enum(item) => {
                for variant in &mut item.variants {
                    variant
                        .attrs
                        .retain(|attribute| !is_dispatch_attribute(attribute));
                }
            }
            Subject::Struct(item) => {
                for field in &mut item.fields {
                    field
                        .attrs
                        .retain(|attribute| !is_dispatch_attribute(attribute));
                }
            }
        }
        self
    }

    /// Returns every variant of the enum, or the one field of the struct
    /// that is forwarded to. A variant that is not `Name(Payload)` is an
    /// entry too, so that the methods which do not forward to it can tell.
    fn entries(&self) -> Result<Vec<Entry>> {
        match self {
            Subject::Enum(item) => item.variants.iter().map(variant_entry).collect(),
            Subject::Struct(item) => Ok(vec![Entry::Target(struct_target(item)?)]),
        }
    }
}

impl Parse for Subject {
    fn parse(input: ParseStream) -> Result<Self> {
        match input.parse::<Item>()? {
            Item::Enum(item) => Ok(Subject::Enum(item)),
            Item::Struct(item) => Ok(Subject::Struct(item)),
            other => Err(Error::new_spanned(other, "expected an enum or a struct")),
        }
    }
}

impl ToTokens for Subject {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Subject::Enum(item) => item.to_tokens(tokens),
            Subject::Struct(item) => item.to_tokens(tokens),
        }
    }
}

/// Expands `#[dispatch]` on an enum or a struct. A rejected item is still
/// emitted, with its macro, so that its derives, its uses and its impl
/// blocks resolve and the one real error is not buried under theirs.
fn expand_subject(
    attr: TokenStream2,
    subject: Subject,
    line: usize,
    column: usize,
) -> TokenStream2 {
    let captured = subject.clone().without_item_attributes();
    let emitted = captured.clone().without_inner_attributes();
    let subject_macro = subject_macro(&captured, true, line, column);
    expand_dispatch(attr, subject, line, column).unwrap_or_else(|error| {
        let error = error.to_compile_error();
        quote! { #emitted #subject_macro #error }
    })
}

/// Writes the subject's own macro, which carries its definition, variant
/// and field options included, to `#[dispatch]` on an impl block. Impl
/// blocks live in the subject's crate, so the macro stays crate-local and
/// is reached through the alias. `rejected` tells the impl side that the
/// item itself was rejected, so it must not report the same error again.
fn subject_macro(captured: &Subject, rejected: bool, line: usize, column: usize) -> TokenStream2 {
    let ident = captured.ident();
    let mut hasher = DefaultHasher::new();
    captured.to_token_stream().to_string().hash(&mut hasher);
    let macro_name = format_ident!(
        "__whichever_enum_{}_{:016x}_{}_{}",
        ident,
        hasher.finish(),
        line,
        column
    );
    let rejected = rejected.then(|| quote!(rejected));
    quote! {
        #[allow(unused_macros)]
        macro_rules! #macro_name {
            (@v1 $facade:ident $($rest:tt)*) => {
                ::$facade::__private::implement_inherent! { @v1 #rejected enum { #captured } $($rest)* }
            };
        }

        #[doc(hidden)]
        #[allow(unused_imports)]
        pub(crate) use #macro_name as #ident;
    }
}

fn expand_dispatch(
    first: TokenStream2,
    mut subject: Subject,
    line: usize,
    column: usize,
) -> Result<TokenStream2> {
    let mut specs = Vec::new();
    if !first.is_empty() {
        specs.push(first);
    }
    let mut kept = vec![];
    for attribute in std::mem::take(subject.attrs_mut()) {
        if !is_dispatch_attribute(&attribute) {
            kept.push(attribute);
            continue;
        }
        match &attribute.meta {
            Meta::List(list) => specs.push(list.tokens.clone()),
            // A bare `#[dispatch]` registers the item without a trait.
            Meta::Path(_) => {}
            Meta::NameValue(_) => {
                return Err(Error::new_spanned(
                    attribute,
                    "expected `#[dispatch(Trait, ...)]`",
                ));
            }
        }
    }
    *subject.attrs_mut() = kept;
    let entries = subject.entries()?;
    let captured = subject;
    let emitted = captured.clone().without_inner_attributes();
    let facade = facade_ident()?;
    let parsed_specs = specs
        .iter()
        .map(|tokens| syn::parse2::<TraitSpec>(tokens.clone()))
        .collect::<Result<Vec<_>>>()?;
    for path in parsed_specs.iter().filter_map(|spec| spec.path.as_ref()) {
        // A standard library trait is never `#[dispatchable]`; rustc would
        // only say that the macro of that name is missing.
        let first = &path.segments[0].ident;
        if path.segments.len() > 1 && (first == "std" || first == "core" || first == "alloc") {
            let spelled = path.to_token_stream().to_string().replace(' ', "");
            let name = &path.segments.last().expect("checked above").ident;
            return Err(Error::new_spanned(
                path,
                format!("`{spelled}` is not `#[dispatchable]`; dispatch `whichever::foreign::{name}` if it is one of the traits `whichever` declares, or restate the trait with `#[whichever::dispatchable(remote = {spelled})]`"),
            ));
        }
    }
    let generate_from = !parsed_specs.iter().any(|spec| spec.from == Some(false));
    let generate_try_from = parsed_specs.iter().any(|spec| spec.try_from == Some(true));
    let conversions = if captured.is_enum() {
        conversion_impls(&captured, &entries, generate_from, generate_try_from)?
    } else {
        if parsed_specs
            .iter()
            .any(|spec| spec.from.is_some() || spec.try_from.is_some())
        {
            return Err(Error::new(
                Span::call_site(),
                "`from` and `try_from` are options of an enum; a struct gets no conversions",
            ));
        }
        TokenStream2::new()
    };
    // Every variant must be forwardable when the item dispatches a trait.
    // The error carries the variant's `cfg`, so a variant that is compiled
    // out is no error, and it is reported here once rather than per trait.
    let shape_errors = if parsed_specs.iter().any(|spec| spec.path.is_some()) {
        entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Unforwardable(unforwardable) => Some(unforwardable.compile_error()),
                Entry::Target(_) => None,
            })
            .collect()
    } else {
        TokenStream2::new()
    };
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
    let subject_macro = subject_macro(&captured, false, line, column);
    Ok(quote! {
        #emitted
        #conversions
        #subject_macro
        #shape_errors
        #invocations
    })
}

/// Turns `#[dispatch]` on an impl block into a call of the subject's macro,
/// which supplies the subject's definition. An inherent block is forwarded
/// method by method; a trait impl block is a partial impl, whose options
/// travel along for the trait's carrier macro.
fn expand_impl_invocation(attr: TokenStream2, item_impl: ItemImpl) -> Result<TokenStream2> {
    let Type::Path(self_type) = &*item_impl.self_ty else {
        return Err(Error::new_spanned(
            &item_impl.self_ty,
            "the impl must name a dispatch enum or struct by path",
        ));
    };
    let mut macro_path = self_type.path.clone();
    if let Some(last) = macro_path.segments.last_mut() {
        last.arguments = PathArguments::None;
    }
    let facade = facade_ident()?;
    match &item_impl.trait_ {
        Some((Some(bang), _, _)) => Err(Error::new_spanned(
            bang,
            "a negative impl has nothing to dispatch",
        )),
        Some((None, _, _)) => Ok(quote! {
            #macro_path! { @v1 #facade partial { #attr } impl { #item_impl } }
        }),
        None => {
            if !attr.is_empty() {
                return Err(Error::new_spanned(
                    attr,
                    "`#[dispatch]` on an inherent impl block takes no arguments; the variants are named on each method",
                ));
            }
            Ok(quote! {
                #macro_path! { @v1 #facade impl { #item_impl } }
            })
        }
    }
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

/// A variant or field a method forwards to.
struct Target {
    /// The variant's name, or the field's name; an unnamed field is
    /// `_<index>`.
    ident: Ident,
    place: Place,
    payload: Type,
    cfgs: Vec<Attribute>,
    /// Whether the trait is called on what the payload dereferences to.
    deref: bool,
    /// Whether the variant takes part in the `From` and `TryFrom` impls.
    from: bool,
    /// The variant's own say on its `TryFrom` impl, over the enum's option.
    try_from: Option<bool>,
}

/// How a target is matched and built.
enum Place {
    /// `Enum::Variant(payload)`.
    Variant,
    /// `Struct { member: payload, .. }` or `Struct(_, payload, ..)`;
    /// `rebuildable` when the struct has no other field, so that the value
    /// built around a payload is whole.
    Field { member: Member, rebuildable: bool },
}

impl Target {
    /// Returns the pattern binding the payload to `binding`, on `prefix`,
    /// which is the subject's name or `Self`.
    fn pattern(&self, prefix: &TokenStream2, binding: &Ident) -> TokenStream2 {
        let ident = &self.ident;
        match &self.place {
            Place::Variant => quote!(#prefix::#ident(#binding)),
            Place::Field {
                member: Member::Unnamed(index),
                ..
            } => {
                let skipped = (0..index.index).map(|_| quote!(_));
                quote!(#prefix(#(#skipped,)* #binding, ..))
            }
            Place::Field { member, .. } => quote!(#prefix { #member: #binding, .. }),
        }
    }

    /// Returns the expression building the subject around `value`, or
    /// `None` when other fields would be missing.
    fn rebuild(&self, prefix: &TokenStream2, value: TokenStream2) -> Option<TokenStream2> {
        let ident = &self.ident;
        match &self.place {
            Place::Variant => Some(quote!(#prefix::#ident(#value))),
            Place::Field {
                member: Member::Unnamed(_),
                rebuildable: true,
            } => Some(quote!(#prefix(#value))),
            Place::Field {
                member,
                rebuildable: true,
            } => Some(quote!(#prefix { #member: #value })),
            Place::Field {
                rebuildable: false, ..
            } => None,
        }
    }

    /// Returns a function building the subject from a payload, for `map`.
    fn constructor(&self, prefix: &TokenStream2) -> Option<TokenStream2> {
        let ident = &self.ident;
        match &self.place {
            Place::Variant => Some(quote!(#prefix::#ident)),
            Place::Field {
                member: Member::Unnamed(_),
                rebuildable: true,
            } => Some(quote!(#prefix)),
            Place::Field {
                member,
                rebuildable: true,
            } => {
                let value = Ident::new("__value", Span::mixed_site());
                Some(quote!(|#value| #prefix { #member: #value }))
            }
            Place::Field {
                rebuildable: false, ..
            } => None,
        }
    }

    /// Returns the type the trait is called on: the payload, or what it
    /// dereferences to.
    fn target_type(&self) -> TokenStream2 {
        let payload = &self.payload;
        if self.deref {
            quote!(<#payload as ::core::ops::Deref>::Target)
        } else {
            quote!(#payload)
        }
    }

    /// Returns the same type for a position outside the subject's own
    /// impls, where a `Self` the payload names would not be the subject.
    fn target_type_outside(&self, self_type: &Type) -> TokenStream2 {
        let mut payload = self.payload.clone();
        SelfReplacer(self_type.clone()).visit_type_mut(&mut payload);
        if self.deref {
            quote!(<#payload as ::core::ops::Deref>::Target)
        } else {
            quote!(#payload)
        }
    }

    /// Returns whether the payload is a `Box`, dereferenced to the
    /// implementor, which is what a `self: Box<Self>` receiver needs.
    fn holds_box(&self) -> bool {
        self.deref && last_segment(&self.payload).is_some_and(|segment| segment.ident == "Box")
    }
}

/// Replaces `Self` with the subject's type, for a payload type written
/// outside the subject's own impls.
struct SelfReplacer(Type);

impl VisitMut for SelfReplacer {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        if is_self(ty) {
            *ty = self.0.clone();
            return;
        }
        visit_mut::visit_type_mut(self, ty);
    }
}

/// A variant that is not `Name(Payload)`, which no method can forward to.
/// Inherent methods leave it to their fallback body; a dispatched trait
/// reports it, under its own `cfg`.
struct Unforwardable {
    ident: Ident,
    cfgs: Vec<Attribute>,
    error: Error,
}

impl Unforwardable {
    fn compile_error(&self) -> TokenStream2 {
        let cfgs = &self.cfgs;
        let error = self.error.to_compile_error();
        quote! { #(#cfgs)* #error }
    }
}

// Short-lived and few; the size difference does not matter here.
#[allow(clippy::large_enum_variant)]
enum Entry {
    Target(Target),
    Unforwardable(Unforwardable),
}

/// Returns the targets among `entries`.
fn targets(entries: &[Entry]) -> Vec<&Target> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Target(target) => Some(target),
            Entry::Unforwardable(_) => None,
        })
        .collect()
}

/// The `#[dispatch(..)]` options of a variant or field.
struct TargetOptions {
    deref: bool,
    from: bool,
    try_from: Option<bool>,
}

/// Parses the options of a variant or field: `deref` on either, `from =
/// false` and `try_from = true` on a variant.
fn target_options(attributes: &[Attribute], on_variant: bool) -> Result<TargetOptions> {
    let mut options = TargetOptions {
        deref: false,
        from: true,
        try_from: None,
    };
    for attribute in attributes
        .iter()
        .filter(|attribute| is_dispatch_attribute(attribute))
    {
        if matches!(attribute.meta, Meta::Path(_)) {
            // A bare `#[dispatch]` marks the field to forward to.
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("deref") {
                options.deref = true;
            } else if meta.path.is_ident("from") && on_variant {
                options.from = meta.value()?.parse::<LitBool>()?.value;
            } else if meta.path.is_ident("try_from") && on_variant {
                options.try_from = Some(meta.value()?.parse::<LitBool>()?.value);
            } else {
                let option = meta.path.to_token_stream();
                let known = if on_variant {
                    "the options are `deref`, `from = false` and `try_from = true`"
                } else {
                    "the only one is `deref`"
                };
                return Err(meta.error(format!("unknown option `{option}`; {known}")));
            }
            Ok(())
        })?;
    }
    Ok(options)
}

fn variant_entry(variant: &Variant) -> Result<Entry> {
    let TargetOptions {
        deref,
        from,
        try_from,
    } = target_options(&variant.attrs, true)?;
    let payload = match &variant.fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => fields.unnamed[0].ty.clone(),
        Fields::Unnamed(_) => {
            return Ok(Entry::Unforwardable(Unforwardable {
                ident: variant.ident.clone(),
                cfgs: cfg_attributes(&variant.attrs),
                error: Error::new_spanned(
                    variant,
                    "a dispatch variant must hold exactly one payload",
                ),
            }))
        }
        _ => {
            return Ok(Entry::Unforwardable(Unforwardable {
                ident: variant.ident.clone(),
                cfgs: cfg_attributes(&variant.attrs),
                error: Error::new_spanned(variant, "a dispatch variant must be `Name(Payload)`"),
            }))
        }
    };
    Ok(Entry::Target(Target {
        ident: variant.ident.clone(),
        place: Place::Variant,
        payload,
        cfgs: cfg_attributes(&variant.attrs),
        deref,
        from,
        try_from,
    }))
}

/// Returns the one field of a struct that is forwarded to: its only field,
/// or the one marked `#[dispatch]`.
fn struct_target(item: &ItemStruct) -> Result<Target> {
    let fields: Vec<(Member, &Field)> = item
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let member = match &field.ident {
                Some(ident) => Member::Named(ident.clone()),
                None => Member::Unnamed(Index::from(index)),
            };
            (member, field)
        })
        .collect();
    let marked: Vec<&(Member, &Field)> = fields
        .iter()
        .filter(|(_, field)| field.attrs.iter().any(is_dispatch_attribute))
        .collect();
    let (member, field) = match (fields.as_slice(), marked.as_slice()) {
        ([], _) => {
            return Err(Error::new_spanned(
                &item.ident,
                "a dispatch struct needs a field to forward to",
            ))
        }
        ([(member, field)], _) | (_, [(member, field)]) => (member.clone(), *field),
        (_, []) => {
            return Err(Error::new_spanned(
                &item.ident,
                format!(
                    "`{}` has several fields; mark the one to forward to with `#[dispatch]`",
                    item.ident
                ),
            ))
        }
        (_, [_, (_, second), ..]) => {
            return Err(Error::new_spanned(
                second,
                "only one field can be forwarded to",
            ))
        }
    };
    let TargetOptions { deref, .. } = target_options(&field.attrs, false)?;
    let ident = match &member {
        Member::Named(ident) => ident.clone(),
        Member::Unnamed(index) => format_ident!("_{}", index.index, span = index.span),
    };
    Ok(Target {
        ident,
        place: Place::Field {
            member,
            rebuildable: fields.len() == 1,
        },
        payload: field.ty.clone(),
        cfgs: vec![],
        deref,
        from: false,
        try_from: None,
    })
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
    subject: &Subject,
    entries: &[Entry],
    from: bool,
    try_from: bool,
) -> Result<TokenStream2> {
    let (impl_generics, ty_generics, where_clause) = subject.generics().split_for_impl();
    let enum_ident = subject.ident();
    let prefix = enum_ident.to_token_stream();
    let self_type: Type = syn::parse_quote!(#enum_ident #ty_generics);
    let converting: Vec<&Target> = targets(entries)
        .into_iter()
        .filter(|target| target.from)
        .collect();
    // A payload that is a type parameter of the enum can be any type, the
    // other payloads included, so its `From` overlaps theirs; rustc would
    // only report conflicting implementations on the attribute.
    if from && converting.len() > 1 {
        for target in &converting {
            let Some(ident) = bare_ident(&target.payload) else {
                continue;
            };
            let is_parameter =
                subject.generics().params.iter().any(
                    |param| matches!(param, GenericParam::Type(param) if &param.ident == ident),
                );
            if is_parameter {
                return Err(Error::new_spanned(
                    &target.payload,
                    format!("`{ident}` is a type parameter of `{enum_ident}`, so `From<{ident}>` would overlap the other variants' `From` impls; add `from = false` on the enum, or `#[dispatch(from = false)]` on this variant"),
                ));
            }
        }
    }
    // A payload type held by two variants gets no conversion: the impls
    // would conflict, and neither variant is the obvious target. A variant
    // marked `from = false` takes no part at all.
    let mut payload_counts = std::collections::HashMap::<String, usize>::new();
    for target in &converting {
        *payload_counts
            .entry(target.payload.to_token_stream().to_string())
            .or_default() += 1;
    }
    let value = Ident::new("__value", Span::mixed_site());
    let other = Ident::new("__other", Span::mixed_site());
    let mut output = TokenStream2::new();
    for target in converting {
        let Target { payload, cfgs, .. } = target;
        if payload_counts[&payload.to_token_stream().to_string()] > 1 {
            continue;
        }
        let built = target.rebuild(&quote!(Self), quote!(#value));
        let pattern = target.pattern(&prefix, &value);
        if from {
            output.extend(quote! {
                #(#cfgs)*
                #[automatically_derived]
                impl #impl_generics ::core::convert::From<#payload> for #enum_ident #ty_generics #where_clause {
                    #[inline]
                    fn from(#value: #payload) -> Self {
                        #built
                    }
                }
            });
        }
        if target.try_from.unwrap_or(try_from) {
            // The payload is the impl's own type here, so a `Self` it names
            // must be spelled out as the enum.
            let mut outside = payload.clone();
            SelfReplacer(self_type.clone()).visit_type_mut(&mut outside);
            output.extend(quote! {
                #(#cfgs)*
                #[automatically_derived]
                impl #impl_generics ::core::convert::TryFrom<#enum_ident #ty_generics> for #outside #where_clause {
                    type Error = #enum_ident #ty_generics;
                    #[inline]
                    #[allow(unreachable_patterns)]
                    fn try_from(#value: #enum_ident #ty_generics) -> ::core::result::Result<Self, Self::Error> {
                        match #value {
                            #pattern => ::core::result::Result::Ok(#value),
                            #other => ::core::result::Result::Err(#other),
                        }
                    }
                }
            });
        }
    }
    Ok(output)
}

// ---------------------------------------------------------------------------
// The implementation macro
// ---------------------------------------------------------------------------

struct ImplementInput {
    item_trait: ItemTrait,
    /// The real path of a trait restated with `remote = ..`, which the impl
    /// is written for.
    remote: Option<Path>,
    subject: Subject,
    spec: TraitSpec,
    /// The impl block the user wrote, holding the items the subject
    /// implements itself; the rest is generated around them.
    partial: Option<ItemImpl>,
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
        let remote = if input.peek(Ident) {
            let tag: Ident = input.parse()?;
            if tag != "remote" {
                return Err(Error::new_spanned(tag, "expected `remote`"));
            }
            let remote_tokens;
            braced!(remote_tokens in input);
            Some(remote_tokens.parse::<Path>()?)
        } else {
            None
        };
        input.parse::<Token![enum]>()?;
        let subject_tokens;
        braced!(subject_tokens in input);
        let subject: Subject = subject_tokens.parse()?;
        let tag: Ident = input.parse()?;
        if tag != "spec" {
            return Err(Error::new_spanned(tag, "expected `spec`"));
        }
        let spec_tokens;
        braced!(spec_tokens in input);
        let spec: TraitSpec = spec_tokens.parse()?;
        let partial = if input.peek(Token![impl]) {
            input.parse::<Token![impl]>()?;
            let impl_tokens;
            braced!(impl_tokens in input);
            Some(impl_tokens.parse::<ItemImpl>()?)
        } else {
            None
        };
        Ok(Self {
            item_trait,
            remote,
            subject,
            spec,
            partial,
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
        remote,
        subject,
        mut spec,
        partial,
    } = input;
    let subject_ident = subject.ident();
    let Some(spec_path) = &spec.path else {
        return Err(Error::new(
            Span::call_site(),
            "the trait to implement is missing",
        ));
    };
    // A restated trait is implemented at its real path, with the generic
    // arguments the enum named on the declaration's path.
    let implemented = match remote {
        Some(mut remote) => {
            if let (Some(last), Some(named)) =
                (remote.segments.last_mut(), spec_path.segments.last())
            {
                last.arguments = named.arguments.clone();
            }
            remote
        }
        None => spec_path.clone(),
    };
    let trait_path = &implemented;
    // A partial impl block supplies the impl's header and the items the enum
    // implements itself; an associated type written there is an override
    // like the `Name = Type` option.
    let mut written = std::collections::HashSet::new();
    let mut user_items = vec![];
    let (generics, self_ty, impl_attrs, unsafety) = match &partial {
        Some(block) => {
            for item in &block.items {
                match item {
                    ImplItem::Fn(function) => {
                        written.insert(function.sig.ident.to_string());
                    }
                    ImplItem::Type(assoc) => {
                        written.insert(assoc.ident.to_string());
                        spec.assoc_overrides
                            .push((assoc.ident.clone(), Override::Type(assoc.ty.clone())));
                    }
                    ImplItem::Const(constant) => {
                        written.insert(constant.ident.to_string());
                    }
                    _ => {}
                }
                user_items.push(item);
            }
            (
                &block.generics,
                block.self_ty.to_token_stream(),
                block.attrs.as_slice(),
                block.unsafety,
            )
        }
        None => {
            let (_, ty_generics, _) = subject.generics().split_for_impl();
            (
                subject.generics(),
                quote!(#subject_ident #ty_generics),
                &[][..],
                None,
            )
        }
    };
    let derived = partial.is_none().then(|| quote!(#[automatically_derived]));
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let entries = subject.entries()?;
    let variants = targets(&entries);
    let Some(first) = variants.first() else {
        return Err(Error::new_spanned(
            &subject,
            "a dispatch enum needs at least one variant",
        ));
    };
    // A partial block is the only dispatch of its trait, so a variant that
    // cannot be forwarded to is reported here, under its own `cfg`.
    let shape_errors: TokenStream2 = match &partial {
        Some(_) => entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Unforwardable(unforwardable) => Some(unforwardable.compile_error()),
                Entry::Target(_) => None,
            })
            .collect(),
        None => TokenStream2::new(),
    };
    let first_target = first.target_type();
    let first_payload = &first_target;
    // The checks live outside the impl, where a `Self` in a payload type
    // has to be spelled out as the implementing type.
    let self_type: Type = syn::parse2(self_ty.clone())?;
    let first_outside = first.target_type_outside(&self_type);
    let mut substitution = Substitution::new(&item_trait, trait_path)?;
    let mut items = TokenStream2::new();
    let mut agreed_types: Vec<&Ident> = vec![];
    let mut agreed_consts: Vec<(&Ident, Type)> = vec![];
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
                if written.contains(&name.to_string()) {
                    continue;
                }
                let ty = match spec.override_for(name) {
                    Some(Override::Type(ty)) => quote!(#ty),
                    Some(Override::Expr(expr)) => {
                        return Err(Error::new_spanned(
                            expr,
                            format!("`{name}` is an associated type; give it a type"),
                        ));
                    }
                    None => {
                        agreed_types.push(name);
                        quote!(<#first_payload as #trait_path>::#name)
                    }
                };
                items.extend(quote! { type #name = #ty; });
            }
            TraitItem::Const(constant) => {
                let name = &constant.ident;
                if written.contains(&name.to_string()) {
                    continue;
                }
                let mut ty = constant.ty.clone();
                substitution.visit_type_mut(&mut ty);
                let value = match spec.const_override(name)? {
                    Some(expr) => quote!(#expr),
                    None => {
                        agreed_consts.push((name, ty.clone()));
                        quote!(<#first_payload as #trait_path>::#name)
                    }
                };
                items.extend(quote! { const #name: #ty = #value; });
            }
            TraitItem::Fn(function) => {
                if written.contains(&function.sig.ident.to_string()) {
                    continue;
                }
                items.extend(expand_method(
                    function,
                    &entries,
                    &subject,
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
        &first_outside,
        generics,
        &self_type,
    );
    let const_check = associated_const_check(
        &agreed_consts,
        &variants,
        trait_path,
        &first_outside,
        generics,
        &self_type,
    );
    Ok(quote! {
        #(#impl_attrs)*
        #derived
        #unsafety impl #impl_generics #trait_path for #self_ty #where_clause {
            #items
            #(#user_items)*
        }
        #agreement_check
        #const_check
        #shape_errors
    })
}

/// How two values of an associated const's type are compared at compile
/// time: with `==` for the primitives that allow it in a `const`, or byte
/// by byte for `&str`.
enum ConstComparison {
    Primitive,
    Str,
}

fn const_comparison(ty: &Type) -> Option<ConstComparison> {
    const PRIMITIVES: [&str; 15] = [
        "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
        "bool", "char", "()",
    ];
    match ty {
        Type::Tuple(tuple) if tuple.elems.is_empty() => Some(ConstComparison::Primitive),
        Type::Reference(reference) if reference.mutability.is_none() => {
            match bare_ident(&reference.elem) {
                Some(ident) if ident == "str" => Some(ConstComparison::Str),
                _ => None,
            }
        }
        _ => bare_ident(ty)
            .filter(|ident| PRIMITIVES.contains(&ident.to_string().as_str()))
            .map(|_| ConstComparison::Primitive),
    }
}

/// Checks at compile time that every variant's associated consts of a
/// comparable type equal the first variant's, with the failure reported on
/// the variant. Const evaluation happens after monomorphisation, so a
/// generic enum gets no check.
fn associated_const_check(
    consts: &[(&Ident, Type)],
    variants: &[&Target],
    trait_path: &Path,
    first_payload: &TokenStream2,
    generics: &syn::Generics,
    self_type: &Type,
) -> TokenStream2 {
    if variants.len() < 2 || !generics.params.is_empty() {
        return TokenStream2::new();
    }
    let mut checks = TokenStream2::new();
    let mut needs_str_eq = false;
    for (name, ty) in consts {
        let Some(comparison) = const_comparison(ty) else {
            continue;
        };
        for variant in variants.iter().skip(1) {
            let payload = variant.target_type_outside(self_type);
            let cfgs = &variant.cfgs;
            let message = format!(
                "`{name}` of variant `{}` differs from the first variant's; set it on the enum with `{name} = value`",
                variant.ident
            );
            let equal = match comparison {
                ConstComparison::Primitive => quote! {
                    <#first_payload as #trait_path>::#name == <#payload as #trait_path>::#name
                },
                ConstComparison::Str => {
                    needs_str_eq = true;
                    quote! {
                        __whichever_str_eq(<#first_payload as #trait_path>::#name, <#payload as #trait_path>::#name)
                    }
                }
            };
            let span = variant.payload.span();
            checks.extend(quote_spanned! { span =>
                #(#cfgs)* assert!(#equal, #message);
            });
        }
    }
    if checks.is_empty() {
        return TokenStream2::new();
    }
    let str_eq = needs_str_eq.then(|| {
        quote! {
            const fn __whichever_str_eq(a: &str, b: &str) -> bool {
                let (a, b) = (a.as_bytes(), b.as_bytes());
                if a.len() != b.len() {
                    return false;
                }
                let mut i = 0;
                while i < a.len() {
                    if a[i] != b[i] {
                        return false;
                    }
                    i += 1;
                }
                true
            }
        }
    });
    quote! {
        const _: () = {
            #str_eq
            #checks
        };
    }
}

/// Checks at compile time that every variant's associated types equal the
/// first variant's, so a disagreement is reported on the variant instead of
/// inside a generated method body.
///
/// The helper carries the enum's own generics, since a nested function
/// could not name them; the outer function passes them on to each call.
fn associated_type_check(
    names: &[&Ident],
    variants: &[&Target],
    trait_path: &Path,
    first_payload: &TokenStream2,
    generics: &syn::Generics,
    self_type: &Type,
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
        let payload = variant.target_type_outside(self_type);
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

/// The receivers a method can be dispatched on.
#[derive(Clone, Copy)]
enum ReceiverKind {
    /// `self`, `&self` and `&mut self`, in the shorthand or the explicit
    /// spelling: the payload is reached by matching on the value.
    Value(Access),
    /// `self: Box<Self>`: the box is moved out of and the payload's own box,
    /// held by a `deref` variant, is handed over.
    Boxed,
    /// `self: Pin<&Self>` and `self: Pin<&mut Self>`: unpinned, matched, and
    /// the payload pinned again, which needs every payload to be `Unpin`.
    Pinned(Access),
}

fn reference_access(reference: &syn::TypeReference) -> Access {
    if reference.mutability.is_some() {
        Access::Exclusive
    } else {
        Access::Shared
    }
}

/// Classifies a receiver by its type; any other type is an error.
fn classify_receiver(receiver: &Receiver) -> Result<ReceiverKind> {
    let ty = &*receiver.ty;
    if is_self(ty) {
        return Ok(ReceiverKind::Value(Access::Owned));
    }
    if let Type::Reference(reference) = ty {
        if is_self(&reference.elem) {
            return Ok(ReceiverKind::Value(reference_access(reference)));
        }
    }
    if let Some(segment) = last_segment(ty) {
        if let [argument] = generic_type_args(segment).as_slice() {
            if segment.ident == "Box" && is_self(argument) {
                return Ok(ReceiverKind::Boxed);
            }
            if segment.ident == "Pin" {
                if let Type::Reference(reference) = argument {
                    if is_self(&reference.elem) {
                        return Ok(ReceiverKind::Pinned(reference_access(reference)));
                    }
                }
            }
        }
    }
    Err(Error::new_spanned(
        ty,
        "this receiver type is not supported; a receiver is `self`, `&self`, `&mut self`, `self: Box<Self>`, `self: Pin<&Self>` or `self: Pin<&mut Self>`",
    ))
}

/// Returns the expression the generated `match` looks at for a receiver:
/// the value itself, the enum moved out of its box, or the pinned reference
/// unpinned.
fn scrutinee(kind: ReceiverKind, self_token: &Token![self]) -> TokenStream2 {
    match kind {
        ReceiverKind::Value(_) => quote!(#self_token),
        ReceiverKind::Boxed => quote!(*#self_token),
        ReceiverKind::Pinned(Access::Exclusive) => quote!(::core::pin::Pin::get_mut(#self_token)),
        ReceiverKind::Pinned(_) => quote!(::core::pin::Pin::get_ref(#self_token)),
    }
}

/// Returns the expression handing the payload bound to `binding` to the
/// method as its receiver, or the error for a payload the receiver cannot
/// reach.
fn receiver_expression(
    kind: ReceiverKind,
    target: &Target,
    binding: &Ident,
    method: &Ident,
) -> Result<TokenStream2> {
    match kind {
        ReceiverKind::Value(access) => Ok(payload_expression(target.deref, access, binding)),
        ReceiverKind::Boxed => {
            if !target.holds_box() {
                let ident = &target.ident;
                let payload = &target.payload;
                return Err(Error::new_spanned(
                    payload,
                    format!("`{method}` takes `self: Box<Self>`, and `{ident}` holds a plain `{}`; hold `Box<{}>` in a `#[dispatch(deref)]` variant, or implement the method by hand", payload.to_token_stream(), payload.to_token_stream()),
                ));
            }
            Ok(quote!(#binding))
        }
        ReceiverKind::Pinned(access) => {
            let payload = payload_expression(target.deref, access, binding);
            Ok(quote!(::core::pin::Pin::new(#payload)))
        }
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
    entries: &[Entry],
    subject: &Subject,
    trait_path: &Path,
    spec: &TraitSpec,
    substitution: &mut Substitution,
) -> Result<TokenStream2> {
    let subject_ident = subject.ident();
    let prefix = subject_ident.to_token_stream();
    let mut signature: Signature = function.sig.clone();
    substitution.visit_signature_mut(&mut signature);
    let method = signature.ident.clone();
    let Some(FnArg::Receiver(receiver)) = signature.inputs.first() else {
        if function.default.is_some() {
            return Ok(TokenStream2::new());
        }
        let trait_ident = &trait_path
            .segments
            .last()
            .expect("a path has at least one segment")
            .ident;
        return Err(Error::new_spanned(
            &function.sig,
            format!("`{method}` has no receiver, so there is no variant to dispatch on; give it a default body, or write it in a `#[whichever::dispatch] impl {trait_ident} for {subject_ident}` block"),
        ));
    };
    let receiver_kind = classify_receiver(receiver)?;
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
    for entry in entries {
        let target = match entry {
            Entry::Target(target) => target,
            // The variant is reported elsewhere, under its `cfg`; this arm
            // only keeps the `match` exhaustive while that error stands.
            Entry::Unforwardable(unforwardable) => {
                if same_variant.is_empty() {
                    let Unforwardable { ident, cfgs, .. } = unforwardable;
                    arms.extend(
                        quote! { #(#cfgs)* #prefix::#ident { .. } => ::core::unreachable!(), },
                    );
                }
                continue;
            }
        };
        let Target {
            ident: variant_ident,
            payload,
            cfgs,
            deref,
            ..
        } = target;
        if *deref && matches!(return_shape, ReturnShape::RewrapSelf | ReturnShape::MapSelf) {
            return Err(Error::new_spanned(
                payload,
                format!("`{method}` returns `Self`, which cannot be wrapped back into the dereferenced payload of `{variant_ident}`; drop `deref` on it or implement the method by hand"),
            ));
        }
        let target_type = target.target_type();
        let receiver_expression = receiver_expression(receiver_kind, target, &inner, &method)?;
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
            <#target_type as #trait_path>::#method(#receiver_expression #(, #call_arguments)*)
        };
        if is_unsafe {
            call = quote! { unsafe { #call } };
        }
        if is_async {
            call = quote! { #call.await };
        }
        let rebuild_error = || {
            Error::new_spanned(
                payload,
                format!("`{method}` returns `Self`, which cannot be built around the other fields of `{subject_ident}`; write the method in a `#[whichever::dispatch] impl` block"),
            )
        };
        call = match return_shape {
            ReturnShape::Plain => call,
            ReturnShape::RewrapSelf => target.rebuild(&prefix, call).ok_or_else(rebuild_error)?,
            ReturnShape::MapSelf => {
                let constructor = target.constructor(&prefix).ok_or_else(rebuild_error)?;
                quote! { #call.map(#constructor) }
            }
            ReturnShape::IntoOverridden => quote! { ::core::convert::Into::into(#call) },
            ReturnShape::MapErrOverridden => {
                quote! { #call.map_err(::core::convert::Into::into) }
            }
        };
        let pattern = if same_variant.is_empty() {
            target.pattern(&prefix, &inner)
        } else {
            let own = target.pattern(&prefix, &inner);
            let same_patterns = same_variant
                .iter()
                .map(|(index, _)| target.pattern(&prefix, &hygienic(&format!("__same{index}"))));
            quote! { (#own, #(#same_patterns),*) }
        };
        arms.extend(quote! { #(#cfgs)* #pattern => #call, });
    }
    let scrutinee = scrutinee(receiver_kind, &self_token);
    let scrutinee = if same_variant.is_empty() {
        scrutinee
    } else {
        let arguments = same_variant
            .iter()
            .map(|(index, _)| &argument_idents[*index]);
        quote! { (#scrutinee, #(#arguments),*) }
    };
    // Two values of a struct always hold the same field, so only an enum
    // needs the arm for two different variants.
    let (fallback, allow_unreachable) = if same_variant.is_empty() || !subject.is_enum() {
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
    /// Whether `#[dispatch]` rejected the subject itself, whose error is
    /// then already reported.
    rejected: bool,
    subject: Subject,
    /// The options of a `#[dispatch(..)]` on a trait impl block, which make
    /// the block a partial impl of that trait.
    partial: Option<TokenStream2>,
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
        let subject_tokens;
        braced!(subject_tokens in input);
        let subject: Subject = subject_tokens.parse()?;
        let partial = if input.peek(Ident) {
            let tag: Ident = input.parse()?;
            if tag != "partial" {
                return Err(Error::new_spanned(tag, "expected `partial`"));
            }
            let option_tokens;
            braced!(option_tokens in input);
            Some(option_tokens.parse::<TokenStream2>()?)
        } else {
            None
        };
        input.parse::<Token![impl]>()?;
        let impl_tokens;
        braced!(impl_tokens in input);
        let item_impl: ItemImpl = impl_tokens.parse()?;
        Ok(Self {
            rejected,
            subject,
            partial,
            item_impl,
        })
    }
}

/// Rewrites an impl block of a dispatch enum or struct; called by the macro
/// that `#[dispatch]` defines for the item.
#[doc(hidden)]
#[proc_macro]
pub fn implement_inherent(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as InherentInput);
    let expanded = match input.partial {
        Some(options) => expand_partial(input.rejected, input.subject, options, input.item_impl),
        None => expand_inherent(input.rejected, input.subject, input.item_impl),
    };
    expanded
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Hands a partial trait impl block to the trait's carrier macro, together
/// with the subject, so that `implement!` can write the rest of the impl
/// around the items the block holds. When the subject was rejected the
/// block is emitted as written; its one error is the missing items.
fn expand_partial(
    rejected: bool,
    subject: Subject,
    options: TokenStream2,
    item_impl: ItemImpl,
) -> Result<TokenStream2> {
    if rejected {
        return Ok(item_impl.to_token_stream());
    }
    let Some((_, trait_path, _)) = &item_impl.trait_ else {
        return Err(Error::new_spanned(&item_impl, "expected a trait impl"));
    };
    let spec: TraitSpec = syn::parse2(options.clone())?;
    if spec.from.is_some() || spec.try_from.is_some() {
        return Err(Error::new_spanned(
            &options,
            "`from` and `try_from` are options of the enum; put them on its `#[dispatch]` attribute",
        ));
    }
    let mut macro_path = trait_path.clone();
    if let Some(last) = macro_path.segments.last_mut() {
        last.arguments = PathArguments::None;
    }
    let spec = if options.is_empty() {
        quote!(#trait_path)
    } else {
        quote!(#trait_path, #options)
    };
    let facade = facade_ident()?;
    Ok(quote! {
        #macro_path! { @v1 #facade enum { #subject } spec { #spec } impl { #item_impl } }
    })
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
fn expand_inherent(
    rejected: bool,
    subject: Subject,
    mut item_impl: ItemImpl,
) -> Result<TokenStream2> {
    let variants = if rejected {
        None
    } else {
        Some(subject.entries()?)
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
                        &subject,
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
                    forward_method(&attrs, &vis, &sig, None, forwarding, variants, &subject)
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
    entries: &[Entry],
    subject: &Subject,
) -> Result<TokenStream2> {
    let subject_ident = subject.ident();
    let prefix = quote!(Self);
    let method = &sig.ident;
    let Some(FnArg::Receiver(receiver)) = sig.inputs.first() else {
        return Err(Error::new_spanned(
            sig,
            format!("`{method}` has no receiver, so there is no variant to forward to"),
        ));
    };
    let receiver_kind = classify_receiver(receiver)?;
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
    let variants = targets(entries);
    // Under `*`, a variant that holds no payload cannot be forwarded to and
    // the method has no body to fall back on, so the arm for it is an
    // error, which fires only when the variant is compiled in.
    let mut unforwardable_arms = TokenStream2::new();
    let selected: Vec<&Target> = if forwarding.all {
        for entry in entries {
            if let Entry::Unforwardable(Unforwardable { ident, cfgs, .. }) = entry {
                let message = format!("`{method}` is forwarded to every variant, but `{ident}` holds no payload to forward to; name the variants that do and give the method a body for the rest");
                unforwardable_arms.extend(quote_spanned! { method.span() =>
                    #(#cfgs)* #prefix::#ident { .. } => ::core::compile_error!(#message),
                });
            }
        }
        variants.clone()
    } else {
        forwarding
            .variants
            .iter()
            .map(|name| {
                if let Some(target) = variants.iter().find(|target| &target.ident == name) {
                    return Ok(*target);
                }
                let noun = if subject.is_enum() { "variant" } else { "field" };
                let message = match entries.iter().find(|entry| {
                    matches!(entry, Entry::Unforwardable(unforwardable) if &unforwardable.ident == name)
                }) {
                    Some(_) => format!("`{name}` holds no payload to forward to"),
                    None => format!("`{subject_ident}` has no {noun} `{name}`"),
                };
                Err(Error::new(name.span(), message))
            })
            .collect::<Result<_>>()?
    };
    let covers_all = forwarding.all
        || entries.iter().all(|entry| match entry {
            Entry::Target(target) => forwarding.variants.contains(&target.ident),
            Entry::Unforwardable(_) => false,
        });
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
    let mut arms = TokenStream2::new();
    for target in selected {
        let Target { cfgs, payload, .. } = target;
        // Method-call syntax reaches through a `deref` payload by itself;
        // a pinned receiver is pinned again around the payload.
        let callee = match receiver_kind {
            ReceiverKind::Pinned(_) | ReceiverKind::Boxed => {
                receiver_expression(receiver_kind, target, &inner, method)?
            }
            ReceiverKind::Value(_) => quote!(#inner),
        };
        let mut call = quote! { #callee.#call_name(#(#arguments),*) };
        if awaited {
            call = quote! { #call.await };
        }
        // A payload's `Self` comes back as the variant. Without `with`, the
        // payload's own `Option` or `Result` is mapped; with it, the payload
        // returns the plain value and `with` lifts the variant into the
        // declared `Option` or `Result`.
        let rebuild_error = || {
            Error::new_spanned(
                payload,
                format!("`{method}` returns `Self`, which cannot be built around the other fields of `{subject_ident}`; give it a body instead"),
            )
        };
        call = match (&return_shape, &forwarding.with) {
            (ReturnShape::MapSelf, None) => {
                let constructor = target.constructor(&prefix).ok_or_else(rebuild_error)?;
                quote! { #call.map(#constructor) }
            }
            (ReturnShape::RewrapSelf | ReturnShape::MapSelf, _) => {
                target.rebuild(&prefix, call).ok_or_else(rebuild_error)?
            }
            _ => call,
        };
        if let Some(with) = &forwarding.with {
            call = quote! { #with(#call) };
        }
        let pattern = target.pattern(&prefix, &inner);
        arms.extend(quote! { #(#cfgs)* #pattern => #call, });
    }
    let fallback = body.map(|body| quote! { _ => #body, });
    let scrutinee = scrutinee(receiver_kind, &self_token);
    let body = method_body(
        quote! { match #scrutinee { #arms #unforwardable_arms #fallback } },
        returns_future,
        method.span(),
    );
    Ok(quote! { #(#attrs)* #vis #sig #body })
}
