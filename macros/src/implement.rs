//! `implement!`: the forwarding impl of one trait for one subject.
use std::collections::HashSet;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::visit_mut::VisitMut;
use syn::{
    parse_quote, Attribute, Error, Generics, Ident, ImplItem, ItemImpl, ItemTrait, Path, Result,
    Token, TraitItem, TraitItemConst, TraitItemFn, TraitItemType, Type,
};

use crate::agreement::{associated_const_check, associated_type_check};
use crate::forwarding::{
    ArgumentNames, Arm, ArmContext, Callee, Forwarding, MethodPlan, SelfArguments,
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
    let Some(spec_path) = &spec.path else {
        return Err(Error::new(
            Span::call_site(),
            "the trait to implement is missing",
        ));
    };
    let implemented = implemented_path(remote, spec_path);
    let trait_path = &implemented;
    let header = ImplHeader::new(partial.as_ref(), &subject, &item_trait, &mut spec);
    let ImplHeader {
        generics,
        self_ty,
        attrs: impl_attrs,
        unsafety,
        written,
        user_items,
    } = &header;
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
    let first_outside = first.target_type_outside(self_ty);
    let mut substitution = Substitution::new(&item_trait, trait_path)?;
    let mut items = TokenStream2::new();
    let mut agreed_types: Vec<&Ident> = vec![];
    let mut agreed_consts: Vec<(&Ident, Type)> = vec![];
    // Every item that cannot be forwarded is reported, not only the first.
    let mut errors: Vec<Error> = vec![];
    for item in &item_trait.items {
        let name = match item {
            TraitItem::Type(assoc) => Some(&assoc.ident),
            TraitItem::Const(constant) => Some(&constant.ident),
            TraitItem::Fn(function) => Some(&function.sig.ident),
            _ => None,
        };
        // An item the partial block writes is the block's.
        if name.is_some_and(|name| written.contains(&name.to_string())) {
            continue;
        }
        let generated = match item {
            TraitItem::Type(assoc) => {
                associated_type(assoc, &spec, first_payload, trait_path, &mut agreed_types)
            }
            TraitItem::Const(constant) => associated_const(
                constant,
                &spec,
                first_payload,
                trait_path,
                &mut substitution,
                &mut agreed_consts,
            ),
            TraitItem::Fn(function) => expand_method(
                function,
                &entries,
                &subject,
                trait_path,
                &spec,
                &mut substitution,
            ),
            TraitItem::Macro(item) => Err(Error::new_spanned(
                item,
                "macro invocations inside the trait are not supported",
            )),
            _ => Ok(TokenStream2::new()),
        };
        match generated {
            Ok(tokens) => items.extend(tokens),
            Err(error) => errors.push(error),
        }
    }
    if let Some(error) = combined(errors) {
        return Err(error);
    }
    let agreement_check = associated_type_check(
        &agreed_types,
        &variants,
        trait_path,
        &first_outside,
        generics,
        self_ty,
    );
    let const_check = associated_const_check(
        &agreed_consts,
        &variants,
        trait_path,
        &first_outside,
        generics,
        self_ty,
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

/// Returns the path the impl is written for: a restated trait's real path,
/// with the generic arguments the subject named on the declaration's path,
/// or the path named.
fn implemented_path(remote: Option<Path>, named: &Path) -> Path {
    match remote {
        Some(mut remote) => {
            if let (Some(last), Some(named)) = (remote.segments.last_mut(), named.segments.last()) {
                last.arguments = named.arguments.clone();
            }
            remote
        }
        None => named.clone(),
    }
}

/// The impl's header: the partial block's when there is one, together with
/// the items the block writes, or the subject's own.
struct ImplHeader<'a> {
    generics: &'a Generics,
    self_ty: Type,
    attrs: &'a [Attribute],
    unsafety: Option<Token![unsafe]>,
    /// The names of the items the block writes; the trait's items of those
    /// names are the block's.
    written: HashSet<String>,
    user_items: Vec<&'a ImplItem>,
}

impl<'a> ImplHeader<'a> {
    fn new(
        partial: Option<&'a ItemImpl>,
        subject: &'a Subject,
        item_trait: &ItemTrait,
        spec: &mut TraitSpec,
    ) -> Self {
        let Some(block) = partial else {
            let ident = subject.ident();
            let (_, ty_generics, _) = subject.generics().split_for_impl();
            return Self {
                generics: subject.generics(),
                self_ty: parse_quote!(#ident #ty_generics),
                attrs: &[],
                // An `unsafe trait` is implemented by an `unsafe impl`.
                unsafety: item_trait.unsafety,
                written: HashSet::new(),
                user_items: vec![],
            };
        };
        let mut written = HashSet::new();
        for item in &block.items {
            match item {
                ImplItem::Fn(function) => {
                    written.insert(function.sig.ident.to_string());
                }
                // An associated type written in the block is an override
                // like the `Name = Type` option.
                ImplItem::Type(assoc) => {
                    written.insert(assoc.ident.to_string());
                    spec.overrides
                        .push(Override::of_type(assoc.ident.clone(), &assoc.ty));
                }
                ImplItem::Const(constant) => {
                    written.insert(constant.ident.to_string());
                }
                _ => {}
            }
        }
        Self {
            generics: &block.generics,
            self_ty: (*block.self_ty).clone(),
            attrs: &block.attrs,
            unsafety: block.unsafety,
            written,
            user_items: block.items.iter().collect(),
        }
    }
}

/// Writes `type Name = ..;`: the override, or the first target's type with
/// `Name` recorded for the agreement check.
fn associated_type<'a>(
    assoc: &'a TraitItemType,
    spec: &TraitSpec,
    first_payload: &TokenStream2,
    trait_path: &Path,
    agreed: &mut Vec<&'a Ident>,
) -> Result<TokenStream2> {
    if !assoc.generics.params.is_empty() {
        return Err(Error::new_spanned(
            assoc,
            "generic associated types are not supported",
        ));
    }
    let name = &assoc.ident;
    let ty = match spec.override_for(name) {
        Some(fixed) => fixed.as_type()?.into_token_stream(),
        None => {
            agreed.push(name);
            quote!(<#first_payload as #trait_path>::#name)
        }
    };
    Ok(quote! { type #name = #ty; })
}

/// Writes `const NAME: T = ..;`: the override, or the first target's value
/// with `NAME` recorded for the agreement check.
fn associated_const<'a>(
    constant: &'a TraitItemConst,
    spec: &TraitSpec,
    first_payload: &TokenStream2,
    trait_path: &Path,
    substitution: &mut Substitution,
    agreed: &mut Vec<(&'a Ident, Type)>,
) -> Result<TokenStream2> {
    let name = &constant.ident;
    let mut ty = constant.ty.clone();
    substitution.visit_type_mut(&mut ty);
    let value = match spec.override_for(name) {
        Some(fixed) => fixed.as_value()?.into_token_stream(),
        None => {
            agreed.push((name, ty.clone()));
            quote!(<#first_payload as #trait_path>::#name)
        }
    };
    Ok(quote! { const #name: #ty = #value; })
}

/// Folds `errors` into the first, so that all of them are reported at once.
fn combined(errors: Vec<Error>) -> Option<Error> {
    let mut errors = errors.into_iter();
    let mut first = errors.next()?;
    for error in errors {
        first.combine(error);
    }
    Some(first)
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
        ArgumentNames::WrittenOrGenerated,
        SelfArguments::Dispatch,
        Some(spec),
    )?;
    let mismatch = match spec.mismatch_for(&method) {
        Some(expr) => Some(expr.clone()),
        None => method_mismatch(function)?,
    };
    let context = ArmContext {
        callee: Callee::Qualified {
            trait_path,
            method: &method,
        },
        method: &method,
        subject: subject_ident,
        prefix: Path::from(subject_ident.clone()),
        lift: None,
    };
    let mut arms = Vec::with_capacity(entries.len());
    for entry in entries {
        match entry {
            Entry::Target(target) => arms.push(plan.arm(target, &context)?),
            // The variant is reported elsewhere, under its `cfg`; this arm
            // only keeps the `match` exhaustive while that error stands,
            // which the fallback arm does when there is one.
            Entry::Unforwardable(unforwardable) if !plan.has_same_variant() => {
                arms.push(Arm::unforwardable(
                    &context.prefix,
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
