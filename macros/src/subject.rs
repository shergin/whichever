//! The subject an impl is written for: an enum over implementors, or a struct
//! wrapping one, and the targets its methods forward to.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::visit_mut::{self, VisitMut};
use syn::{
    Attribute, Error, Field, Fields, GenericParam, Ident, Index, Item, ItemEnum, ItemStruct,
    Member, Path, Result, Type, Variant,
};

use crate::options::{is_dispatch_attribute, TargetOptions};
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
    /// variants or fields.
    pub(crate) fn without_inner_attributes(mut self) -> Self {
        match &mut self {
            Subject::Enum(item) => {
                for variant in &mut item.variants {
                    variant
                        .attrs
                        .retain(|attribute| !is_dispatch_attribute(attribute));
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

    /// Returns every variant of the enum, or the one field of the struct
    /// that is forwarded to. A variant that is not `Name(Payload)` is an
    /// entry too, so that the methods which do not forward to it can tell.
    pub(crate) fn entries(&self) -> Result<Vec<Entry>> {
        match self {
            Subject::Enum(item) => item.variants.iter().map(variant_entry).collect(),
            Subject::Struct(item) => Ok(vec![Entry::Target(struct_target(item)?)]),
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

/// A variant or field a method forwards to.
pub(crate) struct Target {
    /// The variant's name, or the field's name; an unnamed field is
    /// `_<index>`.
    pub(crate) ident: Ident,
    pub(crate) place: Place,
    pub(crate) payload: Type,
    pub(crate) cfgs: Vec<Attribute>,
    /// Whether the trait is called on what the payload dereferences to.
    pub(crate) deref: bool,
}

/// How a target is matched and built.
pub(crate) enum Place {
    /// `Enum::Variant(payload)`, with the variant's say on the conversions.
    Variant(Conversions),
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
    /// The variant's own say on its `TryFrom` impl, over the enum's option.
    pub(crate) try_from: Option<bool>,
}

impl Target {
    /// Returns the variant's part in the conversions; a field has none.
    pub(crate) fn conversions(&self) -> Option<Conversions> {
        match &self.place {
            Place::Variant(conversions) => Some(*conversions),
            Place::Field { .. } => None,
        }
    }

    /// Returns the pattern binding the payload to `binding`, on `prefix`,
    /// which is the subject's name or `Self`.
    pub(crate) fn pattern(&self, prefix: &Path, binding: &Ident) -> TokenStream2 {
        let ident = &self.ident;
        match &self.place {
            Place::Variant(_) => quote!(#prefix::#ident(#binding)),
            Place::Field {
                member: Member::Unnamed(index),
                ..
            } => {
                let skipped = (0..index.index).map(|_| quote!(_));
                quote!(#prefix(#(#skipped,)* #binding, ..))
            }
            Place::Field { member, .. } => quote!(#prefix { #member: #binding, .. }),
        }
    }

    /// Returns the expression building the subject around `value`, or
    /// `None` when other fields would be missing.
    pub(crate) fn rebuild(&self, prefix: &Path, value: TokenStream2) -> Option<TokenStream2> {
        let ident = &self.ident;
        match &self.place {
            Place::Variant(_) => Some(quote!(#prefix::#ident(#value))),
            Place::Field {
                member: Member::Unnamed(_),
                rebuildable: true,
            } => Some(quote!(#prefix(#value))),
            Place::Field {
                member,
                rebuildable: true,
            } => Some(quote!(#prefix { #member: #value })),
            Place::Field {
                rebuildable: false, ..
            } => None,
        }
    }

    /// Returns a function building the subject from a payload, for `map`.
    pub(crate) fn constructor(&self, prefix: &Path) -> Option<TokenStream2> {
        let ident = &self.ident;
        match &self.place {
            Place::Variant(_) => Some(quote!(#prefix::#ident)),
            Place::Field {
                member: Member::Unnamed(_),
                rebuildable: true,
            } => Some(quote!(#prefix)),
            Place::Field {
                member,
                rebuildable: true,
            } => {
                let value = Ident::new("__value", Span::mixed_site());
                Some(quote!(|#value| #prefix { #member: #value }))
            }
            Place::Field {
                rebuildable: false, ..
            } => None,
        }
    }

    /// Returns the type the trait is called on: the payload, or what it
    /// dereferences to.
    pub(crate) fn target_type(&self) -> TokenStream2 {
        let payload = &self.payload;
        if self.deref {
            quote!(<#payload as ::core::ops::Deref>::Target)
        } else {
            quote!(#payload)
        }
    }

    /// Returns the same type for a position outside the subject's own
    /// impls, where a `Self` the payload names would not be the subject.
    pub(crate) fn target_type_outside(&self, self_type: &Type) -> TokenStream2 {
        let mut payload = self.payload.clone();
        SelfReplacer(self_type.clone()).visit_type_mut(&mut payload);
        if self.deref {
            quote!(<#payload as ::core::ops::Deref>::Target)
        } else {
            quote!(#payload)
        }
    }

    /// Returns the type parameter of `generics` that the trait is called
    /// on: the payload when it is a bare parameter, or what a `deref`
    /// payload points at, `T` for `Box<T>`, `Rc<T>` or `&T`. The impl then
    /// needs the bound `T: Trait`, which a hand writes and nothing else
    /// supplies.
    pub(crate) fn bounded_parameter(&self, generics: &syn::Generics) -> Option<&Ident> {
        let candidate = if self.deref {
            match &self.payload {
                Type::Reference(reference) => bare_ident(&reference.elem),
                other => match last_segment(other).map(generic_type_args) {
                    Some(arguments) if arguments.len() == 1 => bare_ident(arguments[0]),
                    _ => None,
                },
            }
        } else {
            bare_ident(&self.payload)
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
        self.deref && last_segment(&self.payload).is_some_and(|segment| segment.ident == "Box")
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
    /// What the variant holds instead of one payload, as a predicate of its
    /// name: "holds no payload", "holds 2 fields, not one payload".
    pub(crate) reason: String,
    pub(crate) error: Error,
}

impl Unforwardable {
    pub(crate) fn compile_error(&self) -> TokenStream2 {
        let cfgs = &self.cfgs;
        let error = self.error.to_compile_error();
        quote! { #(#cfgs)* #error }
    }
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

pub(crate) fn variant_entry(variant: &Variant) -> Result<Entry> {
    let TargetOptions {
        deref,
        from,
        try_from,
    } = TargetOptions::read(&variant.attrs, true)?;
    let reason = match &variant.fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => None,
        Fields::Unnamed(fields) if fields.unnamed.is_empty() => Some("holds no payload".to_owned()),
        Fields::Unnamed(fields) => Some(format!(
            "holds {} fields, not one payload",
            fields.unnamed.len()
        )),
        Fields::Named(_) => Some("has named fields, not one payload".to_owned()),
        Fields::Unit => Some("holds no payload".to_owned()),
    };
    let ident = &variant.ident;
    if let Some(reason) = reason {
        return Ok(Entry::Unforwardable(Unforwardable {
            ident: ident.clone(),
            cfgs: cfg_attributes(&variant.attrs),
            error: Error::new_spanned(
                variant,
                format!("`{ident}` {reason}, so no trait method can be forwarded to it; write it as `{ident}(Payload)`, or forward inherent methods instead, whose body answers for it"),
            ),
            reason,
        }));
    }
    let Fields::Unnamed(fields) = &variant.fields else {
        unreachable!("every other shape has a reason")
    };
    let payload = fields.unnamed[0].ty.clone();
    Ok(Entry::Target(Target {
        ident: variant.ident.clone(),
        place: Place::Variant(Conversions { from, try_from }),
        payload,
        cfgs: cfg_attributes(&variant.attrs),
        deref,
    }))
}

/// Returns the one field of a struct that is forwarded to: its only field,
/// or the one marked `#[dispatch]`.
pub(crate) fn struct_target(item: &ItemStruct) -> Result<Target> {
    let fields: Vec<(Member, &Field)> = item
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let member = match &field.ident {
                Some(ident) => Member::Named(ident.clone()),
                None => Member::Unnamed(Index::from(index)),
            };
            (member, field)
        })
        .collect();
    let marked: Vec<&(Member, &Field)> = fields
        .iter()
        .filter(|(_, field)| field.attrs.iter().any(is_dispatch_attribute))
        .collect();
    let (member, field) = match (fields.as_slice(), marked.as_slice()) {
        ([], _) => {
            return Err(Error::new_spanned(
                &item.ident,
                "a dispatch struct needs a field to forward to",
            ))
        }
        ([(member, field)], _) | (_, [(member, field)]) => (member.clone(), *field),
        (_, []) => {
            return Err(Error::new_spanned(
                &item.ident,
                format!(
                    "`{}` has several fields; mark the one to forward to with `#[dispatch]`",
                    item.ident
                ),
            ))
        }
        (_, [_, (_, second), ..]) => {
            return Err(Error::new_spanned(
                second,
                "only one field can be forwarded to",
            ))
        }
    };
    let TargetOptions { deref, .. } = TargetOptions::read(&field.attrs, false)?;
    let ident = match &member {
        Member::Named(ident) => ident.clone(),
        Member::Unnamed(index) => format_ident!("_{}", index.index, span = index.span),
    };
    Ok(Target {
        ident,
        place: Place::Field {
            member,
            rebuildable: fields.len() == 1,
        },
        payload: field.ty.clone(),
        cfgs: vec![],
        deref,
    })
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
