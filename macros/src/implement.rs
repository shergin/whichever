//! `implement!`: the forwarding impl of one trait for one subject.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::visit_mut::VisitMut;
use syn::{Error, Ident, ImplItem, Result, TraitItem, Type};

use crate::agreement::{associated_const_check, associated_type_check};
use crate::forwarding::expand_method;
use crate::options::Override;
use crate::protocol::ImplementInput;
use crate::subject::{targets, Entry};
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
    let shape_errors: TokenStream2 = match &partial {
        Some(_) => entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Unforwardable(unforwardable) => Some(unforwardable.compile_error()),
                Entry::Target(_) => None,
            })
            .collect(),
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
