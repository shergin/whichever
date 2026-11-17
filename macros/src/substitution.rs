//! Replacing generic parameters with the arguments named for them: the
//! trait's with the arguments the subject names, and the subject's own with
//! the arguments a partial block's header names.
use std::collections::HashSet;

use quote::ToTokens;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{
    ConstParam, Error, Expr, ExprPath, GenericArgument, GenericParam, Generics, Ident, ItemTrait,
    Lifetime, Path, PathArguments, PathSegment, QSelf, Result, Signature, Token, Type, TypeParam,
    TypeParamBound, TypePath,
};

use crate::combined;
use crate::shape::bare_ident;
use crate::spelling;

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
    /// The names that mean something where the result is written, a
    /// lifetime with its apostrophe: the generic parameters in scope, and
    /// what the impl names around a method's signature, in its header and
    /// in its arms. A method's own parameter of such a name would take the
    /// name's meaning away, so it gets another.
    in_scope: HashSet<String>,
    /// Every name the trait itself writes, which a new name must not be.
    written: HashSet<String>,
    /// The projections that could not be written, reported after a visit.
    errors: Vec<Error>,
    /// Whether a type put in a parameter's place is located where the
    /// parameter was: so for a partial block's header renaming the
    /// subject's parameters, whose arguments would otherwise put a
    /// sentence about a variant on the header.
    relocate: bool,
}

/// Returns `ty` with every token located at `span`, its resolution kept.
fn located_at(ty: &Type, span: proc_macro2::Span) -> Type {
    fn relocated(
        tokens: proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        tokens
            .into_iter()
            .map(|mut tree| {
                if let proc_macro2::TokenTree::Group(group) = &tree {
                    let mut inner =
                        proc_macro2::Group::new(group.delimiter(), relocated(group.stream(), span));
                    inner.set_span(group.span().located_at(span));
                    tree = proc_macro2::TokenTree::Group(inner);
                } else {
                    tree.set_span(tree.span().located_at(span));
                }
                tree
            })
            .collect()
    }
    syn::parse2(relocated(ty.to_token_stream(), span)).unwrap_or_else(|_| ty.clone())
}

