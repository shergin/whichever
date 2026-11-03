//! `implement!`: the forwarding impl of one trait for one subject.
use std::collections::hash_map::Entry as MapEntry;
use std::collections::HashMap;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{
    parse_quote, Attribute, Error, Generics, Ident, ImplItem, ItemImpl, ItemTrait, Path, Result,
    Token, TraitItem, TraitItemConst, TraitItemFn, TraitItemType, Type,
};

use crate::agreement::{associated_const_check, associated_type_check};
use crate::combined;
use crate::forwarding::{
    ArgumentNames, Arm, ArmContext, Callee, Forwarding, MethodPlan, SelfArguments,
};
use crate::options::{closest, listed, method_mismatch, Override, TraitSpec};
use crate::protocol::ImplementInput;
use crate::shape::{takes_self_argument, ReturnSite};
use crate::subject::{
    carried_attributes, cfg_attributes, shape_errors, targets, Entry, Subject, Target,
};
use crate::substitution::Substitution;

pub(crate) fn expand_implement(input: ImplementInput) -> Result<TokenStream2> {
    let ImplementInput {
        item_trait,
        remote,
        subject,
        mut spec,
        partial,
    } = input;
    let Some(spec_path) = spec.path.clone() else {
        return Err(Error::new(
            Span::call_site(),
            "the trait to implement is missing",
        ));
    };
    let implemented = implemented_path(remote, &spec_path);
    let trait_path = &implemented;
    let trait_ident = &trait_path
        .segments
        .last()
        .expect("a path has at least one segment")
        .ident;
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
    // The partial impl block a message points at, spelled with the trait as
    // the subject names it and the subject's own generics.
    let block_header = {
        let (impl_generics, ty_generics, _) = subject.generics().split_for_impl();
        let ident = subject.ident();
        spelled(&quote!(impl #impl_generics #spec_path for #ident #ty_generics))
    };
    let mut entries = subject.entries()?;
    // A partial block's header may name the subject's parameters as it
    // likes, `impl<U> Trait for Kind<U>` over `enum Kind<T>`; the payloads
    // are spelled with the names the subject declared, and follow.
    if let Some(block) = &partial {
        if let Some(mut renaming) = Substitution::of_subject(subject.generics(), &block.self_ty) {
            for entry in &mut entries {
                if let Entry::Target(target) = entry {
                    renaming.visit_type_mut(&mut target.payload);
                }
            }
        }
    }
    let variants = targets(&entries);
    // A partial block is the only dispatch of its trait, so a variant that
    // cannot be forwarded to is reported here, under its own `cfg`.
    let shape_errors = match &partial {
        Some(_) => shape_errors(&entries),
        None => TokenStream2::new(),
    };
    if variants.is_empty() {
        if entries.is_empty() {
            return Err(Error::new_spanned(
                &subject,
                "a dispatch enum needs at least one variant",
            ));
        }
        // Every variant was rejected, each with its own sentence; an impl
        // with nothing to forward to would only add rustc's errors to them.
        return Ok(shape_errors);
    }
    // The associated items come from the first variant that is always
    // compiled in; one under `#[cfg]` may not be there to supply them.
    let first = variants
        .iter()
        .find(|target| target.cfgs.is_empty())
        .copied();
    let supplier = Supplier {
        first,
        subject: subject.ident(),
        trait_path,
    };
    let mut substitution = Substitution::of_trait(&item_trait, trait_path, generics)?;
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
        // An item the partial block writes is the block's: in every build
        // when it is written unconditionally, and where its `cfg` holds
        // when it is written under one, in which case the forwarded item
        // takes the builds where it does not, as a hand writes the two arms.
        let written = name.and_then(|name| written.get(&name.to_string()));
        let (complement, written_at) = match written {
            Some(Written::Always) => continue,
            Some(Written::Under { conditions, span }) => {
                (Some(not_all_of(conditions)), Some(*span))
            }
            None => (None, None),
        };
        let generated = match (item, written_at) {
            (TraitItem::Type(assoc), Some(span)) => Err(Error::new(
                span,
                format!("`{}` is declared under `#[cfg]`, and an associated type is declared in every build, since the forwarded methods convert into it; put the whole block under the `cfg`, with a second block for the other builds", assoc.ident),
            )),
            (TraitItem::Const(constant), Some(span)) => Err(Error::new(
                span,
                format!("`{}` is given under `#[cfg]`, and an associated const is given in every build; put the whole block under the `cfg`, with a second block for the other builds", constant.ident),
            )),
            (TraitItem::Type(assoc), None) => {
                associated_type(assoc, &spec, &supplier, &block_header, &mut agreed_types)
            }
            (TraitItem::Const(constant), None) => associated_const(
                constant,
                &spec,
                &supplier,
                &mut substitution,
                &mut agreed_consts,
            ),
            (TraitItem::Fn(function), _) => expand_method(
                function,
                &entries,
                &subject,
                trait_path,
                &spec,
                &block_header,
                &mut substitution,
            )
            .map(|forwarded| quote! { #complement #forwarded }),
            (TraitItem::Macro(item), _) => Err(Error::new_spanned(
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
    // An option naming an item the trait does not have is a typo, which
    // would otherwise turn into a type error somewhere else; the message
    // says what was probably meant, or what there is to mean.
    let associated: Vec<String> = item_trait
        .items
        .iter()
        .filter_map(|item| match item {
            TraitItem::Type(assoc) => Some(assoc.ident.to_string()),
            TraitItem::Const(constant) => Some(constant.ident.to_string()),
            _ => None,
        })
        .collect();
    let on_subject = partial.is_none();
    for fixed in &spec.overrides {
        let name = fixed.name.to_string();
        if associated.contains(&name) {
            continue;
        }
        let hint = unknown_override_hint(&name, &associated, &subject, on_subject);
        errors.push(Error::new(
            fixed.name.span(),
            format!("`{trait_ident}` has no associated type or const `{name}`; {hint}"),
        ));
    }
    let deciding: Vec<String> = item_trait
        .items
        .iter()
        .filter_map(|item| match item {
            TraitItem::Fn(function) if takes_self_argument(&function.sig) => {
                Some(function.sig.ident.to_string())
            }
            _ => None,
        })
        .collect();
    for (method, _) in &spec.mismatch {
        let name = method.to_string();
        if deciding.contains(&name) {
            continue;
        }
        let hint = match closest(&name, deciding.iter().map(String::as_str)) {
            Some(near) => format!("did you mean `{near}`?"),
            None if deciding.is_empty() => "none of its methods takes one".to_owned(),
            None => format!("its methods taking one are {}", listed(&deciding)),
        };
        errors.push(Error::new(
            method.span(),
            format!("`{trait_ident}` has no method `{name}` with a `Self`, `&Self` or `&mut Self` argument, so there is no mismatch to decide; {hint}"),
        ));
    }
    if let Some(error) = combined(errors) {
        return Err(error);
    }
    // The checks live outside the impl, where a `Self` in a payload type
    // has to be spelled out as the implementing type.
    let checks = first.map(|first| {
        let first_outside = first.target_type_outside(self_ty);
        let types = associated_type_check(
            &agreed_types,
            &variants,
            first,
            trait_path,
            &first_outside,
            generics,
            self_ty,
        );
        let consts = associated_const_check(
            &agreed_consts,
            &variants,
            first,
            trait_path,
            &first_outside,
            generics,
            self_ty,
        );
        quote! { #types #consts }
    });
    // The header's own tokens carry the subject's span: the call site here
    // is the trait's attribute, which may be in another crate.
    let site = subject.ident().span();
    Ok(quote_spanned! { site =>
        #(#impl_attrs)*
        #derived
        #unsafety impl #impl_generics #trait_path for #self_ty #where_clause {
            #items
            #(#user_items)*
        }
        #checks
        #shape_errors
    })
}

/// Returns the second half of the message for an override naming nothing in
/// the trait: the item or option probably meant, where the forwarded field
/// or variants are chosen when the name is a field-choosing option of
/// another crate, or else everything there is to name.
fn unknown_override_hint(
    name: &str,
    associated: &[String],
    subject: &Subject,
    on_subject: bool,
) -> String {
    if let Some(near) = closest(name, associated.iter().map(String::as_str)) {
        return format!("did you mean `{near}`?");
    }
    let switches = if on_subject {
        &["from", "try_from"][..]
    } else {
        &[]
    };
    if let Some(near) = closest(name, switches.iter().copied()) {
        return format!("did you mean the enum's option `{near} = ..`?");
    }
    if matches!(name, "target" | "to" | "field" | "delegate" | "via") {
        return if subject.has_one_target() {
            "a struct forwards to its one field, or to the field marked `#[dispatch]`".to_owned()
        } else {
            "an enum forwards to every variant, and an inherent method names the variants it forwards to".to_owned()
        };
    }
    let options = if on_subject {
        "`mismatch(method = expr)`, `from = false` and `try_from = true`"
    } else {
        "`mismatch(method = expr)`"
    };
    if associated.is_empty() {
        return format!("it has none, and the other options are {options}");
    }
    format!(
        "its associated items are {}, and the other options are {options}",
        listed(associated)
    )
}

/// Spells `tokens` as a hand writes them, without the spaces `quote` puts
/// around punctuation.
fn spelled(tokens: &impl ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .replace(" :: ", "::")
        .replace(" < ", "<")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace(" ,", ",")
        .replace("& ", "&")
        .replace(" : ", ": ")
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
    /// The items the block writes, by name, and in which builds.
    written: HashMap<String, Written>,
    user_items: Vec<&'a ImplItem>,
}

/// In which builds a partial block writes an item of a given name.
enum Written {
    /// In every build: at least one of the block's items of the name has
    /// no `#[cfg]`.
    Always,
    /// Under the `#[cfg]` attributes of each of the block's items of the
    /// name, any of which may hold; `span` is the first such item's, for a
    /// message about them.
    Under {
        conditions: Vec<Vec<Attribute>>,
        span: Span,
    },
}

impl Written {
    /// The first item of a name, written under `cfgs` at `span`.
    fn new(cfgs: Vec<Attribute>, span: Span) -> Self {
        if cfgs.is_empty() {
            Written::Always
        } else {
            Written::Under {
                conditions: vec![cfgs],
                span,
            }
        }
    }

    /// Records one more item of the name, written under `cfgs`.
    fn add(&mut self, cfgs: Vec<Attribute>) {
        match self {
            Written::Always => {}
            Written::Under { .. } if cfgs.is_empty() => *self = Written::Always,
            Written::Under { conditions, .. } => conditions.push(cfgs),
        }
    }
}

/// Returns the `#[cfg]` under which none of `conditions` holds: each is
/// the `all` of one item's predicates, and the item exists under `any` of
/// them.
fn not_all_of(conditions: &[Vec<Attribute>]) -> TokenStream2 {
    let predicates = conditions.iter().map(|cfgs| {
        let predicates = cfgs.iter().map(|attribute| match &attribute.meta {
            syn::Meta::List(list) => list.tokens.clone(),
            _ => TokenStream2::new(),
        });
        quote!(all(#(#predicates),*))
    });
    quote!(#[cfg(not(any(#(#predicates),*)))])
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
                written: HashMap::new(),
                user_items: vec![],
            };
        };
        let mut written: HashMap<String, Written> = HashMap::new();
        for item in &block.items {
            let (name, attrs) = match item {
                ImplItem::Fn(function) => (&function.sig.ident, &function.attrs),
                // An associated type written in the block is an override
                // like the `Name = Type` option.
                ImplItem::Type(assoc) => {
                    spec.overrides
                        .push(Override::of_type(assoc.ident.clone(), &assoc.ty));
                    (&assoc.ident, &assoc.attrs)
                }
                ImplItem::Const(constant) => (&constant.ident, &constant.attrs),
                _ => continue,
            };
            let cfgs = cfg_attributes(attrs);
            match written.entry(name.to_string()) {
                MapEntry::Occupied(entry) => entry.into_mut().add(cfgs),
                MapEntry::Vacant(entry) => {
                    entry.insert(Written::new(cfgs, item.span()));
                }
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

/// Where an associated item that is not overridden comes from: the first
/// variant that is always compiled in, when there is one.
struct Supplier<'a> {
    first: Option<&'a Target>,
    subject: &'a Ident,
    trait_path: &'a Path,
}

impl Supplier<'_> {
    /// Returns `<First as Trait>::name`, or the error for an enum whose
    /// every variant is under `#[cfg]`.
    fn item(&self, name: &Ident) -> Result<TokenStream2> {
        let Some(first) = self.first else {
            return Err(Error::new_spanned(
                self.subject,
                format!("every variant of `{}` is under `#[cfg]`, so none can supply `{name}` in every build; fix it on the enum with `{name} = ..`", self.subject),
            ));
        };
        let payload = first.target_type();
        let trait_path = self.trait_path;
        Ok(quote!(<#payload as #trait_path>::#name))
    }
}

/// Writes `type Name = ..;`: the override, or the supplier's type with
/// `Name` recorded for the agreement check. A generic associated type has
/// no supplier: the enum declares it in a partial impl block, which is
/// where one the block writes was taken out of this path.
fn associated_type<'a>(
    assoc: &'a TraitItemType,
    spec: &TraitSpec,
    supplier: &Supplier,
    block_header: &str,
    agreed: &mut Vec<&'a Ident>,
) -> Result<TokenStream2> {
    let name = &assoc.ident;
    if !assoc.generics.params.is_empty() {
        let parameters = spelled(&assoc.generics);
        return Err(Error::new_spanned(
            assoc,
            format!("`{name}` is a generic associated type, which only the enum can declare; write `type {name}{parameters} = ..;` in a `#[whichever::dispatch] {block_header}` block"),
        ));
    }
    let carried = carried_attributes(&assoc.attrs);
    let cfgs = carried
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"));
    let ty = match spec.override_for(name) {
        Some(fixed) => fixed.as_type()?.into_token_stream(),
        None => {
            // The agreement check cannot name an item that may be
            // compiled out.
            if cfgs.count() == 0 {
                agreed.push(name);
            }
            supplier.item(name)?
        }
    };
    Ok(quote! { #(#carried)* type #name = #ty; })
}

/// Writes `const NAME: T = ..;`: the override, or the supplier's value with
/// `NAME` recorded for the agreement check.
fn associated_const<'a>(
    constant: &'a TraitItemConst,
    spec: &TraitSpec,
    supplier: &Supplier,
    substitution: &mut Substitution,
    agreed: &mut Vec<(&'a Ident, Type)>,
) -> Result<TokenStream2> {
    let name = &constant.ident;
    let carried = carried_attributes(&constant.attrs);
    let cfgs = carried
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"));
    let mut ty = constant.ty.clone();
    substitution.visit_type_mut(&mut ty);
    substitution.check()?;
    let value = match spec.override_for(name) {
        Some(fixed) => fixed.as_value()?.into_token_stream(),
        None => {
            if cfgs.count() == 0 {
                agreed.push((name, ty.clone()));
            }
            supplier.item(name)?
        }
    };
    Ok(quote! { #(#carried)* const #name: #ty = #value; })
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
    block_header: &str,
    substitution: &mut Substitution,
) -> Result<TokenStream2> {
    let subject_ident = subject.ident();
    let mut signature = function.sig.clone();
    substitution.visit_signature_mut(&mut signature);
    substitution.check()?;
    let method = signature.ident.clone();
    if signature.receiver().is_none() {
        if function.default.is_some() {
            return Ok(TokenStream2::new());
        }
        return Err(Error::new_spanned(
            &function.sig,
            format!("`{method}` has no receiver, so there is no variant to dispatch on; give it a default body, or write it in a `#[whichever::dispatch] {block_header}` block"),
        ));
    }
    let context = ArmContext {
        callee: Callee::Qualified {
            trait_path,
            method: &method,
        },
        method: &method,
        subject: subject_ident,
        prefix: Path::from(subject_ident.clone()),
        lift: None,
        site: subject_ident.span(),
    };
    let plan = MethodPlan::new(
        &mut signature,
        ArgumentNames::WrittenOrGenerated,
        SelfArguments::Dispatch,
        Some(spec),
        &ReturnSite {
            subject: subject_ident,
            one_target: subject.has_one_target(),
            by_hand: context.callee.by_hand(),
        },
    )?;
    let mismatch = match spec.mismatch_for(&method) {
        Some(expr) => Some(expr.clone()),
        None => method_mismatch(function)?,
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
        Some(plan.fallback(expr.to_token_stream()))
    } else {
        None
    };
    let allow_unreachable = fallback
        .is_some()
        .then(|| quote! { #[allow(unreachable_patterns)] });
    // The trait method's own `#[inline]`, or ours.
    let inline = function
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("inline"))
        .map(ToTokens::to_token_stream)
        .unwrap_or_else(|| quote! { #[inline] });
    let carried = carried_attributes(&function.attrs);
    let body = Forwarding {
        scrutinee: plan.scrutinee(context.site),
        arms,
        fallback,
        future: plan.future,
    }
    .body(method.span());
    Ok(quote! {
        #(#carried)*
        #inline
        #allow_unreachable
        #signature #body
    })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn the_forwarded_item_takes_the_builds_the_blocks_items_leave() {
        let named: Vec<Attribute> = vec![
            parse_quote!(#[cfg(feature = "named")]),
            parse_quote!(#[cfg(not(test))]),
        ];
        let unix: Vec<Attribute> = vec![parse_quote!(#[cfg(unix)])];
        assert_eq!(
            not_all_of(&[named.clone(), unix.clone()]).to_string(),
            quote!(#[cfg(not(any(all(feature = "named", not(test)), all(unix))))]).to_string()
        );
        let mut written = Written::new(named, Span::call_site());
        assert!(matches!(&written, Written::Under { conditions, .. } if conditions.len() == 1));
        written.add(unix);
        assert!(matches!(&written, Written::Under { conditions, .. } if conditions.len() == 2));
        written.add(vec![]);
        assert!(matches!(written, Written::Always));
        assert!(matches!(
            Written::new(vec![], Span::call_site()),
            Written::Always
        ));
    }
}
