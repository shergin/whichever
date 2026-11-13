//! `implement!`: the forwarding impl of one trait for one subject.
use std::collections::hash_map::Entry as MapEntry;
use std::collections::HashMap;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{
    parse_quote_spanned, Attribute, Block, Error, Expr, Generics, Ident, ImplItem, ItemImpl,
    ItemTrait, Meta, Path, Result, Signature, Token, TraitItem, TraitItemConst, TraitItemFn,
    TraitItemType, Type,
};

use crate::agreement::Agreement;
use crate::forwarding::{
    ArgumentNames, Arm, ArmContext, Callee, Forwarding, MethodPlan, SelfArguments,
};
use crate::options::{
    closest, is_dispatch_attribute, listed, method_mismatch, ForwardingOptions, Override, TraitSpec,
};
use crate::protocol::ImplementInput;
use crate::selection::{check_body, one_attribute, select_targets, BodylessMethod, Selection};
use crate::shape::{takes_self_argument, ReturnSite};
use crate::subject::{
    allow_deprecated, carried_attributes, cfg_attributes, is_deprecated, shape_errors, targets,
    Entry, Subject, Target,
};
use crate::substitution::Substitution;
use crate::{at, combined};

pub(crate) fn expand_implement(input: ImplementInput) -> Result<TokenStream2> {
    let ImplementInput {
        item_trait,
        remote,
        provided,
        subject,
        subject_remote,
        mut spec,
        partial,
    } = input;
    let Some(spec_path) = spec.path.clone() else {
        return Err(Error::new(
            Span::call_site(),
            "the trait to implement is missing",
        ));
    };
    // A restatement names the methods it forwards and nothing else, so a
    // method left to the default is left to the real trait's, which the
    // restatement cannot see; rustc reports one the trait does not provide.
    let restated = remote.is_some();
    let implemented = implemented_path(remote, &spec_path);
    let trait_path = &implemented;
    let trait_ident = &trait_path
        .segments
        .last()
        .expect("a path has at least one segment")
        .ident;
    let header = ImplHeader::new(
        partial.as_ref(),
        &subject,
        subject_remote.as_ref(),
        &item_trait,
        &mut spec,
    )?;
    // Patterns and values name the subject by its real path when it is a
    // restatement, and by its own name otherwise.
    let subject_path = subject_remote
        .clone()
        .unwrap_or_else(|| Path::from(subject.ident().clone()));
    let ImplHeader {
        generics,
        self_ty,
        attrs: impl_attrs,
        unsafety,
        written,
        methods,
        user_items,
    } = &header;
    // Every token written here carries the subject's span: the call site is
    // the trait's attribute, which may be in another crate.
    let site = at(subject.ident().span());
    let derived = partial
        .is_none()
        .then(|| quote_spanned!(site => #[automatically_derived]));
    // The partial impl block a message points at, spelled with the trait as
    // the subject names it and the subject's own generics.
    let block_header = {
        let (impl_generics, ty_generics, _) = subject.generics().split_for_impl();
        spelled(
            &quote_spanned!(site => impl #impl_generics #spec_path for #subject_path #ty_generics),
        )
    };
    let mut entries = subject.entries_for(Some(&spec_path))?;
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
    let route_errors = apply_routes(&spec, &mut entries, &subject);
    let variants = targets(&entries);
    // The bounds a hand writes: a payload that is a type parameter of the
    // subject gets `Param: Trait` on the impl, which the arms need and
    // nothing else supplies. A partial block's header is the user's and is
    // written as it is.
    let generics = match partial {
        Some(_) => (*generics).clone(),
        None => with_payload_bounds(generics, &variants, subject.generics(), trait_path),
    };
    let generics = &generics;
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    if variants.is_empty() {
        if entries.is_empty() {
            return Err(Error::new_spanned(
                &subject,
                "a dispatch enum needs at least one variant",
            ));
        }
        // Every variant was rejected, each with its own sentence; an impl
        // with nothing to forward to would only add rustc's errors to them.
        return Ok(match &partial {
            Some(_) => shape_errors(&entries),
            None => TokenStream2::new(),
        });
    }
    // The associated items come from the first variant that is always
    // compiled in and whose target has a type to name; one under `#[cfg]`
    // may not be there to supply them, and one reached through a field or
    // a method has no type to project from.
    let first = variants
        .iter()
        .find(|target| target.cfgs.is_empty() && target.target_type().is_some())
        .copied();
    let supplier = Supplier {
        first,
        subject: subject.ident(),
        trait_path,
    };
    // Every item of the trait is planned and written in turn; every item
    // that cannot be forwarded is reported, not only the first.
    let mut writer = ItemWriter {
        context: MethodContext {
            entries: &entries,
            subject: &subject,
            subject_path: &subject_path,
            trait_path,
            spec: &spec,
            block_header: &block_header,
            provided: &provided,
        },
        supplier: &supplier,
        written,
        methods,
        trait_ident,
        restated,
        substitution: Substitution::of_trait(&item_trait, trait_path, generics)?,
        site,
        items: TokenStream2::new(),
        agreed_types: vec![],
        agreed_consts: vec![],
        deprecated_items: false,
        plain_forwarding: false,
        errors: vec![],
    };
    for item in &item_trait.items {
        writer.write(item);
    }
    let ItemWriter {
        items,
        agreed_types,
        agreed_consts,
        deprecated_items,
        plain_forwarding,
        errors,
        ..
    } = writer;
    // An option naming nothing in the trait, or a route through no variant,
    // is reported beside the impl, which is right without it; only an item
    // that cannot be written withholds the impl.
    let mut reported = route_errors;
    reported.extend(unknown_names(
        &spec,
        &item_trait,
        trait_ident,
        &subject,
        partial.is_none(),
    ));
    if let Some(error) = combined(errors) {
        return Err(error);
    }
    let reported = combined(reported).map(|error| error.to_compile_error());
    // The checks live outside the impl, where a `Self` in a payload type
    // has to be spelled out as the implementing type.
    let checks = first.map(|first| {
        let first_outside = first
            .target_type_outside(self_ty)
            .expect("the supplier is chosen among the targets with a type");
        let agreement = Agreement {
            variants: &variants,
            first,
            trait_path,
            first_payload: &first_outside,
            generics,
            self_type: self_ty,
            site,
            deprecated: deprecated_items,
        };
        let types = agreement.types(&agreed_types);
        let consts = agreement.consts(&agreed_consts);
        quote_spanned! { site => #types #consts }
    });
    // A partial block is the only dispatch of its trait, so a variant that
    // cannot be forwarded to is reported here, under its own `cfg`, once
    // some method forwards to every variant; a block whose every method
    // names its variants leaves such a variant to the bodies.
    let shape_errors = match &partial {
        Some(_) if plain_forwarding => shape_errors(&entries),
        _ => TokenStream2::new(),
    };
    Ok(quote_spanned! { site =>
        #(#impl_attrs)*
        #derived
        #unsafety impl #impl_generics #trait_path for #self_ty #where_clause {
            #items
            #(#user_items)*
        }
        #checks
        #shape_errors
        #reported
    })
}

/// Applies the trait's own routes, `via(Variant = ..)`, to the entries they
/// name, and returns the error for each that names no variant.
fn apply_routes(spec: &TraitSpec, entries: &mut [Entry], subject: &Subject) -> Vec<Error> {
    let mut errors = vec![];
    for (variant, route) in &spec.routes {
        let target = entries.iter_mut().find_map(|entry| match entry {
            Entry::Target(target) if target.ident == *variant => Some(target),
            _ => None,
        });
        match target {
            Some(target) => target.route = route.clone(),
            None => errors.push(Error::new(
                variant.span(),
                format!(
                    "`{}` has no variant `{variant}` to route through",
                    subject.ident()
                ),
            )),
        }
    }
    errors
}

/// Returns the errors for the options naming nothing in the trait: an
/// override of an associated item it does not have, or a `mismatch` for a
/// method without a `Self` argument. Each is a typo that would otherwise
/// turn into a type error somewhere else, so the message says what was
/// probably meant, or what there is to mean.
fn unknown_names(
    spec: &TraitSpec,
    item_trait: &ItemTrait,
    trait_ident: &Ident,
    subject: &Subject,
    on_subject: bool,
) -> Vec<Error> {
    let mut errors = vec![];
    let associated: Vec<String> = item_trait
        .items
        .iter()
        .filter_map(|item| match item {
            TraitItem::Type(assoc) => Some(assoc.ident.to_string()),
            TraitItem::Const(constant) => Some(constant.ident.to_string()),
            _ => None,
        })
        .collect();
    for fixed in &spec.overrides {
        let name = fixed.name.to_string();
        if associated.contains(&name) {
            continue;
        }
        let hint = unknown_override_hint(&name, &associated, subject, on_subject);
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
    errors
}

/// One thing an item of the trait becomes in the impl, decided before
/// anything is written. An item the partial block writes in every build,
/// or leaves to the trait's default, has no plan; a method the block
/// writes or marks under `#[cfg]` has one per way it is written.
enum ItemPlan<'a> {
    /// A method marked `#[dispatch(default)]` on a trait that gives it no
    /// default: written with the sentence as its body, under the `#[cfg]`
    /// of the builds the mark is for.
    Rejected {
        function: &'a TraitItemFn,
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
        function: &'a TraitItemFn,
        forwarded: Option<&'a Forwarded>,
        complement: Option<TokenStream2>,
    },
    /// A shape the impl cannot write.
    Limit(Error),
}

/// Writes the items of one trait's impl: plans each item of the trait, then
/// renders the plan, keeping what the agreement checks and the shape errors
/// need to know afterwards.
struct ItemWriter<'a> {
    context: MethodContext<'a>,
    supplier: &'a Supplier<'a>,
    /// The items the partial block writes, by name, and in which builds.
    written: &'a HashMap<String, Written>,
    /// The methods the partial block marks with `#[dispatch(..)]`, by name.
    methods: &'a HashMap<String, Vec<BlockMethod>>,
    trait_ident: &'a Ident,
    /// Whether the trait is a restatement, whose provided methods it cannot
    /// see.
    restated: bool,
    substitution: Substitution,
    /// The span the items' own tokens carry.
    site: Span,
    items: TokenStream2,
    /// The associated types taken from the first variant, for the check.
    agreed_types: Vec<&'a Ident>,
    /// The associated consts taken from the first variant, with their
    /// types, for the check.
    agreed_consts: Vec<(&'a Ident, Type)>,
    /// Whether the trait deprecates an associated item, which the checks
    /// name and so allow.
    deprecated_items: bool,
    /// Whether some method with a receiver is forwarded to every variant,
    /// so that a variant no method can forward to is reported.
    plain_forwarding: bool,
    /// The items that could not be written, which withhold the impl.
    errors: Vec<Error>,
}

impl<'a> ItemWriter<'a> {
    /// Plans and writes one item of the trait.
    fn write(&mut self, item: &'a TraitItem) {
        match item {
            TraitItem::Type(assoc) => self.deprecated_items |= is_deprecated(&assoc.attrs),
            TraitItem::Const(constant) => self.deprecated_items |= is_deprecated(&constant.attrs),
            _ => {}
        }
        for plan in self.plans(item) {
            let site = self.site;
            let generated = match plan {
                ItemPlan::Rejected {
                    function,
                    error,
                    builds,
                } => rejected_method(function, &mut self.substitution, error)
                    .map(|rejected| quote_spanned! { site => #builds #rejected }),
                ItemPlan::Type(assoc) => associated_type(
                    assoc,
                    self.context.spec,
                    self.supplier,
                    self.context.block_header,
                    &mut self.agreed_types,
                    self.site,
                ),
                ItemPlan::Const(constant) => associated_const(
                    constant,
                    self.context.spec,
                    self.supplier,
                    &mut self.substitution,
                    &mut self.agreed_consts,
                    self.site,
                ),
                ItemPlan::Method {
                    function,
                    forwarded,
                    complement,
                } => {
                    self.plain_forwarding |=
                        forwarded.is_none() && function.sig.receiver().is_some();
                    expand_method(function, &self.context, &mut self.substitution, forwarded)
                        .map(|forwarded| quote_spanned! { site => #complement #forwarded })
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
        let (mut conditions, mut conditional_at) = match self.written.get(&name) {
            Some(Written::Always) => return vec![],
            Some(Written::Under { conditions, span }) => (conditions.clone(), Some(*span)),
            None => (vec![], None),
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
            (TraitItem::Fn(function), _) => function,
            _ => return vec![],
        };
        let marked = self
            .methods
            .get(&name)
            .map(Vec::as_slice)
            .unwrap_or_default();
        for method in marked {
            let cfgs = method.cfgs();
            if !cfgs.is_empty() {
                conditional_at.get_or_insert(method.span());
                conditions.push(cfgs);
            }
        }
        // The builds the block's conditional items leave.
        let rest = conditional_at.map(|span| not_all_of(&conditions, at(span)));
        let site = self.site;
        let mut plans = vec![];
        // Whether a mark without a `#[cfg]` takes those builds.
        let mut taken = false;
        for method in marked {
            let cfgs = method.cfgs();
            taken |= cfgs.is_empty();
            match method {
                BlockMethod::Default { span, .. } => {
                    let provided = self.restated
                        || function.default.is_some()
                        || self.context.provided.contains(&function.sig.ident);
                    if provided {
                        continue;
                    }
                    // Written with the sentence as its body, as a method
                    // that cannot be forwarded is, so that the impl stays
                    // complete.
                    plans.push(ItemPlan::Rejected {
                        function,
                        error: Error::new(
                            *span,
                            format!("`{}` has no default body in `{}` to be left to; give it a body, or forward it", function.sig.ident, self.trait_ident),
                        ),
                        builds: if cfgs.is_empty() {
                            rest.clone()
                        } else {
                            Some(quote_spanned! { site => #(#cfgs)* })
                        },
                    });
                }
                BlockMethod::Forwarded(forwarded) => plans.push(ItemPlan::Method {
                    function,
                    forwarded: Some(forwarded),
                    complement: if cfgs.is_empty() { rest.clone() } else { None },
                }),
            }
        }
        if !taken {
            plans.push(ItemPlan::Method {
                function,
                forwarded: None,
                complement: rest,
            });
        }
        plans
    }
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

/// Returns `generics` with `Param: Trait` added for every target whose
/// payload is the type parameter `Param` of `declared`, each bound once.
fn with_payload_bounds(
    generics: &Generics,
    variants: &[&Target],
    declared: &Generics,
    trait_path: &Path,
) -> Generics {
    let mut generics = generics.clone();
    let mut bounded: Vec<&Ident> = vec![];
    for target in variants {
        let Some(parameter) = target.bounded_parameter(declared) else {
            continue;
        };
        if bounded.contains(&parameter) {
            continue;
        }
        bounded.push(parameter);
        let span = at(parameter.span());
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote_spanned!(span => #parameter: #trait_path));
    }
    generics
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
    /// The methods the block marks with `#[dispatch(..)]`, by name; one
    /// name may be marked once per `#[cfg]`.
    methods: HashMap<String, Vec<BlockMethod>>,
    user_items: Vec<&'a ImplItem>,
}

/// A method of a partial block marked `#[dispatch(..)]`.
// Short-lived and few; the size difference does not matter here.
#[allow(clippy::large_enum_variant)]
enum BlockMethod {
    /// `#[dispatch(default)]`: not forwarded and not written, so the
    /// trait's default body stands, in the builds the method's own `#[cfg]`
    /// attributes name.
    Default { span: Span, cfgs: Vec<Attribute> },
    /// `#[dispatch(Variant, ..)]` or `#[dispatch(*)]`: forwarded to the
    /// named variants, with the body for the rest.
    Forwarded(Forwarded),
}

impl BlockMethod {
    /// Returns the `#[cfg]` attributes the method is written under. Like an
    /// item the block writes, it is the block's where they hold, and the
    /// trait's method is forwarded to every variant where they do not.
    fn cfgs(&self) -> Vec<Attribute> {
        match self {
            BlockMethod::Default { cfgs, .. } => cfgs.clone(),
            BlockMethod::Forwarded(forwarded) => cfg_attributes(&forwarded.attrs),
        }
    }

    /// Returns the span a message about the method's builds carries.
    fn span(&self) -> Span {
        match self {
            BlockMethod::Default { span, .. } => *span,
            BlockMethod::Forwarded(forwarded) => forwarded.sig.ident.span(),
        }
    }
}

/// What a forwarded block method is written from: the user's own
/// signature and attributes, the body for the variants not named, and the
/// names.
struct Forwarded {
    attrs: Vec<Attribute>,
    sig: Signature,
    body: Option<Block>,
    forwarding: ForwardingOptions,
}

impl Forwarded {
    /// Reads a method's `#[dispatch(..)]`: `default`, or the variants and
    /// the options an inherent method takes, of which only the names and
    /// `via` apply to a trait method.
    fn read(
        attribute: Attribute,
        mut attrs: Vec<Attribute>,
        sig: Signature,
        body: Option<Block>,
    ) -> Result<BlockMethod> {
        if let Meta::List(list) = &attribute.meta {
            if list.tokens.to_string() == "default" {
                if let Some(body) = &body {
                    return Err(Error::new_spanned(
                        body,
                        format!("`{}` is left to the trait's default, so it takes no body; end the signature with `;`", sig.ident),
                    ));
                }
                return Ok(BlockMethod::Default {
                    span: attribute.span(),
                    cfgs: cfg_attributes(&attrs),
                });
            }
        }
        let forwarding = one_attribute(&mut attrs, &attribute)?;
        let inherent_only = [
            forwarding.call.as_ref().map(|_| "`call`"),
            forwarding.with.as_ref().map(|_| "`with`"),
            forwarding.field.as_ref().map(|_| "`field`"),
            forwarding.value.then_some("`value`"),
        ];
        if let Some(option) = inherent_only.into_iter().flatten().next() {
            return Err(Error::new_spanned(
                &attribute,
                format!("{option} is an option of an inherent method; a trait method is called by its own name on every variant it names, and takes `via` only"),
            ));
        }
        Ok(BlockMethod::Forwarded(Forwarded {
            attrs,
            sig,
            body,
            forwarding,
        }))
    }
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
/// them. The attribute's own tokens carry `span`, the block item's.
fn not_all_of(conditions: &[Vec<Attribute>], span: Span) -> TokenStream2 {
    let predicates = conditions.iter().map(|cfgs| {
        let predicates = cfgs.iter().map(|attribute| match &attribute.meta {
            syn::Meta::List(list) => list.tokens.clone(),
            _ => TokenStream2::new(),
        });
        quote_spanned!(span => all(#(#predicates),*))
    });
    quote_spanned!(span => #[cfg(not(any(#(#predicates),*)))])
}

impl<'a> ImplHeader<'a> {
    fn new(
        partial: Option<&'a ItemImpl>,
        subject: &'a Subject,
        remote: Option<&Path>,
        item_trait: &ItemTrait,
        spec: &mut TraitSpec,
    ) -> Result<Self> {
        let Some(block) = partial else {
            let (_, ty_generics, _) = subject.generics().split_for_impl();
            let path = remote
                .cloned()
                .unwrap_or_else(|| Path::from(subject.ident().clone()));
            let site = at(subject.ident().span());
            return Ok(Self {
                generics: subject.generics(),
                self_ty: parse_quote_spanned!(site => #path #ty_generics),
                attrs: &[],
                // An `unsafe trait` is implemented by an `unsafe impl`.
                unsafety: item_trait.unsafety,
                written: HashMap::new(),
                methods: HashMap::new(),
                user_items: vec![],
            });
        };
        let mut written: HashMap<String, Written> = HashMap::new();
        let mut methods: HashMap<String, Vec<BlockMethod>> = HashMap::new();
        let mut user_items = vec![];
        for item in &block.items {
            let (name, attrs) = match item {
                // A method marked `#[dispatch(..)]` is the engine's to write.
                ImplItem::Fn(function) if function.attrs.iter().any(is_dispatch_attribute) => {
                    let mut attrs = function.attrs.clone();
                    let position = attrs
                        .iter()
                        .position(is_dispatch_attribute)
                        .expect("checked above");
                    let attribute = attrs.remove(position);
                    let method = Forwarded::read(
                        attribute,
                        attrs,
                        function.sig.clone(),
                        Some(function.block.clone()),
                    )?;
                    methods
                        .entry(function.sig.ident.to_string())
                        .or_default()
                        .push(method);
                    continue;
                }
                // A method without a body is verbatim to `syn`; it is only
                // ever the engine's.
                ImplItem::Verbatim(tokens) => {
                    let Ok(method) = syn::parse2::<BodylessMethod>(tokens.clone()) else {
                        user_items.push(item);
                        continue;
                    };
                    let BodylessMethod {
                        mut attrs,
                        vis: _,
                        sig,
                    } = method;
                    let Some(position) = attrs.iter().position(is_dispatch_attribute) else {
                        return Err(Error::new_spanned(
                            &sig,
                            format!("`{}` has no body; forward it with `#[dispatch(*)]` or `#[dispatch(Variant, ..)]`, or leave it to the trait's default with `#[dispatch(default)]`", sig.ident),
                        ));
                    };
                    let attribute = attrs.remove(position);
                    let name = sig.ident.to_string();
                    methods
                        .entry(name)
                        .or_default()
                        .push(Forwarded::read(attribute, attrs, sig, None)?);
                    continue;
                }
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
            user_items.push(item);
        }
        Ok(Self {
            generics: &block.generics,
            self_ty: (*block.self_ty).clone(),
            attrs: &block.attrs,
            unsafety: block.unsafety,
            written,
            methods,
            user_items,
        })
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
    /// Returns `<First as Trait>::name`, spelled at the first variant's
    /// payload so that an error about the projection lands there, or the
    /// error for an enum whose every variant is under `#[cfg]`.
    fn item(&self, name: &Ident) -> Result<TokenStream2> {
        let Some(first) = self.first else {
            return Err(Error::new_spanned(
                self.subject,
                format!("every variant of `{}` is under `#[cfg]` or reached through a field or a method, so none can supply `{name}` in every build; fix it on the enum with `{name} = ..`", self.subject),
            ));
        };
        let payload = first
            .target_type()
            .expect("the supplier is chosen among the targets with a type");
        let trait_path = self.trait_path;
        let span = at(first.payload.span());
        Ok(quote_spanned!(span => <#payload as #trait_path>::#name))
    }
}

/// Writes `type Name = ..;`: the override, or the supplier's type with
/// `Name` recorded for the agreement check. A generic associated type has
/// no supplier: the enum declares it in a partial impl block, which is
/// where one the block writes was taken out of this path. The item's own
/// tokens carry `site`.
fn associated_type<'a>(
    assoc: &'a TraitItemType,
    spec: &TraitSpec,
    supplier: &Supplier,
    block_header: &str,
    agreed: &mut Vec<&'a Ident>,
    site: Span,
) -> Result<TokenStream2> {
    let name = &assoc.ident;
    if !assoc.generics.params.is_empty() {
        let parameters = spelled(&assoc.generics);
        return Err(Error::new_spanned(
            assoc,
            format!("`{name}` is a generic associated type, which only the enum can declare; write `type {name}{parameters} = ..;` in a `#[whichever::dispatch] {block_header}` block"),
        ));
    }
    let mut carried = carried_attributes(&assoc.attrs);
    if is_deprecated(&assoc.attrs) {
        carried.push(allow_deprecated(site));
    }
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
    Ok(quote_spanned! { site => #(#carried)* type #name = #ty; })
}

/// Writes `const NAME: T = ..;`: the override, or the supplier's value with
/// `NAME` recorded for the agreement check. The item's own tokens carry
/// `site`.
fn associated_const<'a>(
    constant: &'a TraitItemConst,
    spec: &TraitSpec,
    supplier: &Supplier,
    substitution: &mut Substitution,
    agreed: &mut Vec<(&'a Ident, Type)>,
    site: Span,
) -> Result<TokenStream2> {
    let name = &constant.ident;
    let mut carried = carried_attributes(&constant.attrs);
    if is_deprecated(&constant.attrs) {
        carried.push(allow_deprecated(site));
    }
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
    Ok(quote_spanned! { site => #(#carried)* const #name: #ty = #value; })
}

/// Writes the forwarding method for one trait method: a `match` over every
/// variant with a fully qualified call per arm, and the mismatch expression
/// for two different variants when the method takes `Self` arguments. A
/// method without a receiver is left to its default body. A method that
/// cannot be forwarded is still written, with its sentence as its body, so
/// that the impl stays complete, the calls around it resolve, and the one
/// error stands alone; only a signature that cannot be written at all, a
/// projection the substitution could not qualify, is an error of the impl.
fn expand_method(
    function: &TraitItemFn,
    context: &MethodContext,
    substitution: &mut Substitution,
    forwarded: Option<&Forwarded>,
) -> Result<TokenStream2> {
    // A block method's signature is the user's, written where the enum is,
    // and needs no substitution.
    let signature = match forwarded {
        Some(forwarded) => forwarded.sig.clone(),
        None => {
            let mut signature = function.sig.clone();
            substitution.visit_signature_mut(&mut signature);
            substitution.check()?;
            signature
        }
    };
    let written = forward_trait_method(function, signature.clone(), context, forwarded);
    Ok(written.unwrap_or_else(|error| {
        let carried = match forwarded {
            Some(forwarded) => forwarded.attrs.clone(),
            None => carried_attributes(&function.attrs),
        };
        error_body(&signature, carried, error)
    }))
}

/// Reports a method that requires `Self: Sized` forwarded to a trait object,
/// which cannot take it; a hand leaves such a method to the trait's
/// default, and the partial block says so with `#[dispatch(default)]`.
fn check_trait_objects(
    plan: &MethodPlan,
    selection: Option<&Selection>,
    entries: &[Entry],
    function: &TraitItemFn,
    block_header: &str,
) -> Result<()> {
    if !plan.requires_sized() {
        return Ok(());
    }
    let forwarded_to: Vec<&Target> = match selection {
        Some(selection) => selection.targets.iter().collect(),
        None => targets(entries),
    };
    let Some(target) = forwarded_to.iter().find(|target| target.is_trait_object()) else {
        return Ok(());
    };
    let method = &function.sig.ident;
    Err(Error::new_spanned(
        &function.sig,
        format!("`{method}` requires `Self: Sized`, and `{}` holds a trait object, which is not; leave it to the trait's default with `#[dispatch(default)] fn {method}(..);` in a `#[whichever::dispatch] {block_header}` block, or implement the method by hand", target.ident),
    ))
}

/// The arm for everything the targets' arms leave, and whether the lint
/// about an unreachable pattern is allowed on the `match`.
struct Fallback {
    arm: Option<Arm>,
    allow_unreachable: bool,
}

/// Decides the fallback arm of a trait method. A block method's body is the
/// arm for every variant it does not name, the mismatch of two different
/// variants included, so it needs no `mismatch`; under `*` without a body
/// the trait's rule applies. Two values of a struct always hold the same
/// field, so only an enum needs the arm for two different variants, and an
/// enum with one variant that is always compiled in has no second variant
/// to meet: a hand writes no fallback arm there, and neither does the
/// engine. The arm stays unreachable in some builds when a variant is under
/// `cfg`, which is the one case the lint is allowed for.
fn fallback_for(
    plan: &MethodPlan,
    entries: &[Entry],
    subject: &Subject,
    forwarded: Option<&Forwarded>,
    mismatch: Option<Expr>,
    function: &TraitItemFn,
    context: &ArmContext,
) -> Result<Fallback> {
    let always_present = entries
        .iter()
        .filter(|entry| matches!(entry, Entry::Target(target) if target.cfgs.is_empty()))
        .count();
    let conditional = entries.iter().any(|entry| match entry {
        Entry::Target(target) => !target.cfgs.is_empty(),
        Entry::Unforwardable(_) => true,
    });
    if let Some(body) = forwarded.and_then(|forwarded| forwarded.body.as_ref()) {
        return Ok(Fallback {
            arm: Some(Arm::rest(body.to_token_stream(), context.site)),
            allow_unreachable: false,
        });
    }
    let needs_fallback =
        plan.has_same_variant() && subject.can_mismatch() && (always_present >= 2 || conditional);
    if !needs_fallback {
        return Ok(Fallback {
            arm: None,
            allow_unreachable: false,
        });
    }
    let method = &function.sig.ident;
    let Some(expr) = mismatch else {
        return Err(Error::new_spanned(
            &function.sig,
            format!("`{method}` takes `Self` arguments; give the result for two different variants with `#[dispatch(mismatch = <expr>)]` on the trait method or `mismatch({method} = <expr>)` on the enum"),
        ));
    };
    Ok(Fallback {
        arm: Some(plan.fallback(expr.to_token_stream(), context.site)),
        allow_unreachable: forwarded.is_none() && always_present < 2,
    })
}

/// Writes a trait method that cannot be forwarded as the impl writes it:
/// the trait's signature, substituted, with `error`'s sentence as its body.
/// Only a signature the substitution cannot write is an error of the impl.
fn rejected_method(
    function: &TraitItemFn,
    substitution: &mut Substitution,
    error: Error,
) -> Result<TokenStream2> {
    let mut signature = function.sig.clone();
    substitution.visit_signature_mut(&mut signature);
    substitution.check()?;
    let carried = carried_attributes(&function.attrs);
    Ok(error_body(&signature, carried, error))
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

/// What every method of one trait's impl is written against.
struct MethodContext<'a> {
    entries: &'a [Entry],
    subject: &'a Subject,
    /// The subject's real path, for patterns and values: its own name, or
    /// the path a restatement names.
    subject_path: &'a Path,
    trait_path: &'a Path,
    spec: &'a TraitSpec,
    /// The partial impl block a message points at, spelled out.
    block_header: &'a str,
    /// The methods the trait gives a default body, as the carrier names
    /// them; a carrier from before that segment carries the bodies.
    provided: &'a [Ident],
}

/// Writes the forwarding method for a trait method whose signature is
/// written, or the sentence for the shape that stops it.
fn forward_trait_method(
    function: &TraitItemFn,
    mut signature: Signature,
    context: &MethodContext,
    forwarded: Option<&Forwarded>,
) -> Result<TokenStream2> {
    let MethodContext {
        entries,
        subject,
        subject_path,
        trait_path,
        spec,
        block_header,
        provided,
    } = *context;
    let subject_ident = subject.ident();
    let method = signature.ident.clone();
    // A method the trait gives a body can be left to it; a restatement
    // lists its methods without one, and they count as required.
    let has_default = function.default.is_some() || provided.contains(&method);
    if signature.receiver().is_none() {
        if has_default {
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
        prefix: subject_path.clone(),
        lift: None,
        site: at(subject_ident.span()),
    };
    // The enum's `mismatch` wins over the trait method's, and the names the
    // fallback arm binds resolve where the chosen expression was written.
    let (mismatch, resolution) = match spec.mismatch_for(&method) {
        Some(expr) => (Some(expr.clone()), Some(expr.span())),
        None => (method_mismatch(function)?, None),
    };
    // A block method's parameter names are the user's; the trait's are
    // kept where they are plain identifiers and generated otherwise.
    let names = match forwarded {
        Some(_) => ArgumentNames::Written,
        None => ArgumentNames::WrittenOrGenerated,
    };
    let plan = MethodPlan::new(
        &mut signature,
        names,
        SelfArguments::Dispatch,
        Some(spec),
        &ReturnSite {
            subject: subject_ident,
            one_target: subject.has_one_target(),
            by_hand: context.callee.by_hand(),
            default: has_default.then_some((&method, block_header)),
        },
        resolution,
    )?
    .over(entries);
    // A block method selects its variants and gives the body for the rest;
    // the trait's attribute forwards to every variant.
    let selection = match forwarded {
        Some(forwarded) => {
            let selection = select_targets(
                &forwarded.forwarding,
                entries,
                &context.prefix,
                &method,
                subject,
            )?;
            check_body(
                &selection,
                forwarded.body.as_ref(),
                &signature,
                subject.target_noun(),
            )?;
            Some(selection)
        }
        None => None,
    };
    check_trait_objects(&plan, selection.as_ref(), entries, function, block_header)?;
    let mut arms = Vec::with_capacity(entries.len());
    match &selection {
        Some(selection) => {
            for target in &selection.targets {
                arms.push(plan.arm(target, &context)?);
            }
        }
        None => {
            for entry in entries {
                match entry {
                    Entry::Target(target) => arms.push(plan.arm(target, &context)?),
                    // The variant is reported elsewhere, under its `cfg`;
                    // this arm only keeps the `match` exhaustive while that
                    // error stands, which the fallback arm does when there
                    // is one.
                    Entry::Unforwardable(unforwardable) if !plan.has_same_variant() => {
                        let site = context.site;
                        arms.push(Arm::unforwardable(
                            &context.prefix,
                            unforwardable,
                            quote_spanned!(site => ::core::unreachable!()),
                            site,
                        ));
                    }
                    Entry::Unforwardable(_) => {}
                }
            }
        }
    }
    let Fallback {
        arm: fallback,
        allow_unreachable,
    } = fallback_for(
        &plan, entries, subject, forwarded, mismatch, function, &context,
    )?;
    if let Some(selection) = selection {
        arms.extend(selection.unforwardable_arms);
    }
    let site = context.site;
    let allow_unreachable =
        allow_unreachable.then(|| quote_spanned! { site => #[allow(unreachable_patterns)] });
    // The method's own `#[inline]`, the block's or the trait's, or ours.
    let own_attrs: &[Attribute] = match forwarded {
        Some(forwarded) => &forwarded.attrs,
        None => &function.attrs,
    };
    let inline = own_attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("inline"))
        .map(ToTokens::to_token_stream)
        .unwrap_or_else(|| quote_spanned! { site => #[inline] });
    let mut carried: Vec<Attribute> = match forwarded {
        Some(forwarded) => forwarded
            .attrs
            .iter()
            .filter(|attribute| !attribute.path().is_ident("inline"))
            .cloned()
            .collect(),
        None => carried_attributes(&function.attrs),
    };
    // A deprecated method is forwarded under `#[allow(deprecated)]`, as a
    // hand writes it; the deprecation reaches the callers through the
    // trait's item.
    if is_deprecated(&function.attrs) {
        carried.push(allow_deprecated(site));
    }
    let body = Forwarding {
        scrutinee: plan.scrutinee(context.site),
        arms,
        fallback,
        future: plan.future.clone(),
    }
    .body(at(method.span()));
    Ok(quote_spanned! { site =>
        #(#carried)*
        #inline
        #allow_unreachable
        #signature #body
    })
}

#[cfg(test)]
mod tests {
    use quote::quote;
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
            not_all_of(&[named.clone(), unix.clone()], Span::call_site()).to_string(),
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
