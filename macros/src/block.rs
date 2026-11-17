//! `implement_block!`: inherent blocks forwarded method by method, and
//! partial trait impl blocks handed on to the trait's carrier.
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{Attribute, Block, Error, ImplItem, Result, Signature, Visibility};

use crate::forwarding::{forward, Forward, Generated, Mark, Origin};
use crate::marked::BlockItem;
use crate::options::{trait_key, ForwardingOptions, TraitOptions};
use crate::protocol::{carrier_path, BlockInput, TraitRequest};
use crate::site::Site;
use crate::spelling;
use crate::subject::{cfg_attributes, Entries, Subject};

/// Hands a partial trait impl block to the trait's carrier macro, together
/// with the subject, so that `implement!` can write the rest of the impl
/// around the items the block holds. When the subject was rejected the
/// block is emitted as written; its one error is the missing items. A block
/// of a trait the subject's own attribute dispatches would be a second impl,
/// which rustc reports as conflicting on the trait's attribute; it is
/// reported here instead, and the attribute's impl stands.
pub(crate) fn expand_partial(input: BlockInput, options: TraitOptions) -> Result<TokenStream2> {
    let BlockInput {
        rejected,
        facade,
        dispatched,
        remote,
        subject,
        item_impl,
        ..
    } = input;
    if rejected {
        return Ok(item_impl.to_token_stream());
    }
    let Some((trait_path, _)) = &item_impl.trait_ else {
        return Err(Error::new_spanned(&item_impl, "expected a trait impl"));
    };
    let trait_name = trait_key(trait_path);
    if dispatched.iter().any(|path| trait_key(path) == trait_name) {
        let subject = subject.ident();
        return Err(Error::new_spanned(
            trait_path,
            format!("`{subject}` already dispatches `{trait_name}` from its own attribute; this block is the impl, so drop `{trait_name}` there"),
        ));
    }
    let macro_path = carrier_path(trait_path);
    let request = TraitRequest {
        facade: &facade,
        subject: &subject,
        path: trait_path,
        options: &options,
        remote: remote.as_ref(),
        partial: Some(&item_impl),
    };
    // The invocation carries the block's trait path, so that an error rustc
    // finds in the impl is labelled on the block.
    let site = trait_path.span();
    Ok(quote_spanned! { site => #macro_path! { #request } })
}

/// Rewrites every marked method of the block. A method that cannot be
/// forwarded is reported and kept as written, without the attribute, or
/// dropped when it has no body, so that the rest of the block still
/// compiles and the real error stands alone. The sentence stands under the
/// method's own `#[cfg]`, as the method does, so a build that compiles the
/// method out has neither. When the enum itself was rejected nothing is
/// reported here.
pub(crate) fn expand_inherent(input: BlockInput) -> Result<TokenStream2> {
    let BlockInput {
        rejected,
        remote,
        subject,
        mut item_impl,
        ..
    } = input;
    // An inherent impl of another crate's type is not this crate's to
    // write, with or without forwarding.
    if let Some(remote) = &remote {
        let spelled = spelling::spelled(remote);
        return Err(Error::new_spanned(
            &item_impl.self_ty,
            format!("`{spelled}` is another crate's enum, which gets no inherent impl here; forward through a trait of this crate, or write free functions"),
        ));
    }
    let variants = if rejected {
        None
    } else {
        Some(subject.read()?)
    };
    let mut items = Vec::with_capacity(item_impl.items.len());
    let site = Site::at(item_impl.impl_token.span);
    let mut sentences = TokenStream2::new();
    let mut report = |cfgs: &[Attribute], error: Error| {
        let sentence = error.to_compile_error();
        sentences.extend(quote_spanned! { *site => #(#cfgs)* #sentence });
    };
    for item in std::mem::take(&mut item_impl.items) {
        let mut method = match BlockItem::read(item) {
            BlockItem::Marked(method) => method,
            BlockItem::Bodyless { attrs, sig } => {
                report(
                    &cfg_attributes(&attrs),
                    Error::new_spanned(
                        &sig,
                        "a method without a body needs `#[dispatch(*)]` to be forwarded to every variant",
                    ),
                );
                continue;
            }
            BlockItem::Unread(tokens) => {
                report(
                    &[],
                    Error::new_spanned(
                        &tokens,
                        "only methods, with or without a body, are forwarded in a `#[dispatch]` impl block",
                    ),
                );
                continue;
            }
            BlockItem::Plain(item) => {
                items.push(item);
                continue;
            }
        };
        // The subject's own error stands alone; the method stays as it is
        // written, without its mark.
        let Some(variants) = &variants else {
            items.extend(method.unmarked());
            continue;
        };
        let cfgs = cfg_attributes(&method.attrs);
        let forwarded = method.mark(ForwardingOptions::read).and_then(|forwarding| {
            forward_method(
                &method.attrs,
                &method.vis,
                &method.sig,
                method.body.as_ref(),
                forwarding,
                variants,
                &subject,
            )
        });
        match forwarded {
            Ok(generated) => items.push(ImplItem::Verbatim(generated)),
            Err(error) => {
                report(&cfgs, error);
                items.extend(method.unmarked());
            }
        }
    }
    item_impl.items = items;
    Ok(quote_spanned! { *site => #item_impl #sentences })
}

/// Writes one forwarded inherent method: a `match` with one arm per selected
/// variant calling the payload's own method, and the body as the arm for
/// every other variant.
pub(crate) fn forward_method(
    attrs: &[Attribute],
    vis: &Visibility,
    sig: &Signature,
    body: Option<&Block>,
    forwarding: ForwardingOptions,
    entries: &Entries,
    subject: &Subject,
) -> Result<TokenStream2> {
    let method = &sig.ident;
    let site = Site::at(method.span());
    if sig.receiver().is_none() {
        let noun = subject.target_noun();
        return Err(Error::new_spanned(
            sig,
            format!("`{method}` has no receiver, so there is no {noun} to forward to"),
        ));
    }
    let Generated {
        signature,
        allowances,
        body,
    } = forward(Forward {
        signature: sig.clone(),
        origin: Origin::Inherent {
            mark: Mark {
                options: &forwarding,
                body,
            },
        },
        entries,
        subject,
        site,
        body_site: site,
    })?;
    Ok(quote_spanned! { *site => #(#attrs)* #allowances #vis #signature #body })
}
