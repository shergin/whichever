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
//! what the carrier puts in front  @v1 [rejected] facade { <facade> } [traits { <paths> }] enum { <subject> }
//! what `implement_block!` reads   @v1 [rejected] facade { <facade> } [traits { <paths> }] enum { <subject> } [partial { <options> }] impl { <block> }
//! ```
//!
//! A carrier macro takes `@v1 <facade>` off its request and calls the
//! facade's implementation macro with its own prefix in front of the rest.
//! A trait's carrier is compiled into the trait's crate and read by whatever
//! version of this crate the subject's crate builds with, so the first group
//! is a compatibility surface. Between two keywords the message is a run of
//! segments, `tag` or `tag { .. }`; a segment is only ever added, and a
//! reader skips a tag it does not know, so an older reader accepts a newer
//! carrier. `v1` becomes `v2` when a segment changes meaning or order. A
//! subject's carrier never leaves its crate.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{braced, token, Error, Ident, ItemImpl, ItemTrait, Path, PathArguments, Result, Token};

use crate::options::TraitSpec;
use crate::subject::Subject;

/// The version every message starts with.
const VERSION: &str = "v1";

fn version() -> Ident {
    Ident::new(VERSION, Span::call_site())
}

/// The implementation macro a carrier calls, under `__private` of the
/// facade.
#[derive(Clone, Copy)]
pub(crate) enum Implementation {
    /// `implement!`, which writes a trait's forwarding impl.
    Trait,
    /// `implement_block!`, which rewrites an impl block.
    Block,
}

impl Implementation {
    fn ident(self) -> Ident {
        let name = match self {
            Implementation::Trait => "implement",
            Implementation::Block => "implement_block",
        };
        Ident::new(name, Span::call_site())
    }
}

/// Writes a carrier macro named `name`: one rule that takes `@v1` and the
/// facade's name off a request and calls the facade's `implementation`
/// macro with `carried` in front of the rest.
pub(crate) fn carrier_macro(
    name: &Ident,
    attributes: TokenStream2,
    implementation: Implementation,
    carried: impl ToTokens,
) -> TokenStream2 {
    let version = version();
    let implementation = implementation.ident();
    quote! {
        #attributes
        macro_rules! #name {
            (@#version $facade:ident $($rest:tt)*) => {
                ::$facade::__private::#implementation! { #carried $($rest)* }
            };
        }
    }
}

/// Returns the path a carrier is invoked by: the trait's or the subject's
/// path, without the generic arguments of its last segment.
pub(crate) fn carrier_path(path: &Path) -> Path {
    let mut path = path.clone();
    if let Some(last) = path.segments.last_mut() {
        last.arguments = PathArguments::None;
    }
    path
}

