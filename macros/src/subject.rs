//! The subject an impl is written for: an enum over implementors, or a struct
//! wrapping one, and the targets its methods forward to.
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::visit_mut::{self, VisitMut};
use syn::{
    Attribute, Error, Field, Fields, Ident, Index, Item, ItemEnum, ItemStruct, LitBool, Member,
    Meta, Result, Type, Variant,
};

use crate::options::is_dispatch_attribute;
use crate::shape::{is_self, last_segment};

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

    pub(crate) fn is_enum(&self) -> bool {
        matches!(self, Subject::Enum(_))
    }

    /// Returns the subject without the `#[dispatch]` attributes on the item
    /// itself; the variants or fields keep theirs.
    pub(crate) fn without_item_attributes(mut self) -> Self {
        self.attrs_mut()
            .retain(|attribute| !is_dispatch_attribute(attribute));
        self
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
    /// Whether the variant takes part in the `From` and `TryFrom` impls.
    pub(crate) from: bool,
    /// The variant's own say on its `TryFrom` impl, over the enum's option.
    pub(crate) try_from: Option<bool>,
}

/// How a target is matched and built.
pub(crate) enum Place {
    /// `Enum::Variant(payload)`.
    Variant,
    /// `Struct { member: payload, .. }` or `Struct(_, payload, ..)`;
    /// `rebuildable` when the struct has no other field, so that the value
    /// built around a payload is whole.
    Field { member: Member, rebuildable: bool },
}

impl Target {
    /// Returns the pattern binding the payload to `binding`, on `prefix`,
    /// which is the subject's name or `Self`.
    pub(crate) fn pattern(&self, prefix: &TokenStream2, binding: &Ident) -> TokenStream2 {
        let ident = &self.ident;
        match &self.place {
            Place::Variant => quote!(#prefix::#ident(#binding)),
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
    pub(crate) fn rebuild(
        &self,
        prefix: &TokenStream2,
        value: TokenStream2,
    ) -> Option<TokenStream2> {
        let ident = &self.ident;
        match &self.place {
            Place::Variant => Some(quote!(#prefix::#ident(#value))),
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
    pub(crate) fn constructor(&self, prefix: &TokenStream2) -> Option<TokenStream2> {
        let ident = &self.ident;
        match &self.place {
            Place::Variant => Some(quote!(#prefix::#ident)),
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

/// The `#[dispatch(..)]` options of a variant or field.
pub(crate) struct TargetOptions {
    pub(crate) deref: bool,
    pub(crate) from: bool,
    pub(crate) try_from: Option<bool>,
}

/// Parses the options of a variant or field: `deref` on either, `from =
/// false` and `try_from = true` on a variant.
pub(crate) fn target_options(attributes: &[Attribute], on_variant: bool) -> Result<TargetOptions> {
    let mut options = TargetOptions {
        deref: false,
        from: true,
        try_from: None,
    };
    for attribute in attributes
        .iter()
        .filter(|attribute| is_dispatch_attribute(attribute))
    {
        if matches!(attribute.meta, Meta::Path(_)) {
            // A bare `#[dispatch]` marks the field to forward to.
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("deref") {
                options.deref = true;
            } else if meta.path.is_ident("from") && on_variant {
                options.from = meta.value()?.parse::<LitBool>()?.value;
            } else if meta.path.is_ident("try_from") && on_variant {
                options.try_from = Some(meta.value()?.parse::<LitBool>()?.value);
            } else {
                let option = meta.path.to_token_stream();
                let known = if on_variant {
                    "the options are `deref`, `from = false` and `try_from = true`"
                } else {
                    "the only one is `deref`"
                };
                return Err(meta.error(format!("unknown option `{option}`; {known}")));
            }
            Ok(())
        })?;
    }
    Ok(options)
}

pub(crate) fn variant_entry(variant: &Variant) -> Result<Entry> {
    let TargetOptions {
        deref,
        from,
        try_from,
    } = target_options(&variant.attrs, true)?;
    let payload = match &variant.fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => fields.unnamed[0].ty.clone(),
        Fields::Unnamed(_) => {
            return Ok(Entry::Unforwardable(Unforwardable {
                ident: variant.ident.clone(),
                cfgs: cfg_attributes(&variant.attrs),
                error: Error::new_spanned(
                    variant,
                    "a dispatch variant must hold exactly one payload",
                ),
            }))
        }
        _ => {
            return Ok(Entry::Unforwardable(Unforwardable {
                ident: variant.ident.clone(),
                cfgs: cfg_attributes(&variant.attrs),
                error: Error::new_spanned(variant, "a dispatch variant must be `Name(Payload)`"),
            }))
        }
    };
    Ok(Entry::Target(Target {
        ident: variant.ident.clone(),
        place: Place::Variant,
        payload,
        cfgs: cfg_attributes(&variant.attrs),
        deref,
        from,
        try_from,
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
    let TargetOptions { deref, .. } = target_options(&field.attrs, false)?;
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
        from: false,
        try_from: None,
    })
}

pub(crate) fn cfg_attributes(attributes: &[Attribute]) -> Vec<Attribute> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .cloned()
        .collect()
}
