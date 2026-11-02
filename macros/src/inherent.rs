//! `implement_inherent!`: inherent blocks forwarded method by method, and
//! partial trait impl blocks handed on to the trait's carrier.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{
    Attribute, Block, Error, FnArg, Ident, ImplItem, ItemImpl, Pat, Path, PathArguments, Result,
    Signature, Token, Visibility,
};

use crate::dispatch::facade_ident;
use crate::forwarding::method_body;
use crate::options::{is_dispatch_attribute, TraitSpec};
use crate::shape::{
    classify_receiver, classify_return, receiver_expression, scrutinee, ReceiverKind, ReturnShape,
};
use crate::subject::{targets, Entry, Subject, Target, Unforwardable};

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
pub(crate) struct Forwarding {
    pub(crate) all: bool,
    pub(crate) variants: Vec<Ident>,
    pub(crate) call: Option<Ident>,
    pub(crate) with: Option<Path>,
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
