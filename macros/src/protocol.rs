//! The messages between `#[dispatch]`, the carrier macros and the
//! implementation macros: one struct each, with its encoder and its decoder
//! side by side.
//!
//! ```text
//! request to a trait's carrier    @v1 <facade> enum { <subject> } spec { <options> } [impl { <block> }]
//! what the carrier puts in front  @v1 trait { <trait> } [remote { <path> }]
//! what `implement!` reads         @v1 trait { <trait> } [remote { <path> }] enum { <subject> } spec { <options> } [impl { <block> }]
//!
//! request to a subject's carrier  @v1 <facade> [partial { <options> }] impl { <block> }
//! what the carrier puts in front  @v1 [rejected] enum { <subject> }
//! what `implement_block!` reads   @v1 [rejected] enum { <subject> } [partial { <options> }] impl { <block> }
//! ```
//!
//! A carrier macro takes `@v1 <facade>` off its request and calls the
//! facade's implementation macro with its own prefix in front of the rest.
//! A trait's carrier is compiled into the trait's crate and read by whatever
//! version of this crate the subject's crate builds with, so the first group
//! is a compatibility surface: a segment is only ever added, optional, at
//! the end of the part it belongs to, and `v1` becomes `v2` when a segment
//! changes meaning or order. A subject's carrier never leaves its crate.
use std::hash::Hasher;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{braced, Error, Ident, ItemImpl, ItemTrait, Path, Result, Token};

use crate::options::TraitSpec;
use crate::subject::Subject;

/// The version every message starts with.
const VERSION: &str = "v1";

fn version() -> Ident {
    Ident::new(VERSION, Span::call_site())
}

/// FNV-1a over 64 bits, written out so that a carrier macro's name comes
/// out the same on every compiler release; `std`'s `DefaultHasher` is
/// documented as unstable across releases.
pub(crate) struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Writes a carrier macro named `name`: one rule that takes `@v1` and the
/// facade's name off a request and calls the facade's `implementation`
/// macro with `carried` in front of the rest.
pub(crate) fn carrier_macro(
    name: &Ident,
    attributes: TokenStream2,
    implementation: &str,
    carried: impl ToTokens,
) -> TokenStream2 {
    let version = version();
    let implementation = Ident::new(implementation, Span::call_site());
    quote! {
        #attributes
        macro_rules! #name {
            (@#version $facade:ident $($rest:tt)*) => {
                ::$facade::__private::#implementation! { #carried $($rest)* }
            };
        }
    }
}

/// What `#[dispatch]` sends to a trait's carrier macro: the subject, the
/// options written after the trait's path, and the partial impl block when
/// the request comes from one.
pub(crate) struct TraitRequest<'a> {
    pub(crate) facade: &'a Ident,
    pub(crate) subject: &'a Subject,
    pub(crate) spec: &'a TokenStream2,
    pub(crate) partial: Option<&'a ItemImpl>,
}

