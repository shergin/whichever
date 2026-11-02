//! Replacing the trait's generic parameters with the arguments the subject
//! names.
use syn::visit_mut::{self, VisitMut};
use syn::{
    Error, Expr, GenericArgument, GenericParam, Ident, ItemTrait, Lifetime, Path, PathArguments,
    PathSegment, QSelf, Result, Type, TypeParamBound, TypePath,
};

use crate::shape::bare_ident;

/// Replaces the trait's generic parameters with the arguments the enum names.
pub(crate) struct Substitution {
    pub(crate) types: Vec<(Ident, Type, Option<Path>)>,
    pub(crate) lifetimes: Vec<(Lifetime, Lifetime)>,
    pub(crate) consts: Vec<(Ident, Expr)>,
}

impl Substitution {
    pub(crate) fn new(item_trait: &ItemTrait, trait_path: &Path) -> Result<Self> {
        let params: Vec<&GenericParam> = item_trait.generics.params.iter().collect();
        let last = trait_path
            .segments
            .last()
            .expect("a path has at least one segment");
        let args: Vec<&GenericArgument> = match &last.arguments {
            PathArguments::AngleBracketed(arguments) => arguments
                .args
                .iter()
                .filter(|argument| {
                    !matches!(
                        argument,
                        GenericArgument::AssocType(_)
                            | GenericArgument::AssocConst(_)
                            | GenericArgument::Constraint(_)
                    )
                })
                .collect(),
            _ => vec![],
        };
        if args.len() != params.len() {
            return Err(Error::new_spanned(
                trait_path,
                format!(
                    "`{}` takes {} generic argument(s); name them, e.g. `Trait<Arg>`",
                    item_trait.ident,
                    params.len()
                ),
            ));
        }
        let mut substitution = Self {
            types: vec![],
            lifetimes: vec![],
            consts: vec![],
        };
        for (param, argument) in params.into_iter().zip(args) {
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
}

impl VisitMut for Substitution {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        if let Type::Path(type_path) = ty {
            if type_path.qself.is_none() {
                let first = &type_path.path.segments[0];
                let replacement = self
                    .types
                    .iter()
                    .find(|(ident, _, _)| *ident == first.ident);
                if let Some((_, replacement, bound)) = replacement {
                    if type_path.path.segments.len() == 1 {
                        *ty = replacement.clone();
                        return;
                    }
                    // `W::Assoc` becomes `<Arg as Bound>::Assoc` when the
                    // parameter has exactly one bound to qualify it with.
                    if let Some(bound) = bound {
                        let rest: Vec<PathSegment> =
                            type_path.path.segments.iter().skip(1).cloned().collect();
                        let mut path = bound.clone();
                        let position = path.segments.len();
                        path.segments.extend(rest);
                        *ty = Type::Path(TypePath {
                            qself: Some(QSelf {
                                lt_token: Default::default(),
                                ty: Box::new(replacement.clone()),
                                position,
                                as_token: Some(Default::default()),
                                gt_token: Default::default(),
                            }),
                            path,
                        });
                        return;
                    }
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
