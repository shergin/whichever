//! The items of one trait's impl: each item of the trait planned, then
//! written.
//!
//! An item of the trait has zero or more plans: none when the partial
//! block writes it in every build or leaves it to the trait's default, and
//! one per way it is written when the block writes or marks a method under
//! `#[cfg]`. [`ItemWriter`] renders the plans and keeps what the agreement
//! checks and the shape errors need afterwards.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{
    Attribute, Error, GenericParam, Ident, Result, Signature, TraitItem, TraitItemConst,
    TraitItemType, Type,
};

use crate::builds::Builds;
use crate::forwarding::{
    forward, forward_associated, Associated, Forward, Generated, Mark, Origin, TraitMethod,
};
use crate::implement::Impl;
use crate::partial::{BlockMethod, Forwarded};
use crate::signatures::{carried_attributes, Method};
use crate::site::{at, Site};
use crate::spelling;
use crate::subject::{allow_deprecated, is_deprecated};
use crate::substitution::{Renaming, Substitution};

/// One thing an item of the trait becomes in the impl, decided before
/// anything is written. An item the partial block writes in every build,
/// or leaves to the trait's default, has no plan; a method the block
/// writes or marks under `#[cfg]` has one per way it is written.
enum ItemPlan<'a> {
    /// A method marked `#[dispatch(default)]` on a trait that gives it no
    /// default: written with the sentence as its body, under the `#[cfg]`
    /// of the builds the mark is for.
    Rejected {
        method: Method<'a>,
        error: Error,
        builds: Option<TokenStream2>,
    },
    /// An associated type, from the override or the first variant.
    Type(&'a TraitItemType),
    /// An associated const, from the override or the first variant.
    Const(&'a TraitItemConst),
    /// A method, forwarded to every variant or as the block's attribute
    /// names, under the complement of the block's `cfg` when the block
    /// writes or marks it in some builds only. A marked method under its
    /// own `#[cfg]` carries that attribute itself.
    Method {
        method: Method<'a>,
        forwarded: Option<&'a Forwarded>,
        complement: Option<Leftover>,
    },
    /// A shape the impl cannot write.
    Limit(Error),
}

/// The builds the block's conditional items of one name leave to the rest,
/// and the span the attribute naming them carries, the first such item's.
#[derive(Clone)]
struct Leftover {
    builds: Builds,
    span: Span,
}

impl Leftover {
    /// Returns the `#[cfg]` of those builds.
    fn attribute(&self) -> Option<TokenStream2> {
        self.builds.attribute(self.span)
    }
}

/// Writes the items of one trait's impl: plans each item of the trait, then
/// renders the plan, keeping what the agreement checks and the shape errors
/// need to know afterwards.
pub(crate) struct ItemWriter<'a> {
    /// The impl the items are written for.
    of: &'a Impl<'a>,
    /// The trait's generics replaced by the arguments the subject named.
    substitution: Substitution,
    pub(crate) items: TokenStream2,
    /// The associated types taken from the first variant, for the check.
    pub(crate) agreed_types: Vec<&'a Ident>,
    /// The associated consts taken from the first variant, with their
    /// types, for the check.
    pub(crate) agreed_consts: Vec<(&'a Ident, Type)>,
    /// Whether the trait deprecates an associated item, which the checks
    /// name and so allow.
    pub(crate) deprecated_items: bool,
    /// The builds in which some method with a receiver is forwarded to
    /// every variant. A variant no method can forward to is reported in
    /// those builds and in no other: a forwarding that exists in no build,
    /// the complement of a mark under the trait method's own `cfg`, reports
    /// nothing, and neither does a block whose every method names its
    /// variants.
    pub(crate) plain_forwarding: Builds,
    /// The items that could not be written, which withhold the impl.
    pub(crate) errors: Vec<Error>,
}

impl<'a> ItemWriter<'a> {
    pub(crate) fn new(of: &'a Impl<'a>, substitution: Substitution) -> Self {
        Self {
            of,
            substitution,
            items: TokenStream2::new(),
            agreed_types: vec![],
            agreed_consts: vec![],
            deprecated_items: false,
            plain_forwarding: Builds::Never,
            errors: vec![],
        }
    }

    /// Plans and writes one item of the trait.
    pub(crate) fn write(&mut self, item: &'a TraitItem) {
        match item {
            TraitItem::Type(assoc) => self.deprecated_items |= is_deprecated(&assoc.attrs),
            TraitItem::Const(constant) => self.deprecated_items |= is_deprecated(&constant.attrs),
            _ => {}
        }
        for plan in self.plans(item) {
            let site = self.of.site;
            let generated = match plan {
                ItemPlan::Rejected {
                    method,
                    error,
                    builds,
                } => self
                    .rejected_method(method, error)
                    .map(|rejected| quote_spanned! { *site => #builds #rejected }),
                ItemPlan::Type(assoc) => self.associated_type(assoc),
                ItemPlan::Const(constant) => self.associated_const(constant),
                ItemPlan::Method {
                    method,
                    forwarded,
                    complement,
                } => {
                    if forwarded.is_none() && method.function.sig.receiver().is_some() {
                        let builds = match &complement {
                            Some(leftover) => leftover.builds.clone(),
                            None => Builds::Every,
                        };
                        let forwarded =
                            std::mem::replace(&mut self.plain_forwarding, Builds::Never);
                        self.plain_forwarding = forwarded.or(builds);
                    }
                    let complement = complement.as_ref().and_then(Leftover::attribute);
                    self.method(method, forwarded)
                        .map(|forwarded| quote_spanned! { *site => #complement #forwarded })
                }
                ItemPlan::Limit(error) => Err(error),
            };
            match generated {
                Ok(tokens) => self.items.extend(tokens),
                Err(error) => self.errors.push(error),
            }
        }
    }

    /// Decides what `item` becomes. An item the partial block writes is the
    /// block's: in every build when it is written unconditionally, and
    /// where its `cfg` holds when it is written under one, in which case
    /// the forwarded item takes the builds where it does not, as a hand
    /// writes the two arms. A method the block marks is forwarded to the
    /// variants it names around its body, or left to the trait's default,
    /// and a mark under `#[cfg]` is the block's in the same way: it holds
    /// where the `cfg` does, and the builds it leaves go to a mark without
    /// a `#[cfg]`, or to the plain forwarding, so that a payload's override
    /// of a provided method is never lost to the trait's default.
    fn plans(&self, item: &'a TraitItem) -> Vec<ItemPlan<'a>> {
        let name = match item {
            TraitItem::Type(assoc) => &assoc.ident,
            TraitItem::Const(constant) => &constant.ident,
            TraitItem::Fn(function) => &function.sig.ident,
            TraitItem::Macro(item) => {
                return vec![ItemPlan::Limit(Error::new_spanned(
                    item,
                    "macro invocations inside the trait are not supported",
                ))]
            }
            _ => return vec![],
        };
        let name = name.to_string();
        // The builds in which the block writes the item itself.
        let (mut written, mut conditional_at) =
            match self.of.block.and_then(|block| block.written(&name)) {
                Some(written) if written.builds.is_every() => return vec![],
                Some(written) => (written.builds.clone(), Some(written.span)),
                None => (Builds::Never, None),
            };
        let function = match (item, conditional_at) {
            (TraitItem::Type(assoc), Some(span)) => return vec![ItemPlan::Limit(Error::new(
                span,
                format!("`{}` is declared under `#[cfg]`, and an associated type is declared in every build, since the forwarded methods convert into it; put the whole block under the `cfg`, with a second block for the other builds", assoc.ident),
            ))],
            (TraitItem::Const(constant), Some(span)) => return vec![ItemPlan::Limit(Error::new(
                span,
                format!("`{}` is given under `#[cfg]`, and an associated const is given in every build; put the whole block under the `cfg`, with a second block for the other builds", constant.ident),
            ))],
            (TraitItem::Type(assoc), None) => return vec![ItemPlan::Type(assoc)],
            (TraitItem::Const(constant), None) => return vec![ItemPlan::Const(constant)],
            (TraitItem::Fn(function), _) => self.of.signatures.method(function),
            _ => return vec![],
        };
        let marked = self
            .of
            .block
            .map(|block| block.marked(&name))
            .unwrap_or_default();
        for method in marked {
            let builds = Builds::of(&method.cfgs());
            if !builds.is_every() {
                conditional_at.get_or_insert(method.span());
                written = written.or(builds);
            }
        }
        // The builds the block's conditional items leave.
        let rest = conditional_at.map(|span| Leftover {
            builds: written.complement(),
            span: at(span),
        });
        let site = self.of.site;
        let mut plans = vec![];
        // Whether a mark without a `#[cfg]` takes those builds.
        let mut taken = false;
        for method in marked {
            let cfgs = method.cfgs();
            taken |= cfgs.is_empty();
            match method {
                BlockMethod::Default { span, .. } => {
                    if function.may_be_left_to_default() {
                        continue;
                    }
                    // Written with the sentence as its body, as a method
                    // that cannot be forwarded is, so that the impl stays
                    // complete.
                    plans.push(ItemPlan::Rejected {
                        method: function,
                        error: Error::new(
                            *span,
                            format!("`{}` has no default body in `{}` to be left to; give it a body, or forward it", function.ident(), self.of.trait_ident),
                        ),
                        builds: if cfgs.is_empty() {
                            rest.as_ref().and_then(Leftover::attribute)
                        } else {
                            Some(quote_spanned! { *site => #(#cfgs)* })
                        },
                    });
                }
                BlockMethod::Forwarded(forwarded) => plans.push(ItemPlan::Method {
                    method: function,
                    forwarded: Some(forwarded),
                    complement: if cfgs.is_empty() { rest.clone() } else { None },
                }),
            }
        }
        if !taken {
            plans.push(ItemPlan::Method {
                method: function,
                forwarded: None,
                complement: rest,
            });
        }
        plans
    }

    /// Writes `type Name = ..;`: the override, or the supplier's type with
    /// `Name` recorded for the agreement check. A generic associated type has
    /// no supplier: the enum declares it in a partial impl block, which is
    /// where one the block writes was taken out of this path. The item's own
    /// tokens carry `site`.
    fn associated_type(&mut self, assoc: &'a TraitItemType) -> Result<TokenStream2> {
        let Impl {
            options,
            block_header,
            site,
            ..
        } = *self.of;
        let name = &assoc.ident;
        let mut carried = carried_attributes(&assoc.attrs);
        if is_deprecated(&assoc.attrs) {
            carried.push(allow_deprecated(site));
        }
        // A generic associated type is the struct's field's, where the
        // field's type can be named: the same projection with the type's
        // own parameters handed on. An enum's has no supplier, since
        // nothing can check that the variants agree on it for every
        // argument, and neither has a struct's field reached through a
        // field or a method: it is written holding its sentence, as an
        // item no variant can supply is, so that the impl stays complete
        // and the sentence stands alone. Its parameters and `where` clause
        // are the trait's, which the impl may not strengthen.
        if !assoc.generics.params.is_empty() {
            let mut generics = assoc.generics.clone();
            self.substitution.distinguish_generics(&mut generics);
            self.substitution.visit_generics_mut(&mut generics);
            self.substitution.check()?;
            let where_clause = &generics.where_clause;
            let supplied = match self.of.supplier {
                Some(_) if self.of.subject.has_one_target() => Some(self.of.supplied(name)?),
                _ => None,
            };
            if let Some(supplied) = supplied {
                let arguments = generics.params.iter().map(|parameter| match parameter {
                    GenericParam::Lifetime(parameter) => parameter.lifetime.to_token_stream(),
                    GenericParam::Type(parameter) => parameter.ident.to_token_stream(),
                    GenericParam::Const(parameter) => parameter.ident.to_token_stream(),
                });
                return Ok(quote_spanned! { *site =>
                    #(#carried)* type #name #generics = #supplied<#(#arguments),*> #where_clause;
                });
            }
            let parameters = spelling::spelled(&assoc.generics);
            let subject = self.of.subject;
            let message = if subject.has_one_target() {
                // The field is reached through a field or a method.
                format!("`{name}` is a generic associated type, and the field of `{}` is reached through a field or a method, so its type cannot be named to supply `{name}`; write `type {name}{parameters} = ..;` in a `#[whichever::dispatch] {block_header}` block", subject.ident())
            } else {
                format!("`{name}` is a generic associated type, which only the enum can declare; write `type {name}{parameters} = ..;` in a `#[whichever::dispatch] {block_header}` block")
            };
            let error = match self.of.unchosen {
                Some(error) => error.clone(),
                None => Error::new_spanned(assoc, message),
            }
            .to_compile_error();
            return Ok(quote_spanned! { *site =>
                #(#carried)* type #name #generics = [(); { #error; 0 }] #where_clause;
            });
        }
        let cfgs = carried
            .iter()
            .filter(|attribute| attribute.path().is_ident("cfg"));
        let ty = match options.override_for(name) {
            Some(fixed) => fixed.as_type()?.into_token_stream(),
            None => {
                // The agreement check cannot name an item that may be
                // compiled out.
                if cfgs.count() == 0 {
                    self.agreed_types.push(name);
                }
                match self.of.supplied(name) {
                    Ok(item) => item,
                    // An array whose length is an error is a type rustc takes
                    // as erroneous: nothing that names the item is checked
                    // further, so the impl is complete, the calls around it
                    // resolve, and the sentence stands alone.
                    Err(error) => {
                        let error = error.to_compile_error();
                        quote_spanned! { *site => [(); { #error; 0 }] }
                    }
                }
            }
        };
        Ok(quote_spanned! { *site => #(#carried)* type #name = #ty; })
    }

    /// Writes `const NAME: T = ..;`: the override, or the supplier's value with
    /// `NAME` recorded for the agreement check. The item's own tokens carry
    /// `site`.
    fn associated_const(&mut self, constant: &'a TraitItemConst) -> Result<TokenStream2> {
        let Impl { options, site, .. } = *self.of;
        let name = &constant.ident;
        let mut carried = carried_attributes(&constant.attrs);
        if is_deprecated(&constant.attrs) {
            carried.push(allow_deprecated(site));
        }
        let cfgs = carried
            .iter()
            .filter(|attribute| attribute.path().is_ident("cfg"));
        let mut ty = constant.ty.clone();
        self.substitution.visit_type_mut(&mut ty);
        self.substitution.check()?;
        let value = match options.override_for(name) {
            Some(fixed) => fixed.as_value()?.into_token_stream(),
            None => {
                if cfgs.count() == 0 {
                    self.agreed_consts.push((name, ty.clone()));
                }
                // A value that is an error is of any type, as a method's body
                // that is one is.
                match self.of.supplied(name) {
                    Ok(item) => item,
                    Err(error) => {
                        let error = error.to_compile_error();
                        quote_spanned! { *site => { #error } }
                    }
                }
            }
        };
        Ok(quote_spanned! { *site => #(#carried)* const #name: #ty = #value; })
    }

    /// Writes the forwarding method for one trait method: a `match` over every
    /// variant with a fully qualified call per arm, and the mismatch expression
    /// for two different variants when the method takes `Self` arguments. A
    /// method without a receiver is left to its default body. A method that
    /// cannot be forwarded is still written, with its sentence as its body, so
    /// that the impl stays complete, the calls around it resolve, and the one
    /// error stands alone; only a signature that cannot be written at all, a
    /// projection the substitution could not qualify, is an error of the impl.
    fn method(&mut self, method: Method, forwarded: Option<&Forwarded>) -> Result<TokenStream2> {
        // A block method's signature is the user's, written where the enum is,
        // and needs no substitution.
        let (signature, renaming) = match forwarded {
            Some(forwarded) => (forwarded.sig.clone(), Renaming::default()),
            None => {
                let mut signature = method.function.sig.clone();
                let renaming = self.substitution.distinguish(&mut signature);
                self.substitution.visit_signature_mut(&mut signature);
                self.substitution.check()?;
                (signature, renaming)
            }
        };
        let carried = || match forwarded {
            Some(forwarded) => forwarded.attrs.clone(),
            None => method.carried_attributes(),
        };
        // A struct that chose no field for the trait has nothing to forward
        // to, and its sentence is every forwarded method's.
        if let Some(error) = self.of.unchosen {
            return Ok(error_body(&signature, carried(), error.clone()));
        }
        let written = forward_trait_method(self.of, method, signature.clone(), forwarded, renaming);
        Ok(written.unwrap_or_else(|error| error_body(&signature, carried(), error)))
    }

    /// Writes a trait method that cannot be forwarded as the impl writes it:
    /// the trait's signature, substituted, with `error`'s sentence as its body.
    /// Only a signature the substitution cannot write is an error of the impl.
    fn rejected_method(&mut self, method: Method, error: Error) -> Result<TokenStream2> {
        let mut signature = method.function.sig.clone();
        self.substitution.distinguish(&mut signature);
        self.substitution.visit_signature_mut(&mut signature);
        self.substitution.check()?;
        let carried = method.carried_attributes();
        Ok(error_body(&signature, carried, error))
    }
}

/// Writes `signature` with `error`'s sentence as its body: rustc treats a
/// `compile_error!` as an error expression and type-checks no further, so
/// the impl is complete, the calls around it resolve, and the sentence
/// stands alone.
fn error_body(signature: &Signature, carried: Vec<Attribute>, error: Error) -> TokenStream2 {
    let error = error.to_compile_error();
    let span = at(signature.ident.span());
    quote_spanned! { span => #(#carried)* #signature { #error } }
}

/// Writes the forwarding method for a trait method whose signature is
/// written, or the sentence for the shape that stops it.
fn forward_trait_method(
    of: &Impl,
    declared: Method,
    signature: Signature,
    forwarded: Option<&Forwarded>,
    mut renaming: Renaming,
) -> Result<TokenStream2> {
    let Impl {
        entries,
        subject,
        subject_path,
        trait_path,
        options,
        block_header,
        site,
        ..
    } = *of;
    let function = declared.function;
    let method = signature.ident.clone();
    // A method the trait gives a body can be left to it; a restatement
    // lists its methods without one, and they count as required.
    let has_default = declared.has_default();
    let has_receiver = signature.receiver().is_some();
    // An enum has no variant to ask before it has a value: a method
    // without a receiver is left to its default, or reported.
    if !has_receiver && !subject.has_one_target() {
        if has_default {
            return Ok(TokenStream2::new());
        }
        return Err(Error::new_spanned(
            &function.sig,
            format!("`{method}` has no receiver, so there is no variant to forward to; give it a default body, or write it in a `#[whichever::dispatch] {block_header}` block"),
        ));
    }
    // The enum's `mismatch` wins over the trait method's, and the names the
    // fallback arm binds resolve where the chosen expression was written.
    // The trait's own expression is written against the method's
    // parameters as the trait names them, and follows their renaming.
    let (mismatch, resolution) = match options.mismatch_for(&method) {
        Some(expr) => (Some(expr.clone()), Some(expr.span())),
        None => {
            let mut mismatch = declared.mismatch()?;
            if let Some(expression) = &mut mismatch {
                renaming.expression(expression);
            }
            (mismatch, None)
        }
    };
    let request = Forward {
        signature,
        origin: Origin::Trait {
            method: TraitMethod {
                trait_path,
                subject_path,
                declared: &function.sig,
                has_default,
                block_header,
                options,
                mismatch,
                resolution,
                derived: of.block.is_none(),
            },
            mark: forwarded.map(|forwarded| Mark {
                options: &forwarded.forwarding,
                body: forwarded.body.as_ref(),
            }),
        },
        entries,
        subject,
        site,
        body_site: Site::at(method.span()),
    };
    // A struct has one target, whose type it names: a method without a
    // receiver is that type's, the provided ones included, so that what
    // the field's type overrides is what runs. One the type cannot be
    // asked is left to its default, or reported.
    let generated = if has_receiver {
        forward(request)?
    } else {
        match forward_associated(request)? {
            Associated::Forwarded(generated) => generated,
            Associated::Left(_) if has_default => return Ok(TokenStream2::new()),
            Associated::Left(error) => return Err(error),
        }
    };
    let Generated {
        signature,
        allowances,
        body,
    } = generated;
    // The method's own `#[inline]`, the block's or the trait's, or ours.
    let own_attrs: &[Attribute] = match forwarded {
        Some(forwarded) => &forwarded.attrs,
        None => &function.attrs,
    };
    let inline = own_attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("inline"))
        .map(ToTokens::to_token_stream)
        .unwrap_or_else(|| quote_spanned! { *site => #[inline] });
    let mut carried: Vec<Attribute> = match forwarded {
        Some(forwarded) => forwarded
            .attrs
            .iter()
            .filter(|attribute| !attribute.path().is_ident("inline"))
            .cloned()
            .collect(),
        None => declared.carried_attributes(),
    };
    // A deprecated method is forwarded under `#[allow(deprecated)]`, as a
    // hand writes it; the deprecation reaches the callers through the
    // trait's item.
    if declared.is_deprecated() {
        carried.push(allow_deprecated(site));
    }
    Ok(quote_spanned! { *site =>
        #(#carried)*
        #inline
        #allowances
        #signature #body
    })
}
