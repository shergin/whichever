//! Classifying the shapes of receivers, arguments and return types.
use std::collections::HashSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote_spanned;
use syn::visit::Visit;
use syn::{
    Error, FnArg, GenericArgument, GenericParam, Ident, PathArguments, PathSegment, Receiver,
    Result, ReturnType, Signature, Type, TypeParamBound, TypePath,
};

use crate::options::TraitSpec;

/// Returns the identifier of a bare single-segment path type without
/// arguments, such as a type parameter or a primitive.
pub(crate) fn bare_ident(ty: &Type) -> Option<&Ident> {
    single_segment(ty)
        .filter(|segment| segment.arguments.is_none())
        .map(|segment| &segment.ident)
}

/// What the arms of a method do with the payload's result, as a tree over
/// the return type: nothing, a rewrap of `Self` into the variant, an `Into`
/// of an overridden `Self::X`, those through `.map(..)` and `.map_err(..)`
/// of an `Option` or a `Result`, or element by element through a tuple.
/// `Result<(Self, Self::Error), E>` is a `Map` of a `Tuple` of a `Rewrap`
/// and an `Into`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Conversion {
    /// The payload's result as it is.
    Keep,
    /// `Self`: the payload's result goes back into the variant that produced
    /// it.
    Rewrap,
    /// `Self::X` with `X` overridden: the payload's own type converts with
    /// `Into`.
    Into,
    /// `Option<T>`, or a `Result<T>` with one argument, as `io::Result<T>`:
    /// `.map(..)` of the inner conversion, which has to be a path.
    Map(Box<Conversion>),
    /// `Result<T, E>`: `.map(..)` of the first and `.map_err(..)` of the
    /// second, each a path.
    MapErr(Box<Conversion>, Box<Conversion>),
    /// `(A, B, ..)`: the tuple taken apart, each element converted, and the
    /// tuple built again.
    Tuple(Vec<Conversion>),
}

impl Conversion {
    /// Returns whether a `Self` is rewrapped anywhere in the result, which
    /// a routed target cannot do.
    pub(crate) fn rewraps(&self) -> bool {
        match self {
            Conversion::Keep | Conversion::Into => false,
            Conversion::Rewrap => true,
            Conversion::Map(inner) => inner.rewraps(),
            Conversion::MapErr(value, error) => value.rewraps() || error.rewraps(),
            Conversion::Tuple(items) => items.iter().any(Conversion::rewraps),
        }
    }

    /// Returns the conversion of the value under an `Option` or a `Result`,
    /// for a `with` that lifts the rebuilt value into the wrapper itself.
    pub(crate) fn unwrapped(self) -> Self {
        match self {
            Conversion::Map(inner) | Conversion::MapErr(inner, _) => *inner,
            other => other,
        }
    }

    /// Returns whether the conversion is a path a `.map(..)` can take:
    /// nothing, the variant's constructor, or `Into::into`.
    fn is_a_path(&self) -> bool {
        matches!(
            self,
            Conversion::Keep | Conversion::Rewrap | Conversion::Into
        )
    }

    /// Folds a tree whose every leaf keeps the value into `Keep`.
    fn simplified(self) -> Self {
        match self {
            Conversion::Map(inner) if *inner == Conversion::Keep => Conversion::Keep,
            Conversion::MapErr(value, error)
                if *value == Conversion::Keep && *error == Conversion::Keep =>
            {
                Conversion::Keep
            }
            Conversion::Tuple(items) if items.iter().all(|item| *item == Conversion::Keep) => {
                Conversion::Keep
            }
            other => other,
        }
    }
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

/// What the classification of a return type depends on besides the type.
pub(crate) struct ReturnSite<'a> {
    /// The subject's name, for the message about a `Self` that cannot be
    /// wrapped back into it.
    pub(crate) subject: &'a Ident,
    /// Whether the subject holds one target, as a struct does, so that the
    /// `match` over it has one arm and an opaque return type is the one
    /// type that arm returns.
    pub(crate) one_target: bool,
    /// The way out a message offers: an impl by hand on the trait path, a
    /// body on the inherent path.
    pub(crate) by_hand: &'a str,
}

