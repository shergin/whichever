//! The grammars of `#[dispatch(..)]` on a subject, a partial impl block and a
//! trait method.
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    parenthesized, Attribute, Error, Expr, Ident, LitBool, Path, Result, Token, TraitItemFn, Type,
};

/// Returns whether `attribute` is this crate's `dispatch` attribute, written
/// bare or as `whichever::dispatch`.
pub(crate) fn is_dispatch_attribute(attribute: &Attribute) -> bool {
    attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "dispatch")
}

/// One `#[dispatch(..)]` attribute on an enum: a trait path with its
/// arguments, options, or both. The path is absent when the attribute only
/// carries enum-level options.
pub(crate) struct TraitSpec {
    pub(crate) path: Option<Path>,
    /// `Name = ..` options: a type for an associated type, a value for an
    /// associated const. Which one is meant is known once the trait is.
    pub(crate) assoc_overrides: Vec<(Ident, Override)>,
    pub(crate) from: Option<bool>,
    pub(crate) try_from: Option<bool>,
    pub(crate) mismatch: Vec<(Ident, Expr)>,
}

/// The right-hand side of a `Name = ..` option. A bare path parses as a
/// type and is read as a value when `Name` turns out to be a const.
pub(crate) enum Override {
    Type(Type),
    Expr(Expr),
}

impl Parse for Override {
    fn parse(input: ParseStream) -> Result<Self> {
        let fork = input.fork();
        if let Ok(ty) = fork.parse::<Type>() {
            if fork.is_empty() || fork.peek(Token![,]) {
                input.advance_to(&fork);
                return Ok(Self::Type(ty));
            }
        }
        Ok(Self::Expr(input.parse()?))
    }
}

impl TraitSpec {
    pub(crate) fn override_for(&self, name: &Ident) -> Option<&Override> {
        self.assoc_overrides
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    /// Returns the type given for the associated type `name`.
    pub(crate) fn assoc_override(&self, name: &Ident) -> Option<&Type> {
        match self.override_for(name) {
            Some(Override::Type(ty)) => Some(ty),
            _ => None,
        }
    }

    /// Returns the value given for the associated const `name`.
    pub(crate) fn const_override(&self, name: &Ident) -> Result<Option<Expr>> {
        match self.override_for(name) {
            None => Ok(None),
            Some(Override::Expr(expr)) => Ok(Some(expr.clone())),
            Some(Override::Type(Type::Path(path))) => Ok(Some(Expr::Path(syn::ExprPath {
                attrs: vec![],
                qself: path.qself.clone(),
                path: path.path.clone(),
            }))),
            Some(Override::Type(other)) => Err(Error::new_spanned(
                other,
                format!("`{name}` is an associated const; give it a value"),
            )),
        }
    }

    pub(crate) fn mismatch_for(&self, method: &Ident) -> Option<&Expr> {
        self.mismatch
            .iter()
            .find(|(key, _)| key == method)
            .map(|(_, expr)| expr)
    }
}

pub(crate) struct KeyedExpr {
    pub(crate) key: Ident,
    pub(crate) value: Expr,
}

impl Parse for KeyedExpr {
    fn parse(input: ParseStream) -> Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;
        Ok(Self { key, value })
    }
}

impl Parse for TraitSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let starts_with_option =
            input.peek(Ident) && (input.peek2(Token![=]) || input.peek2(syn::token::Paren));
        let path = if input.is_empty() || starts_with_option {
            None
        } else {
            Some(input.parse::<Path>()?)
        };
        let mut spec = TraitSpec {
            path,
            assoc_overrides: vec![],
            from: None,
            try_from: None,
            mismatch: vec![],
        };
        let mut expect_comma = spec.path.is_some();
        while !input.is_empty() {
            if expect_comma {
                input.parse::<Token![,]>()?;
                if input.is_empty() {
                    break;
                }
            }
            expect_comma = true;
            let key: Ident = input.parse()?;
            if key == "mismatch" {
                let content;
                parenthesized!(content in input);
                let pairs = Punctuated::<KeyedExpr, Token![,]>::parse_terminated(&content)?;
                spec.mismatch
                    .extend(pairs.into_iter().map(|pair| (pair.key, pair.value)));
                continue;
            }
            if !input.peek(Token![=]) {
                return Err(Error::new(
                    key.span(),
                    format!("unknown option `{key}`; the options are `Name = Type` for an associated type, `Name = value` for an associated const, `mismatch(method = expr)`, `from = false` and `try_from = true`"),
                ));
            }
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "from" => spec.from = Some(input.parse::<LitBool>()?.value),
                "try_from" => spec.try_from = Some(input.parse::<LitBool>()?.value),
                _ => spec.assoc_overrides.push((key, input.parse()?)),
            }
        }
        Ok(spec)
    }
}

/// Returns the `mismatch` expression given on the trait method itself.
pub(crate) fn method_mismatch(function: &TraitItemFn) -> Result<Option<Expr>> {
    for attribute in &function.attrs {
        if !is_dispatch_attribute(attribute) {
            continue;
        }
        let pair: KeyedExpr = attribute.parse_args()?;
        if pair.key != "mismatch" {
            return Err(Error::new_spanned(
                pair.key,
                "expected `#[dispatch(mismatch = <expr>)]`",
            ));
        }
        return Ok(Some(pair.value));
    }
    Ok(None)
}
