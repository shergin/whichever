//! Classifying the shapes of receivers, arguments and return types.
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::visit::Visit;
use syn::{
    Error, GenericArgument, Ident, PathArguments, PathSegment, Receiver, Result, ReturnType, Token,
    Type, TypeParamBound, TypePath,
};

use crate::options::TraitSpec;
use crate::subject::Target;

/// Returns the identifier of a bare single-segment path type.
pub(crate) fn bare_ident(ty: &Type) -> Option<&Ident> {
    match ty {
        Type::Path(TypePath { qself: None, path }) if path.segments.len() == 1 => {
            Some(&path.segments[0].ident)
        }
        _ => None,
    }
}

/// What the arms of a method do with the payload's result, in two parts
/// that vary on their own: what wraps the value, and what the value itself
/// needs. `Result<Self, Self::Error>` with `Error` overridden is a `Result`
/// whose error converts, around a value that is rewrapped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReturnModel {
    pub(crate) wrapper: Wrapper,
    pub(crate) value: Value,
}

/// The container a method returns its value in, recognised by the name the
/// path ends in, so that `io::Result<T>` and `core::option::Option<T>`
/// count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wrapper {
    None,
    Option,
    Result { error: Conversion },
}

/// What the returned value needs on its way out of an arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Plain,
    /// `Self`: the payload's result goes back into the variant that produced
    /// it.
    Rewrap,
    /// `Self::X` with `X` overridden: the payload's own type converts with
    /// `Into`.
    Into,
}

/// What a position that cannot hold `Self` itself, such as a `Result`'s
/// error, needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Conversion {
    Keep,
    Into,
}

pub(crate) fn single_segment(ty: &Type) -> Option<&PathSegment> {
    match ty {
        Type::Path(TypePath { qself: None, path }) if path.segments.len() == 1 => {
            Some(&path.segments[0])
        }
        _ => None,
    }
}

pub(crate) fn is_self(ty: &Type) -> bool {
    single_segment(ty).is_some_and(|segment| segment.ident == "Self" && segment.arguments.is_none())
}

/// Returns the name of the associated type in a `Self::Name` type.
pub(crate) fn self_assoc(ty: &Type) -> Option<&Ident> {
    match ty {
        Type::Path(TypePath { qself: None, path })
            if path.segments.len() == 2 && path.segments[0].ident == "Self" =>
        {
            Some(&path.segments[1].ident)
        }
        _ => None,
    }
}