/// Classifies what the arms of a method must do with the payload's result,
/// and whether the method returns `impl Future`, in which case the arms are
/// awaited inside one `async` block and the classification applies to the
/// future's `Output`. `spec` carries the associated type overrides of a
/// trait; inherent methods have none.
///
/// A `match` with several arms cannot return `impl Trait`, since every arm
/// would return a different opaque type; a future is the exception, because
/// one `async` block around the `match` is one future. A `match` with one
/// arm returns that arm's type, so a struct forwards any opaque type as it
/// is, a future included, unless its `Output` has to be converted, in which
/// case the `async` block is where the conversion happens.
pub(crate) fn classify_return(
    output: &ReturnType,
    spec: Option<&TraitSpec>,
    site: &ReturnSite,
) -> Result<(Conversion, bool)> {
    let ReturnType::Type(_, ty) = output else {
        return Ok((Conversion::Keep, false));
    };
    let (ty, returns_future) = match future_output(ty) {
        Some(Some(output)) => (output, true),
        Some(None) => return Ok((Conversion::Keep, !site.one_target)),
        None => (&**ty, false),
    };
    if site.one_target {
        if let Type::ImplTrait(_) = ty {
            return Ok((Conversion::Keep, false));
        }
        let conversion = return_model(ty, spec, site)?;
        let awaited = returns_future && conversion != Conversion::Keep;
        return Ok((conversion, awaited));
    }
    if let Some(opaque) = first_impl_trait(ty) {
        return Err(Error::new_spanned(
            opaque,
            "a return-position `impl Trait` cannot be dispatched by a `match` because every variant returns a different opaque type; return a concrete type or a boxed trait object, or `impl Future<Output = T>`, which is awaited like an `async fn`",
        ));
    }
    Ok((return_model(ty, spec, site)?, returns_future))
}

/// Returns the first `impl Trait` in `ty`, at any depth: the type itself,
/// or one inside `Result<impl Stream, E>`.
fn first_impl_trait(ty: &Type) -> Option<&Type> {
    struct Finder<'ast>(Option<&'ast Type>);
    impl<'ast> Visit<'ast> for Finder<'ast> {
        fn visit_type(&mut self, ty: &'ast Type) {
            if self.0.is_some() {
                return;
            }
            if let Type::ImplTrait(_) = ty {
                self.0 = Some(ty);
                return;
            }
            syn::visit::visit_type(self, ty);
        }
    }
    let mut finder = Finder(None);
    finder.visit_type(ty);
    finder.0
}

/// Classifies a return type as the conversion its arms apply, and checks
/// that the conversion is one a hand writes without a closure: a path
/// under `.map(..)` and `.map_err(..)`, an `Into` or nothing in a
/// `Result`'s error position, and anything through a tuple. Errors are
/// reported on the whole type, since the message speaks of the return
/// type.
pub(crate) fn return_model(
    ty: &Type,
    spec: Option<&TraitSpec>,
    site: &ReturnSite,
) -> Result<Conversion> {
    let conversion = classify(ty, ty, spec, site)?;
    check_expressible(&conversion, ty, site)?;
    Ok(conversion)
}

/// Classifies one position of the return type `whole`.
fn classify(
    ty: &Type,
    whole: &Type,
    spec: Option<&TraitSpec>,
    site: &ReturnSite,
) -> Result<Conversion> {
    if is_self(ty) {
        return Ok(Conversion::Rewrap);
    }
    if let Some(name) = self_assoc(ty) {
        let overridden = spec.is_some_and(|spec| spec.is_overridden(name));
        return Ok(if overridden {
            Conversion::Into
        } else {
            Conversion::Keep
        });
    }
    if let Some(segment) = last_segment(ty) {
        let arguments = generic_type_args(segment);
        let wrapped = match (segment.ident.to_string().as_str(), arguments.as_slice()) {
            ("Option", [inner]) => Some(Conversion::Map(Box::new(classify(
                inner, whole, spec, site,
            )?))),
            // `io::Result<T>`: the alias fixes the error type.
            ("Result", [inner]) => Some(Conversion::Map(Box::new(classify(
                inner, whole, spec, site,
            )?))),
            ("Result", [inner, error]) => Some(Conversion::MapErr(
                Box::new(classify(inner, whole, spec, site)?),
                Box::new(classify(error, whole, spec, site)?),
            )),
            _ => None,
        };
        if let Some(conversion) = wrapped {
            return Ok(conversion.simplified());
        }
    }
    if let Type::Tuple(tuple) = ty {
        let items = tuple
            .elems
            .iter()
            .map(|item| classify(item, whole, spec, site))
            .collect::<Result<Vec<_>>>()?;
        return Ok(Conversion::Tuple(items).simplified());
    }
    if mentions_self(ty) {
        return Err(self_limit(whole, site));
    }
    Ok(Conversion::Keep)
}

