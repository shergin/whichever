//! `implement_block!`: inherent blocks forwarded method by method, and
//! partial trait impl blocks handed on to the trait's carrier.
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{
    parse_quote_spanned, Attribute, Block, Error, Ident, ImplItem, ItemImpl, Path, Result,
    ReturnType, Signature, Type, Visibility,
};

use crate::at;
use crate::forwarding::{
    ArgumentNames, Arm, ArmContext, Callee, Forwarding, MethodPlan, SelfArguments,
};
use crate::options::{is_dispatch_attribute, trait_key, ForwardingOptions, TraitSpec};
use crate::protocol::{carrier_path, TraitRequest};
use crate::selection::{check_body, one_attribute, select_targets, BodylessMethod};
use crate::shape::ReturnSite;
use crate::subject::{Entry, Subject};

/// Hands a partial trait impl block to the trait's carrier macro, together
/// with the subject, so that `implement!` can write the rest of the impl
/// around the items the block holds. When the subject was rejected the
/// block is emitted as written; its one error is the missing items. A block
/// of a trait the subject's own attribute dispatches would be a second impl,
/// which rustc reports as conflicting on the trait's attribute; it is
/// reported here instead, and the attribute's impl stands.
pub(crate) fn expand_partial(
    rejected: bool,
    facade: &Ident,
    dispatched: &[Path],
    remote: Option<&Path>,
    subject: Subject,
    mut spec: TraitSpec,
    item_impl: ItemImpl,
) -> Result<TokenStream2> {
    if rejected {
        return Ok(item_impl.to_token_stream());
    }
    let Some((_, trait_path, _)) = &item_impl.trait_ else {
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
    spec.path = Some(trait_path.clone());
    let macro_path = carrier_path(trait_path);
    let request = TraitRequest {
        facade,
        subject: &subject,
        spec: &spec,
        remote,
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
/// compiles and the real error stands alone. When the enum itself was
/// rejected nothing is reported here.
pub(crate) fn expand_inherent(
    rejected: bool,
    remote: Option<&Path>,
    subject: Subject,
    mut item_impl: ItemImpl,
) -> Result<TokenStream2> {
    // An inherent impl of another crate's type is not this crate's to
    // write, with or without forwarding.
    if let Some(remote) = remote {
        let spelled = remote.to_token_stream().to_string().replace(' ', "");
        return Err(Error::new_spanned(
            &item_impl.self_ty,
            format!("`{spelled}` is another crate's enum, which gets no inherent impl here; forward through a trait of this crate, or write free functions"),
        ));
    }
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
                let forwarded =
                    one_attribute(&mut function.attrs, &attribute).and_then(|forwarding| {
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
                let forwarded = one_attribute(&mut attrs, &attribute).and_then(|forwarding| {
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
    let site = at(item_impl.impl_token.span);
    Ok(quote_spanned! { site => #item_impl #(#errors)* })
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
    entries: &[Entry],
    subject: &Subject,
) -> Result<TokenStream2> {
    let subject_ident = subject.ident();
    let noun = subject.target_noun();
    let method = &sig.ident;
    let site = at(method.span());
    let prefix: Path = parse_quote_spanned!(site => Self);
    if sig.receiver().is_none() {
        return Err(Error::new_spanned(
            sig,
            format!("`{method}` has no receiver, so there is no {noun} to forward to"),
        ));
    }
    // `field` and `value` make the arm a place instead of a call, borrowed
    // as the declared return type asks: `&inner.f` for `-> &T`, `inner.f`
    // for a value.
    let borrow = match &sig.output {
        ReturnType::Type(_, ty) => match &**ty {
            Type::Reference(reference) => Some(reference.mutability),
            _ => None,
        },
        ReturnType::Default => None,
    };
    let callee = if forwarding.value || forwarding.field.is_some() {
        Callee::Place {
            field: forwarding.field.as_ref(),
            borrow,
        }
    } else {
        Callee::Method {
            name: forwarding.call.as_ref().unwrap_or(method),
        }
    };
    let context = ArmContext {
        callee,
        method,
        subject: subject_ident,
        prefix,
        lift: forwarding.with.as_ref(),
        site,
    };
    // In an inherent method `Self` is the subject. For an enum the argument
    // is handed over as it is: a payload method taking the enum is a
    // legitimate target, and matching it on the same variant would need a
    // mismatch the method cannot declare. Two values of a struct always
    // hold the same field, so there the argument is unwrapped to the field,
    // as trait dispatch unwraps it.
    let self_arguments = if subject.has_one_target() {
        SelfArguments::Dispatch
    } else {
        SelfArguments::PassThrough
    };
    let mut signature = sig.clone();
    let plan = MethodPlan::new(
        &mut signature,
        ArgumentNames::Written,
        self_arguments,
        None,
        &ReturnSite {
            subject: subject_ident,
            one_target: subject.has_one_target(),
            by_hand: context.callee.by_hand(),
            default: None,
        },
        None,
    )?;
    let selection = select_targets(&forwarding, entries, &context.prefix, method, subject)?;
    check_body(&selection, body, sig, noun)?;
    let mut arms = selection
        .targets
        .iter()
        .map(|target| plan.arm(target, &context))
        .collect::<Result<Vec<_>>>()?;
    arms.extend(selection.unforwardable_arms);
    let body = Forwarding {
        scrutinee: plan.scrutinee(context.site),
        arms,
        fallback: body.map(|body| Arm::rest(body.to_token_stream(), context.site)),
        future: plan.future.clone(),
    }
    .body(site);
    Ok(quote_spanned! { site => #(#attrs)* #vis #signature #body })
}