/// Returns `bound` as the trait a projection is qualified with, `<Arg as
/// Bound>::Assoc`: its path and generic arguments without the constraints
/// on its associated items, which a bound may carry and a qualified path
/// may not. `F: Format<Value = u64>` qualifies `F::Value` as `<Json as
/// Format>::Value`, which names the same type.
fn qualifier(bound: &Path) -> Path {
    let mut path = bound.clone();
    let Some(last) = path.segments.last_mut() else {
        return path;
    };
    let PathArguments::AngleBracketed(arguments) = &mut last.arguments else {
        return path;
    };
    arguments.args = std::mem::take(&mut arguments.args)
        .into_iter()
        .filter(|argument| {
            matches!(
                argument,
                GenericArgument::Lifetime(_) | GenericArgument::Type(_) | GenericArgument::Const(_)
            )
        })
        .collect();
    if arguments.args.is_empty() {
        last.arguments = PathArguments::None;
    }
    path
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
        let mut substitution = Self::of_pairs(params.into_iter().zip(args.iter()), scope)?;
        named(
            item_trait.to_token_stream(),
            Which::Every,
            &mut substitution.written,
        );
        substitution.names(trait_path.to_token_stream());
        Ok(substitution)
    }

    /// Records the names `tokens` write as meaning something where the
    /// result is written: a payload's type, the subject's path, the
    /// header's bounds, an override. They are what the impl names around
    /// the signature of a method, whose own generic parameters keep clear
    /// of them, see `distinguish`.
    pub(crate) fn names(&mut self, tokens: proc_macro2::TokenStream) {
        named(tokens, Which::First, &mut self.in_scope);
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
        let mut substitution = Self::of_pairs(pairs, &scope).ok()?;
        substitution.relocate = true;
        Some(substitution)
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
            in_scope: scope.params.iter().map(parameter_name).collect(),
            written: HashSet::new(),
            errors: vec![],
            relocate: false,
        };
        for (param, argument) in pairs {
            match (param, argument) {
                (GenericParam::Type(param), GenericArgument::Type(ty)) => {
                    let single_bound = match param.bounds.iter().collect::<Vec<_>>().as_slice() {
                        [TypeParamBound::Trait(bound)] => Some(qualifier(&bound.path)),
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

    /// Gives the generic parameters of `signature`, a trait method's own,
    /// names that mean nothing else in the impl, before the trait's
    /// parameters are replaced in it. A method's parameter may be named
    /// like one of the subject's, `fn eq<I>(self, other: I)` over `struct
    /// Tagged<I>`: in the trait the two never meet, and in the impl they
    /// are two parameters of one name, which rustc refuses. It may as well
    /// be named like a type the impl writes into the method, a payload's
    /// in an arm or an argument the subject gave the trait, which the
    /// parameter would then stand for. A hand gives the method's
    /// parameter another name, and so does this: the name with the first
    /// number that neither the impl nor the trait writes, `I2`, at the
    /// parameter's own span. The renaming is returned for what else the
    /// trait wrote against the method's parameters, its `mismatch`
    /// expression.
    pub(crate) fn distinguish(&self, signature: &mut Signature) -> Renaming {
        let mut renaming = self.fresh_names(&signature.generics, signature.to_token_stream());
        if !renaming.is_empty() {
            renaming.visit_signature_mut(signature);
        }
        renaming
    }

    /// Does the same for `generics`, the parameters of a generic
    /// associated type, which the impl declares as the trait does: `type
    /// Many<T>` over `struct Wrap<T>` is two parameters of one name there
    /// as well.
    pub(crate) fn distinguish_generics(&self, generics: &mut Generics) {
        let mut renaming = self.fresh_names(generics, generics.to_token_stream());
        if !renaming.is_empty() {
            renaming.visit_generics_mut(generics);
        }
    }

    /// Returns the names that keep the parameters `generics` declares
    /// clear of what the impl names, where `written` is everything written
    /// against them.
    fn fresh_names(&self, generics: &Generics, written: proc_macro2::TokenStream) -> Renaming {
        let mut taken: HashSet<String> = self.in_scope.clone();
        taken.extend(self.written.iter().cloned());
        named(written, Which::Every, &mut taken);
        let mut renaming = Renaming::default();
        for param in &generics.params {
            let name = parameter_name(param);
            if !self.in_scope.contains(&name) {
                continue;
            }
            let fresh = (2..)
                .map(|number| format!("{name}{number}"))
                .find(|candidate| !taken.contains(candidate))
                .expect("some number is free");
            taken.insert(fresh.clone());
            match param {
                GenericParam::Type(param) => renaming
                    .types
                    .push((param.ident.clone(), Ident::new(&fresh, param.ident.span()))),
                GenericParam::Const(param) => renaming
                    .consts
                    .push((param.ident.clone(), Ident::new(&fresh, param.ident.span()))),
                GenericParam::Lifetime(param) => renaming.lifetimes.push((
                    param.lifetime.clone(),
                    Lifetime::new(&fresh, param.lifetime.span()),
                )),
            }
        }
        renaming
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
        let projection = spelling::spelled(&*type_path);
        let argument = spelling::spelled(replacement);
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

/// Which identifiers of some tokens are names.
#[derive(Clone, Copy)]
enum Which {
    /// Every one.
    Every,
    /// The ones that stand first in a path, which a generic parameter of
    /// the same name would stand for: `Square` and `std` in `Pair<Square,
    /// std::vec::IntoIter<u8>>`, and not `vec` or `IntoIter`, which follow
    /// `::` and are looked up in what stands before.
    First,
}

/// Collects into `names` the identifiers among `tokens` that `which` says,
/// at any depth, and every lifetime with its apostrophe.
fn named(tokens: proc_macro2::TokenStream, which: Which, names: &mut HashSet<String>) {
    let mut after_apostrophe = false;
    // The colons seen in a row: two are a path separator.
    let mut colons = 0;
    for tree in tokens {
        let (apostrophe, colon) = match &tree {
            proc_macro2::TokenTree::Punct(punct) => {
                (punct.as_char() == '\'', punct.as_char() == ':')
            }
            _ => (false, false),
        };
        match tree {
            proc_macro2::TokenTree::Ident(ident) if after_apostrophe => {
                names.insert(format!("'{ident}"));
            }
            proc_macro2::TokenTree::Ident(ident) => {
                if matches!(which, Which::Every) || colons < 2 {
                    names.insert(ident.to_string());
                }
            }
            proc_macro2::TokenTree::Group(group) => named(group.stream(), which, names),
            proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => {}
        }
        after_apostrophe = apostrophe;
        colons = if colon { colons + 1 } else { 0 };
    }
}

/// Returns the name of a generic parameter, a lifetime's with its
/// apostrophe, so that a lifetime and a type of one name are two.
fn parameter_name(param: &GenericParam) -> String {
    match param {
        GenericParam::Lifetime(param) => param.lifetime.to_string(),
        GenericParam::Type(param) => param.ident.to_string(),
        GenericParam::Const(param) => param.ident.to_string(),
    }
}

/// The new names of a method's own generic parameters, wherever the trait
/// names them: in the signature, where they are declared, in the bounds
/// and the `where` clause and in the types of the arguments and of the
/// result, and in the method's `mismatch` expression.
#[derive(Default)]
pub(crate) struct Renaming {
    types: Vec<(Ident, Ident)>,
    lifetimes: Vec<(Lifetime, Lifetime)>,
    consts: Vec<(Ident, Ident)>,
}

impl Renaming {
    fn is_empty(&self) -> bool {
        self.types.is_empty() && self.lifetimes.is_empty() && self.consts.is_empty()
    }

    /// Renames the parameters in `expression`, which the trait wrote
    /// against the method's signature as it declares it.
    pub(crate) fn expression(&mut self, expression: &mut Expr) {
        if !self.is_empty() {
            self.visit_expr_mut(expression);
        }
    }

    fn renamed(pairs: &[(Ident, Ident)], ident: &Ident) -> Option<Ident> {
        pairs
            .iter()
            .find(|(from, _)| from == ident)
            .map(|(_, to)| to.clone())
    }
}

impl VisitMut for Renaming {
    fn visit_type_param_mut(&mut self, param: &mut TypeParam) {
        if let Some(renamed) = Self::renamed(&self.types, &param.ident) {
            param.ident = renamed;
        }
        visit_mut::visit_type_param_mut(self, param);
    }

    fn visit_const_param_mut(&mut self, param: &mut ConstParam) {
        if let Some(renamed) = Self::renamed(&self.consts, &param.ident) {
            param.ident = renamed;
        }
        visit_mut::visit_const_param_mut(self, param);
    }

    /// A type parameter stands first in a path, alone or before `::`; a
    /// const parameter in argument position parses as a type when it is a
    /// bare identifier.
    fn visit_type_path_mut(&mut self, type_path: &mut TypePath) {
        if type_path.qself.is_none() {
            let alone = type_path.path.segments.len() == 1;
            let first = &mut type_path.path.segments[0];
            let renamed = Self::renamed(&self.types, &first.ident).or_else(|| {
                alone
                    .then(|| Self::renamed(&self.consts, &first.ident))
                    .flatten()
            });
            if let Some(renamed) = renamed {
                first.ident = renamed;
            }
        }
        visit_mut::visit_type_path_mut(self, type_path);
    }

    /// In an expression a const parameter stands alone, and a type
    /// parameter before `::`, `T::default()`.
    fn visit_expr_path_mut(&mut self, path: &mut ExprPath) {
        if path.qself.is_none() {
            let pairs = match path.path.segments.len() {
                1 => &self.consts,
                _ => &self.types,
            };
            let first = &mut path.path.segments[0];
            if let Some(renamed) = Self::renamed(pairs, &first.ident) {
                first.ident = renamed;
            }
        }
        visit_mut::visit_expr_path_mut(self, path);
    }

    fn visit_lifetime_mut(&mut self, lifetime: &mut Lifetime) {
        if let Some((_, renamed)) = self.lifetimes.iter().find(|(from, _)| from == lifetime) {
            *lifetime = renamed.clone();
        }
    }

    /// The arguments of a macro are tokens nothing has read: the trait's
    /// expression may name a parameter among them, `vec![T::default()]`,
    /// or capture a const one in a format string, `format!("{N}")`.
    fn visit_macro_mut(&mut self, invocation: &mut syn::Macro) {
        let name = invocation
            .path
            .segments
            .last()
            .map(|name| name.ident.to_string());
        let tokens = std::mem::take(&mut invocation.tokens);
        invocation.tokens = self.arguments(name.as_deref(), tokens);
    }
}

/// Returns which argument of the macro `name` is a format string, counted
/// from nothing, for the macros of `core` and `std` that take one. In any
/// other macro a string is text, whatever braces it holds.
fn format_argument(name: &str) -> Option<usize> {
    match name {
        "format" | "format_args" | "print" | "println" | "eprint" | "eprintln" | "panic"
        | "unreachable" | "todo" | "unimplemented" => Some(0),
        "write" | "writeln" | "assert" | "debug_assert" => Some(1),
        "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => Some(2),
        _ => None,
    }
}

impl Renaming {
    /// Renames the parameters among `tokens`, the arguments of the macro
    /// `name`, which nothing has read. A macro that takes a format string
    /// takes expressions between commas: they are read as such, so each is
    /// renamed as any expression is, and the format string, which only
    /// its place tells from another string, has the const parameters it
    /// captures renamed. The arguments of any other macro are renamed
    /// token by token, as the code they most often are, and a string among
    /// them is text; `stringify!` and `cfg!` take no code, and theirs are
    /// left as they are.
    fn arguments(
        &mut self,
        name: Option<&str>,
        tokens: proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        // What `stringify!` and `cfg!` take is no code: the first turns its
        // tokens into the text they are, and the second reads a predicate.
        if matches!(name, Some("stringify" | "cfg")) {
            return tokens;
        }
        let Some(format) = name.and_then(format_argument) else {
            return self.among(tokens);
        };
        let expressions = Punctuated::<Expr, Token![,]>::parse_terminated;
        let Ok(mut arguments) = expressions.parse2(tokens.clone()) else {
            return self.among(tokens);
        };
        for (position, argument) in arguments.iter_mut().enumerate() {
            if position == format {
                self.capture(argument);
            }
            self.visit_expr_mut(argument);
        }
        arguments.into_token_stream()
    }

    /// Renames the const parameters that `argument` captures, when it is
    /// a format string written out.
    fn capture(&self, argument: &mut Expr) {
        let Expr::Lit(syn::ExprLit {
            lit: lit @ syn::Lit::Str(_),
            ..
        }) = argument
        else {
            return;
        };
        let syn::Lit::Str(text) = &*lit else {
            return;
        };
        let literal = text.token();
        let written = literal.to_string();
        let mut captured = written.clone();
        for (from, to) in &self.consts {
            captured = capturing(&captured, &from.to_string(), &to.to_string());
        }
        if captured == written {
            return;
        }
        if let Ok(mut rewritten) = captured.parse::<proc_macro2::Literal>() {
            rewritten.set_span(literal.span());
            *lit = syn::Lit::new(rewritten);
        }
    }

    /// Renames the parameters among `tokens`, token by token: a name that
    /// stands first in a path, and a lifetime. A name after `::` or after
    /// `.` is looked up in what stands before it, and is left alone. A
    /// macro called among the tokens has its arguments renamed as its own
    /// name says.
    fn among(&mut self, tokens: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        use proc_macro2::{Group, TokenTree};
        let mut after_apostrophe = false;
        let mut colons = 0;
        let mut after_dot = false;
        // The name before a `!`, which calls a macro when a group follows.
        let mut last_name: Option<String> = None;
        let mut called: Option<String> = None;
        let mut renamed = vec![];
        for tree in tokens {
            let punct = match &tree {
                TokenTree::Punct(punct) => Some(punct.as_char()),
                _ => None,
            };
            let name = match &tree {
                TokenTree::Ident(ident) => Some(ident.to_string()),
                _ => None,
            };
            renamed.push(match tree {
                TokenTree::Group(group) => {
                    let inner = match called.as_deref() {
                        Some(called) => self.arguments(Some(called), group.stream()),
                        None => self.among(group.stream()),
                    };
                    let mut inner = Group::new(group.delimiter(), inner);
                    inner.set_span(group.span());
                    TokenTree::Group(inner)
                }
                TokenTree::Ident(ident) if after_apostrophe => {
                    let lifetime = self
                        .lifetimes
                        .iter()
                        .find(|(from, _)| from.ident == ident)
                        .map(|(_, to)| Ident::new(&to.ident.to_string(), ident.span()));
                    TokenTree::Ident(lifetime.unwrap_or(ident))
                }
                TokenTree::Ident(ident) if colons < 2 && !after_dot => {
                    let name = Self::renamed(&self.types, &ident)
                        .or_else(|| Self::renamed(&self.consts, &ident))
                        .map(|to| Ident::new(&to.to_string(), ident.span()));
                    TokenTree::Ident(name.unwrap_or(ident))
                }
                other => other,
            });
            after_apostrophe = punct == Some('\'');
            colons = if punct == Some(':') { colons + 1 } else { 0 };
            after_dot = punct == Some('.');
            called = match punct {
                Some('!') => last_name.take(),
                _ => None,
            };
            last_name = name;
        }
        renamed.into_iter().collect()
    }
}

/// Returns `literal`, a format string as written, with `from` renamed
/// `to` where the string captures it: as the argument a placeholder opens
/// with, `{N}` and `{N:>4}`, and as a width or a precision, `{:N$}`. `{{`
/// is a brace, and anything else in the text is text. A literal that is no
/// string is returned as it is.
fn capturing(literal: &str, from: &str, to: &str) -> String {
    if !(literal.starts_with('"') || literal.starts_with('r')) {
        return literal.to_owned();
    }
    let word = |character: char| character.is_alphanumeric() || character == '_';
    let mut output = String::with_capacity(literal.len());
    let mut rest = literal;
    while let Some(open) = rest.find('{') {
        output.push_str(&rest[..=open]);
        rest = &rest[open + 1..];
        if let Some(escaped) = rest.strip_prefix('{') {
            output.push('{');
            rest = escaped;
            continue;
        }
        let close = rest.find('}').unwrap_or(rest.len());
        let (placeholder, after) = rest.split_at(close);
        let mut position = 0;
        while position < placeholder.len() {
            let tail = &placeholder[position..];
            let length = tail
                .find(|character| !word(character))
                .unwrap_or(tail.len());
            if length == 0 {
                let next = tail.chars().next().map_or(1, char::len_utf8);
                output.push_str(&tail[..next]);
                position += next;
                continue;
            }
            let (name, following) = tail.split_at(length);
            let argument =
                position == 0 && (following.is_empty() || following.starts_with([':', ' ']));
            let parameter = following.starts_with('$');
            output.push_str(if name == from && (argument || parameter) {
                to
            } else {
                name
            });
            position += length;
        }
        rest = after;
    }
    output.push_str(rest);
    output
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
                        *ty = match self.relocate {
                            true => located_at(&replacement, first.ident.span()),
                            false => replacement,
                        };
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
    fn a_projection_is_qualified_without_the_bounds_constraints() {
        let qualified = |item_trait: ItemTrait| {
            let path: Path = parse_quote!(Encoder<Json>);
            let mut substitution =
                Substitution::of_trait(&item_trait, &path, &Generics::default()).unwrap();
            let mut ty: Type = parse_quote!(F::Value);
            substitution.visit_type_mut(&mut ty);
            assert!(substitution.check().is_ok());
            spelled(&ty)
        };
        assert_eq!(
            qualified(parse_quote!(
                trait Encoder<F: Format<Value = u64>> {
                    fn encode(&self, value: F::Value) -> u64;
                }
            )),
            "<JsonasFormat>::Value"
        );
        assert_eq!(
            qualified(parse_quote!(
                trait Encoder<F: Format<Text, Value = u64>> {
                    fn encode(&self, value: F::Value) -> u64;
                }
            )),
            "<JsonasFormat<Text>>::Value"
        );
        assert_eq!(
            qualified(parse_quote!(
                trait Encoder<F: Format<'static, Text, 4, Value = u64, Error: Debug>> {
                    fn encode(&self, value: F::Value) -> u64;
                }
            )),
            "<JsonasFormat<'static,Text,4>>::Value"
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

    /// The signature `distinguish` writes for `signature` under an impl
    /// whose header declares `scope`, the trait being `Shape<..>` as `path`
    /// names it.
    fn distinguished(
        item_trait: ItemTrait,
        path: Path,
        scope: Generics,
        signature: Signature,
    ) -> String {
        let substitution = Substitution::of_trait(&item_trait, &path, &scope).unwrap();
        let mut signature = signature;
        substitution.distinguish(&mut signature);
        signature.to_token_stream().to_string().replace(' ', "")
    }

    fn plain_trait() -> ItemTrait {
        parse_quote!(
            trait Shape {}
        )
    }

    #[test]
    fn a_methods_parameter_named_like_the_headers_takes_the_first_free_number() {
        let renamed = |scope: Generics, signature: Signature| {
            distinguished(plain_trait(), parse_quote!(Shape), scope, signature)
        };
        assert_eq!(
            renamed(
                parse_quote!(<I>),
                parse_quote!(fn eq<I>(self, other: I) -> bool)
            ),
            "fneq<I2>(self,other:I2)->bool"
        );
        assert_eq!(
            renamed(
                parse_quote!(<'a, T>),
                parse_quote!(fn get<'a>(&'a self, other: &'a u8) -> &'a u8)
            ),
            "fnget<'a2>(&'a2self,other:&'a2u8)->&'a2u8"
        );
        assert_eq!(
            renamed(
                parse_quote!(<const N: usize>),
                parse_quote!(fn fill<const N: usize>(&self, with: [u8; N]) -> [u8; N])
            ),
            "fnfill<constN2:usize>(&self,with:[u8;N2])->[u8;N2]"
        );
        assert_eq!(
            renamed(
                parse_quote!(<T>),
                parse_quote!(fn make<T: Default>(&self) -> Option<T>)
            ),
            "fnmake<T2:Default>(&self)->Option<T2>"
        );
    }

    #[test]
    fn a_methods_parameter_without_a_clash_keeps_its_name() {
        let signature: Signature =
            parse_quote!(fn eq<'b, I, const M: usize>(self, other: &'b [I; M]) -> bool);
        assert_eq!(
            distinguished(
                plain_trait(),
                parse_quote!(Shape),
                parse_quote!(<'a, T, const N: usize>),
                signature
            ),
            "fneq<'b,I,constM:usize>(self,other:&'b[I;M])->bool"
        );
        assert_eq!(
            distinguished(
                plain_trait(),
                parse_quote!(Shape),
                parse_quote!(<'I>),
                parse_quote!(fn eq<I>(self, other: I) -> bool)
            ),
            "fneq<I>(self,other:I)->bool",
            "a lifetime and a type of one name are two"
        );
    }

    #[test]
    fn the_next_number_is_taken_where_the_first_is_used() {
        let eq: Signature = parse_quote!(fn eq<I>(self, other: I) -> bool);
        assert_eq!(
            distinguished(
                plain_trait(),
                parse_quote!(Shape),
                parse_quote!(<I, I2>),
                eq.clone()
            ),
            "fneq<I3>(self,other:I3)->bool",
            "the header uses `I2`"
        );
        let generic: ItemTrait = parse_quote!(
            trait Shape<I2> {}
        );
        assert_eq!(
            distinguished(generic, parse_quote!(Shape<u8>), parse_quote!(<I>), eq),
            "fneq<I3>(self,other:I3)->bool",
            "the trait's own parameter is `I2`"
        );
        assert_eq!(
            distinguished(
                plain_trait(),
                parse_quote!(Shape),
                parse_quote!(<I>),
                parse_quote!(fn zip<I, I2>(self, one: I, other: I2))
            ),
            "fnzip<I3,I2>(self,one:I3,other:I2)",
            "another parameter of the method is `I2`"
        );
        assert_eq!(
            distinguished(
                plain_trait(),
                parse_quote!(Shape),
                parse_quote!(<A, B>),
                parse_quote!(fn zip<A, B>(self, one: A, other: B))
            ),
            "fnzip<A2,B2>(self,one:A2,other:B2)"
        );
    }

    #[test]
    fn a_renamed_parameter_is_followed_into_projections_and_bounds() {
        let signature: Signature = parse_quote!(
            fn eq<I>(self, other: I, first: <I as IntoIterator>::Item) -> Option<I::Item>
            where
                I: IntoIterator,
                Self::Item: PartialEq<I::Item>
        );
        assert_eq!(
            distinguished(plain_trait(), parse_quote!(Shape), parse_quote!(<I>), signature),
            "fneq<I2>(self,other:I2,first:<I2asIntoIterator>::Item)->Option<I2::Item>whereI2:IntoIterator,Self::Item:PartialEq<I2::Item>"
        );
        let signature: Signature = parse_quote!(
            fn find<P: FnMut(&P) -> bool, F>(&mut self, predicate: P) -> Option<F>
            where
                F: Iterator<Item = P>
        );
        assert_eq!(
            distinguished(
                plain_trait(),
                parse_quote!(Shape),
                parse_quote!(<P, F>),
                signature
            ),
            "fnfind<P2:FnMut(&P2)->bool,F2>(&mutself,predicate:P2)->Option<F2>whereF2:Iterator<Item=P2>",
            "a trait bound's own path is left alone, and the parameters inside it follow"
        );
    }

    /// The signature `distinguish` writes for `signature` when the impl
    /// also names `named`, as a payload's type or the header's bounds do.
    fn distinguished_beside(
        item_trait: ItemTrait,
        scope: Generics,
        named: proc_macro2::TokenStream,
        signature: Signature,
    ) -> String {
        let mut substitution =
            Substitution::of_trait(&item_trait, &parse_quote!(Shape), &scope).unwrap();
        substitution.names(named);
        let mut signature = signature;
        substitution.distinguish(&mut signature);
        signature.to_token_stream().to_string().replace(' ', "")
    }

    #[test]
    fn a_name_the_impl_writes_is_a_reason_to_rename_and_no_name_to_pick() {
        assert_eq!(
            distinguished_beside(
                plain_trait(),
                Generics::default(),
                quote!(Vec<Square>),
                parse_quote!(fn pick<Square: Copy>(&self, one: Square) -> Square)
            ),
            "fnpick<Square2:Copy>(&self,one:Square2)->Square2",
            "a payload's type"
        );
        assert_eq!(
            distinguished_beside(
                plain_trait(),
                parse_quote!(<T>),
                quote!(T2),
                parse_quote!(fn pick<T: Copy>(&self, one: T) -> T)
            ),
            "fnpick<T3:Copy>(&self,one:T3)->T3",
            "`T2` is a payload's type, so `T` takes the next number"
        );
        assert_eq!(
            distinguished_beside(
                plain_trait(),
                parse_quote!(<T>),
                quote!(),
                parse_quote!(fn pick<T: Copy>(&self, one: T, other: T2) -> T)
            ),
            "fnpick<T3:Copy>(&self,one:T3,other:T2)->T3",
            "`T2` is a type the signature names"
        );
        let encoder: ItemTrait = parse_quote!(
            trait Encoder<F> {
                fn encode<Json>(&self, format: F, extra: Json) -> (F, Json);
            }
        );
        let substitution =
            Substitution::of_trait(&encoder, &parse_quote!(Encoder<Json>), &Generics::default())
                .unwrap();
        let mut signature: Signature =
            parse_quote!(fn encode<Json>(&self, format: F, extra: Json) -> (F, Json));
        substitution.distinguish(&mut signature);
        assert_eq!(
            signature.to_token_stream().to_string().replace(' ', ""),
            "fnencode<Json2>(&self,format:F,extra:Json2)->(F,Json2)",
            "an argument given to the trait"
        );
    }

    #[test]
    fn a_name_only_the_trait_writes_is_no_reason_to_rename_but_is_not_picked() {
        let writes: ItemTrait = parse_quote!(
            trait Shape {
                fn other(&self, one: Json, two: I2);
            }
        );
        assert_eq!(
            distinguished_beside(
                writes.clone(),
                Generics::default(),
                quote!(),
                parse_quote!(fn pick<Json>(&self, one: Json) -> Json)
            ),
            "fnpick<Json>(&self,one:Json)->Json"
        );
        assert_eq!(
            distinguished_beside(
                writes,
                parse_quote!(<I>),
                quote!(),
                parse_quote!(fn eq<I>(self, other: I) -> bool)
            ),
            "fneq<I3>(self,other:I3)->bool"
        );
    }

    #[test]
    fn a_lifetime_a_payload_writes_is_a_lifetime_and_not_a_type() {
        assert_eq!(
            distinguished_beside(
                plain_trait(),
                Generics::default(),
                quote!(Slice<'s, u8>),
                parse_quote!(fn get<'s>(&'s self) -> &'s u8)
            ),
            "fnget<'s2>(&'s2self)->&'s2u8"
        );
        assert_eq!(
            distinguished_beside(
                plain_trait(),
                Generics::default(),
                quote!(Slice<'s, u8>),
                parse_quote!(fn get<s>(&self, one: s) -> s)
            ),
            "fnget<s>(&self,one:s)->s"
        );
    }

    /// The renaming of `fn f<T, const N: usize>` under a header that
    /// declares both, `T2` and `N2`.
    fn renaming() -> Renaming {
        let substitution = Substitution::of_trait(
            &plain_trait(),
            &parse_quote!(Shape),
            &parse_quote!(<T, const N: usize>),
        )
        .unwrap();
        let mut signature: Signature = parse_quote!(fn f<T, const N: usize>(&self));
        substitution.distinguish(&mut signature)
    }

    #[test]
    fn an_expression_follows_the_renaming() {
        let renamed = |mut expression: Expr| {
            renaming().expression(&mut expression);
            expression.to_token_stream().to_string().replace(' ', "")
        };
        assert_eq!(renamed(parse_quote!(T::default())), "T2::default()");
        assert_eq!(renamed(parse_quote!([0; N])), "[0;N2]");
        assert_eq!(renamed(parse_quote!(N + 1)), "N2+1", "a const stands alone");
        assert_eq!(renamed(parse_quote!(make::<T>())), "make::<T2>()");
        assert_eq!(
            renamed(parse_quote!(<T as Default>::default())),
            "<T2asDefault>::default()"
        );
        assert_eq!(
            renamed(parse_quote!(Other::T)),
            "Other::T",
            "a name after `::` is looked up in what stands before"
        );
        let mut untouched: Expr = parse_quote!(T::default());
        Renaming::default().expression(&mut untouched);
        assert_eq!(
            untouched.to_token_stream().to_string().replace(' ', ""),
            "T::default()"
        );
    }

    #[test]
    fn a_macros_arguments_follow_the_renaming() {
        let among = |tokens: proc_macro2::TokenStream| {
            renaming().among(tokens).to_string().replace(' ', "")
        };
        assert_eq!(among(quote!(T::default())), "T2::default()");
        assert_eq!(among(quote!(Vec::<T>::new())), "Vec::<T2>::new()");
        assert_eq!(among(quote!(Other::T)), "Other::T", "a name after `::`");
        assert_eq!(
            among(quote!([0; N], (T::default(), [N]))),
            "[0;N2],(T2::default(),[N2])"
        );
        let substitution =
            Substitution::of_trait(&plain_trait(), &parse_quote!(Shape), &parse_quote!(<'a>))
                .unwrap();
        let mut signature: Signature = parse_quote!(fn f<'a>(&'a self));
        let mut lifetimes = substitution.distinguish(&mut signature);
        assert_eq!(
            lifetimes
                .among(quote!(&'a str, a))
                .to_string()
                .replace(' ', ""),
            "&'a2str,a",
            "a lifetime, and not the identifier of its name"
        );
    }

    #[test]
    fn a_format_macro_names_the_argument_that_is_its_format_string() {
        for name in [
            "format",
            "format_args",
            "print",
            "println",
            "eprint",
            "eprintln",
            "panic",
            "unreachable",
            "todo",
            "unimplemented",
        ] {
            assert_eq!(format_argument(name), Some(0), "{name}");
        }
        for name in ["write", "writeln", "assert", "debug_assert"] {
            assert_eq!(format_argument(name), Some(1), "{name}");
        }
        for name in [
            "assert_eq",
            "assert_ne",
            "debug_assert_eq",
            "debug_assert_ne",
        ] {
            assert_eq!(format_argument(name), Some(2), "{name}");
        }
        for name in ["vec", "concat", "stringify", "matches", "dbg"] {
            assert_eq!(format_argument(name), None, "{name}");
        }
    }

    #[test]
    fn only_a_format_macros_format_string_captures() {
        let arguments = |name: &str, tokens: proc_macro2::TokenStream| {
            renaming()
                .arguments(Some(name), tokens)
                .to_string()
                .replace(' ', "")
        };
        let among = |tokens: proc_macro2::TokenStream| {
            renaming().among(tokens).to_string().replace(' ', "")
        };
        assert_eq!(
            arguments("format", quote!("{N}", "{N}")),
            "\"{N2}\",\"{N}\"",
            "a second string of `format!` is an argument, not the format"
        );
        assert_eq!(
            among(quote!(String::from("{N}"), format!("{N}"))),
            "String::from(\"{N}\"),format!(\"{N2}\")",
            "a nested `format!` is read by its name"
        );
        assert_eq!(
            among(quote!(std::format!("{}{N}", "{N}"))),
            "std::format!(\"{}{N2}\",\"{N}\")",
            "with a path before the name"
        );
        assert_eq!(arguments("write", quote!(out, "{N}")), "out,\"{N2}\"");
        assert_eq!(arguments("assert", quote!(ok, "{N}")), "ok,\"{N2}\"");
        assert_eq!(arguments("assert_eq", quote!(a, b, "{N}")), "a,b,\"{N2}\"");
        assert_eq!(
            arguments("vec", quote!("{N}"; N)),
            "\"{N}\";N2",
            "a plain string under `vec!` is text"
        );
        assert_eq!(
            among(quote!(self.T, value.N)),
            "self.T,value.N",
            "a name after `.`"
        );
    }

    #[test]
    fn a_format_macros_arguments_are_read_as_expressions() {
        let arguments = |name: &str, tokens: proc_macro2::TokenStream| {
            renaming()
                .arguments(Some(name), tokens)
                .to_string()
                .replace(' ', "")
        };
        assert_eq!(
            arguments(
                "assert_eq",
                quote!(std::collections::HashMap::<u8, u8>::new().len(), 0, "{N}")
            ),
            "std::collections::HashMap::<u8,u8>::new().len(),0,\"{N2}\"",
            "a comma inside generic arguments is no argument's end"
        );
        assert_eq!(
            arguments("write", quote!(out(|a, b| a + b), "{:N$}", N)),
            "out(|a,b|a+b),\"{:N2$}\",N2",
            "nor is one between a closure's parameters"
        );
        assert_eq!(
            arguments("write", quote!(|a, b| a + b, "{N}")),
            "|a,b|a+b,\"{N2}\"",
            "a closure written as the first argument"
        );
        assert_eq!(
            arguments("format", quote!("{x}", x = N)),
            "\"{x}\",x=N2",
            "a named argument is an expression"
        );
        assert_eq!(
            arguments("format", quote!("{N}", N = N)),
            "\"{N2}\",N2=N2",
            "a named argument of the parameter's name follows it"
        );
        assert_eq!(
            arguments("vec", quote!(T::default(); N)),
            "T2::default();N2",
            "no format macro: token by token"
        );
        assert_eq!(
            arguments("format", quote!("{N}" => T)),
            "\"{N}\"=>T2",
            "arguments that are no expressions fall back to tokens, where a string is text"
        );
        assert_eq!(
            arguments("matches", quote!(T::default(), x if x == N => true)),
            "T2::default(),xifx==N2=>true"
        );
        assert_eq!(
            arguments("vec", quote!(format!("{N}"), String::from("{N}"))),
            "format!(\"{N2}\"),String::from(\"{N}\")",
            "a format macro among another's arguments"
        );
        assert_eq!(
            arguments("assert", quote!(ok, "{}", format!("{N}"))),
            "ok,\"{}\",format!(\"{N2}\")",
            "a format macro among a format macro's arguments"
        );
    }

    #[test]
    fn what_stringify_and_cfg_take_is_no_code() {
        let arguments = |name: &str, tokens: proc_macro2::TokenStream| {
            renaming()
                .arguments(Some(name), tokens)
                .to_string()
                .replace(' ', "")
        };
        assert_eq!(arguments("stringify", quote!(N)), "N");
        assert_eq!(arguments("stringify", quote!(T::default())), "T::default()");
        assert_eq!(
            arguments("vec", quote!(stringify!(N), N)),
            "stringify!(N),N2",
            "a `stringify!` among another macro's arguments"
        );
        assert_eq!(
            arguments("format", quote!("{}{N}", stringify!(N))),
            "\"{}{N2}\",stringify!(N)",
            "and among a format macro's"
        );
        let substitution =
            Substitution::of_trait(&plain_trait(), &parse_quote!(Shape), &parse_quote!(<test>))
                .unwrap();
        let mut signature: Signature = parse_quote!(fn f<test>(&self, one: test));
        let mut renaming = substitution.distinguish(&mut signature);
        assert_eq!(
            signature.to_token_stream().to_string().replace(' ', ""),
            "fnf<test2>(&self,one:test2)"
        );
        assert_eq!(
            renaming.arguments(Some("cfg"), quote!(test)).to_string(),
            "test",
            "a predicate, beside a parameter named `test`"
        );
    }

    #[test]
    fn a_format_string_captures_a_const_where_a_placeholder_names_it() {
        let captured = |literal: &str| capturing(literal, "N", "N2");
        assert_eq!(captured("\"{N}\""), "\"{N2}\"");
        assert_eq!(captured("\"{N:>4}\""), "\"{N2:>4}\"");
        assert_eq!(captured("\"{:N$}\""), "\"{:N2$}\"");
        assert_eq!(captured("\"{:.N$}\""), "\"{:.N2$}\"");
        assert_eq!(captured("\"{0:N$}\""), "\"{0:N2$}\"");
        assert_eq!(
            captured("\"{{N}}\""),
            "\"{{N}}\"",
            "an escaped brace is text"
        );
        assert_eq!(
            captured("\"N and {}\""),
            "\"N and {}\"",
            "text outside a placeholder"
        );
        assert_eq!(captured("r#\"{N}\"#"), "r#\"{N2}\"#", "a raw string");
        assert_eq!(captured("7usize"), "7usize", "a literal that is no string");
        assert_eq!(captured("'N'"), "'N'");
        assert_eq!(
            capturing("\"{:X} {X}\"", "X", "X2"),
            "\"{:X} {X2}\"",
            "a format type is no argument"
        );
    }

    #[test]
    fn the_names_of_some_tokens_are_the_first_in_a_path_or_every_one() {
        let names = |tokens: proc_macro2::TokenStream, which: Which| {
            let mut names = HashSet::new();
            named(tokens, which, &mut names);
            let mut names: Vec<String> = names.into_iter().collect();
            names.sort();
            names
        };
        let pair = quote!(Pair<Square, std::vec::IntoIter<u8>>);
        assert_eq!(
            names(pair.clone(), Which::First),
            ["Pair", "Square", "std", "u8"]
        );
        assert_eq!(
            names(pair, Which::Every),
            ["IntoIter", "Pair", "Square", "std", "u8", "vec"]
        );
        let rooted = quote!(::core::option::Option<T>);
        assert_eq!(names(rooted.clone(), Which::First), ["T"]);
        assert_eq!(
            names(rooted, Which::Every),
            ["Option", "T", "core", "option"]
        );
        let qualified = quote!(<T as Trait>::Item);
        assert_eq!(names(qualified.clone(), Which::First), ["T", "Trait", "as"]);
        assert_eq!(names(qualified, Which::Every), ["Item", "T", "Trait", "as"]);
        let borrowed = quote!(&'a Slice<'a, T>);
        assert_eq!(names(borrowed.clone(), Which::First), ["'a", "Slice", "T"]);
        assert_eq!(names(borrowed, Which::Every), ["'a", "Slice", "T"]);
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