/// What `#[dispatch]` sends to a trait's carrier macro: the subject, the
/// trait's options, and the partial impl block when the request comes from
/// one.
pub(crate) struct TraitRequest<'a> {
    pub(crate) facade: &'a Ident,
    pub(crate) subject: &'a Subject,
    pub(crate) spec: &'a TraitSpec,
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
/// tokens, and the real path of a restated trait. Both stay tokens, since
/// `$crate` in them resolves only inside the carrier.
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
        let mut remote = None;
        segments(
            input,
            |input| input.peek(Token![enum]),
            |tag, content| {
                if let ("remote", Some(tokens)) = (tag.to_string().as_str(), content) {
                    remote = Some(syn::parse2(tokens)?);
                }
                Ok(())
            },
        )?;
        input.parse::<Token![enum]>()?;
        let subject = parse_braced(input)?;
        let mut spec = None;
        segments(
            input,
            |input| input.peek(Token![impl]),
            |tag, content| {
                if let ("spec", Some(tokens)) = (tag.to_string().as_str(), content) {
                    spec = Some(syn::parse2(tokens)?);
                }
                Ok(())
            },
        )?;
        let spec = spec.ok_or_else(|| missing("spec"))?;
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
/// the block, and the trait's options when the block is a partial impl.
pub(crate) struct BlockRequest<'a> {
    pub(crate) facade: &'a Ident,
    pub(crate) partial: Option<&'a TraitSpec>,
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
        let partial = partial.map(|spec| quote!(partial { #spec }));
        tokens.extend(quote! { @#version #facade #partial impl { #block } });
    }
}

/// What a subject's carrier macro puts in front of a request: whether
/// `#[dispatch]` rejected the subject, in which case its error is already
/// reported, the facade the request named, the traits the subject's own
/// attributes dispatch, and the subject as captured, variant and field
/// options included.
pub(crate) struct CarriedSubject<'a> {
    pub(crate) rejected: bool,
    pub(crate) dispatched: &'a [&'a Path],
    pub(crate) subject: &'a Subject,
}

impl ToTokens for CarriedSubject<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let CarriedSubject {
            rejected,
            dispatched,
            subject,
        } = self;
        let version = version();
        let rejected = rejected.then(|| quote!(rejected));
        let dispatched = (!dispatched.is_empty()).then(|| quote!(traits { #(#dispatched),* }));
        // `$facade` is the carrier rule's own variable, bound to the facade
        // the request named.
        tokens.extend(
            quote! { @#version #rejected facade { $facade } #dispatched enum { #subject } },
        );
    }
}

/// What `implement_block!` reads: a carried subject followed by the request.
pub(crate) struct BlockInput {
    /// Whether `#[dispatch]` rejected the subject itself, whose error is
    /// then already reported.
    pub(crate) rejected: bool,
    /// The name the facade crate has in the subject's crate.
    pub(crate) facade: Ident,
    /// The traits the subject's own attributes dispatch, as written there.
    pub(crate) dispatched: Vec<Path>,
    pub(crate) subject: Subject,
    /// The trait's options when the block is a partial impl of that trait.
    pub(crate) partial: Option<TraitSpec>,
    pub(crate) item_impl: ItemImpl,
}

impl Parse for BlockInput {
    fn parse(input: ParseStream) -> Result<Self> {
        parse_version(input)?;
        let mut rejected = false;
        let mut facade = None;
        let mut dispatched = vec![];
        segments(
            input,
            |input| input.peek(Token![enum]),
            |tag, content| {
                match (tag.to_string().as_str(), content) {
                    ("rejected", None) => rejected = true,
                    ("facade", Some(tokens)) => facade = Some(syn::parse2(tokens)?),
                    ("traits", Some(tokens)) => {
                        let paths = Punctuated::<Path, Token![,]>::parse_terminated;
                        dispatched = paths.parse2(tokens)?.into_iter().collect();
                    }
                    _ => {}
                }
                Ok(())
            },
        )?;
        let facade = facade.ok_or_else(|| missing("facade"))?;
        input.parse::<Token![enum]>()?;
        let subject = parse_braced(input)?;
        let mut partial = None;
        segments(
            input,
            |input| input.peek(Token![impl]),
            |tag, content| {
                if let ("partial", Some(tokens)) = (tag.to_string().as_str(), content) {
                    partial = Some(syn::parse2(tokens)?);
                }
                Ok(())
            },
        )?;
        input.parse::<Token![impl]>()?;
        let item_impl = parse_braced(input)?;
        Ok(Self {
            rejected,
            facade,
            dispatched,
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

/// Reads the segments up to the keyword `stop` recognises, each a tag with
/// or without a braced body, handing every one to `on`, which skips the
/// tags it does not know.
fn segments(
    input: ParseStream,
    stop: impl Fn(ParseStream) -> bool,
    mut on: impl FnMut(&Ident, Option<TokenStream2>) -> Result<()>,
) -> Result<()> {
    while !input.is_empty() && !stop(input) {
        let tag: Ident = input.parse()?;
        let content = if input.peek(token::Brace) {
            Some(parse_braced::<TokenStream2>(input)?)
        } else {
            None
        };
        on(&tag, content)?;
    }
    Ok(())
}

fn missing(segment: &str) -> Error {
    Error::new(
        Span::call_site(),
        format!("the request has no `{segment}` segment"),
    )
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::*;

    fn subject() -> Subject {
        Subject::Enum(parse_quote!(
            enum Any {
                A(A),
            }
        ))
    }

    /// The tokens a carrier sees: a request without its `@v1 <facade>`.
    fn rest_of(request: impl ToTokens) -> TokenStream2 {
        request.to_token_stream().into_iter().skip(3).collect()
    }

    #[test]
    fn a_trait_request_reads_back_behind_its_carried_trait() {
        let facade: Ident = parse_quote!(whichever);
        let subject = subject();
        let spec: TraitSpec = syn::parse_str("Trait, Error = X").unwrap();
        let block: ItemImpl = parse_quote!(
            impl Trait for Any {
                fn make() -> Self {
                    todo!()
                }
            }
        );
        let request = TraitRequest {
            facade: &facade,
            subject: &subject,
            spec: &spec,
            partial: Some(&block),
        };
        let trait_tokens = quote!(
            pub trait Trait {
                type Error;
                fn make() -> Self;
            }
        );
        let remote = quote!(::other::Trait);
        let carried = CarriedTrait {
            tokens: &trait_tokens,
            remote: Some(&remote),
        };
        let rest = rest_of(request);
        let input: ImplementInput = syn::parse2(quote!(#carried #rest)).unwrap();
        assert_eq!(input.item_trait.ident, "Trait");
        assert_eq!(
            input.remote.map(|path| path.to_token_stream().to_string()),
            Some(remote.to_string())
        );
        assert_eq!(input.subject.ident(), "Any");
        assert!(input.spec.is_overridden(&parse_quote!(Error)));
        assert_eq!(input.partial.map(|block| block.items.len()), Some(1));
    }

    #[test]
    fn a_block_request_reads_back_behind_its_carried_subject() {
        let facade: Ident = parse_quote!(wh);
        let block: ItemImpl = parse_quote!(
            impl Any {
                fn plain(&self) {}
            }
        );
        let spec: TraitSpec = syn::parse_str("Error = X").unwrap();
        let request = BlockRequest {
            facade: &facade,
            partial: Some(&spec),
            block: &block,
        };
        let rest = rest_of(request);
        let subject = subject();
        // What `CarriedSubject` writes, with the carrier's `$facade` filled in.
        let message = quote!(@v1 rejected facade { wh } traits { Shape, a::Encoder<X> } enum { #subject } #rest);
        let input: BlockInput = syn::parse2(message).unwrap();
        assert!(input.rejected);
        assert_eq!(input.facade, "wh");
        assert_eq!(input.dispatched.len(), 2);
        assert!(input
            .partial
            .is_some_and(|spec| spec.is_overridden(&parse_quote!(Error))));
        assert_eq!(input.item_impl.items.len(), 1);
    }

    #[test]
    fn unknown_segments_are_skipped_and_a_wrong_version_is_not() {
        let subject = subject();
        let newer = quote! {
            @v1 trait { trait T { fn f(&self); } } later { 1 } enum { #subject } spec { T } someday
        };
        assert!(syn::parse2::<ImplementInput>(newer).is_ok());
        let other_version = quote!(@v2 trait { trait T {} } enum { #subject } spec { T });
        assert!(syn::parse2::<ImplementInput>(other_version).is_err());
        let without_spec = quote!(@v1 trait { trait T {} } enum { #subject });
        assert!(syn::parse2::<ImplementInput>(without_spec).is_err());
    }
}
