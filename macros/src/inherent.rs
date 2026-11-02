//! `implement_block!`: inherent blocks forwarded method by method, and
//! partial trait impl blocks handed on to the trait's carrier.
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{
    Attribute, Block, Error, ImplItem, ItemImpl, PathArguments, Result, Signature, Token,
    Visibility,
};

use crate::dispatch::facade_ident;
use crate::forwarding::{
    ArgumentNames, Arm, Arms, Callee, Forwarding, Hints, MethodPlan, SelfArguments,
};
use crate::options::{is_dispatch_attribute, ForwardingOptions, TraitSpec};
use crate::protocol::TraitRequest;
use crate::subject::{targets, Entry, Subject, Target};

/// Hands a partial trait impl block to the trait's carrier macro, together
/// with the subject, so that `implement!` can write the rest of the impl
/// around the items the block holds. When the subject was rejected the
/// block is emitted as written; its one error is the missing items.
pub(crate) fn expand_partial(
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
    let mut spec: TraitSpec = syn::parse2(options.clone())?;
    if spec.has_switches() {
        return Err(Error::new_spanned(
            &options,
            "`from` and `try_from` are options of the enum; put them on its `#[dispatch]` attribute",
        ));
    }
    if let Some(path) = &spec.path {
        return Err(Error::new_spanned(
            path,
            "`#[dispatch]` on an impl block takes the trait's options only; the trait is the one the block implements",
        ));
    }
    spec.path = Some(trait_path.clone());
    let mut macro_path = trait_path.clone();
    if let Some(last) = macro_path.segments.last_mut() {
        last.arguments = PathArguments::None;
    }
    let spec = spec.to_token_stream();
    let facade = facade_ident()?;
    let request = TraitRequest {
        facade: &facade,
        subject: &subject,
        spec: &spec,
        partial: Some(&item_impl),
    };
    Ok(quote! { #macro_path! { #request } })
}

/// A method declared without a body, which `syn` leaves as verbatim tokens.
pub(crate) struct BodylessMethod {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) vis: Visibility,
    pub(crate) sig: Signature,
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
pub(crate) fn expand_inherent(
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
    let prefix = quote!(Self);
    let method = &sig.ident;
    if sig.receiver().is_none() {
        return Err(Error::new_spanned(
            sig,
            format!("`{method}` has no receiver, so there is no variant to forward to"),
        ));
    }
    let mut signature = sig.clone();
    let plan = MethodPlan::new(
        &mut signature,
        ArgumentNames::Written,
        SelfArguments::PassThrough,
        None,
    )?;
    let variants = targets(entries);
    // Under `*`, a variant that holds no payload cannot be forwarded to and
    // the method has no body to fall back on, so the arm for it is an
    // error, which fires only when the variant is compiled in.
    let mut unforwardable_arms = vec![];
    let selected: Vec<&Target> = if forwarding.all {
        for entry in entries {
            if let Entry::Unforwardable(unforwardable) = entry {
                let ident = &unforwardable.ident;
                let message = format!("`{method}` is forwarded to every variant, but `{ident}` holds no payload to forward to; name the variants that do and give the method a body for the rest");
                unforwardable_arms.push(Arm::unforwardable(
                    &prefix,
                    unforwardable,
                    quote_spanned! { method.span() => ::core::compile_error!(#message) },
                ));
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
                let noun = subject.target_noun();
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
    let shared = Arms {
        callee: Callee::Method {
            name: forwarding.call.as_ref().unwrap_or(method),
        },
        method,
        subject: subject_ident,
        prefix,
        lift: forwarding.with.as_ref(),
        hints: Hints {
            deref: "give it a body",
            rebuild: "give it a body instead",
        },
    };
    let mut arms = selected
        .iter()
        .map(|target| plan.arm(target, &shared))
        .collect::<Result<Vec<_>>>()?;
    arms.extend(unforwardable_arms);
    let body = Forwarding {
        scrutinee: plan.scrutinee(),
        arms,
        fallback: body.map(ToTokens::to_token_stream),
        future: plan.future,
    }
    .body(method.span());
    Ok(quote! { #(#attrs)* #vis #signature #body })
}
