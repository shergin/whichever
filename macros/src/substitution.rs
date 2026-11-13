//! Replacing generic parameters with the arguments named for them: the
//! trait's with the arguments the subject names, and the subject's own with
//! the arguments a partial block's header names.
use std::collections::HashSet;

use quote::ToTokens;
use syn::visit_mut::{self, VisitMut};
use syn::{
    ConstParam, Error, Expr, GenericArgument, GenericParam, Generics, Ident, ItemTrait, Lifetime,
    Path, PathArguments, PathSegment, QSelf, Result, Type, TypeParam, TypeParamBound, TypePath,
};

use crate::combined;
use crate::shape::bare_ident;

/// Replaces generic parameters with the arguments named for them.
pub(crate) struct Substitution {
    /// Each type parameter, its argument, and the one bound that qualifies a
    /// projection through it, when it has exactly one.
    pub(crate) types: Vec<(Ident, Type, Option<Path>)>,
    pub(crate) lifetimes: Vec<(Lifetime, Lifetime)>,
    pub(crate) consts: Vec<(Ident, Expr)>,
    /// The type parameters in scope where the result is written, which may
    /// stand before `::` unqualified.
    parameters: HashSet<String>,
    /// The projections that could not be written, reported after a visit.
    errors: Vec<Error>,
}

impl Substitution {
    /// The trait's parameters replaced by the arguments `trait_path` names,
    /// for a result written where `scope` is in scope.
    pub(crate) fn of_trait(
        item_trait: &ItemTrait,
        trait_path: &Path,
        scope: &Generics,
    ) -> Result<Self> {
        let params: Vec<&GenericParam> = item_trait.generics.params.iter().collect();
        let last = trait_path
            .segments
            .last()
            .expect("a path has at least one segment");
        let mut args: Vec<GenericArgument> = vec![];
        if let PathArguments::AngleBracketed(arguments) = &last.arguments {
            for argument in &arguments.args {
                if let GenericArgument::AssocType(_)
                | GenericArgument::AssocConst(_)
                | GenericArgument::Constraint(_) = argument
                {
                    return Err(Error::new_spanned(
                        argument,
                        "an associated type or const is fixed with an option after the path, `Name = ..`, not inside it",
                    ));
                }
                args.push(argument.clone());
            }
        }
        // A parameter with a default may be left out, as `impl Enc for A`
        // leaves out the `F = Json` of `trait Enc<F = Json>`; only trailing
        // parameters have one.
        for param in params.iter().skip(args.len()) {
            match param {
                GenericParam::Type(TypeParam {
                    default: Some((_, default)),
                    ..
                }) => args.push(GenericArgument::Type(default.clone())),
                GenericParam::Const(ConstParam {
                    default: Some((_, default)),
                    ..
                }) => args.push(GenericArgument::Const(default.clone())),
                _ => break,
            }
        }
        if args.len() != params.len() {
            let count = params.len();
            let plural = if count == 1 { "" } else { "s" };
            let named = params.iter().map(|param| match param {
                GenericParam::Lifetime(param) => param.lifetime.to_token_stream().to_string(),
                GenericParam::Type(param) => param.ident.to_string(),
                GenericParam::Const(param) => param.ident.to_string(),
            });
            let named = named.collect::<Vec<_>>().join(", ");
            return Err(Error::new_spanned(
                trait_path,
                format!(
                    "`{}` takes {count} generic argument{plural}; name {}, `{}<{named}>`, or write the impl's header in an empty `#[whichever::dispatch] impl<{named}> {}<{named}> for ..` block",
                    item_trait.ident,
                    if count == 1 { "it" } else { "them" },
                    item_trait.ident,
                    item_trait.ident,
                ),
            ));
        }
        Self::of_pairs(params.into_iter().zip(args.iter()), scope)
    }

