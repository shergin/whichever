//! A partial impl block as read: the header the impl is written under, the
//! items the block writes itself, and the methods it marks for forwarding.
use std::collections::hash_map::Entry as MapEntry;
use std::collections::HashMap;

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::{
    Attribute, Block, Error, Generics, Ident, ImplItem, ItemImpl, Result, Signature, Token, Type,
};

use crate::builds::Builds;
use crate::marked::{BlockItem, Marked};
use crate::options::{ForwardingOptions, TraitMark};
use crate::subject::cfg_attributes;

/// A partial impl block: the user's own impl of a trait for the subject,
/// holding the items the subject implements itself; the rest is forwarded
/// around them.
pub(crate) struct PartialBlock {
    pub(crate) generics: Generics,
    pub(crate) self_ty: Type,
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) unsafety: Option<Token![unsafe]>,
    /// The items the block writes, by name, and in which builds.
    written: HashMap<String, Written>,
    /// The methods the block marks with `#[dispatch(..)]`, by name; one
    /// name may be marked once per `#[cfg]`.
    marked: HashMap<String, Vec<BlockMethod>>,
    /// The associated types the block declares, each an override like the
    /// `Name = Type` option.
    pub(crate) declared_types: Vec<(Ident, Type)>,
    /// The items the block writes, emitted as they are.
    pub(crate) user_items: Vec<ImplItem>,
}

impl PartialBlock {
    /// Reads the block. A method marked `#[dispatch(..)]`, or declared
    /// without a body, is the engine's to write; every other item is the
    /// user's.
    pub(crate) fn read(block: ItemImpl) -> Result<Self> {
        let mut written: HashMap<String, Written> = HashMap::new();
        let mut marked: HashMap<String, Vec<BlockMethod>> = HashMap::new();
        let mut declared_types = vec![];
        let mut user_items = vec![];
        for item in block.items {
            let item = match BlockItem::read(item) {
                BlockItem::Marked(method) => {
                    let name = method.sig.ident.to_string();
                    marked
                        .entry(name)
                        .or_default()
                        .push(BlockMethod::read(method)?);
                    continue;
                }
                BlockItem::Bodyless(sig) => {
                    return Err(Error::new_spanned(
                        &sig,
                        format!("`{}` has no body; forward it with `#[dispatch(*)]` or `#[dispatch(Variant, ..)]`, or leave it to the trait's default with `#[dispatch(default)]`", sig.ident),
                    ));
                }
                BlockItem::Unread(tokens) => {
                    user_items.push(ImplItem::Verbatim(tokens));
                    continue;
                }
                BlockItem::Plain(item) => item,
            };
            let named = match &item {
                ImplItem::Fn(function) => Some((&function.sig.ident, &function.attrs)),
                ImplItem::Type(assoc) => {
                    declared_types.push((assoc.ident.clone(), assoc.ty.clone()));
                    Some((&assoc.ident, &assoc.attrs))
                }
                ImplItem::Const(constant) => Some((&constant.ident, &constant.attrs)),
                // What a macro writes is not seen here, so an item it
                // writes would be forwarded as well and defined twice.
                ImplItem::Macro(invocation) => {
                    return Err(Error::new_spanned(
                        invocation,
                        "a macro invocation in a partial impl block is not read, so the items it writes cannot be told from the ones to forward; write the items out in the block",
                    ));
                }
                _ => None,
            };
            if let Some((name, attrs)) = named {
                match written.entry(name.to_string()) {
                    MapEntry::Occupied(entry) => entry.into_mut().add(attrs),
                    MapEntry::Vacant(entry) => {
                        entry.insert(Written {
                            builds: Builds::of(attrs),
                            span: item.span(),
                        });
                    }
                }
            }
            user_items.push(item);
        }
        Ok(Self {
            generics: block.generics,
            self_ty: *block.self_ty,
            attrs: block.attrs,
            unsafety: block.unsafety,
            written,
            marked,
            declared_types,
            user_items,
        })
    }

    /// Returns in which builds the block writes the item `name` itself,
    /// when it writes it at all.
    pub(crate) fn written(&self, name: &str) -> Option<&Written> {
        self.written.get(name)
    }

    /// Returns the marks the block puts on the method `name`.
    pub(crate) fn marked(&self, name: &str) -> &[BlockMethod] {
        self.marked.get(name).map(Vec::as_slice).unwrap_or_default()
    }
}

/// The builds in which a partial block writes the items of one name: where
/// any of them exists. `span` is the first such item's, for a message about
/// them.
pub(crate) struct Written {
    pub(crate) builds: Builds,
    pub(crate) span: Span,
}

impl Written {
    /// Records one more item of the name, written under `attributes`.
    fn add(&mut self, attributes: &[Attribute]) {
        let written = std::mem::replace(&mut self.builds, Builds::Never);
        self.builds = written.or(Builds::of(attributes));
    }
}

/// A method of a partial block marked `#[dispatch(..)]`.
// Short-lived and few; the size difference does not matter here.
#[allow(clippy::large_enum_variant)]
pub(crate) enum BlockMethod {
    /// `#[dispatch(default)]`: not forwarded and not written, so the
    /// trait's default body stands, in the builds the method's own `#[cfg]`
    /// attributes name.
    Default { span: Span, cfgs: Vec<Attribute> },
    /// `#[dispatch(Variant, ..)]` or `#[dispatch(*)]`: forwarded to the
    /// named variants, with the body for the rest.
    Forwarded(Forwarded),
}

impl BlockMethod {
    /// Reads a marked method by its mark: left to the trait's default, or
    /// forwarded to the variants it names.
    fn read(mut method: Marked) -> Result<Self> {
        match method.mark(TraitMark::read)? {
            TraitMark::Default => {
                if let Some(body) = &method.body {
                    return Err(Error::new_spanned(
                        body,
                        format!("`{}` is left to the trait's default, so it takes no body; end the signature with `;`", method.sig.ident),
                    ));
                }
                Ok(BlockMethod::Default {
                    span: method.attribute.span(),
                    cfgs: cfg_attributes(&method.attrs),
                })
            }
            TraitMark::Forward(forwarding) => Ok(BlockMethod::Forwarded(Forwarded {
                attrs: method.attrs,
                sig: method.sig,
                body: method.body,
                forwarding,
            })),
        }
    }

    /// Returns the `#[cfg]` attributes the method is written under. Like an
    /// item the block writes, it is the block's where they hold, and the
    /// trait's method is forwarded to every variant where they do not.
    pub(crate) fn cfgs(&self) -> Vec<Attribute> {
        match self {
            BlockMethod::Default { cfgs, .. } => cfgs.clone(),
            BlockMethod::Forwarded(forwarded) => cfg_attributes(&forwarded.attrs),
        }
    }

    /// Returns the span a message about the method's builds carries.
    pub(crate) fn span(&self) -> Span {
        match self {
            BlockMethod::Default { span, .. } => *span,
            BlockMethod::Forwarded(forwarded) => forwarded.sig.ident.span(),
        }
    }
}

/// What a forwarded block method is written from: the user's own
/// signature and attributes, the body for the variants not named, and the
/// names.
pub(crate) struct Forwarded {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) sig: Signature,
    pub(crate) body: Option<Block>,
    pub(crate) forwarding: ForwardingOptions,
}
