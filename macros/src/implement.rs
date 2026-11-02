//! `implement!`: the forwarding impl of one trait for one subject.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::visit_mut::VisitMut;
use syn::{Error, Ident, ImplItem, Path, Result, TraitItem, TraitItemFn, Type};

use crate::agreement::{associated_const_check, associated_type_check};
use crate::forwarding::{
    ArgumentNames, Arm, Arms, Callee, Forwarding, Hints, MethodPlan, SelfArguments,
};
use crate::options::{method_mismatch, Override, TraitSpec};
use crate::protocol::ImplementInput;
use crate::subject::{cfg_attributes, shape_errors, targets, Entry, Subject};
use crate::substitution::Substitution;

pub(crate) fn expand_implement(input: ImplementInput) -> Result<TokenStream2> {
    let ImplementInput {
        item_trait,
        remote,
        subject,
        mut spec,
        partial,
    } = input;
    let subject_ident = subject.ident();
    let Some(spec_path) = &spec.path else {
        return Err(Error::new(
            Span::call_site(),
            "the trait to implement is missing",
        ));
    };
    // A restated trait is implemented at its real path, with the generic
    // arguments the enum named on the declaration's path.
    let implemented = match remote {
        Some(mut remote) => {
            if let (Some(last), Some(named)) =
                (remote.segments.last_mut(), spec_path.segments.last())
            {
                last.arguments = named.arguments.clone();
            }
            remote
        }
        None => spec_path.clone(),
    };
    let trait_path = &implemented;
    // A partial impl block supplies the impl's header and the items the enum
    // implements itself; an associated type written there is an override
    // like the `Name = Type` option.
    let mut written = std::collections::HashSet::new();
    let mut user_items = vec![];
    let (generics, self_ty, impl_attrs, unsafety) = match &partial {
        Some(block) => {
            for item in &block.items {
                match item {
                    ImplItem::Fn(function) => {
                        written.insert(function.sig.ident.to_string());
                    }
                    ImplItem::Type(assoc) => {
                        written.insert(assoc.ident.to_string());
                        spec.assoc_overrides
                            .push((assoc.ident.clone(), Override::Type(assoc.ty.clone())));
                    }
                    ImplItem::Const(constant) => {
                        written.insert(constant.ident.to_string());
                    }
                    _ => {}
                }
                user_items.push(item);
            }
            (
                &block.generics,
                block.self_ty.to_token_stream(),
                block.attrs.as_slice(),
                block.unsafety,
            )
        }
        None => {
            let (_, ty_generics, _) = subject.generics().split_for_impl();
            (
                subject.generics(),
                quote!(#subject_ident #ty_generics),
                &[][..],
                None,
            )
        }
    };
    let derived = partial.is_none().then(|| quote!(#[automatically_derived]));
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let entries = subject.entries()?;
    let variants = targets(&entries);
    let Some(first) = variants.first() else {
        return Err(Error::new_spanned(
            &subject,
            "a dispatch enum needs at least one variant",
        ));
    };
    // A partial block is the only dispatch of its trait, so a variant that
    // cannot be forwarded to is reported here, under its own `cfg`.
    let shape_errors = match &partial {
        Some(_) => shape_errors(&entries),
        None => TokenStream2::new(),
    };
    let first_target = first.target_type();
    let first_payload = &first_target;
    // The checks live outside the impl, where a `Self` in a payload type
    // has to be spelled out as the implementing type.
    let self_type: Type = syn::parse2(self_ty.clone())?;
    let first_outside = first.target_type_outside(&self_type);
    let mut substitution = Substitution::new(&item_trait, trait_path)?;
    let mut items = TokenStream2::new();
    let mut agreed_types: Vec<&Ident> = vec![];
    let mut agreed_consts: Vec<(&Ident, Type)> = vec![];
    for item in &item_trait.items {
        match item {
            TraitItem::Type(assoc) => {
                if !assoc.generics.params.is_empty() {
                    return Err(Error::new_spanned(
                        assoc,
                        "generic associated types are not supported",
                    ));
                }
                let name = &assoc.ident;
                if written.contains(&name.to_string()) {
                    continue;
                }
                let ty = match spec.override_for(name) {
                    Some(Override::Type(ty)) => quote!(#ty),
                    Some(Override::Expr(expr)) => {
                        return Err(Error::new_spanned(
                            expr,
                            format!("`{name}` is an associated type; give it a type"),
                        ));
                    }
                    None => {
                        agreed_types.push(name);
                        quote!(<#first_payload as #trait_path>::#name)
                    }
                };
                items.extend(quote! { type #name = #ty; });
            }
            TraitItem::Const(constant) => {
                let name = &constant.ident;
                if written.contains(&name.to_string()) {
                    continue;
                }
                let mut ty = constant.ty.clone();
                substitution.visit_type_mut(&mut ty);
                let value = match spec.const_override(name)? {
                    Some(expr) => quote!(#expr),
                    None => {
                        agreed_consts.push((name, ty.clone()));
                        quote!(<#first_payload as #trait_path>::#name)
                    }
                };
                items.extend(quote! { const #name: #ty = #value; });
            }
            TraitItem::Fn(function) => {
                if written.contains(&function.sig.ident.to_string()) {
                    continue;
                }
                items.extend(expand_method(
                    function,
                    &entries,
                    &subject,
                    trait_path,
                    &spec,
                    &mut substitution,
                )?);
            }
            TraitItem::Macro(item) => {
                return Err(Error::new_spanned(
                    item,
                    "macro invocations inside the trait are not supported",
                ));
            }
            _ => {}
        }
    }
    let agreement_check = associated_type_check(
        &agreed_types,
        &variants,
        trait_path,
        &first_outside,
        generics,
        &self_type,
    );
    let const_check = associated_const_check(
        &agreed_consts,
        &variants,
        trait_path,
        &first_outside,
        generics,
        &self_type,
    );
    Ok(quote! {
        #(#impl_attrs)*
        #derived
        #unsafety impl #impl_generics #trait_path for #self_ty #where_clause {
            #items
            #(#user_items)*
        }
        #agreement_check
        #const_check
        #shape_errors
    })
}

/// Writes the forwarding method for one trait method: a `match` over every
/// variant with a fully qualified call per arm, and the mismatch expression
/// for two different variants when the method takes `Self` arguments. A
/// method without a receiver is left to its default body.
fn expand_method(
    function: &TraitItemFn,
    entries: &[Entry],
    subject: &Subject,
    trait_path: &Path,
    spec: &TraitSpec,
    substitution: &mut Substitution,
) -> Result<TokenStream2> {
    let subject_ident = subject.ident();
    let mut signature = function.sig.clone();
    substitution.visit_signature_mut(&mut signature);
    let method = signature.ident.clone();
    if signature.receiver().is_none() {
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
    }
    let plan = MethodPlan::new(
        &mut signature,
        ArgumentNames::Generated,
        SelfArguments::Dispatch,
        Some(spec),
    )?;
    let mismatch = match spec.mismatch_for(&method) {
        Some(expr) => Some(expr.clone()),
        None => method_mismatch(function)?,
    };
    let shared = Arms {
        callee: Callee::Qualified {
            trait_path,
            method: &method,
        },
        method: &method,
        subject: subject_ident,
        prefix: subject_ident.to_token_stream(),
        lift: None,
        hints: Hints {
            deref: "implement the method by hand",
            rebuild: "write the method in a `#[whichever::dispatch] impl` block",
        },
    };
    let mut arms = Vec::with_capacity(entries.len());
    for entry in entries {
        match entry {
            Entry::Target(target) => arms.push(plan.arm(target, &shared)?),
            // The variant is reported elsewhere, under its `cfg`; this arm
            // only keeps the `match` exhaustive while that error stands,
            // which the fallback arm does when there is one.
            Entry::Unforwardable(unforwardable) if !plan.has_same_variant() => {
                arms.push(Arm::unforwardable(
                    &shared.prefix,
                    unforwardable,
                    quote!(::core::unreachable!()),
                ));
            }
            Entry::Unforwardable(_) => {}
        }
    }
    // Two values of a struct always hold the same field, so only an enum
    // needs the arm for two different variants.
    let fallback = if plan.has_same_variant() && subject.can_mismatch() {
        let Some(expr) = mismatch else {
            return Err(Error::new_spanned(
                &function.sig,
                format!("`{method}` takes `Self` arguments; give the result for two different variants with `#[dispatch(mismatch = <expr>)]` on the trait method or `mismatch({method} = <expr>)` on the enum"),
            ));
        };
        Some(expr.to_token_stream())
    } else {
        None
    };
    let allow_unreachable = fallback
        .is_some()
        .then(|| quote! { #[allow(unreachable_patterns)] });
    let inline = (!function
        .attrs
        .iter()
        .any(|attribute| attribute.path().is_ident("inline")))
    .then(|| quote! { #[inline] });
    let cfgs = cfg_attributes(&function.attrs);
    let body = Forwarding {
        scrutinee: plan.scrutinee(),
        arms,
        fallback,
        future: plan.future,
    }
    .body(method.span());
    Ok(quote! {
        #(#cfgs)*
        #inline
        #allow_unreachable
        #signature #body
    })
}
