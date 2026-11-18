//! The `From<Payload>`, `TryFrom<Enum>` and `From<Enum> for Target` impls.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::visit_mut::VisitMut;
use syn::{parse_quote, Error, GenericParam, Ident, Lifetime, Path, Type};

use crate::options::{IntoTarget, TryFromForms};
use crate::shape::bare_ident;
use crate::site::Site;
use crate::spelling;
use crate::subject::{Conversions, Entries, SelfReplacer, Subject, Target};

/// Writes the `From<Payload>`, `TryFrom<Enum>` and `From<Enum> for Target`
/// impls that were asked for. A payload whose impl cannot be written is
/// reported into `errors` and the other payloads' impls are written all
/// the same, so that the code around the enum resolves and each error
/// stands alone.
pub(crate) fn conversion_impls(
    subject: &Subject,
    entries: &Entries,
    from: bool,
    try_from: TryFromForms,
    into: &[&IntoTarget],
    errors: &mut Vec<Error>,
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = subject.generics().split_for_impl();
    let enum_ident = subject.ident();
    let prefix = Path::from(enum_ident.clone());
    let self_path: Path = parse_quote!(Self);
    let self_type: Type = parse_quote!(#enum_ident #ty_generics);
    let converting: Vec<(Target, Conversions)> = entries
        .targets()
        .into_iter()
        .filter_map(|target| Some((target, target.conversions()?)))
        .filter(|(_, conversions)| conversions.from)
        .collect();
    // A payload that is a type parameter of the enum can be any type, the
    // other payloads included, so its `From` overlaps theirs; rustc would
    // only report conflicting implementations on the attribute.
    let overlapping = from && converting.len() > 1;
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
    // The call site here is the subject's own attribute; its tokens are
    // spelled at the subject's name all the same.
    let site = Site::at(enum_ident.span());
    let mut output = TokenStream2::new();
    for (target, conversions) in &converting {
        let Target { payload, .. } = *target;
        if payload_counts[&payload.to_token_stream().to_string()] > 1 {
            continue;
        }
        // The variant's `cfg`, and `#[allow(deprecated)]` for a deprecated
        // variant, which every impl naming it carries.
        let cfgs = target.arm_attributes(site);
        let built = target.rebuild(&self_path, quote!(#value), site);
        let pattern = target.pattern(&prefix, &value, site);
        let parameter = type_parameter(subject, target);
        let projected = projected_parameter(subject, target);
        if let (true, Some(ident)) = (overlapping, parameter) {
            errors.push(Error::new_spanned(
                payload,
                format!("`{ident}` is a type parameter of `{enum_ident}`, so `From<{ident}>` would overlap the other variants' `From` impls; add `from = false` on the enum, or `#[dispatch(from = false)]` on this variant"),
            ));
        } else if let (true, Some(ident)) = (from, projected) {
            // An associated type of a type parameter can be any type, the
            // enum itself included, so its `From` overlaps `core`'s `impl<T>
            // From<T> for T` whatever the other variants hold; rustc would
            // only report conflicting implementations on the attribute.
            let spelled = spelling::spelled(payload);
            errors.push(Error::new_spanned(
                payload,
                format!("`{spelled}` is an associated type of `{ident}`, a type parameter of `{enum_ident}`, and may be `{enum_ident}` itself, so `From<{spelled}>` would overlap `From<T> for T`; add `from = false` on the enum, or `#[dispatch(from = false)]` on this variant"),
            ));
        } else if from {
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
        let forms = conversions.try_from.unwrap_or(try_from);
        if forms == TryFromForms::NONE {
            continue;
        }
        // `TryFrom<Enum<T>> for T` implements a foreign trait for a type
        // parameter, which the orphan rules forbid; rustc would report
        // that on the enum's generics.
        if let Some(ident) = parameter {
            errors.push(Error::new_spanned(
                payload,
                format!("`{ident}` is a type parameter of `{enum_ident}`, so `TryFrom<{enum_ident}> for {ident}` would implement a foreign trait for a type parameter, which the orphan rules forbid; add `#[dispatch(try_from = false)]` on this variant"),
            ));
            continue;
        }
        // An associated type of a type parameter can be any type, so the
        // impl overlaps the `TryFrom` that `core` implements for every
        // type through `Into`, as its `From` overlaps the reflexive one.
        if let Some(ident) = projected {
            let spelled = spelling::spelled(payload);
            errors.push(Error::new_spanned(
                payload,
                format!("`{spelled}` is an associated type of `{ident}`, a type parameter of `{enum_ident}`, and may be any type, so `TryFrom<{enum_ident}> for {spelled}` would overlap the `TryFrom` that `core` implements for every type; add `#[dispatch(try_from = false)]` on this variant"),
            ));
            continue;
        }
        // The payload is the impl's own type here, so a `Self` it names
        // must be spelled out as the enum.
        let mut outside = payload.clone();
        SelfReplacer(self_type.clone()).visit_type_mut(&mut outside);
        if forms.owned {
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
        // The by-reference forms borrow for one lifetime of their own,
        // which the enum's generics may not name.
        let lifetime = Lifetime::new("'__enum", Span::mixed_site());
        let mut by_reference = subject.generics().clone();
        by_reference
            .params
            .insert(0, GenericParam::Lifetime(parse_quote!(#lifetime)));
        let (reference_generics, _, _) = by_reference.split_for_impl();
        let references = [
            (forms.by_ref, quote!(&#lifetime)),
            (forms.by_mut, quote!(&#lifetime mut)),
        ];
        for (wanted, reference) in references {
            if !wanted {
                continue;
            }
            output.extend(quote! {
                #(#cfgs)*
                #[automatically_derived]
                impl #reference_generics ::core::convert::TryFrom<#reference #enum_ident #ty_generics> for #reference #outside #where_clause {
                    type Error = #reference #enum_ident #ty_generics;
                    #[inline]
                    #[allow(unreachable_patterns)]
                    fn try_from(#value: #reference #enum_ident #ty_generics) -> ::core::result::Result<Self, Self::Error> {
                        match #value {
                            #pattern => ::core::result::Result::Ok(#value),
                            #other => ::core::result::Result::Err(#other),
                        }
                    }
                }
            });
        }
    }
    // `into = Target`: the enum converts into the target through every
    // payload's own `Into<Target>`, so every variant has to hold a payload
    // and nothing else.
    for IntoTarget { key, target } in into {
        let mut arms = vec![];
        let mut whole = true;
        for entry in entries.iter() {
            match entry.target() {
                Some(variant) if variant.conversions().is_some() => {
                    let pattern = variant.pattern(&prefix, &value, site);
                    let cfgs = variant.arm_attributes(site);
                    arms.push(
                        quote! { #(#cfgs)* #pattern => ::core::convert::Into::into(#value), },
                    );
                }
                Some(variant) => {
                    whole = false;
                    errors.push(Error::new(
                        key.span(),
                        format!("`into` converts every variant's payload, and `{}` has other fields beside its payload", variant.ident()),
                    ));
                }
                None => {
                    whole = false;
                    let reason = entry
                        .unforwardable()
                        .map(|unforwardable| unforwardable.reason.as_str())
                        .unwrap_or_default();
                    errors.push(Error::new(
                        key.span(),
                        format!(
                            "`into` converts every variant's payload, and `{}` {}",
                            entry.ident, reason
                        ),
                    ));
                }
            }
        }
        if !whole {
            continue;
        }
        output.extend(quote! {
            #[automatically_derived]
            impl #impl_generics ::core::convert::From<#enum_ident #ty_generics> for #target #where_clause {
                #[inline]
                fn from(#value: #enum_ident #ty_generics) -> Self {
                    match #value {
                        #(#arms)*
                    }
                }
            }
        });
    }
    output
}

/// Returns the payload of `target` when it is a bare type parameter of the
/// subject.
fn type_parameter<'t>(subject: &Subject, target: &Target<'t>) -> Option<&'t Ident> {
    bare_ident(target.payload).filter(|ident| is_type_parameter(subject, ident))
}

/// Returns the type parameter of the subject that the payload of `target`
/// is an associated type of: `S` for `S::Out` and for `<S as Source>::Out`.
/// rustc cannot tell what such a type is, so it stands for any type, as
/// the parameter itself does. A projection from any other type is one
/// rustc can normalize, and is left to it.
fn projected_parameter<'t>(subject: &Subject, target: &Target<'t>) -> Option<&'t Ident> {
    let Type::Path(projection) = target.payload else {
        return None;
    };
    let projected_from = match &projection.qself {
        Some(qself) => bare_ident(&qself.ty)?,
        None => {
            let segments = &projection.path.segments;
            if projection.path.leading_colon.is_some() || segments.len() < 2 {
                return None;
            }
            let first = segments.first()?;
            if !first.arguments.is_none() {
                return None;
            }
            &first.ident
        }
    };
    is_type_parameter(subject, projected_from).then_some(projected_from)
}

/// Returns whether `ident` names a type parameter of the subject.
fn is_type_parameter(subject: &Subject, ident: &Ident) -> bool {
    subject
        .generics()
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Type(param) if param.ident == *ident))
}
