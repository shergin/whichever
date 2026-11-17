//! The subject an impl is written for, an enum over implementors or a struct
//! wrapping one; its entries, the variants or fields as they are read; and
//! the targets its methods forward to.
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{
    parse_quote_spanned, Attribute, Error, Field, Fields, GenericParam, Ident, Index, Item,
    ItemEnum, ItemStruct, Member, Path, Result, Type, Variant,
};

use crate::options::{is_dispatch_attribute, trait_key, Position, TargetOptions, TryFromForms};
use crate::route::Route;
use crate::shape::{bare_ident, is_self, pointee, Pointer};
use crate::site::Site;
use crate::spelling;
use crate::words::Words;

/// The item an impl is written for: an enum over implementors, or a struct
/// wrapping one.
#[derive(Clone)]
pub(crate) enum Subject {
    Enum(ItemEnum),
    Struct(ItemStruct),
}

impl Subject {
    pub(crate) fn ident(&self) -> &Ident {
        match self {
            Subject::Enum(item) => &item.ident,
            Subject::Struct(item) => &item.ident,
        }
    }

    pub(crate) fn generics(&self) -> &syn::Generics {
        match self {
            Subject::Enum(item) => &item.generics,
            Subject::Struct(item) => &item.generics,
        }
    }

    pub(crate) fn attrs_mut(&mut self) -> &mut Vec<Attribute> {
        match self {
            Subject::Enum(item) => &mut item.attrs,
            Subject::Struct(item) => &mut item.attrs,
        }
    }

    /// Returns whether two values of the subject can hold different
    /// targets, so that a method taking `Self` arguments needs an arm for
    /// the mismatch. Two values of a struct always hold the same field.
    pub(crate) fn can_mismatch(&self) -> bool {
        matches!(self, Subject::Enum(_))
    }

    /// Returns whether the subject gets `From` and `TryFrom` impls: an enum
    /// converts from and into its payloads, a struct does not.
    pub(crate) fn has_conversions(&self) -> bool {
        matches!(self, Subject::Enum(_))
    }

    /// Returns whether every value of the subject holds the one target a
    /// struct's field is, so that the `match` over it has one arm.
    pub(crate) fn has_one_target(&self) -> bool {
        matches!(self, Subject::Struct(_))
    }

    /// Returns the words a sentence about the subject speaks: an enum's,
    /// with its variants, or a struct's, with its field.
    pub(crate) fn words(&self) -> Words {
        match self {
            Subject::Enum(_) => Words::Enum,
            Subject::Struct(_) => Words::Struct,
        }
    }

    /// Returns what the subject's targets are called in messages.
    pub(crate) fn target_noun(&self) -> &'static str {
        self.words().target()
    }

    /// Returns the subject with `field` naming `home` as well, as `home`
    /// spells its trait. A home that finds its trait on a field in another
    /// spelling says so, and hands the struct on as it was meant, so that
    /// the impl goes to that field and the sentence stands alone.
    pub(crate) fn naming(&self, field: &Ident, home: &Path) -> Self {
        let mut subject = self.clone();
        if let Subject::Struct(item) = &mut subject {
            let found = item
                .fields
                .iter_mut()
                .enumerate()
                .find(|(index, candidate)| match &candidate.ident {
                    Some(ident) => ident == field,
                    None => *field == format!("_{index}"),
                });
            if let Some((_, found)) = found {
                found
                    .attrs
                    .push(parse_quote_spanned!(field.span() => #[dispatch(#home)]));
            }
        }
        subject
    }

    /// Returns the subject without the `#[dispatch]` attributes on its
    /// variants, their fields, or its fields.
    pub(crate) fn without_inner_attributes(mut self) -> Self {
        match &mut self {
            Subject::Enum(item) => {
                for variant in &mut item.variants {
                    variant
                        .attrs
                        .retain(|attribute| !is_dispatch_attribute(attribute));
                    for field in &mut variant.fields {
                        field
                            .attrs
                            .retain(|attribute| !is_dispatch_attribute(attribute));
                    }
                }
            }
            Subject::Struct(item) => {
                for field in &mut item.fields {
                    field
                        .attrs
                        .retain(|attribute| !is_dispatch_attribute(attribute));
                }
            }
        }
        self
    }

    /// Reads the subject: every variant of the enum, or every field of the
    /// struct. A variant that is not `Name(Payload)` is an entry too, so
    /// that the methods which do not forward to it can tell.
    pub(crate) fn read(&self) -> Result<Entries<'_>> {
        match self {
            Subject::Enum(item) => Ok(Entries {
                subject: self,
                entries: item
                    .variants
                    .iter()
                    .map(variant_entry)
                    .collect::<Result<_>>()?,
                marks: None,
            }),
            Subject::Struct(item) => {
                let count = item.fields.len();
                let mut entries = vec![];
                let mut marks = vec![];
                for (index, field) in item.fields.iter().enumerate() {
                    let options = TargetOptions::read(&field.attrs, Position::StructField)?;
                    entries.push(field_entry(member_of(field, index), field, &options, count));
                    marks.push(Marks {
                        marked: field.attrs.iter().any(is_dispatch_attribute),
                        traits: options.traits,
                    });
                }
                Ok(Entries {
                    subject: self,
                    entries,
                    marks: Some(marks),
                })
            }
        }
    }
}

impl Parse for Subject {
    fn parse(input: ParseStream) -> Result<Self> {
        match input.parse::<Item>()? {
            Item::Enum(item) => Ok(Subject::Enum(item)),
            Item::Struct(item) => Ok(Subject::Struct(item)),
            other => Err(Error::new_spanned(other, "expected an enum or a struct")),
        }
    }
}

impl ToTokens for Subject {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Subject::Enum(item) => item.to_tokens(tokens),
            Subject::Struct(item) => item.to_tokens(tokens),
        }
    }
}

/// The entries of a subject as read: every variant of an enum, or the
/// fields of a struct, each a target when it holds a payload.
pub(crate) struct Entries<'a> {
    subject: &'a Subject,
    entries: Vec<Entry>,
    /// What each field of a struct is marked with, in the fields' order,
    /// until one field is chosen for a trait; nothing on an enum.
    marks: Option<Vec<Marks>>,
}

/// What a struct's field is marked with.
struct Marks {
    /// Whether the field carries `#[dispatch]` in any form.
    marked: bool,
    /// The traits the field is named the target of, `#[dispatch(Rom)]`.
    traits: Vec<Path>,
}

impl<'a> Entries<'a> {
    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Entry> {
        self.entries.iter()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the targets among the entries, each with the route its
    /// variant or field gives.
    pub(crate) fn targets(&self) -> Vec<Target<'_>> {
        self.entries.iter().filter_map(Entry::target).collect()
    }

    /// Returns the entries `trait_path` is forwarded to: every variant of
    /// an enum, or the one field of a struct that names the trait,
    /// `#[dispatch(Rom)] rom: Rom`; else the one marked bare, `#[dispatch]`,
    /// which every trait not named elsewhere goes to; else its one field.
    pub(crate) fn for_trait(mut self, trait_path: &Path) -> Result<Self> {
        let Some(marks) = self.marks.take() else {
            return Ok(self);
        };
        let chosen = self.chosen(&marks, Some(trait_path))?;
        let entry = self.entries.swap_remove(chosen);
        self.entries = vec![entry];
        Ok(self)
    }

