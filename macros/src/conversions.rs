//! The `From<Payload>` and `TryFrom<Enum>` impls.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::visit_mut::VisitMut;
use syn::{parse_quote, Error, GenericParam, Ident, Path, Result, Type};

use crate::shape::bare_ident;
use crate::subject::{targets, Conversions, Entry, SelfReplacer, Subject, Target};

/// Writes the `From<Payload>` and `TryFrom<Enum>` impls that were asked for.
pub(crate) fn conversion_impls(
    subject: &Subject,
    entries: &[Entry],
    from: bool,
    try_from: bool,
) -> Result<TokenStream2> {
    let (impl_generics, ty_generics, where_clause) = subject.generics().split_for_impl();
    let enum_ident = subject.ident();
    let prefix = Path::from(enum_ident.clone());
    let self_path: Path = parse_quote!(Self);
    let self_type: Type = parse_quote!(#enum_ident #ty_generics);
    let converting: Vec<(&Target, Conversions)> = targets(entries)
        .into_iter()
        .filter_map(|target| Some((target, target.conversions()?)))
        .filter(|(_, conversions)| conversions.from)
        .collect();
    // A payload that is a type parameter of the enum can be any type, the
    // other payloads included, so its `From` overlaps theirs; rustc would
    // only report conflicting implementations on the attribute.
    if from && converting.len() > 1 {
        for (target, _) in &converting {
            if let Some(ident) = type_parameter(subject, target) {
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
    for (target, _) in &converting {
        *payload_counts
            .entry(target.payload.to_token_stream().to_string())
            .or_default() += 1;
    }
    let value = Ident::new("__value", Span::mixed_site());
    let other = Ident::new("__other", Span::mixed_site());
    let mut output = TokenStream2::new();
    for (target, conversions) in converting {
        let Target { payload, cfgs, .. } = target;
        if payload_counts[&payload.to_token_stream().to_string()] > 1 {
            continue;
        }
        let built = target.rebuild(&self_path, quote!(#value));
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
        if conversions.try_from.unwrap_or(try_from) {
            // `TryFrom<Enum<T>> for T` implements a foreign trait for a type
            // parameter, which the orphan rules forbid; rustc would report
            // that on the enum's generics.
            if let Some(ident) = type_parameter(subject, target) {
                return Err(Error::new_spanned(
                    payload,
                    format!("`{ident}` is a type parameter of `{enum_ident}`, so `TryFrom<{enum_ident}> for {ident}` would implement a foreign trait for a type parameter, which the orphan rules forbid; add `#[dispatch(try_from = false)]` on this variant"),
                ));
            }
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

/// Returns the payload of `target` when it is a bare type parameter of the
/// subject.
fn type_parameter<'t>(subject: &Subject, target: &'t Target) -> Option<&'t Ident> {
    bare_ident(&target.payload).filter(|ident| {
        subject
            .generics()
            .params
            .iter()
            .any(|param| matches!(param, GenericParam::Type(param) if &param.ident == *ident))
    })
}
