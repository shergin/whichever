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

use crate::options::{is_dispatch_attribute, trait_key, TargetOptions, TryFromForms};
use crate::route::Route;
use crate::shape::{bare_ident, is_self, pointee, Pointer};
use crate::site::Site;

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

    /// Returns what the subject's targets are called in messages.
    pub(crate) fn target_noun(&self) -> &'static str {
        match self {
            Subject::Enum(_) => "variant",
            Subject::Struct(_) => "field",
        }
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
                let only = item.fields.len() == 1;
                let mut entries = vec![];
                let mut marks = vec![];
                for (index, field) in item.fields.iter().enumerate() {
                    let options = TargetOptions::read(&field.attrs, false)?;
                    entries.push(field_entry(member_of(field, index), field, &options, only));
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
                None => errors.push(Error::new(
                    variant.span(),
                    format!(
                        "`{}` has no variant `{variant}` to route through",
                        self.subject.ident()
                    ),
                )),
            }
        }
        errors
    }

    /// Returns the traits the struct's fields name, with the field each is
    /// named on, so that one the struct does not dispatch can be reported.
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
    /// payload, .. }`, which no value can be built around.
    Beside(Member),
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
        match (&self.place.holder, &self.place.field) {
            (Holder::Variant(_), PayloadField::Only) => {
                quote_spanned!(*site => #holder(#binding))
            }
            (Holder::Struct, PayloadField::Only) => {
                quote_spanned!(*site => #holder(#binding, ..))
            }
            (_, PayloadField::OnlyNamed(name)) => {
                quote_spanned!(*site => #holder { #name: #binding, .. })
            }
            (_, PayloadField::Beside(Member::Unnamed(index))) => {
                let skipped = (0..index.index).map(|_| quote_spanned!(*site => _));
                quote_spanned!(*site => #holder(#(#skipped,)* #binding, ..))
            }
            (_, PayloadField::Beside(member)) => {
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
            PayloadField::Beside(_) => None,
        }
    }

    /// Returns whether the subject can be built around a payload: no
    /// other field would be missing.
    pub(crate) fn is_rebuilt(&self) -> bool {
        !matches!(self.place.field, PayloadField::Beside(_))
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
            PayloadField::OnlyNamed(_) | PayloadField::Beside(_) => None,
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
    /// path builds it: a struct whose one field is named.
    pub(crate) fn is_built_by_no_path(&self) -> bool {
        matches!(
            (&self.place.holder, &self.place.field),
            (Holder::Struct, PayloadField::OnlyNamed(_))
        )
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
    } = TargetOptions::read(&variant.attrs, true)?;
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
    let (member, field) = match (&variant.fields, marked.as_slice()) {
        (Fields::Unnamed(fields), []) if fields.unnamed.len() == 1 => (None, &fields.unnamed[0]),
        (Fields::Unnamed(fields), [(member, field)]) if fields.unnamed.len() == 1 => {
            let _ = member;
            (None, *field)
        }
        (_, [(member, field)]) => (Some(member.clone()), *field),
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
                }),
            });
        }
    };
    let field_options = TargetOptions::read(&field.attrs, false)?;
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
                field: match member {
                    Some(member) => PayloadField::Beside(member),
                    None => PayloadField::Only,
                },
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
/// for an unnamed one, with the field's own route. A field that is the
/// struct's `only` one is what the struct is built around.
fn field_entry(member: Member, field: &Field, options: &TargetOptions, only: bool) -> Entry {
    let ident = match &member {
        Member::Named(ident) => ident.clone(),
        Member::Unnamed(index) => format_ident!("_{}", index.index, span = index.span),
    };
    let field_in_struct = match member {
        Member::Unnamed(_) if only => PayloadField::Only,
        Member::Named(name) if only => PayloadField::OnlyNamed(name),
        member => PayloadField::Beside(member),
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

pub(crate) fn cfg_attributes(attributes: &[Attribute]) -> Vec<Attribute> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .cloned()
        .collect()
}
