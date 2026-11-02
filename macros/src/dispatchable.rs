//! `#[dispatchable]`: the carrier macro a trait leaves next to itself.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use proc_macro2::{Group, Punct, Spacing, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, ItemTrait, Path, Result, Token, TraitItem, Visibility};

use crate::options::is_dispatch_attribute;

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

pub(crate) fn expand_dispatchable(
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
pub(crate) fn hash_token_positions(tokens: TokenStream2, hasher: &mut DefaultHasher) {
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
pub(crate) fn file_key(path: &str) -> String {
    let mut components = path.rsplit(['/', '\\']);
    let file = components.next().unwrap_or_default();
    match components.next() {
        Some(directory) => format!("{directory}/{file}"),
        None => file.to_owned(),
    }
}

/// Replaces every `crate::` path prefix in `tokens` with `$crate::`.
pub(crate) fn rewrite_crate_paths(tokens: TokenStream2) -> TokenStream2 {
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