/// The sentence for a `Self` the arms cannot wrap back into the subject.
fn self_limit(whole: &Type, site: &ReturnSite) -> Error {
    let ReturnSite {
        subject, by_hand, ..
    } = site;
    Error::new_spanned(
        whole,
        format!("this return type mentions `Self` where `whichever` cannot wrap the payload's result back into `{subject}`; return `Self`, `Option<Self>`, `Result<Self, E>` or a tuple of those, spelled with those names, or {by_hand}"),
    )
}

/// Checks that the arms can apply `conversion` without a closure, which a
/// hand would write and the attribute does not: `.map(..)` takes a path,
/// so the value under an `Option` or a `Result` converts as one, and a
/// `Result`'s error converts with `Into` or not at all.
fn check_expressible(conversion: &Conversion, whole: &Type, site: &ReturnSite) -> Result<()> {
    match conversion {
        Conversion::Keep | Conversion::Rewrap | Conversion::Into => Ok(()),
        Conversion::Map(inner) if inner.is_a_path() => Ok(()),
        Conversion::MapErr(value, error)
            if value.is_a_path() && matches!(**error, Conversion::Keep | Conversion::Into) =>
        {
            Ok(())
        }
        Conversion::Tuple(items) => items
            .iter()
            .try_for_each(|item| check_expressible(item, whole, site)),
        _ => Err(self_limit(whole, site)),
    }
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

/// Returns the generic arguments a call of the method has to spell out:
/// every type and const parameter, when at least one of them is named by
/// no argument's type, so that inference could not find it. A hand writes
/// the turbofish only then, and so does the generated call.
pub(crate) fn turbofish(signature: &Signature) -> Option<TokenStream2> {
    let mut named = IdentCollector::default();
    for input in &signature.inputs {
        if let FnArg::Typed(pat_type) = input {
            named.visit_type(&pat_type.ty);
        }
    }
    let parameters: Vec<&Ident> = signature
        .generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Type(param) => Some(&param.ident),
            GenericParam::Const(param) => Some(&param.ident),
            GenericParam::Lifetime(_) => None,
        })
        .collect();
    let inferred = parameters
        .iter()
        .all(|parameter| named.0.contains(&parameter.to_string()));
    if parameters.is_empty() || inferred {
        return None;
    }
    let site = crate::at(signature.ident.span());
    Some(quote_spanned!(site => ::<#(#parameters),*>))
}

/// Every identifier a type names: in its paths, their generic arguments,
/// and the expressions of its array lengths.
#[derive(Default)]
struct IdentCollector(HashSet<String>);

impl<'ast> Visit<'ast> for IdentCollector {
    fn visit_ident(&mut self, ident: &'ast Ident) {
        self.0.insert(ident.to_string());
    }
}

/// Returns whether a method takes a `Self`, `&Self` or `&mut Self` argument,
/// which the generated `match` looks at beside the receiver.
pub(crate) fn takes_self_argument(signature: &Signature) -> bool {
    signature.inputs.iter().any(|input| match input {
        FnArg::Typed(pat_type) => {
            matches!(
                argument_kind(&pat_type.ty),
                Ok(ArgumentKind::SameVariant(_))
            )
        }
        FnArg::Receiver(_) => false,
    })
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

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::*;

    /// The site of a trait method on `Kind`, an enum, or on a struct.
    fn site(subject: &Ident, one_target: bool) -> ReturnSite<'_> {
        ReturnSite {
            subject,
            one_target,
            by_hand: "implement the method by hand",
        }
    }

    fn kind() -> Ident {
        Ident::new("Kind", proc_macro2::Span::call_site())
    }

    fn model(ty: Type, spec: Option<&TraitSpec>) -> Result<Conversion> {
        return_model(&ty, spec, &site(&kind(), false))
    }

    /// A spec fixing the associated type `name` on the enum.
    fn overriding(name: &str) -> TraitSpec {
        syn::parse_str(&format!("Trait, {name} = Fixed")).expect("a valid spec")
    }

    #[test]
    fn wrappers_are_recognised_by_their_last_segment() {
        assert_eq!(model(parse_quote!(u32), None).unwrap(), Conversion::Keep);
        assert_eq!(model(parse_quote!(Self), None).unwrap(), Conversion::Rewrap);
        assert_eq!(
            model(parse_quote!(io::Result<Self>), None).unwrap(),
            Conversion::Map(Box::new(Conversion::Rewrap))
        );
        assert_eq!(
            model(parse_quote!(::core::option::Option<Self>), None).unwrap(),
            Conversion::Map(Box::new(Conversion::Rewrap))
        );
        assert_eq!(
            model(parse_quote!(Option<u32>), None).unwrap(),
            Conversion::Keep,
            "a wrapper around nothing to convert is nothing to convert"
        );
    }

    #[test]
    fn an_overridden_associated_type_converts_in_every_position() {
        let spec = overriding("Error");
        let spec = Some(&spec);
        assert_eq!(
            model(parse_quote!(Self::Error), spec).unwrap(),
            Conversion::Into
        );
        assert_eq!(
            model(parse_quote!(Option<Self::Error>), spec).unwrap(),
            Conversion::Map(Box::new(Conversion::Into))
        );
        assert_eq!(
            model(parse_quote!(Result<Self, Self::Error>), spec).unwrap(),
            Conversion::MapErr(Box::new(Conversion::Rewrap), Box::new(Conversion::Into))
        );
        assert_eq!(
            model(parse_quote!(Self::Other), spec).unwrap(),
            Conversion::Keep
        );
    }

    #[test]
    fn a_tuple_converts_element_by_element() {
        let spec = overriding("Out");
        assert_eq!(
            model(parse_quote!((Self, u32)), None).unwrap(),
            Conversion::Tuple(vec![Conversion::Rewrap, Conversion::Keep])
        );
        assert_eq!(
            model(parse_quote!((Self::Out, Option<Self>)), Some(&spec)).unwrap(),
            Conversion::Tuple(vec![
                Conversion::Into,
                Conversion::Map(Box::new(Conversion::Rewrap))
            ])
        );
        assert_eq!(
            model(parse_quote!((u32, bool)), None).unwrap(),
            Conversion::Keep
        );
        assert!(
            model(parse_quote!(Option<(Self, u32)>), None).is_err(),
            "a tuple under a wrapper needs a closure"
        );
    }

    #[test]
    fn self_anywhere_else_is_a_limit() {
        assert!(model(parse_quote!(Vec<Self>), None).is_err());
        assert!(model(parse_quote!(Result<u32, Self>), None).is_err());
        assert!(model(parse_quote!(Option<Option<Self>>), None).is_err());
    }

    #[test]
    fn receivers_are_classified_by_their_type() {
        let receiver = |receiver: Receiver| classify_receiver(&receiver);
        assert!(matches!(
            receiver(parse_quote!(&self)),
            Ok(ReceiverKind::Value(Access::Shared))
        ));
        assert!(matches!(
            receiver(parse_quote!(&mut self)),
            Ok(ReceiverKind::Value(Access::Exclusive))
        ));
        assert!(matches!(
            receiver(parse_quote!(self)),
            Ok(ReceiverKind::Value(Access::Owned))
        ));
        assert!(matches!(
            receiver(parse_quote!(self: Box<Self>)),
            Ok(ReceiverKind::Boxed)
        ));
        assert!(matches!(
            receiver(parse_quote!(self: Pin<&mut Self>)),
            Ok(ReceiverKind::Pinned(Access::Exclusive))
        ));
        assert!(matches!(
            receiver(parse_quote!(self: Pin<&Self>)),
            Ok(ReceiverKind::Pinned(Access::Shared))
        ));
        assert!(receiver(parse_quote!(self: Rc<Self>)).is_err());
    }

    #[test]
    fn self_arguments_are_told_from_the_rest() {
        assert!(matches!(
            argument_kind(&parse_quote!(&Self)),
            Ok(ArgumentKind::SameVariant(Access::Shared))
        ));
        assert!(matches!(
            argument_kind(&parse_quote!(&mut Self)),
            Ok(ArgumentKind::SameVariant(Access::Exclusive))
        ));
        assert!(matches!(
            argument_kind(&parse_quote!(Self)),
            Ok(ArgumentKind::SameVariant(Access::Owned))
        ));
        assert!(matches!(
            argument_kind(&parse_quote!(u32)),
            Ok(ArgumentKind::Plain)
        ));
        assert!(argument_kind(&parse_quote!(Option<&Self>)).is_err());
    }

    #[test]
    fn futures_are_awaited_and_other_opaque_returns_are_limits() {
        let kind = kind();
        let returns = |output: ReturnType| classify_return(&output, None, &site(&kind, false));
        let (model, future) = returns(parse_quote!(-> impl Future<Output = Self> + Send)).unwrap();
        assert!(future);
        assert_eq!(model, Conversion::Rewrap);
        let (model, future) = returns(parse_quote!(-> impl Future)).unwrap();
        assert!(future);
        assert_eq!(model, Conversion::Keep);
        assert!(returns(parse_quote!(-> impl Iterator<Item = u32>)).is_err());
        assert!(
            returns(parse_quote!(-> Result<impl Iterator<Item = u32>, E>)).is_err(),
            "an opaque type is found at any depth"
        );
        let (_, future) = returns(parse_quote!()).unwrap();
        assert!(!future);
    }

    #[test]
    fn one_target_forwards_an_opaque_return_as_it_is() {
        let kind = kind();
        let returns =
            |output: ReturnType| classify_return(&output, None, &site(&kind, true)).unwrap();
        assert_eq!(
            returns(parse_quote!(-> impl Iterator<Item = u32>)),
            (Conversion::Keep, false)
        );
        assert_eq!(
            returns(parse_quote!(-> impl Future<Output = u32> + Send)),
            (Conversion::Keep, false),
            "a future whose output needs nothing is forwarded without an `async` block"
        );
        let (model, future) = returns(parse_quote!(-> impl Future<Output = Self>));
        assert!(
            future,
            "a future whose output is rewrapped is awaited in an `async` block"
        );
        assert_eq!(model, Conversion::Rewrap);
    }

    #[test]
    fn the_self_limit_names_the_subject_and_the_way_out() {
        let error = model(parse_quote!(Vec<Self>), None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("back into `Kind`"), "{error}");
        assert!(
            error.ends_with("or implement the method by hand"),
            "{error}"
        );
    }

    #[test]
    fn a_turbofish_is_written_only_when_no_argument_fixes_a_parameter() {
        let spelled = |signature: Signature| turbofish(&signature).map(|tokens| tokens.to_string());
        assert_eq!(
            spelled(parse_quote!(fn parse<T: FromStr>(&self) -> Option<T>)),
            Some(quote!(::<T>).to_string())
        );
        assert_eq!(
            spelled(parse_quote!(fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32)),
            None
        );
        assert_eq!(
            spelled(parse_quote!(fn zeros<const N: usize>(&self) -> [u8; N])),
            Some(quote!(::<N>).to_string())
        );
        assert_eq!(
            spelled(parse_quote!(fn pick<'a, T>(&'a self, from: &'a [T]) -> &'a T)),
            None
        );
        assert_eq!(
            spelled(parse_quote!(fn both<T, U>(&self, t: T) -> U)),
            Some(quote!(::<T, U>).to_string())
        );
    }
}
