//! The messages the carrier macros hand to the implementation macros.
use proc_macro2::TokenStream as TokenStream2;
use syn::parse::{Parse, ParseStream};
use syn::{braced, Error, Ident, ItemImpl, ItemTrait, Path, Result, Token};

use crate::options::TraitSpec;
use crate::subject::Subject;

pub(crate) struct ImplementInput {
    pub(crate) item_trait: ItemTrait,
    /// The real path of a trait restated with `remote = ..`, which the impl
    /// is written for.
    pub(crate) remote: Option<Path>,
    pub(crate) subject: Subject,
    pub(crate) spec: TraitSpec,
    /// The impl block the user wrote, holding the items the subject
    /// implements itself; the rest is generated around them.
    pub(crate) partial: Option<ItemImpl>,
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

pub(crate) struct InherentInput {
    /// Whether `#[dispatch]` rejected the subject itself, whose error is
    /// then already reported.
    pub(crate) rejected: bool,
    pub(crate) subject: Subject,
    /// The options of a `#[dispatch(..)]` on a trait impl block, which make
    /// the block a partial impl of that trait.
    pub(crate) partial: Option<TokenStream2>,
    pub(crate) item_impl: ItemImpl,
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