    /// Returns the targets `*` names: every one of an enum, or of the one
    /// field a trait is forwarded to; and among all the fields of a
    /// struct, which an inherent block sees, the field marked bare or its
    /// one field, or the error for a struct that has neither.
    pub(crate) fn star(&self) -> Result<Vec<Target<'_>>> {
        let Some(marks) = &self.marks else {
            return Ok(self.targets());
        };
        let chosen = self.chosen(marks, None)?;
        Ok(self.entries[chosen].target().into_iter().collect())
    }

    /// Returns the index of the struct's field `trait_path` is forwarded
    /// to, or without a trait the field that stands for the struct.
    fn chosen(&self, marks: &[Marks], trait_path: Option<&Path>) -> Result<usize> {
        let Subject::Struct(item) = self.subject else {
            unreachable!("only a struct's fields are marked")
        };
        let field = |index: usize| {
            item.fields
                .iter()
                .nth(index)
                .expect("every mark is a field's")
        };
        let marked: Vec<usize> = (0..marks.len())
            .filter(|index| marks[*index].marked)
            .collect();
        let bare: Vec<usize> = marked
            .iter()
            .copied()
            .filter(|index| marks[*index].traits.is_empty())
            .collect();
        let wanted = trait_path.map(trait_key);
        let same_trait = |path: &Path| {
            wanted
                .as_ref()
                .is_some_and(|wanted| *wanted == trait_key(path))
        };
        let named: Vec<usize> = marked
            .iter()
            .copied()
            .filter(|index| marks[*index].traits.iter().any(same_trait))
            .collect();
        if let [_, second, ..] = bare.as_slice() {
            return Err(Error::new_spanned(
                field(*second),
                "only one field can be the target of every trait not named elsewhere; name the traits on the others",
            ));
        }
        if let [_, second, ..] = named.as_slice() {
            let trait_ident = trait_path
                .and_then(|path| path.segments.last())
                .expect("named a trait");
            return Err(Error::new_spanned(
                field(*second),
                format!(
                    "`{}` is named on two fields; one is its target",
                    trait_ident.ident
                ),
            ));
        }
        match (marks.len(), named.first(), bare.first()) {
            (0, _, _) => Err(Error::new_spanned(
                &item.ident,
                "a dispatch struct needs a field to forward to",
            )),
            (_, Some(named), _) => Ok(*named),
            (_, None, Some(bare)) => Ok(*bare),
            (1, None, None) => Ok(0),
            // Under `*` with no trait, several fields each naming their traits
            // and none marked bare name no field; the method names one.
            (_, None, None) if trait_path.is_none() && !marked.is_empty() => {
                Err(Error::new_spanned(
                    &item.ident,
                    format!(
                        "`{}` names traits on its fields and marks none bare, so `*` names no field; mark the field to forward to with `#[dispatch]`, or name it on the method, `#[dispatch(field)]`",
                        item.ident
                    ),
                ))
            }
            (_, None, None) => {
                let about = match trait_path.and_then(|path| path.segments.last()) {
                    Some(segment) => format!(" `{}` to", segment.ident),
                    None => String::new(),
                };
                Err(Error::new_spanned(
                    &item.ident,
                    format!(
                        "`{}` has several fields; mark the one to forward{about} with `#[dispatch]`, or name the trait on it, `#[dispatch(Trait)]`",
                        item.ident
                    ),
                ))
            }
        }
    }

    /// Spells the payloads with the names `renaming` gives the subject's
    /// parameters: a partial block's header may name them as it likes,
    /// `impl<U> Trait for Kind<U>` over `enum Kind<T>`.
    pub(crate) fn rename(&mut self, renaming: &mut impl VisitMut) {
        for entry in &mut self.entries {
            if let Holds::Payload { ty, .. } = &mut entry.holds {
                renaming.visit_type_mut(ty);
            }
        }
    }

    /// Gives the variants `routes` name a trait's own routes, `via(Variant
    /// = ..)`, over the ones they give themselves, and returns the error
    /// for each route that names no variant.
    pub(crate) fn route(&mut self, routes: &[(Ident, Route)]) -> Vec<Error> {
        let mut errors = vec![];
        for (variant, route) in routes {
            let own = self
                .entries
                .iter_mut()
                .find_map(|entry| match &mut entry.holds {
                    Holds::Payload { route, .. } if entry.ident == *variant => Some(route),
                    _ => None,
                });
            match own {
                Some(own) => *own = route.clone(),
                None => {
                    let subject = self.subject.ident();
                    let message = match self.forwarded_elsewhere(variant) {
                        Some(kept) => format!("`{variant}` is a field of `{subject}`, but this trait is forwarded to `{kept}`, which is the field its route goes through; name the trait on `{variant}`, `#[dispatch(Trait)]`, to forward it there"),
                        None => format!(
                            "`{subject}` has no {} `{variant}` to route through",
                            self.subject.target_noun()
                        ),
                    };
                    errors.push(Error::new(variant.span(), message));
                }
            }
        }
        errors
    }

    /// Returns the field that names the trait of `home` in another
    /// spelling, with the trait as the field spells it. A trait is forwarded
    /// to the field that names it as its impl's home spells it, the
    /// struct's attribute or a partial impl block's header, and a macro
    /// cannot resolve names, so another spelling is another trait: the home
    /// finds no field and the field's mark is the target of nothing. That
    /// is undecidable in general, a field's trait the struct's attributes
    /// do not name being a partial block's, which the struct does not see;
    /// what can be told is one name spelled two ways. `dispatched` are the
    /// traits the struct's own attributes name, whose fields are spoken
    /// for.
    pub(crate) fn spelt_otherwise(
        &self,
        home: &Path,
        dispatched: &[&Path],
    ) -> Option<(&Ident, &Path)> {
        let traits = self.field_traits();
        let key = trait_key(home);
        if traits.iter().any(|(_, path)| trait_key(path) == key) {
            return None;
        }
        traits.into_iter().find(|(_, path)| {
            let spelled = trait_key(path);
            !dispatched.iter().any(|known| trait_key(known) == spelled) && same_name(path, home)
        })
    }

    /// Returns the field the trait is forwarded to when `name` is another
    /// field of the struct, one `for_trait` left out: a sentence about it
    /// says where the trait goes, where one about a name the struct lacks
    /// says so.
    pub(crate) fn forwarded_elsewhere(&self, name: &Ident) -> Option<&Ident> {
        let Subject::Struct(item) = self.subject else {
            return None;
        };
        if self.marks.is_some() {
            return None;
        }
        let [kept] = self.entries.as_slice() else {
            return None;
        };
        let is_field = item
            .fields
            .iter()
            .enumerate()
            .any(|(index, field)| match &field.ident {
                Some(ident) => ident == name,
                None => *name == format!("_{index}"),
            });
        (is_field && kept.ident != *name).then_some(&kept.ident)
    }

    /// Returns the traits the struct's fields name, with the field each is
    /// named on.
    pub(crate) fn field_traits(&self) -> Vec<(&Ident, &Path)> {
        let Some(marks) = &self.marks else {
            return vec![];
        };
        self.entries
            .iter()
            .zip(marks)
            .flat_map(|(entry, marks)| marks.traits.iter().map(move |path| (&entry.ident, path)))
            .collect()
    }

    /// Returns the errors of the entries no method can forward to, each
    /// under its own `cfg`, so that a variant compiled out is no error, and
    /// under `builds`, the `#[cfg]` of the builds in which something
    /// forwards to every variant, when that is not every build.
    pub(crate) fn shape_errors(&self, builds: Option<&TokenStream2>) -> TokenStream2 {
        self.entries
            .iter()
            .filter_map(|entry| {
                let error = entry.unforwardable()?.error.to_compile_error();
                let cfgs = &entry.cfgs;
                Some(quote_spanned! { entry.ident.span() => #builds #(#cfgs)* #error })
            })
            .collect()
    }
}

/// A variant of the enum, or a field of the struct.
pub(crate) struct Entry {
    /// The variant's name, or the field's name; an unnamed field is
    /// `_<index>`.
    pub(crate) ident: Ident,
    pub(crate) cfgs: Vec<Attribute>,
    /// Whether the variant or field is `#[deprecated]`, so that the arms
    /// and the conversions that name it are written under
    /// `#[allow(deprecated)]`, as a hand writes them.
    pub(crate) deprecated: bool,
    pub(crate) holds: Holds,
}

/// What an entry holds.
// Short-lived and few; the size difference does not matter here.
#[allow(clippy::large_enum_variant)]
pub(crate) enum Holds {
    /// One payload a method can forward to.
    Payload {
        place: Place,
        ty: Type,
        /// How an arm reaches the value the trait is called on from the
        /// payload, as the variant or field says: the payload itself, what
        /// it dereferences to, a field, or what a method returns.
        route: Route,
    },
    /// Another shape, which no method can forward to. Inherent methods
    /// leave it to their fallback body; a dispatched trait reports it,
    /// under its own `cfg`.
    Other(Unforwardable),
}

/// Why no method can forward to a variant that is not `Name(Payload)`.
pub(crate) struct Unforwardable {
    /// What the variant holds instead of one payload, as a predicate of its
    /// name: "holds no payload", "holds 2 fields, not one payload".
    pub(crate) reason: String,
    /// What follows the variant's name in the pattern a hand writes for it
    /// whatever it holds: nothing, `(..)` or ` { .. }`.
    pub(crate) rest: &'static str,
    pub(crate) error: Error,
}

impl Entry {
    /// Returns the target the entry is, reached by the route its variant
    /// or field gives; nothing for an entry that holds no payload.
    pub(crate) fn target(&self) -> Option<Target<'_>> {
        match &self.holds {
            Holds::Payload { place, ty, route } => Some(Target {
                entry: self,
                place,
                payload: ty,
                route,
            }),
            Holds::Other(_) => None,
        }
    }

    /// Returns why no method can forward to the entry, when none can.
    pub(crate) fn unforwardable(&self) -> Option<&Unforwardable> {
        match &self.holds {
            Holds::Payload { .. } => None,
            Holds::Other(unforwardable) => Some(unforwardable),
        }
    }

    /// Returns whether some build leaves the entry out.
    pub(crate) fn is_conditional(&self) -> bool {
        !self.cfgs.is_empty()
    }

    /// Returns the attributes an arm or an impl naming the entry carries:
    /// its `#[cfg]`, and `#[allow(deprecated)]` when it is deprecated, the
    /// attribute a hand writes on the arm of a deprecated variant, so that
    /// the deprecation reaches the code that names the variant and not the
    /// forwarding that has to.
    pub(crate) fn arm_attributes(&self, site: Site) -> Vec<Attribute> {
        let mut attributes = self.cfgs.clone();
        if self.deprecated {
            attributes.push(allow_deprecated(site));
        }
        attributes
    }

    /// Returns the pattern that matches the variant, or the struct,
    /// whatever it holds, binding nothing: the pattern of an arm that does
    /// not reach the payload.
    pub(crate) fn whole(&self, prefix: &Path, site: Site) -> TokenStream2 {
        let ident = &self.ident;
        match &self.holds {
            Holds::Payload {
                place:
                    Place {
                        holder: Holder::Struct,
                        ..
                    },
                ..
            } => quote_spanned!(*site => #prefix { .. }),
            _ => quote_spanned!(*site => #prefix::#ident { .. }),
        }
    }
}

/// How a target is matched and built.
pub(crate) struct Place {
    /// What holds the payload.
    pub(crate) holder: Holder,
    /// Where the payload sits in it.
    pub(crate) field: PayloadField,
}

/// What holds a payload.
pub(crate) enum Holder {
    /// A variant of the enum, `Enum::Variant`, with the variant's say on
    /// the conversions.
    Variant(Conversions),
    /// The struct itself.
    Struct,
}

/// Where the payload sits in its variant or struct.
pub(crate) enum PayloadField {
    /// The one field, unnamed: `Name(payload)`, which the name builds as a
    /// path.
    Only,
    /// The one field, named: `Name { field: payload }`, which the braces
    /// build and no path does.
    OnlyNamed(Ident),
    /// One field beside others, `Name(_, payload, ..)` or `Name { field:
    /// payload, .. }`, which no value can be built around. `followed` says
    /// whether a rest pattern has fields to skip after it: an unnamed
    /// field that is the last has none, and a named field is found by its
    /// name wherever it sits, with the others always left to the rest.
    Beside { member: Member, followed: bool },
}

impl PayloadField {
    /// Returns the field `member` beside the others of `count` fields.
    fn beside(member: Member, count: usize) -> Self {
        let followed = match &member {
            Member::Named(_) => true,
            Member::Unnamed(index) => (index.index as usize) + 1 < count,
        };
        PayloadField::Beside { member, followed }
    }
}

/// A variant's part in the `From` and `TryFrom` impls.
#[derive(Clone, Copy)]
pub(crate) struct Conversions {
    /// Whether the variant takes part at all; `#[dispatch(from = false)]`
    /// leaves it out of both impls.
    pub(crate) from: bool,
    /// The variant's own say on its `TryFrom` impls, over the enum's option.
    pub(crate) try_from: Option<TryFromForms>,
}

/// A variant or field a method forwards to, and the route this trait or
/// method takes from its payload: the method's own, the trait's for the
/// variant, or the one the variant or field gives.
#[derive(Clone, Copy)]
pub(crate) struct Target<'a> {
    pub(crate) entry: &'a Entry,
    pub(crate) place: &'a Place,
    pub(crate) payload: &'a Type,
    pub(crate) route: &'a Route,
}

impl<'a> Target<'a> {
    /// Returns the target reached through `route` instead.
    pub(crate) fn through(self, route: &'a Route) -> Self {
        Target { route, ..self }
    }

    /// Returns the variant's name, or the field's.
    pub(crate) fn ident(&self) -> &'a Ident {
        &self.entry.ident
    }

    /// Returns the attributes an arm or an impl naming the target carries:
    /// its `#[cfg]`, and `#[allow(deprecated)]` when it is deprecated.
    pub(crate) fn arm_attributes(&self, site: Site) -> Vec<Attribute> {
        self.entry.arm_attributes(site)
    }

    /// Returns the variant's part in the conversions; a field has none,
    /// and neither has a variant with other fields beside the payload.
    pub(crate) fn conversions(&self) -> Option<Conversions> {
        match (&self.place.holder, &self.place.field) {
            (Holder::Variant(conversions), PayloadField::Only) => Some(*conversions),
            _ => None,
        }
    }

    /// Returns what the payload's holder is written as on `prefix`, which
    /// is the subject's name or `Self`: `prefix::Variant`, or `prefix`.
    fn holder(&self, prefix: &Path, site: Site) -> TokenStream2 {
        let ident = &self.entry.ident;
        match self.place.holder {
            Holder::Variant(_) => quote_spanned!(*site => #prefix::#ident),
            Holder::Struct => quote_spanned!(*site => #prefix),
        }
    }

    /// Returns the pattern binding the payload to `binding`, on `prefix`,
    /// which is the subject's name or `Self`; the pattern's own tokens
    /// carry `site`.
    pub(crate) fn pattern(
        &self,
        prefix: &Path,
        binding: &impl ToTokens,
        site: Site,
    ) -> TokenStream2 {
        let holder = self.holder(prefix, site);
        // A rest pattern skips the fields after the payload's. The one
        // field of a variant or a struct, and an unnamed field that is the
        // last, have none to skip, and a hand writes no `..` there.
        match &self.place.field {
            PayloadField::Only => quote_spanned!(*site => #holder(#binding)),
            PayloadField::OnlyNamed(name) => {
                quote_spanned!(*site => #holder { #name: #binding })
            }
            PayloadField::Beside {
                member: Member::Unnamed(index),
                followed,
            } => {
                let skipped = (0..index.index).map(|_| quote_spanned!(*site => _));
                let rest = followed.then(|| quote_spanned!(*site => , ..));
                quote_spanned!(*site => #holder(#(#skipped,)* #binding #rest))
            }
            PayloadField::Beside { member, .. } => {
                quote_spanned!(*site => #holder { #member: #binding, .. })
            }
        }
    }

    /// Returns the pattern that matches the variant, or the struct,
    /// whatever it holds, binding nothing: the pattern of an arm that does
    /// not reach the payload.
    pub(crate) fn whole(&self, prefix: &Path, site: Site) -> TokenStream2 {
        self.entry.whole(prefix, site)
    }

    /// Returns the expression building the subject around `value`, or
    /// `None` when other fields would be missing.
    pub(crate) fn rebuild(
        &self,
        prefix: &Path,
        value: TokenStream2,
        site: Site,
    ) -> Option<TokenStream2> {
        let holder = self.holder(prefix, site);
        match &self.place.field {
            PayloadField::Only => Some(quote_spanned!(*site => #holder(#value))),
            PayloadField::OnlyNamed(name) => {
                Some(quote_spanned!(*site => #holder { #name: #value }))
            }
            PayloadField::Beside { .. } => None,
        }
    }

    /// Returns whether the subject can be built around a payload: no
    /// other field would be missing.
    pub(crate) fn is_rebuilt(&self) -> bool {
        !matches!(self.place.field, PayloadField::Beside { .. })
    }

    /// Returns whether a path builds the subject from a payload, as
    /// `.map(..)` needs: the variant's name or the tuple struct's.
    pub(crate) fn is_built_by_a_path(&self) -> bool {
        matches!(self.place.field, PayloadField::Only)
    }

    /// Returns a function building the subject from a payload, for `map`.
    /// A named field is built by its braces, which a closure would have to
    /// hold; the caller says so.
    pub(crate) fn constructor(&self, prefix: &Path, site: Site) -> Option<TokenStream2> {
        match &self.place.field {
            PayloadField::Only => Some(self.holder(prefix, site)),
            PayloadField::OnlyNamed(_) | PayloadField::Beside { .. } => None,
        }
    }

    /// Returns the name a sentence gives the target: the variant's, or on a
    /// struct the struct's, since a field is no item of its own and the
    /// error's span already points at it.
    pub(crate) fn spoken(&self, subject: &'a Ident) -> &'a Ident {
        match self.place.holder {
            Holder::Variant(_) => &self.entry.ident,
            Holder::Struct => subject,
        }
    }

    /// Returns what a sentence calls the item that holds the payload.
    pub(crate) fn noun(&self) -> &'static str {
        match self.place.holder {
            Holder::Variant(_) => "variant",
            Holder::Struct => "field",
        }
    }

    /// Returns whether the subject can be built around a payload and yet no
    /// path builds it: a struct, or a variant, whose one field is named.
    pub(crate) fn is_built_by_no_path(&self) -> bool {
        matches!(self.place.field, PayloadField::OnlyNamed(_))
    }

    /// Returns the type the trait is called on, when the route lets it be
    /// named from the payload's: the payload, or what it dereferences to. A
    /// field's or a method's type cannot be, and the call is qualified by
    /// the trait alone. The tokens carry the payload's span, so that an
    /// error about the type lands on the variant.
    pub(crate) fn target_type(&self) -> Option<TokenStream2> {
        Self::routed_type(self.payload, self.route, Site::at(self.payload.span()))
    }

    /// Returns the same type for a position outside the subject's own
    /// impls, where a `Self` the payload names would not be the subject.
    pub(crate) fn target_type_outside(&self, self_type: &Type) -> Option<TokenStream2> {
        let mut payload = self.payload.clone();
        SelfReplacer(self_type.clone()).visit_type_mut(&mut payload);
        Self::routed_type(&payload, self.route, Site::at(self.payload.span()))
    }

    fn routed_type(payload: &Type, route: &Route, site: Site) -> Option<TokenStream2> {
        if !route.names_the_target() {
            return None;
        }
        if route.is_deref() {
            Some(quote_spanned!(*site => <#payload as ::core::ops::Deref>::Target))
        } else {
            Some(quote_spanned!(*site => #payload))
        }
    }

    /// Returns the type parameter of `generics` that the trait is called
    /// on: the payload when it is a bare parameter, or what a `deref`
    /// payload points at, `T` for `Box<T>`, `Rc<T>` or `&T`. The impl then
    /// needs the bound `T: Trait`, which a hand writes and nothing else
    /// supplies.
    pub(crate) fn bounded_parameter(&self, generics: &syn::Generics) -> Option<&'a Ident> {
        let candidate = if self.route.is_direct() {
            bare_ident(self.payload)
        } else if self.route.is_deref() {
            pointee(self.payload).and_then(bare_ident)
        } else {
            None
        }?;
        generics
            .params
            .iter()
            .any(|param| matches!(param, GenericParam::Type(param) if param.ident == *candidate))
            .then_some(candidate)
    }

    /// Returns whether the payload is a `Box`, dereferenced to the
    /// implementor, which is what a `self: Box<Self>` receiver needs.
    pub(crate) fn holds_box(&self) -> bool {
        self.route.is_deref() && Pointer::of(self.payload) == Pointer::Box
    }

    /// Returns whether the trait is called on a trait object: a `deref`
    /// payload that is a reference to one, `&dyn Trait`, or a pointer with
    /// one as its one type argument, `Box<dyn Trait>` or `Rc<dyn Trait>`.
    /// Such a target cannot take a method that requires `Self: Sized`.
    pub(crate) fn is_trait_object(&self) -> bool {
        self.route.is_deref() && matches!(pointee(self.payload), Some(Type::TraitObject(_)))
    }
}

/// Replaces `Self` with the subject's type, for a payload type written
/// outside the subject's own impls.
pub(crate) struct SelfReplacer(pub(crate) Type);

impl VisitMut for SelfReplacer {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        if is_self(ty) {
            *ty = self.0.clone();
            return;
        }
        visit_mut::visit_type_mut(self, ty);
    }
}

/// Returns whether `attributes` deprecate their item.
pub(crate) fn is_deprecated(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("deprecated"))
}

/// Returns `#[allow(deprecated)]`, written at `site`.
pub(crate) fn allow_deprecated(site: Site) -> Attribute {
    parse_quote_spanned!(*site => #[allow(deprecated)])
}

/// Reads a variant: `Name(Payload)` forwards to its one field, a variant
/// with several fields to the one marked `#[dispatch]`, and any other shape
/// is an entry no trait method can forward to. The route is the marked
/// field's or the variant's own.
fn variant_entry(variant: &Variant) -> Result<Entry> {
    let TargetOptions {
        route,
        from,
        try_from,
        traits: _,
    } = TargetOptions::read(&variant.attrs, Position::Variant)?;
    let ident = &variant.ident;
    let cfgs = cfg_attributes(&variant.attrs);
    let deprecated = is_deprecated(&variant.attrs);
    let marked: Vec<(Member, &Field)> = variant
        .fields
        .iter()
        .enumerate()
        .filter(|(_, field)| field.attrs.iter().any(is_dispatch_attribute))
        .map(|(index, field)| (member_of(field, index), field))
        .collect();
    let (payload_field, field) = match (&variant.fields, marked.as_slice()) {
        (Fields::Unnamed(fields), []) if fields.unnamed.len() == 1 => {
            (PayloadField::Only, &fields.unnamed[0])
        }
        (Fields::Unnamed(fields), [(_, field)]) if fields.unnamed.len() == 1 => {
            (PayloadField::Only, *field)
        }
        // The one field, named: the braces build the variant around it,
        // as they build a struct around its one named field.
        (Fields::Named(fields), [(Member::Named(name), field)]) if fields.named.len() == 1 => {
            (PayloadField::OnlyNamed(name.clone()), *field)
        }
        (_, [(member, field)]) => (
            PayloadField::beside(member.clone(), variant.fields.len()),
            *field,
        ),
        (_, [_, (_, second), ..]) => {
            return Err(Error::new_spanned(
                second,
                format!("only one field of `{ident}` can be forwarded to; `#[dispatch]` marks it"),
            ))
        }
        (fields, []) => {
            let reason = match fields {
                Fields::Unnamed(fields) if fields.unnamed.is_empty() => {
                    "holds no payload".to_owned()
                }
                Fields::Unnamed(fields) => {
                    format!("holds {} fields, not one payload", fields.unnamed.len())
                }
                Fields::Named(_) => "has named fields, not one payload".to_owned(),
                Fields::Unit => "holds no payload".to_owned(),
            };
            let marking = match fields {
                Fields::Unit => "",
                Fields::Unnamed(fields) if fields.unnamed.is_empty() => "",
                _ => ", mark the field to forward to with `#[dispatch]`",
            };
            let rest = match fields {
                Fields::Unit => "",
                Fields::Unnamed(_) => "(..)",
                Fields::Named(_) => " { .. }",
            };
            return Ok(Entry {
                ident: ident.clone(),
                cfgs,
                deprecated,
                holds: Holds::Other(Unforwardable {
                    error: Error::new_spanned(
                        variant,
                        format!("`{ident}` {reason}, so no trait method can be forwarded to it; write it as `{ident}(Payload)`{marking}, or forward inherent methods instead, whose body answers for it"),
                    ),
                    reason,
                    rest,
                }),
            });
        }
    };
    let field_options = TargetOptions::read(&field.attrs, Position::VariantField)?;
    let route = match (route, field_options.route) {
        (Some(_), Some(_)) => {
            return Err(Error::new_spanned(
                field,
                format!("`{ident}` and its field both give a route; give one"),
            ))
        }
        (route, field_route) => route.or(field_route).unwrap_or_default(),
    };
    Ok(Entry {
        ident: ident.clone(),
        cfgs,
        deprecated,
        holds: Holds::Payload {
            place: Place {
                holder: Holder::Variant(Conversions { from, try_from }),
                field: payload_field,
            },
            ty: field.ty.clone(),
            route,
        },
    })
}

fn member_of(field: &Field, index: usize) -> Member {
    match &field.ident {
        Some(ident) => Member::Named(ident.clone()),
        None => Member::Unnamed(Index::from(index)),
    }
}

/// Returns the entry a struct's field is: named by the field, or `_<index>`
/// for an unnamed one, with the field's own route. The one field of a
/// struct of `count` fields is what the struct is built around.
fn field_entry(member: Member, field: &Field, options: &TargetOptions, count: usize) -> Entry {
    let ident = match &member {
        Member::Named(ident) => ident.clone(),
        Member::Unnamed(index) => format_ident!("_{}", index.index, span = index.span),
    };
    let field_in_struct = match member {
        Member::Unnamed(_) if count == 1 => PayloadField::Only,
        Member::Named(name) if count == 1 => PayloadField::OnlyNamed(name),
        member => PayloadField::beside(member, count),
    };
    Entry {
        ident,
        cfgs: vec![],
        deprecated: is_deprecated(&field.attrs),
        holds: Holds::Payload {
            place: Place {
                holder: Holder::Struct,
                field: field_in_struct,
            },
            ty: field.ty.clone(),
            route: options.route.clone().unwrap_or_default(),
        },
    }
}

/// Returns whether `one` and `other`, two traits as written, may be one
/// trait spelled two ways: one path is the other with more or fewer
/// segments in front, `Catalog` and `traits::Catalog`, and their generic
/// arguments are the same or one has none, `Encoder` and `Encoder<F>`.
/// Two instantiations, `Encoder<Json>` and `Encoder<Xml>`, and two paths
/// that part ways, `a::Named` and `b::Named`, are two traits.
fn same_name(one: &Path, other: &Path) -> bool {
    let names = |path: &Path| -> Vec<String> {
        path.segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect()
    };
    let arguments = |path: &Path| -> Option<String> {
        let last = path.segments.last()?;
        (!last.arguments.is_none()).then(|| spelling::spelled(&last.arguments))
    };
    let (one_names, other_names) = (names(one), names(other));
    let (shorter, longer) = if one_names.len() <= other_names.len() {
        (&one_names, &other_names)
    } else {
        (&other_names, &one_names)
    };
    if !longer.ends_with(shorter) {
        return false;
    }
    match (arguments(one), arguments(other)) {
        (Some(one), Some(other)) => one == other,
        _ => true,
    }
}

pub(crate) fn cfg_attributes(attributes: &[Attribute]) -> Vec<Attribute> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::quote;
    use syn::parse_quote;

    use super::*;
    use crate::options::TraitOptions;

    fn site() -> Site {
        Site::at(Span::call_site())
    }

    fn shapes() -> Subject {
        parse_quote! {
            enum Any {
                Plain(Circle),
                #[cfg(unix)]
                #[deprecated]
                #[dispatch(deref)]
                Boxed(Box<Circle>),
                Tagged(u32, #[dispatch] Circle),
                Named {
                    #[dispatch]
                    inner: Circle,
                },
                Labelled {
                    label: u32,
                    #[dispatch]
                    inner: Circle,
                },
                #[dispatch(from = false)]
                Kept(Circle),
                Empty,
                Pair(Circle, Circle),
                Fields { a: u32 },
            }
        }
    }

    fn target<'a>(entries: &'a Entries, name: &str) -> Target<'a> {
        entries
            .targets()
            .into_iter()
            .find(|target| target.ident() == name)
            .expect("a target of that name")
    }

    fn names(targets: &[Target]) -> Vec<String> {
        targets
            .iter()
            .map(|target| target.ident().to_string())
            .collect()
    }

    /// Where a target's payload sits, in words.
    fn field(target: &Target) -> String {
        match &target.place.field {
            PayloadField::Only => "only".to_owned(),
            PayloadField::OnlyNamed(name) => format!("only named {name}"),
            PayloadField::Beside { member, .. } => {
                format!("beside {}", member.to_token_stream())
            }
        }
    }

    fn message<T>(result: Result<T>) -> String {
        match result {
            Ok(_) => panic!("expected an error"),
            Err(error) => error.to_string(),
        }
    }

    fn spelled(tokens: Option<TokenStream2>) -> Option<String> {
        tokens.map(|tokens| tokens.to_string())
    }

    #[test]
    fn an_enum_reads_every_variant_and_says_why_one_holds_no_target() {
        let subject = shapes();
        let entries = subject.read().unwrap();
        assert_eq!(entries.iter().count(), 9);
        assert_eq!(
            names(&entries.targets()),
            ["Plain", "Boxed", "Tagged", "Named", "Labelled", "Kept"]
        );
        let reasons: Vec<&str> = entries
            .iter()
            .filter_map(|entry| entry.unforwardable())
            .map(|unforwardable| unforwardable.reason.as_str())
            .collect();
        assert_eq!(
            reasons,
            [
                "holds no payload",
                "holds 2 fields, not one payload",
                "has named fields, not one payload"
            ]
        );
        let boxed = target(&entries, "Boxed");
        assert!(boxed.entry.is_conditional() && boxed.entry.deprecated);
        assert!(boxed.route.is_deref());
        assert_eq!(
            boxed.arm_attributes(site()).len(),
            2,
            "its `cfg` and an allowance"
        );
        let plain = target(&entries, "Plain");
        assert!(!plain.entry.is_conditional() && !plain.entry.deprecated);
        assert!(plain.arm_attributes(site()).is_empty());
        assert_eq!(field(&plain), "only");
        assert_eq!(field(&target(&entries, "Tagged")), "beside 1");
        assert_eq!(
            field(&target(&entries, "Named")),
            "only named inner",
            "a variant's one field, named and marked, is rebuilt by its braces"
        );
        assert_eq!(field(&target(&entries, "Labelled")), "beside inner");
    }

    #[test]
    fn a_struct_reads_its_fields_and_the_traits_they_name() {
        let subject: Subject = parse_quote! {
            struct Mapper {
                #[dispatch(Rom)]
                rom: Cartridge,
                #[dispatch(deref)]
                bus: Box<Bus>,
            }
        };
        let entries = subject.read().unwrap();
        let targets = entries.targets();
        assert_eq!(names(&targets), ["rom", "bus"]);
        assert!(targets
            .iter()
            .all(|target| matches!(target.place.holder, Holder::Struct)));
        assert_eq!(field(&targets[0]), "beside rom");
        assert!(targets[1].route.is_deref());
        let traits: Vec<String> = entries
            .field_traits()
            .into_iter()
            .map(|(field, path)| format!("{field}: {}", path.to_token_stream()))
            .collect();
        assert_eq!(traits, ["rom: Rom"]);
        let tuple: Subject = parse_quote!(
            struct Wrapped(Circle);
        );
        let entries = tuple.read().unwrap();
        assert_eq!(names(&entries.targets()), ["_0"]);
        assert_eq!(field(&entries.targets()[0]), "only");
        assert!(shapes().read().unwrap().field_traits().is_empty());
    }

    #[test]
    fn a_trait_goes_to_the_field_naming_it_else_the_bare_one_else_the_only_one() {
        let subject: Subject = parse_quote! {
            struct Mapper {
                #[dispatch(Rom)]
                rom: Cartridge,
                #[dispatch]
                bus: Bus,
                clock: u32,
            }
        };
        let for_trait =
            |path: Path| names(&subject.read().unwrap().for_trait(&path).unwrap().targets());
        assert_eq!(for_trait(parse_quote!(Rom)), ["rom"]);
        assert_eq!(
            for_trait(parse_quote!(crate::Rom)),
            ["bus"],
            "a macro cannot resolve names, so another spelling is another trait"
        );
        assert_eq!(for_trait(parse_quote!(Interrupt)), ["bus"]);
        let only: Subject = parse_quote!(
            struct Wrapped {
                inner: Circle,
            }
        );
        let entries = only
            .read()
            .unwrap()
            .for_trait(&parse_quote!(Shape))
            .unwrap();
        assert_eq!(names(&entries.targets()), ["inner"]);
        assert_eq!(field(&entries.targets()[0]), "only named inner");
        assert_eq!(
            names(
                &shapes()
                    .read()
                    .unwrap()
                    .for_trait(&parse_quote!(Shape))
                    .unwrap()
                    .targets()
            )
            .len(),
            6,
            "an enum forwards every trait to every variant"
        );
    }

    #[test]
    fn one_name_spelled_two_ways_is_told_from_two_traits() {
        let same = |one: Path, other: Path| same_name(&one, &other) && same_name(&other, &one);
        assert!(same(parse_quote!(Catalog), parse_quote!(Catalog)));
        assert!(same(parse_quote!(Catalog), parse_quote!(traits::Catalog)));
        assert!(same(
            parse_quote!(crate::traits::Catalog),
            parse_quote!(traits::Catalog)
        ));
        assert!(same(parse_quote!(Encoder), parse_quote!(Encoder<F>)));
        assert!(same(
            parse_quote!(Encoder<Json>),
            parse_quote!(codec::Encoder<Json>)
        ));
        assert!(
            !same(parse_quote!(Encoder<Json>), parse_quote!(Encoder<Xml>)),
            "two instantiations are two traits"
        );
        assert!(
            !same(parse_quote!(a::Named), parse_quote!(b::Named)),
            "two paths that part ways are two traits"
        );
        assert!(!same(parse_quote!(Catalog), parse_quote!(Named)));
        assert!(
            !same(parse_quote!(Catalog), parse_quote!(Catalog::Inner)),
            "the shorter path ends the longer one"
        );
    }

    #[test]
    fn a_home_finds_the_field_that_spells_its_trait_another_way() {
        let subject: Subject = parse_quote! {
            struct Env {
                #[dispatch(Catalog)]
                catalog: Books,
                #[dispatch(Encoder)]
                encoder: Json,
                #[dispatch(Encoder<Xml>)]
                xml: Xml,
                #[dispatch(Named)]
                label: Label,
                #[dispatch]
                named: Label,
            }
        };
        let entries = subject.read().unwrap();
        let found = |home: Path, dispatched: &[&Path]| {
            entries
                .spelt_otherwise(&home, dispatched)
                .map(|(field, path)| format!("{field}: {}", path.to_token_stream()))
        };
        assert_eq!(
            found(parse_quote!(traits::Catalog), &[]),
            Some("catalog: Catalog".to_owned())
        );
        assert_eq!(
            found(parse_quote!(Catalog), &[]),
            None,
            "a field that spells the home's trait as the home does is its field"
        );
        assert_eq!(
            found(parse_quote!(Encoder<F>), &[]),
            Some("encoder: Encoder".to_owned())
        );
        assert_eq!(
            found(parse_quote!(Encoder<Json>), &[]),
            Some("encoder: Encoder".to_owned()),
            "`Encoder<Xml>` on `xml` is another instantiation"
        );
        let catalog: Path = parse_quote!(Catalog);
        assert_eq!(
            found(parse_quote!(traits::Catalog), &[&catalog]),
            None,
            "a field's trait an attribute names is that attribute's"
        );
        assert_eq!(found(parse_quote!(Unrelated), &[]), None);
        assert_eq!(
            found(parse_quote!(wire::Named), &[]),
            Some("label: Named".to_owned())
        );
        assert!(shapes()
            .read()
            .unwrap()
            .spelt_otherwise(&parse_quote!(Shape), &[])
            .is_none());
    }

    #[test]
    fn a_subject_named_as_meant_forwards_the_homes_trait_to_that_field() {
        let subject: Subject = parse_quote! {
            struct Env {
                #[dispatch(Catalog)]
                catalog: Books,
                #[dispatch]
                named: Label,
            }
        };
        let home: Path = parse_quote!(traits::Catalog);
        let target =
            |subject: &Subject| names(&subject.read().unwrap().for_trait(&home).unwrap().targets());
        assert_eq!(target(&subject), ["named"], "as written, the bare field");
        let meant = subject.naming(&parse_quote!(catalog), &home);
        assert_eq!(target(&meant), ["catalog"]);
        let traits: Vec<String> = meant
            .read()
            .unwrap()
            .field_traits()
            .into_iter()
            .map(|(field, path)| format!("{field}: {}", path.to_token_stream()))
            .collect();
        assert_eq!(
            traits,
            ["catalog: Catalog", "catalog: traits :: Catalog"],
            "the field keeps its own spelling beside the home's"
        );
        let tuple: Subject = parse_quote!(
            struct Pair(#[dispatch] Label, #[dispatch(Encoder)] Json);
        );
        let encoder: Path = parse_quote!(Encoder<F>);
        let meant = tuple.naming(&parse_quote!(_1), &encoder);
        assert_eq!(
            names(&meant.read().unwrap().for_trait(&encoder).unwrap().targets()),
            ["_1"]
        );
        let enum_subject = shapes();
        let unchanged = enum_subject.naming(&parse_quote!(Circle), &home);
        assert_eq!(
            unchanged.to_token_stream().to_string(),
            enum_subject.to_token_stream().to_string(),
            "an enum has no field to name"
        );
    }

    #[test]
    fn a_name_left_out_of_a_trait_is_another_field_or_none() {
        let subject: Subject = parse_quote! {
            struct Env {
                #[dispatch(Catalog)]
                catalog: Books,
                #[dispatch]
                named: Label,
                plain: u32,
            }
        };
        let entries = subject.read().unwrap();
        assert!(
            entries.forwarded_elsewhere(&parse_quote!(named)).is_none(),
            "before `for_trait` keeps one field, no field is the trait's"
        );
        let catalog = subject
            .read()
            .unwrap()
            .for_trait(&parse_quote!(Catalog))
            .unwrap();
        let elsewhere = |name: Ident| catalog.forwarded_elsewhere(&name).map(Ident::to_string);
        assert_eq!(elsewhere(parse_quote!(named)), Some("catalog".to_owned()));
        assert_eq!(elsewhere(parse_quote!(plain)), Some("catalog".to_owned()));
        assert_eq!(elsewhere(parse_quote!(catalog)), None, "the kept field");
        assert_eq!(elsewhere(parse_quote!(missing)), None, "no such field");
        let tuple: Subject = parse_quote!(
            struct Pair(#[dispatch] A, B);
        );
        let pair = tuple
            .read()
            .unwrap()
            .for_trait(&parse_quote!(Shape))
            .unwrap();
        assert_eq!(
            pair.forwarded_elsewhere(&parse_quote!(_1))
                .map(Ident::to_string),
            Some("_0".to_owned())
        );
        let enum_subject = shapes();
        let enum_entries = enum_subject
            .read()
            .unwrap()
            .for_trait(&parse_quote!(Shape))
            .unwrap();
        assert!(enum_entries
            .forwarded_elsewhere(&parse_quote!(Circle))
            .is_none());
    }

    #[test]
    fn a_struct_that_names_no_field_for_a_trait_says_how_to_name_one() {
        let error =
            |subject: Subject| message(subject.read().unwrap().for_trait(&parse_quote!(Rom)));
        assert_eq!(
            error(parse_quote!(struct Mapper { #[dispatch] a: A, #[dispatch] b: B })),
            "only one field can be the target of every trait not named elsewhere; name the traits on the others"
        );
        assert_eq!(
            error(parse_quote!(
                struct Mapper {
                    #[dispatch(Rom)]
                    a: A,
                    #[dispatch(Rom)]
                    b: B,
                }
            )),
            "`Rom` is named on two fields; one is its target"
        );
        assert_eq!(
            error(parse_quote!(
                struct Mapper {}
            )),
            "a dispatch struct needs a field to forward to"
        );
        assert_eq!(
            error(parse_quote!(struct Mapper { a: A, b: B })),
            "`Mapper` has several fields; mark the one to forward `Rom` to with `#[dispatch]`, or name the trait on it, `#[dispatch(Trait)]`"
        );
    }

    #[test]
    fn star_names_every_variant_or_the_field_that_stands_for_the_struct() {
        let enum_entries = shapes();
        let entries = enum_entries.read().unwrap();
        assert_eq!(names(&entries.star().unwrap()), names(&entries.targets()));
        let bare: Subject = parse_quote!(
            struct Mapper {
                #[dispatch(Rom)]
                a: A,
                #[dispatch]
                b: B,
            }
        );
        assert_eq!(names(&bare.read().unwrap().star().unwrap()), ["b"]);
        let only: Subject = parse_quote!(
            struct Wrapped(Circle);
        );
        assert_eq!(names(&only.read().unwrap().star().unwrap()), ["_0"]);
        let named: Subject = parse_quote!(
            struct Mapper {
                #[dispatch(Rom)]
                a: A,
                #[dispatch(Bus)]
                b: B,
            }
        );
        assert!(message(named.read().unwrap().star()).starts_with(
            "`Mapper` names traits on its fields and marks none bare, so `*` names no field"
        ));
        let unmarked: Subject = parse_quote!(
            struct Mapper {
                a: A,
                b: B,
            }
        );
        assert_eq!(
            message(unmarked.read().unwrap().star()),
            "`Mapper` has several fields; mark the one to forward with `#[dispatch]`, or name the trait on it, `#[dispatch(Trait)]`"
        );
    }

    #[test]
    fn a_trait_routes_through_the_variants_it_names_and_reports_the_rest() {
        let subject = shapes();
        let mut entries = subject.read().unwrap();
        let options: TraitOptions = syn::parse_str("via(Plain = .0, Wide = .1)").unwrap();
        let errors: Vec<String> = entries
            .route(&options.routes)
            .into_iter()
            .map(|error| error.to_string())
            .collect();
        assert_eq!(errors, ["`Any` has no variant `Wide` to route through"]);
        assert_eq!(target(&entries, "Plain").route.spelled(), ".0");
        assert!(target(&entries, "Kept").route.is_direct());
    }

    #[test]
    fn the_payloads_are_spelled_with_the_names_a_block_gives_the_parameters() {
        struct Renaming;
        impl VisitMut for Renaming {
            fn visit_ident_mut(&mut self, ident: &mut Ident) {
                if ident == "T" {
                    *ident = parse_quote!(U);
                }
            }
        }
        let subject: Subject = parse_quote!(
            enum Kind<T> {
                One(T),
                Many(Vec<T>),
                Empty,
            }
        );
        let mut entries = subject.read().unwrap();
        entries.rename(&mut Renaming);
        let payloads: Vec<String> = entries
            .targets()
            .iter()
            .map(|target| target.payload.to_token_stream().to_string())
            .collect();
        assert_eq!(payloads, ["U", quote!(Vec<U>).to_string().as_str()]);
    }

    #[test]
    fn an_entry_no_method_can_forward_to_is_reported_under_its_cfg_and_the_builds() {
        let subject: Subject = parse_quote! {
            enum Any {
                Plain(Circle),
                Empty,
                #[cfg(unix)]
                Unit,
            }
        };
        let entries = subject.read().unwrap();
        let errors = entries.shape_errors(None).to_string();
        assert_eq!(errors.matches("compile_error").count(), 2);
        assert!(errors.contains(&quote!(#[cfg(unix)] ::core::compile_error!).to_string()));
        let builds = quote!(#[cfg(test)]);
        let under = entries.shape_errors(Some(&builds)).to_string();
        assert!(
            under.contains(&quote!(#[cfg(test)] #[cfg(unix)] ::core::compile_error!).to_string())
        );
        assert!(under.contains(&quote!(#[cfg(test)] ::core::compile_error!).to_string()));
    }

    /// Whether a rest pattern follows the payload of `target`: nothing for
    /// a payload that is the only field.
    fn followed(target: &Target) -> Option<bool> {
        match &target.place.field {
            PayloadField::Beside { followed, .. } => Some(*followed),
            PayloadField::Only | PayloadField::OnlyNamed(_) => None,
        }
    }

    #[test]
    fn a_rest_pattern_follows_a_payload_only_where_fields_follow_it() {
        let subject: Subject = parse_quote! {
            enum Any {
                First(#[dispatch] Circle, u32),
                Middle(u8, #[dispatch] Circle, u32),
                Last(u8, #[dispatch] Circle),
                Labelled {
                    #[dispatch]
                    inner: Circle,
                    label: u32,
                },
                Ending {
                    label: u32,
                    #[dispatch]
                    inner: Circle,
                },
                Plain(Circle),
            }
        };
        let entries = subject.read().unwrap();
        let prefix: Path = parse_quote!(Any);
        let binding = quote!(__inner);
        let read = |name: &str| {
            let target = target(&entries, name);
            (
                followed(&target),
                target.pattern(&prefix, &binding, site()).to_string(),
            )
        };
        assert_eq!(
            read("First"),
            (Some(true), quote!(Any::First(__inner, ..)).to_string())
        );
        assert_eq!(
            read("Middle"),
            (Some(true), quote!(Any::Middle(_, __inner, ..)).to_string())
        );
        assert_eq!(
            read("Last"),
            (Some(false), quote!(Any::Last(_, __inner)).to_string())
        );
        assert_eq!(
            read("Labelled"),
            (
                Some(true),
                quote!(Any::Labelled { inner: __inner, .. }).to_string()
            )
        );
        assert_eq!(
            read("Ending"),
            (
                Some(true),
                quote!(Any::Ending { inner: __inner, .. }).to_string()
            ),
            "a named field is found by its name, and the others are left to the rest wherever it sits"
        );
        assert_eq!(
            read("Plain"),
            (None, quote!(Any::Plain(__inner)).to_string())
        );
        let pair: Subject = parse_quote!(
            struct Pair(First, Second);
        );
        let fields = pair.read().unwrap();
        let prefix: Path = parse_quote!(Pair);
        let read = |index: usize| {
            let target = &fields.targets()[index];
            (
                followed(target),
                target.pattern(&prefix, &binding, site()).to_string(),
            )
        };
        assert_eq!(read(0), (Some(true), quote!(Pair(__inner, ..)).to_string()));
        assert_eq!(read(1), (Some(false), quote!(Pair(_, __inner)).to_string()));
        let labelled: Subject = parse_quote!(
            struct Labelled {
                label: u32,
                inner: Circle,
            }
        );
        let fields = labelled.read().unwrap();
        let prefix: Path = parse_quote!(Labelled);
        assert_eq!(
            fields
                .targets()
                .iter()
                .map(|target| (
                    followed(target),
                    target.pattern(&prefix, &binding, site()).to_string()
                ))
                .collect::<Vec<_>>(),
            [
                (
                    Some(true),
                    quote!(Labelled { label: __inner, .. }).to_string()
                ),
                (
                    Some(true),
                    quote!(Labelled { inner: __inner, .. }).to_string()
                ),
            ]
        );
    }

    #[test]
    fn a_target_is_matched_and_built_as_its_holder_and_field_say() {
        let subject = shapes();
        let entries = subject.read().unwrap();
        let prefix: Path = parse_quote!(Any);
        let value = || quote!(__value);
        let binding = quote!(__inner);
        let pattern = |name: &str| {
            target(&entries, name)
                .pattern(&prefix, &binding, site())
                .to_string()
        };
        let rebuild =
            |name: &str| spelled(target(&entries, name).rebuild(&prefix, value(), site()));
        let constructor = |name: &str| spelled(target(&entries, name).constructor(&prefix, site()));
        assert_eq!(pattern("Plain"), quote!(Any::Plain(__inner)).to_string());
        assert_eq!(
            pattern("Tagged"),
            quote!(Any::Tagged(_, __inner)).to_string(),
            "the last field leaves nothing for a rest pattern to skip"
        );
        assert_eq!(
            pattern("Named"),
            quote!(Any::Named { inner: __inner }).to_string()
        );
        assert_eq!(
            pattern("Labelled"),
            quote!(Any::Labelled { inner: __inner, .. }).to_string()
        );
        assert_eq!(rebuild("Plain"), spelled(Some(quote!(Any::Plain(__value)))));
        assert_eq!(
            rebuild("Named"),
            spelled(Some(quote!(Any::Named { inner: __value })))
        );
        assert_eq!(rebuild("Tagged"), None);
        assert_eq!(rebuild("Labelled"), None);
        assert_eq!(constructor("Plain"), spelled(Some(quote!(Any::Plain))));
        assert_eq!(constructor("Named"), None, "braces are no path");
        assert_eq!(constructor("Tagged"), None);
        assert_eq!(
            target(&entries, "Tagged")
                .whole(&prefix, site())
                .to_string(),
            quote!(Any::Tagged { .. }).to_string()
        );
        let wrapped: Subject = parse_quote!(
            struct Wrapped(Circle);
        );
        let tuple = wrapped.read().unwrap();
        let named: Subject = parse_quote!(
            struct Wrapped {
                inner: Circle,
            }
        );
        let braces = named.read().unwrap();
        let prefix: Path = parse_quote!(Wrapped);
        let only = &tuple.targets()[0];
        let one_named = &braces.targets()[0];
        assert_eq!(
            only.pattern(&prefix, &binding, site()).to_string(),
            quote!(Wrapped(__inner)).to_string()
        );
        assert_eq!(
            spelled(only.rebuild(&prefix, value(), site())),
            spelled(Some(quote!(Wrapped(__value))))
        );
        assert_eq!(
            spelled(only.constructor(&prefix, site())),
            spelled(Some(quote!(Wrapped)))
        );
        assert_eq!(
            one_named.pattern(&prefix, &binding, site()).to_string(),
            quote!(Wrapped { inner: __inner }).to_string()
        );
        assert_eq!(
            spelled(one_named.rebuild(&prefix, value(), site())),
            spelled(Some(quote!(Wrapped { inner: __value })))
        );
        assert_eq!(
            only.whole(&prefix, site()).to_string(),
            quote!(Wrapped { .. }).to_string()
        );
    }

    #[test]
    fn a_target_says_how_it_is_rebuilt_converted_and_spoken_of() {
        let subject = shapes();
        let entries = subject.read().unwrap();
        let wrapped: Subject = parse_quote!(
            struct Wrapped(Circle);
        );
        let tuple = wrapped.read().unwrap();
        let named: Subject = parse_quote!(
            struct Wrapped {
                inner: Circle,
            }
        );
        let braces = named.read().unwrap();
        let several: Subject = parse_quote!(
            struct Wrapped {
                #[dispatch]
                inner: Circle,
                n: u32,
            }
        );
        let beside = several.read().unwrap();
        let subject_ident: Ident = parse_quote!(Wrapped);
        // Rebuilt at all, by a path, by no path, converted, spoken of as,
        // and called.
        let said = |target: Target<'_>| {
            (
                target.is_rebuilt(),
                target.is_built_by_a_path(),
                target.is_built_by_no_path(),
                target.conversions().map(|conversions| conversions.from),
                target.spoken(&subject_ident).to_string(),
                target.noun(),
            )
        };
        let variant = |name: &str| said(target(&entries, name));
        let owned = |name: &str| name.to_owned();
        assert_eq!(
            variant("Plain"),
            (true, true, false, Some(true), owned("Plain"), "variant")
        );
        assert_eq!(
            variant("Kept"),
            (true, true, false, Some(false), owned("Kept"), "variant"),
            "`from = false` leaves the variant out of the conversions"
        );
        assert_eq!(
            variant("Named"),
            (true, false, true, None, owned("Named"), "variant")
        );
        assert_eq!(
            variant("Tagged"),
            (false, false, false, None, owned("Tagged"), "variant")
        );
        assert_eq!(
            variant("Labelled"),
            (false, false, false, None, owned("Labelled"), "variant")
        );
        assert_eq!(
            said(tuple.targets()[0]),
            (true, true, false, None, owned("Wrapped"), "field")
        );
        assert_eq!(
            said(braces.targets()[0]),
            (true, false, true, None, owned("Wrapped"), "field")
        );
        assert_eq!(
            said(target(&beside, "inner")),
            (false, false, false, None, owned("Wrapped"), "field")
        );
    }
}
