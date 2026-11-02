//! Writing the forwarding method for one trait method.
use proc_macro2::{Delimiter, Group, Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::visit_mut::VisitMut;
use syn::{Error, FnArg, Ident, Pat, PatIdent, Path, Result, Signature, TraitItemFn};

use crate::options::{method_mismatch, TraitSpec};
use crate::shape::{
    argument_kind, classify_receiver, classify_return, payload_expression, receiver_expression,
    scrutinee, Access, ArgumentKind, ReturnShape,
};
use crate::subject::{cfg_attributes, Entry, Subject, Target, Unforwardable};
use crate::substitution::Substitution;

pub(crate) fn expand_method(
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
pub(crate) fn method_body(body: TokenStream2, returns_future: bool, span: Span) -> TokenStream2 {
    let body = if returns_future {
        quote! { async move { #body } }
    } else {
        body
    };
    let mut group = Group::new(Delimiter::Brace, body);
    group.set_span(span);
    TokenTree::Group(group).into()
}