    /// The subject's parameters replaced by the arguments a partial block's
    /// self type names, `Kind<U>` over `enum Kind<T>`; nothing when the
    /// header names them otherwise than the subject declares them, which
    /// rustc reports on the header.
    pub(crate) fn of_subject(generics: &Generics, self_ty: &Type) -> Option<Self> {
        let Type::Path(TypePath {
            qself: None, path, ..
        }) = self_ty
        else {
            return None;
        };
        let PathArguments::AngleBracketed(arguments) = &path.segments.last()?.arguments else {
            return None;
        };
        if arguments.args.len() != generics.params.len() {
            return None;
        }
        let scope = Generics::default();
        let pairs = generics.params.iter().zip(arguments.args.iter());
        Self::of_pairs(pairs, &scope).ok()
    }

    fn of_pairs<'a>(
        pairs: impl Iterator<Item = (&'a GenericParam, &'a GenericArgument)>,
        scope: &Generics,
    ) -> Result<Self> {
        let mut substitution = Self {
            types: vec![],
            lifetimes: vec![],
            consts: vec![],
            parameters: scope
                .params
                .iter()
                .filter_map(|param| match param {
                    GenericParam::Type(param) => Some(param.ident.to_string()),
                    _ => None,
                })
                .collect(),
            errors: vec![],
        };
        for (param, argument) in pairs {
            match (param, argument) {
                (GenericParam::Type(param), GenericArgument::Type(ty)) => {
                    let single_bound = match param.bounds.iter().collect::<Vec<_>>().as_slice() {
                        [TypeParamBound::Trait(bound)] => Some(bound.path.clone()),
                        _ => None,
                    };
                    substitution
                        .types
                        .push((param.ident.clone(), ty.clone(), single_bound));
                }
                (GenericParam::Lifetime(param), GenericArgument::Lifetime(lifetime)) => {
                    substitution
                        .lifetimes
                        .push((param.lifetime.clone(), lifetime.clone()));
                }
                (GenericParam::Const(param), GenericArgument::Const(expr)) => {
                    substitution
                        .consts
                        .push((param.ident.clone(), expr.clone()));
                }
                // A bare identifier given for a const parameter parses as a type.
                (GenericParam::Const(param), GenericArgument::Type(Type::Path(path)))
                    if path.qself.is_none() =>
                {
                    let expr = Expr::Path(syn::ExprPath {
                        attrs: vec![],
                        qself: None,
                        path: path.path.clone(),
                    });
                    substitution.consts.push((param.ident.clone(), expr));
                }
                _ => {
                    return Err(Error::new_spanned(
                        argument,
                        "generic argument does not match the trait's parameter",
                    ))
                }
            }
        }
        Ok(substitution)
    }

    pub(crate) fn const_for(&self, ident: &Ident) -> Option<&Expr> {
        self.consts
            .iter()
            .find(|(name, _)| name == ident)
            .map(|(_, expr)| expr)
    }

    /// Reports the projections the last visits could not write.
    pub(crate) fn check(&mut self) -> Result<()> {
        match combined(std::mem::take(&mut self.errors)) {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Rewrites `W::Assoc`, a projection through the parameter `W` given as
    /// `replacement`: `<Arg as Bound>::Assoc` when `W` has exactly one bound
    /// to qualify it with, `U::Assoc` when the argument is a type parameter
    /// in scope where the result is written, and otherwise nothing that can
    /// be written, which is recorded for `check`.
    fn project(&mut self, type_path: &mut TypePath, replacement: &Type, bound: Option<&Path>) {
        let rest: Vec<PathSegment> = type_path.path.segments.iter().skip(1).cloned().collect();
        if let Some(bound) = bound {
            let mut path = bound.clone();
            let position = path.segments.len();
            path.segments.extend(rest);
            *type_path = TypePath {
                attrs: std::mem::take(&mut type_path.attrs),
                qself: Some(QSelf {
                    lt_token: Default::default(),
                    ty: Box::new(replacement.clone()),
                    position,
                    as_token: Some(Default::default()),
                    gt_token: Default::default(),
                }),
                path,
            };
            return;
        }
        if let Some(parameter) =
            bare_ident(replacement).filter(|ident| self.parameters.contains(&ident.to_string()))
        {
            let mut path = Path::from(parameter.clone());
            path.segments.extend(rest);
            type_path.path = path;
            return;
        }
        let parameter = &type_path.path.segments[0].ident;
        let projection = type_path.to_token_stream().to_string().replace(' ', "");
        let argument = replacement.to_token_stream().to_string().replace(' ', "");
        let assoc = rest
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        self.errors.push(Error::new_spanned(
            &*type_path,
            format!("`{projection}` cannot be written with `{argument}` for `{parameter}`, which has no single bound to say which trait `{assoc}` belongs to; write `<{parameter} as Trait>::{assoc}` in the trait"),
        ));
    }
}

impl VisitMut for Substitution {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        if let Type::Path(type_path) = ty {
            if type_path.qself.is_none() {
                let first = &type_path.path.segments[0];
                let replacement = self
                    .types
                    .iter()
                    .find(|(ident, _, _)| *ident == first.ident)
                    .map(|(_, replacement, bound)| (replacement.clone(), bound.clone()));
                if let Some((replacement, bound)) = replacement {
                    if type_path.path.segments.len() == 1 {
                        *ty = replacement;
                        return;
                    }
                    // The arguments of the later segments may name
                    // parameters too, and are visited before the splice.
                    for segment in type_path.path.segments.iter_mut().skip(1) {
                        self.visit_path_segment_mut(segment);
                    }
                    self.project(type_path, &replacement, bound.as_ref());
                    return;
                }
            }
        }
        visit_mut::visit_type_mut(self, ty);
    }

    fn visit_lifetime_mut(&mut self, lifetime: &mut Lifetime) {
        if let Some((_, replacement)) = self.lifetimes.iter().find(|(from, _)| from == lifetime) {
            *lifetime = replacement.clone();
        }
    }

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Path(path) = expr {
            if path.qself.is_none() && path.path.segments.len() == 1 {
                if let Some(replacement) = self.const_for(&path.path.segments[0].ident) {
                    *expr = replacement.clone();
                    return;
                }
            }
        }
        visit_mut::visit_expr_mut(self, expr);
    }

    /// A const parameter in argument position parses as a type when it is a
    /// bare identifier; it becomes the const argument the enum named.
    fn visit_generic_argument_mut(&mut self, argument: &mut GenericArgument) {
        if let GenericArgument::Type(ty) = argument {
            if let Some(replacement) = bare_ident(ty).and_then(|ident| self.const_for(ident)) {
                *argument = GenericArgument::Const(replacement.clone());
                return;
            }
        }
        visit_mut::visit_generic_argument_mut(self, argument);
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::*;

    fn spelled(ty: &Type) -> String {
        ty.to_token_stream().to_string().replace(' ', "")
    }

    #[test]
    fn a_trait_parameter_is_replaced_and_its_projection_qualified() {
        let item_trait: ItemTrait = parse_quote!(
            trait Encoder<F: Format, G> {
                fn encode(&self, value: F::Value, other: G) -> Vec<F::Value>;
            }
        );
        let path: Path = parse_quote!(Encoder<Json, u8>);
        let mut substitution =
            Substitution::of_trait(&item_trait, &path, &Generics::default()).unwrap();
        let mut ty: Type = parse_quote!((F::Value, G, Vec<F::Value>));
        substitution.visit_type_mut(&mut ty);
        assert!(substitution.check().is_ok());
        assert_eq!(
            spelled(&ty),
            "(<JsonasFormat>::Value,u8,Vec<<JsonasFormat>::Value>)"
        );
    }

    #[test]
    fn a_projection_through_a_parameter_with_two_bounds_is_a_limit() {
        let item_trait: ItemTrait = parse_quote!(
            trait Encoder<F: Format + Clone> {
                fn encode(&self) -> F::Value;
            }
        );
        let path: Path = parse_quote!(Encoder<Json>);
        let scope: Generics = parse_quote!(<T>);
        let mut substitution = Substitution::of_trait(&item_trait, &path, &scope).unwrap();
        let mut ty: Type = parse_quote!(F::Value);
        substitution.visit_type_mut(&mut ty);
        let error = substitution.check().unwrap_err().to_string();
        assert!(
            error.starts_with("`F::Value` cannot be written with `Json` for `F`"),
            "{error}"
        );
        // The enum's own parameter stands before `::` as it is.
        let path: Path = parse_quote!(Encoder<T>);
        let mut substitution = Substitution::of_trait(&item_trait, &path, &scope).unwrap();
        let mut ty: Type = parse_quote!(F::Value);
        substitution.visit_type_mut(&mut ty);
        assert!(substitution.check().is_ok());
        assert_eq!(spelled(&ty), "T::Value");
    }

    #[test]
    fn a_parameter_with_a_default_may_be_left_out() {
        let item_trait: ItemTrait = parse_quote!(
            trait Encoder<F = Json, const N: usize = 4> {
                fn encode(&self, value: F) -> [u8; N];
            }
        );
        let bare: Path = parse_quote!(Encoder);
        let mut substitution =
            Substitution::of_trait(&item_trait, &bare, &Generics::default()).unwrap();
        let mut ty: Type = parse_quote!((F, [u8; N]));
        substitution.visit_type_mut(&mut ty);
        assert_eq!(spelled(&ty), "(Json,[u8;4])");
        let first_only: Path = parse_quote!(Encoder<Xml>);
        let mut substitution =
            Substitution::of_trait(&item_trait, &first_only, &Generics::default()).unwrap();
        let mut ty: Type = parse_quote!((F, [u8; N]));
        substitution.visit_type_mut(&mut ty);
        assert_eq!(spelled(&ty), "(Xml,[u8;4])");
        let without_default: ItemTrait = parse_quote!(
            trait Encoder<F> {
                fn encode(&self, value: F);
            }
        );
        assert!(Substitution::of_trait(&without_default, &bare, &Generics::default()).is_err());
    }

    #[test]
    fn the_argument_count_message_names_the_parameters() {
        let item_trait: ItemTrait = parse_quote!(
            trait ViewOp<'a, T, const N: usize> {}
        );
        let path: Path = parse_quote!(ViewOp);
        let error = Substitution::of_trait(&item_trait, &path, &Generics::default())
            .err()
            .expect("an error")
            .to_string();
        assert_eq!(
            error,
            "`ViewOp` takes 3 generic arguments; name them, `ViewOp<'a, T, N>`, or write the impl's header in an empty `#[whichever::dispatch] impl<'a, T, N> ViewOp<'a, T, N> for ..` block"
        );
    }

    #[test]
    fn a_blocks_header_renames_the_subjects_parameters() {
        let generics: Generics = parse_quote!(<'a, T, const N: usize>);
        let self_ty: Type = parse_quote!(Kind<'b, U, 4>);
        let mut renaming = Substitution::of_subject(&generics, &self_ty).unwrap();
        let mut payload: Type = parse_quote!(Wrap<'a, T, N>);
        renaming.visit_type_mut(&mut payload);
        assert_eq!(spelled(&payload), "Wrap<'b,U,4>");
        let plain: Type = parse_quote!(Kind);
        assert!(Substitution::of_subject(&generics, &plain).is_none());
        let fewer: Type = parse_quote!(Kind<'b>);
        assert!(Substitution::of_subject(&generics, &fewer).is_none());
        assert_eq!(quote!(#self_ty).to_string(), "Kind < 'b , U , 4 >");
    }
}
