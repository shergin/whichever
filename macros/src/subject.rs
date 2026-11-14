//! The subject an impl is written for: an enum over implementors, or a struct
//! wrapping one, and the targets its methods forward to.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{
    parse_quote_spanned, Attribute, Error, Field, Fields, GenericParam, Ident, Index, Item,
    ItemEnum, ItemStruct, Member, Path, Result, Type, Variant,
};

use crate::at;
use crate::options::{is_dispatch_attribute, trait_key, TargetOptions, TryFromForms};
use crate::route::Route;
use crate::shape::{bare_ident, generic_type_args, is_self, last_segment};

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

    /// Returns every variant of the enum, or the field of the struct that
    /// is forwarded to when no trait is asked for: its one field, the one
    /// marked bare, or any marked one. A variant that is not
    /// `Name(Payload)` is an entry too, so that the methods which do not
    /// forward to it can tell.
    pub(crate) fn entries(&self) -> Result<Vec<Entry>> {
        self.entries_for(None)
    }

    /// Returns the entries `trait_path` is forwarded to: every variant of
    /// the enum, or the struct's field that names the trait, else the one
    /// marked bare, else its one field. Without a trait, for an inherent
    /// block, every field of the struct, so that a method may name any;
    /// `*` then takes [`Subject::star_target`].
    pub(crate) fn entries_for(&self, trait_path: Option<&Path>) -> Result<Vec<Entry>> {
        match self {
            Subject::Enum(item) => item.variants.iter().map(variant_entry).collect(),
            Subject::Struct(item) => match trait_path {
                Some(_) => Ok(vec![Entry::Target(struct_target(item, trait_path)?)]),
                None => struct_fields(item),
            },
        }
    }

    /// Returns the target `*` names on a struct, the field marked bare or
    /// its one field, or the error for a struct that has neither; an enum's
    /// `*` names every variant, so `None`.
    pub(crate) fn star_target(&self) -> Result<Option<Target>> {
        match self {
            Subject::Enum(_) => Ok(None),
            Subject::Struct(item) => struct_target(item, None).map(Some),
        }
    }

    /// Returns the traits the struct's fields name, with the field each is
    /// named on, so that one the struct does not dispatch can be reported.
    pub(crate) fn field_traits(&self) -> Vec<(Ident, Path)> {
        let Subject::Struct(item) = self else {
            return vec![];
        };
        let mut named = vec![];
        for (index, field) in item.fields.iter().enumerate() {
            let Ok(options) = TargetOptions::read(&field.attrs, false) else {
                continue;
            };
            let ident = match &field.ident {
                Some(ident) => ident.clone(),
                None => format_ident!("_{}", index, span = field.ty.span()),
            };
            named.extend(options.traits.into_iter().map(|path| (ident.clone(), path)));
        }
        named
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

/// A variant or field a method forwards to.
#[derive(Clone)]
pub(crate) struct Target {
    /// The variant's name, or the field's name; an unnamed field is
    /// `_<index>`.
    pub(crate) ident: Ident,
    pub(crate) place: Place,
    pub(crate) payload: Type,
    pub(crate) cfgs: Vec<Attribute>,
    /// Whether the variant or field is `#[deprecated]`, so that the arms
    /// and the conversions that name it are written under
    /// `#[allow(deprecated)]`, as a hand writes them.
    pub(crate) deprecated: bool,
    /// How an arm reaches the value the trait is called on from the
    /// payload: the payload itself, what it dereferences to, a field, or
    /// what a method returns.
    pub(crate) route: Route,
}

/// How a target is matched and built.
#[derive(Clone)]
pub(crate) enum Place {
    /// `Enum::Variant(payload)`, with the variant's say on the conversions;
    /// or, with `field`, the marked field of a variant with several,
    /// `Enum::Variant(_, payload, ..)` or `Enum::Variant { member: payload,
    /// .. }`, which no value can be built around.
    Variant {
        conversions: Conversions,
        field: Option<Member>,
    },
    /// `Struct { member: payload, .. }` or `Struct(_, payload, ..)`;
    /// `rebuildable` when the struct has no other field, so that the value
    /// built around a payload is whole.
    Field { member: Member, rebuildable: bool },
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

impl Target {
    /// Returns the attributes an arm or an impl naming the target carries:
    /// its `#[cfg]`, and `#[allow(deprecated)]` when it is deprecated.
    pub(crate) fn arm_attributes(&self, site: Span) -> Vec<Attribute> {
        arm_attributes(&self.cfgs, self.deprecated, site)
    }

    /// Returns the variant's part in the conversions; a field has none,
    /// and neither has a variant with other fields beside the payload.
    pub(crate) fn conversions(&self) -> Option<Conversions> {
        match &self.place {
            Place::Variant {
                conversions,
                field: None,
            } => Some(*conversions),
            Place::Variant { .. } | Place::Field { .. } => None,
        }
    }

    /// Returns whether the target is the marked field of a variant with
    /// several, which the pattern binds beside the fields that ride along.
    pub(crate) fn is_marked_field(&self) -> bool {
        matches!(self.place, Place::Variant { field: Some(_), .. })
    }

    /// Returns the pattern binding the payload to `binding`, on `prefix`,
    /// which is the subject's name or `Self`; the pattern's own tokens
    /// carry `site`.
    pub(crate) fn pattern(
        &self,
        prefix: &Path,
        binding: &impl ToTokens,
        site: Span,
    ) -> TokenStream2 {
        let ident = &self.ident;
        match &self.place {
            Place::Variant { field: None, .. } => {
                quote_spanned!(site => #prefix::#ident(#binding))
            }
            Place::Variant {
                field: Some(Member::Unnamed(index)),
                ..
            } => {
                let skipped = (0..index.index).map(|_| quote_spanned!(site => _));
                quote_spanned!(site => #prefix::#ident(#(#skipped,)* #binding, ..))
            }
            Place::Variant {
                field: Some(member),
                ..
            } => quote_spanned!(site => #prefix::#ident { #member: #binding, .. }),
            Place::Field {
                member: Member::Unnamed(index),
                ..
            } => {
                let skipped = (0..index.index).map(|_| quote_spanned!(site => _));
                quote_spanned!(site => #prefix(#(#skipped,)* #binding, ..))
            }
            Place::Field { member, .. } => {
                quote_spanned!(site => #prefix { #member: #binding, .. })
            }
        }
    }

    /// Returns the expression building the subject around `value`, or
    /// `None` when other fields would be missing.
    pub(crate) fn rebuild(
        &self,
        prefix: &Path,
        value: TokenStream2,
        site: Span,
    ) -> Option<TokenStream2> {
        let ident = &self.ident;
        match &self.place {
            Place::Variant { field: None, .. } => {
                Some(quote_spanned!(site => #prefix::#ident(#value)))
            }
            Place::Variant { .. } => None,
            Place::Field {
                member: Member::Unnamed(_),
                rebuildable: true,
            } => Some(quote_spanned!(site => #prefix(#value))),
            Place::Field {
                member,
                rebuildable: true,
            } => Some(quote_spanned!(site => #prefix { #member: #value })),
            Place::Field {
                rebuildable: false, ..
            } => None,
        }
    }

    /// Returns a function building the subject from a payload, for `map`.
    pub(crate) fn constructor(&self, prefix: &Path, site: Span) -> Option<TokenStream2> {
        let ident = &self.ident;
        match &self.place {
            Place::Variant { field: None, .. } => Some(quote_spanned!(site => #prefix::#ident)),
            Place::Variant { .. } => None,
            Place::Field {
                member: Member::Unnamed(_),
                rebuildable: true,
            } => Some(quote_spanned!(site => #prefix)),
            Place::Field {
                member,
                rebuildable: true,
            } => {
                let value = Ident::new("__value", Span::mixed_site());
                Some(quote_spanned!(site => |#value| #prefix { #member: #value }))
            }
            Place::Field {
                rebuildable: false, ..
            } => None,
        }
    }

    /// Returns the type the trait is called on, when the route lets it be
    /// named from the payload's: the payload, or what it dereferences to. A
    /// field's or a method's type cannot be, and the call is qualified by
    /// the trait alone. The tokens carry the payload's span, so that an
    /// error about the type lands on the variant.
    pub(crate) fn target_type(&self) -> Option<TokenStream2> {
        Self::routed_type(&self.payload, &self.route, at(self.payload.span()))
    }

    /// Returns the same type for a position outside the subject's own
    /// impls, where a `Self` the payload names would not be the subject.
    pub(crate) fn target_type_outside(&self, self_type: &Type) -> Option<TokenStream2> {
        let mut payload = self.payload.clone();
        SelfReplacer(self_type.clone()).visit_type_mut(&mut payload);
        Self::routed_type(&payload, &self.route, at(self.payload.span()))
    }

    fn routed_type(payload: &Type, route: &Route, site: Span) -> Option<TokenStream2> {
        if !route.names_the_target() {
            return None;
        }
        if route.is_deref() {
            Some(quote_spanned!(site => <#payload as ::core::ops::Deref>::Target))
        } else {
            Some(quote_spanned!(site => #payload))
        }
    }

    /// Returns the type parameter of `generics` that the trait is called
    /// on: the payload when it is a bare parameter, or what a `deref`
    /// payload points at, `T` for `Box<T>`, `Rc<T>` or `&T`. The impl then
    /// needs the bound `T: Trait`, which a hand writes and nothing else
    /// supplies.
    pub(crate) fn bounded_parameter(&self, generics: &syn::Generics) -> Option<&Ident> {
        let candidate = if self.route.is_direct() {
            bare_ident(&self.payload)
        } else if self.route.is_deref() {
            match &self.payload {
                Type::Reference(reference) => bare_ident(&reference.elem),
                other => match last_segment(other).map(generic_type_args) {
                    Some(arguments) if arguments.len() == 1 => bare_ident(arguments[0]),
                    _ => None,
                },
            }
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
        self.route.is_deref()
            && last_segment(&self.payload).is_some_and(|segment| segment.ident == "Box")
    }

    /// Returns whether the trait is called on a trait object: a `deref`
    /// payload that is a reference to one, `&dyn Trait`, or a pointer with
    /// one as its one type argument, `Box<dyn Trait>` or `Rc<dyn Trait>`.
    /// Such a target cannot take a method that requires `Self: Sized`.
    pub(crate) fn is_trait_object(&self) -> bool {
        if !self.route.is_deref() {
            return false;
        }
        let pointee = match &self.payload {
            Type::Reference(reference) => Some(&*reference.elem),
            other => match last_segment(other).map(generic_type_args) {
                Some(arguments) if arguments.len() == 1 => Some(arguments[0]),
                _ => None,
            },
        };
        matches!(pointee, Some(Type::TraitObject(_)))
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

/// A variant that is not `Name(Payload)`, which no method can forward to.
/// Inherent methods leave it to their fallback body; a dispatched trait
/// reports it, under its own `cfg`.
pub(crate) struct Unforwardable {
    pub(crate) ident: Ident,
    pub(crate) cfgs: Vec<Attribute>,
    /// Whether the variant is `#[deprecated]`, so that an arm naming it is
    /// written under `#[allow(deprecated)]`.
    pub(crate) deprecated: bool,
    /// What the variant holds instead of one payload, as a predicate of its
    /// name: "holds no payload", "holds 2 fields, not one payload".
    pub(crate) reason: String,
    pub(crate) error: Error,
}

impl Unforwardable {
    pub(crate) fn compile_error(&self) -> TokenStream2 {
        let cfgs = &self.cfgs;
        let error = self.error.to_compile_error();
        quote_spanned! { self.ident.span() => #(#cfgs)* #error }
    }

    /// Returns the attributes an arm naming the variant carries: its
    /// `#[cfg]`, and `#[allow(deprecated)]` when it is deprecated.
    pub(crate) fn arm_attributes(&self, site: Span) -> Vec<Attribute> {
        arm_attributes(&self.cfgs, self.deprecated, site)
    }
}

/// Returns `cfgs` with `#[allow(deprecated)]` added when `deprecated`: the
/// attribute a hand writes on the arm of a deprecated variant, so that the
/// deprecation reaches the code that names the variant and not the
/// forwarding that has to.
fn arm_attributes(cfgs: &[Attribute], deprecated: bool, site: Span) -> Vec<Attribute> {
    let mut attributes = cfgs.to_vec();
    if deprecated {
        attributes.push(allow_deprecated(site));
    }
    attributes
}

/// Returns whether `attributes` deprecate their item.
pub(crate) fn is_deprecated(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("deprecated"))
}

/// Returns `#[allow(deprecated)]`, written at `site`.
pub(crate) fn allow_deprecated(site: Span) -> Attribute {
    parse_quote_spanned!(site => #[allow(deprecated)])
}

// Short-lived and few; the size difference does not matter here.
#[allow(clippy::large_enum_variant)]
pub(crate) enum Entry {
    Target(Target),
    Unforwardable(Unforwardable),
}

/// Returns the targets among `entries`.
pub(crate) fn targets(entries: &[Entry]) -> Vec<&Target> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Target(target) => Some(target),
            Entry::Unforwardable(_) => None,
        })
        .collect()
}

/// Returns the errors of the entries no method can forward to, each under
/// its own `cfg`, so that a variant compiled out is no error.
pub(crate) fn shape_errors(entries: &[Entry]) -> TokenStream2 {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Unforwardable(unforwardable) => Some(unforwardable.compile_error()),
            Entry::Target(_) => None,
        })
        .collect()
}

/// Reads a variant: `Name(Payload)` forwards to its one field, a variant
/// with several fields to the one marked `#[dispatch]`, and any other shape
/// is an entry no trait method can forward to. The route is the marked
/// field's or the variant's own.
pub(crate) fn variant_entry(variant: &Variant) -> Result<Entry> {
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
            return Ok(Entry::Unforwardable(Unforwardable {
                ident: ident.clone(),
                cfgs,
                deprecated,
                error: Error::new_spanned(
                    variant,
                    format!("`{ident}` {reason}, so no trait method can be forwarded to it; write it as `{ident}(Payload)`{marking}, or forward inherent methods instead, whose body answers for it"),
                ),
                reason,
            }));
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
    Ok(Entry::Target(Target {
        ident: ident.clone(),
        place: Place::Variant {
            conversions: Conversions { from, try_from },
            field: member,
        },
        payload: field.ty.clone(),
        cfgs,
        deprecated,
        route,
    }))
}

fn member_of(field: &Field, index: usize) -> Member {
    match &field.ident {
        Some(ident) => Member::Named(ident.clone()),
        None => Member::Unnamed(Index::from(index)),
    }
}

/// Returns the field of a struct that `trait_path` is forwarded to: the
/// field that names the trait, `#[dispatch(Rom)] rom: Rom`; else the one
/// marked bare, `#[dispatch]`, which every trait not named elsewhere goes
/// to; else the struct's one field. Without a trait, the bare field or any
/// marked one stands for the struct.
pub(crate) fn struct_target(item: &ItemStruct, trait_path: Option<&Path>) -> Result<Target> {
    let fields: Vec<(Member, &Field, TargetOptions)> = item
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let member = match &field.ident {
                Some(ident) => Member::Named(ident.clone()),
                None => Member::Unnamed(Index::from(index)),
            };
            Ok((member, field, TargetOptions::read(&field.attrs, false)?))
        })
        .collect::<Result<_>>()?;
    let marked: Vec<&(Member, &Field, TargetOptions)> = fields
        .iter()
        .filter(|(_, field, _)| field.attrs.iter().any(is_dispatch_attribute))
        .collect();
    let bare: Vec<&&(Member, &Field, TargetOptions)> = marked
        .iter()
        .filter(|(_, _, options)| options.traits.is_empty())
        .collect();
    let wanted = trait_path.map(trait_key);
    let same_trait = |path: &Path| {
        wanted
            .as_ref()
            .is_some_and(|wanted| *wanted == trait_key(path))
    };
    let named: Vec<&&(Member, &Field, TargetOptions)> = marked
        .iter()
        .filter(|(_, _, options)| options.traits.iter().any(same_trait))
        .collect();
    if let [_, (_, second, _), ..] = bare.as_slice() {
        return Err(Error::new_spanned(
            second,
            "only one field can be the target of every trait not named elsewhere; name the traits on the others",
        ));
    }
    if let [_, (_, second, _), ..] = named.as_slice() {
        let trait_ident = trait_path
            .and_then(|path| path.segments.last())
            .expect("named a trait");
        return Err(Error::new_spanned(
            second,
            format!(
                "`{}` is named on two fields; one is its target",
                trait_ident.ident
            ),
        ));
    }
    let chosen = match (fields.as_slice(), named.first(), bare.first()) {
        ([], _, _) => {
            return Err(Error::new_spanned(
                &item.ident,
                "a dispatch struct needs a field to forward to",
            ))
        }
        (_, Some(named), _) => **named,
        (_, None, Some(bare)) => **bare,
        ([one], None, None) => one,
        // Under `*` with no trait, several fields each naming their traits
        // and none marked bare name no field; the method names one.
        (_, None, None) if trait_path.is_none() && !marked.is_empty() => {
            return Err(Error::new_spanned(
                &item.ident,
                format!(
                    "`{}` names traits on its fields and marks none bare, so `*` names no field; mark the field to forward to with `#[dispatch]`, or name it on the method, `#[dispatch(field)]`",
                    item.ident
                ),
            ));
        }
        (_, None, None) => {
            let about = match trait_path.and_then(|path| path.segments.last()) {
                Some(segment) => format!(" `{}` to", segment.ident),
                None => String::new(),
            };
            return Err(Error::new_spanned(
                &item.ident,
                format!(
                    "`{}` has several fields; mark the one to forward{about} with `#[dispatch]`, or name the trait on it, `#[dispatch(Trait)]`",
                    item.ident
                ),
            ));
        }
    };
    let (member, field, options) = chosen;
    Ok(field_target(member, field, options, fields.len() == 1))
}

/// Returns every field of a struct as a target, for an inherent block whose
/// methods name the field they forward to.
fn struct_fields(item: &ItemStruct) -> Result<Vec<Entry>> {
    let rebuildable = item.fields.len() == 1;
    item.fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let member = member_of(field, index);
            let options = TargetOptions::read(&field.attrs, false)?;
            Ok(Entry::Target(field_target(
                &member,
                field,
                &options,
                rebuildable,
            )))
        })
        .collect()
}

/// Returns the target a struct's field is: named by the field, or `_<index>`
/// for an unnamed one, with the field's own route.
fn field_target(
    member: &Member,
    field: &Field,
    options: &TargetOptions,
    rebuildable: bool,
) -> Target {
    let member = member.clone();
    let ident = match &member {
        Member::Named(ident) => ident.clone(),
        Member::Unnamed(index) => format_ident!("_{}", index.index, span = index.span),
    };
    Target {
        ident,
        place: Place::Field {
            member,
            rebuildable,
        },
        payload: field.ty.clone(),
        cfgs: vec![],
        deprecated: is_deprecated(&field.attrs),
        route: options.route.clone().unwrap_or_default(),
    }
}

pub(crate) fn cfg_attributes(attributes: &[Attribute]) -> Vec<Attribute> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .cloned()
        .collect()
}

/// Returns the attributes of a trait item that the generated item carries:
/// `#[cfg]`, since the item exists under the same condition, and the
/// `#[allow]` the trait's author wrote, since the signature the lint is
/// allowed on is the one the generated item repeats. A level that only
/// silences travels; one that raises the level, `warn`, `deny` or `expect`,
/// is the trait crate's policy and stays there. An `#[allow]` another macro
/// added, as `#[async_trait]` adds to every method, stays too: rustc exempts
/// it from the lints about lint names where it was written, and would not
/// on the way through the carrier.
pub(crate) fn carried_attributes(attributes: &[Attribute]) -> Vec<Attribute> {
    attributes
        .iter()
        .filter(|attribute| {
            let path = attribute.path();
            path.is_ident("cfg") || (path.is_ident("allow") && written_by_hand(attribute))
        })
        .cloned()
        .collect()
}

/// Returns whether `attribute` was written in the source rather than added
/// by a macro: a token's span is then exactly the token's own text.
fn written_by_hand(attribute: &Attribute) -> bool {
    attribute
        .path()
        .get_ident()
        .and_then(|ident| ident.span().source_text())
        .is_some_and(|text| text == "allow")
}