pub(crate) fn generic_type_args(segment: &PathSegment) -> Vec<&Type> {
    match &segment.arguments {
        PathArguments::AngleBracketed(arguments) => arguments
            .args
            .iter()
            .filter_map(|argument| match argument {
                GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

/// Returns the last segment of a path type without a qualified self, so
/// that `Result<Self, E>`, `io::Result<Self>` and `core::option::Option<Self>`
/// are all seen by the name they end in.
pub(crate) fn last_segment(ty: &Type) -> Option<&PathSegment> {
    match ty {
        Type::Path(TypePath { qself: None, path }) => path.segments.last(),
        _ => None,
    }
}

/// Returns the `Output` of a return-position `impl Future`: `Some(Some(T))`
/// for `impl Future<Output = T>` with any further bounds, `Some(None)` for
/// an `impl Future` without an `Output` binding, and `None` for any other
/// type.
pub(crate) fn future_output(ty: &Type) -> Option<Option<&Type>> {
    let Type::ImplTrait(impl_trait) = ty else {
        return None;
    };
    impl_trait.bounds.iter().find_map(|bound| {
        let TypeParamBound::Trait(bound) = bound else {
            return None;
        };
        let last = bound.path.segments.last()?;
        if last.ident != "Future" {
            return None;
        }
        let PathArguments::AngleBracketed(arguments) = &last.arguments else {
            return Some(None);
        };
        Some(arguments.args.iter().find_map(|argument| match argument {
            GenericArgument::AssocType(assoc) if assoc.ident == "Output" => Some(&assoc.ty),
            _ => None,
        }))
    })
}

/// Classifies what the arms of a method must do with the payload's result,
/// and whether the method returns `impl Future`, in which case the arms are
/// awaited inside one `async` block and the classification applies to the
/// future's `Output`. `spec` carries the associated type overrides of a
/// trait; inherent methods have none.
pub(crate) fn classify_return(
    output: &ReturnType,
    spec: Option<&TraitSpec>,
) -> Result<(ReturnModel, bool)> {
    let ReturnType::Type(_, ty) = output else {
        return Ok((ReturnModel::PLAIN, false));
    };
    let (ty, returns_future) = match future_output(ty) {
        Some(Some(output)) => (output, true),
        Some(None) => return Ok((ReturnModel::PLAIN, true)),
        None => (&**ty, false),
    };
    if let Type::ImplTrait(_) = ty {
        return Err(Error::new_spanned(
            ty,
            "a return-position `impl Trait` cannot be dispatched by a `match` because every variant returns a different opaque type; return a concrete type or a boxed trait object, or `impl Future<Output = T>`, which is awaited like an `async fn`",
        ));
    }
    Ok((return_model(ty, spec)?, returns_future))
}

impl ReturnModel {
    const PLAIN: Self = Self {
        wrapper: Wrapper::None,
        value: Value::Plain,
    };
}

/// Classifies a return type: an `Option` or a `Result` around a value, or
/// the value itself. Errors are reported on the whole type, since the
/// message speaks of the return type.
pub(crate) fn return_model(ty: &Type, spec: Option<&TraitSpec>) -> Result<ReturnModel> {
    if let Some(segment) = last_segment(ty) {
        let arguments = generic_type_args(segment);
        let wrapped = match (segment.ident.to_string().as_str(), arguments.as_slice()) {
            ("Option", [inner]) => Some((Wrapper::Option, *inner)),
            // `io::Result<T>`: the alias fixes the error type.
            ("Result", [inner]) => Some((
                Wrapper::Result {
                    error: Conversion::Keep,
                },
                *inner,
            )),
            ("Result", [inner, error]) => Some((
                Wrapper::Result {
                    error: conversion(error, ty, spec)?,
                },
                *inner,
            )),
            _ => None,
        };
        if let Some((wrapper, inner)) = wrapped {
            return Ok(ReturnModel {
                wrapper,
                value: value(inner, ty, spec)?,
            });
        }
    }
    Ok(ReturnModel {
        wrapper: Wrapper::None,
        value: value(ty, ty, spec)?,
    })
}

/// Classifies a value position of the return type `whole`: `Self`, or what
/// a position that cannot be `Self` needs.
fn value(ty: &Type, whole: &Type, spec: Option<&TraitSpec>) -> Result<Value> {
    if is_self(ty) {
        return Ok(Value::Rewrap);
    }
    Ok(match conversion(ty, whole, spec)? {
        Conversion::Keep => Value::Plain,
        Conversion::Into => Value::Into,
    })
}

/// Classifies a position of the return type `whole` that cannot hold `Self`
/// itself: an overridden `Self::X` converts, and any other mention of `Self`
/// is a limit.
fn conversion(ty: &Type, whole: &Type, spec: Option<&TraitSpec>) -> Result<Conversion> {
    if let Some(name) = self_assoc(ty) {
        let overridden = spec.is_some_and(|spec| spec.is_overridden(name));
        return Ok(if overridden {
            Conversion::Into
        } else {
            Conversion::Keep
        });
    }
    if mentions_self(ty) {
        return Err(Error::new_spanned(
            whole,
            "this return type mentions `Self` where `whichever` cannot wrap the payload's result back into the enum; return `Self`, `Option<Self>` or `Result<Self, E>`, spelled with those names, or implement the method by hand",
        ));
    }
    Ok(Conversion::Keep)
}

/// Returns whether `ty` names the implementing type itself anywhere; the
/// `Self` of a projection such as `Self::Item` or `<Self as Trait>::Item`
/// names an associated type instead and does not count.
pub(crate) fn mentions_self(ty: &Type) -> bool {
    let mut finder = SelfFinder(false);
    finder.visit_type(ty);
    finder.0
}

pub(crate) struct SelfFinder(bool);

impl<'ast> Visit<'ast> for SelfFinder {
    fn visit_type_path(&mut self, type_path: &'ast TypePath) {
        if let Some(qself) = &type_path.qself {
            if !is_self(&qself.ty) {
                self.visit_type(&qself.ty);
            }
            self.visit_path(&type_path.path);
            return;
        }
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Self" {
            self.0 = true;
        }
        syn::visit::visit_type_path(self, type_path);
    }
}

/// How a method reaches its receiver or a `Self` argument.
#[derive(Clone, Copy)]
pub(crate) enum Access {
    Shared,
    Exclusive,
    Owned,
}

pub(crate) enum ArgumentKind {
    Plain,
    SameVariant(Access),
}

/// The receivers a method can be dispatched on.
#[derive(Clone, Copy)]
pub(crate) enum ReceiverKind {
    /// `self`, `&self` and `&mut self`, in the shorthand or the explicit
    /// spelling: the payload is reached by matching on the value.
    Value(Access),
    /// `self: Box<Self>`: the box is moved out of and the payload's own box,
    /// held by a `deref` variant, is handed over.
    Boxed,
    /// `self: Pin<&Self>` and `self: Pin<&mut Self>`: unpinned, matched, and
    /// the payload pinned again, which needs every payload to be `Unpin`.
    Pinned(Access),
}

pub(crate) fn reference_access(reference: &syn::TypeReference) -> Access {
    if reference.mutability.is_some() {
        Access::Exclusive
    } else {
        Access::Shared
    }
}

/// Classifies a receiver by its type; any other type is an error.
pub(crate) fn classify_receiver(receiver: &Receiver) -> Result<ReceiverKind> {
    let ty = &*receiver.ty;
    if is_self(ty) {
        return Ok(ReceiverKind::Value(Access::Owned));
    }
    if let Type::Reference(reference) = ty {
        if is_self(&reference.elem) {
            return Ok(ReceiverKind::Value(reference_access(reference)));
        }
    }
    if let Some(segment) = last_segment(ty) {
        if let [argument] = generic_type_args(segment).as_slice() {
            if segment.ident == "Box" && is_self(argument) {
                return Ok(ReceiverKind::Boxed);
            }
            if segment.ident == "Pin" {
                if let Type::Reference(reference) = argument {
                    if is_self(&reference.elem) {
                        return Ok(ReceiverKind::Pinned(reference_access(reference)));
                    }
                }
            }
        }
    }
    Err(Error::new_spanned(
        ty,
        "this receiver type is not supported; a receiver is `self`, `&self`, `&mut self`, `self: Box<Self>`, `self: Pin<&Self>` or `self: Pin<&mut Self>`",
    ))
}

/// Returns the expression the generated `match` looks at for a receiver:
/// the value itself, the enum moved out of its box, or the pinned reference
/// unpinned.
pub(crate) fn scrutinee(kind: ReceiverKind, self_token: &Token![self]) -> TokenStream2 {
    match kind {
        ReceiverKind::Value(_) => quote!(#self_token),
        ReceiverKind::Boxed => quote!(*#self_token),
        ReceiverKind::Pinned(Access::Exclusive) => quote!(::core::pin::Pin::get_mut(#self_token)),
        ReceiverKind::Pinned(_) => quote!(::core::pin::Pin::get_ref(#self_token)),
    }
}

/// Returns the expression handing the payload bound to `binding` to the
/// method as its receiver, or the error for a payload the receiver cannot
/// reach.
pub(crate) fn receiver_expression(
    kind: ReceiverKind,
    target: &Target,
    binding: &Ident,
    method: &Ident,
) -> Result<TokenStream2> {
    match kind {
        ReceiverKind::Value(access) => Ok(payload_expression(target.deref, access, binding)),
        ReceiverKind::Boxed => {
            if !target.holds_box() {
                let ident = &target.ident;
                let payload = &target.payload;
                return Err(Error::new_spanned(
                    payload,
                    format!("`{method}` takes `self: Box<Self>`, and `{ident}` holds a plain `{}`; hold `Box<{}>` in a `#[dispatch(deref)]` variant, or implement the method by hand", payload.to_token_stream(), payload.to_token_stream()),
                ));
            }
            Ok(quote!(#binding))
        }
        ReceiverKind::Pinned(access) => {
            let payload = payload_expression(target.deref, access, binding);
            Ok(quote!(::core::pin::Pin::new(#payload)))
        }
    }
}

pub(crate) fn argument_kind(ty: &Type) -> Result<ArgumentKind> {
    let (inner, access) = match ty {
        Type::Reference(reference) if reference.mutability.is_some() => {
            (&*reference.elem, Access::Exclusive)
        }
        Type::Reference(reference) => (&*reference.elem, Access::Shared),
        other => (other, Access::Owned),
    };
    if is_self(inner) {
        return Ok(ArgumentKind::SameVariant(access));
    }
    if mentions_self(ty) {
        return Err(Error::new_spanned(
            ty,
            "only `Self`, `&Self` and `&mut Self` arguments can be dispatched; this type mentions `Self` elsewhere",
        ));
    }
    Ok(ArgumentKind::Plain)
}

/// Returns the expression that hands `binding`, bound to a variant's
/// payload, to the trait method: the payload itself, or what it
/// dereferences to for a `#[dispatch(deref)]` variant.
pub(crate) fn payload_expression(deref: bool, access: Access, binding: &Ident) -> TokenStream2 {
    if !deref {
        return quote!(#binding);
    }
    match access {
        Access::Shared => quote!(&**#binding),
        Access::Exclusive => quote!(&mut **#binding),
        Access::Owned => quote!(*#binding),
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    fn model(ty: Type, spec: Option<&TraitSpec>) -> Result<ReturnModel> {
        return_model(&ty, spec)
    }

    /// A spec fixing the associated type `name` on the enum.
    fn overriding(name: &str) -> TraitSpec {
        syn::parse_str(&format!("Trait, {name} = Fixed")).expect("a valid spec")
    }

    #[test]
    fn wrappers_are_recognised_by_their_last_segment() {
        let plain = ReturnModel {
            wrapper: Wrapper::None,
            value: Value::Plain,
        };
        assert_eq!(model(parse_quote!(u32), None).unwrap(), plain);
        assert_eq!(
            model(parse_quote!(Self), None).unwrap(),
            ReturnModel {
                wrapper: Wrapper::None,
                value: Value::Rewrap,
            }
        );
        assert_eq!(
            model(parse_quote!(io::Result<Self>), None).unwrap(),
            ReturnModel {
                wrapper: Wrapper::Result {
                    error: Conversion::Keep,
                },
                value: Value::Rewrap,
            }
        );
        assert_eq!(
            model(parse_quote!(::core::option::Option<Self>), None)
                .unwrap()
                .wrapper,
            Wrapper::Option
        );
    }

    #[test]
    fn an_overridden_associated_type_converts_in_every_position() {
        let spec = overriding("Error");
        let spec = Some(&spec);
        assert_eq!(
            model(parse_quote!(Self::Error), spec).unwrap().value,
            Value::Into
        );
        assert_eq!(
            model(parse_quote!(Option<Self::Error>), spec).unwrap(),
            ReturnModel {
                wrapper: Wrapper::Option,
                value: Value::Into,
            }
        );
        assert_eq!(
            model(parse_quote!(Result<Self, Self::Error>), spec).unwrap(),
            ReturnModel {
                wrapper: Wrapper::Result {
                    error: Conversion::Into,
                },
                value: Value::Rewrap,
            }
        );
        assert_eq!(
            model(parse_quote!(Self::Other), spec).unwrap().value,
            Value::Plain
        );
    }

    #[test]
    fn self_anywhere_else_is_a_limit() {
        assert!(model(parse_quote!(Vec<Self>), None).is_err());
        assert!(model(parse_quote!(Result<u32, Self>), None).is_err());
        assert!(model(parse_quote!(Option<Option<Self>>), None).is_err());
    }
}