impl ToTokens for TraitRequest<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let TraitRequest {
            facade,
            subject,
            spec,
            partial,
        } = self;
        let version = version();
        let partial = partial.map(|block| quote!(impl { #block }));
        tokens.extend(quote! { @#version #facade enum { #subject } spec { #spec } #partial });
    }
}

/// What a trait's carrier macro puts in front of a request: the trait's
/// tokens, and the real path of a restated trait.
pub(crate) struct CarriedTrait<'a> {
    pub(crate) tokens: &'a TokenStream2,
    pub(crate) remote: Option<&'a TokenStream2>,
}

impl ToTokens for CarriedTrait<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let CarriedTrait {
            tokens: trait_tokens,
            remote,
        } = self;
        let version = version();
        let remote = remote.map(|path| quote!(remote { #path }));
        tokens.extend(quote! { @#version trait { #trait_tokens } #remote });
    }
}

/// What `implement!` reads: a carried trait followed by the request.
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
        parse_version(input)?;
        input.parse::<Token![trait]>()?;
        let item_trait = parse_braced(input)?;
        let remote = if take_tag(input, "remote") {
            Some(parse_braced(input)?)
        } else {
            None
        };
        input.parse::<Token![enum]>()?;
        let subject = parse_braced(input)?;
        expect_tag(input, "spec")?;
        let spec = parse_braced(input)?;
        let partial = if input.peek(Token![impl]) {
            input.parse::<Token![impl]>()?;
            Some(parse_braced(input)?)
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

/// What `#[dispatch]` on an impl block sends to the subject's carrier macro:
/// the block, and the options of a partial trait impl.
pub(crate) struct BlockRequest<'a> {
    pub(crate) facade: &'a Ident,
    pub(crate) partial: Option<&'a TokenStream2>,
    pub(crate) block: &'a ItemImpl,
}

impl ToTokens for BlockRequest<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let BlockRequest {
            facade,
            partial,
            block,
        } = self;
        let version = version();
        let partial = partial.map(|options| quote!(partial { #options }));
        tokens.extend(quote! { @#version #facade #partial impl { #block } });
    }
}

/// What a subject's carrier macro puts in front of a request: the subject
/// as captured, variant and field options included, and whether
/// `#[dispatch]` rejected it, in which case its error is already reported.
pub(crate) struct CarriedSubject<'a> {
    pub(crate) rejected: bool,
    pub(crate) subject: &'a Subject,
}

impl ToTokens for CarriedSubject<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let CarriedSubject { rejected, subject } = self;
        let version = version();
        let rejected = rejected.then(|| quote!(rejected));
        tokens.extend(quote! { @#version #rejected enum { #subject } });
    }
}

/// What `implement_block!` reads: a carried subject followed by the request.
pub(crate) struct BlockInput {
    /// Whether `#[dispatch]` rejected the subject itself, whose error is
    /// then already reported.
    pub(crate) rejected: bool,
    pub(crate) subject: Subject,
    /// The options of a `#[dispatch(..)]` on a trait impl block, which make
    /// the block a partial impl of that trait.
    pub(crate) partial: Option<TokenStream2>,
    pub(crate) item_impl: ItemImpl,
}

impl Parse for BlockInput {
    fn parse(input: ParseStream) -> Result<Self> {
        parse_version(input)?;
        let rejected = take_tag(input, "rejected");
        input.parse::<Token![enum]>()?;
        let subject = parse_braced(input)?;
        let partial = if take_tag(input, "partial") {
            Some(parse_braced(input)?)
        } else {
            None
        };
        input.parse::<Token![impl]>()?;
        let item_impl = parse_braced(input)?;
        Ok(Self {
            rejected,
            subject,
            partial,
            item_impl,
        })
    }
}

fn parse_version(input: ParseStream) -> Result<()> {
    input.parse::<Token![@]>()?;
    let version: Ident = input.parse()?;
    if version != VERSION {
        return Err(Error::new_spanned(
            version,
            "unsupported `whichever` protocol version",
        ));
    }
    Ok(())
}

/// Parses a braced segment as a `T`.
fn parse_braced<T: Parse>(input: ParseStream) -> Result<T> {
    let content;
    braced!(content in input);
    content.parse()
}

/// Takes the identifier `tag` off the input when it is next, and says so.
fn take_tag(input: ParseStream, tag: &str) -> bool {
    let next = input.fork().parse::<Ident>();
    if next.is_ok_and(|ident| ident == tag) {
        let _: Ident = input.parse().expect("the fork parsed it");
        return true;
    }
    false
}

/// Requires the identifier `tag` next.
fn expect_tag(input: ParseStream, tag: &str) -> Result<()> {
    let ident: Ident = input.parse()?;
    if ident != tag {
        return Err(Error::new_spanned(ident, format!("expected `{tag}`")));
    }
    Ok(())
}
