//! The grammars of `#[dispatch(..)]` and `#[dispatchable(..)]`, one typed
//! struct per position the attribute is written in: a subject
//! ([`DispatchAttribute`]), a partial impl block ([`TraitOptions`]), a variant
//! or a field ([`TargetOptions`]), a method of an inherent block
//! ([`ForwardingOptions`]), a method of a partial impl block
//! ([`TraitMark`]), a trait method ([`method_mismatch`]) and a trait
//! ([`DispatchableOptions`]).
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{
    parenthesized, Attribute, Error, Expr, Ident, ItemTrait, LitBool, LitInt, Member, Meta, Path,
    Result, Token, TraitItem, TraitItemFn, Type,
};

use crate::route::Route;
use crate::shape::takes_self_argument;
use crate::spelling;
use crate::words::Words;

/// Returns the candidate closest to `name` when it is within one edit, or
/// within two when both names are long enough for two edits to leave them
/// recognisable, so that a message can say what was probably meant.
pub(crate) fn closest<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    candidates
        .into_iter()
        .filter_map(|candidate| {
            let distance = edit_distance(name, candidate);
            let close = distance == 1 || (distance == 2 && name.len() >= 5 && candidate.len() >= 5);
            close.then_some((distance, candidate))
        })
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

/// Returns the number of insertions, deletions, substitutions and
/// transpositions of adjacent characters that turn `a` into `b`.
fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut rows: Vec<Vec<usize>> = (0..=a.len()).map(|i| vec![i; b.len() + 1]).collect();
    for (j, cell) in rows[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (rows[i - 1][j] + 1)
                .min(rows[i][j - 1] + 1)
                .min(rows[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(rows[i - 2][j - 2] + 1);
            }
            rows[i][j] = best;
        }
    }
    rows[a.len()][b.len()]
}

/// Spells `names` out as a list, `` `a` ``, `` `a` and `b` `` or
/// `` `a`, `b` and `c` ``.
pub(crate) fn listed(names: &[String]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    match quoted.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// Returns what tells two traits apart wherever the macros compare them,
/// which is also how a sentence names the trait: the path as written,
/// generic arguments included, spelled as a hand spells it, whatever
/// spaces the source put between its tokens. A macro cannot resolve
/// names, so two spellings of one trait are two traits, and two
/// instantiations of one generic trait, or two traits of one name from two
/// modules, are two as well, as they are to the compiler.
pub(crate) fn trait_key(path: &Path) -> String {
    spelling::spelled(path)
}

/// Returns whether `attribute` is this crate's `dispatch` attribute, written
/// bare or as `whichever::dispatch`.
pub(crate) fn is_dispatch_attribute(attribute: &Attribute) -> bool {
    attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "dispatch")
}

/// Parses options separated by commas, a trailing comma allowed, handing
/// each key to `option` with the input positioned right after it.
fn parse_options(
    input: ParseStream,
    mut option: impl FnMut(Ident, ParseStream) -> Result<()>,
) -> Result<()> {
    parse_separated(input, |input| {
        let key: Ident = input.parse()?;
        option(key, input)
    })
}

/// Parses what `element` reads, separated by commas, a trailing comma
/// allowed.
fn parse_separated(
    input: ParseStream,
    mut element: impl FnMut(ParseStream) -> Result<()>,
) -> Result<()> {
    while !input.is_empty() {
        element(input)?;
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(())
}

/// A trait's options: the `Name = ..` overrides, the `mismatch` results and
/// the routes. This is what travels to the implementation macro beside the
/// trait's path, and what a partial impl block's attribute holds.
pub(crate) struct TraitOptions {
    pub(crate) overrides: Vec<Override>,
    pub(crate) mismatch: Vec<(Ident, Expr)>,
    /// The routes this trait takes through named variants, `via(Variant =
    /// .0, ..)`, over the variants' own.
    pub(crate) routes: Vec<(Ident, Route)>,
}

impl TraitOptions {
    pub(crate) fn override_for(&self, name: &Ident) -> Option<&Override> {
        self.overrides.iter().find(|fixed| &fixed.name == name)
    }

    /// Returns whether the associated type or const `name` is fixed on the
    /// subject.
    pub(crate) fn is_overridden(&self, name: &Ident) -> bool {
        self.override_for(name).is_some()
    }

    pub(crate) fn mismatch_for(&self, method: &Ident) -> Option<&Expr> {
        self.mismatch
            .iter()
            .find(|(key, _)| key == method)
            .map(|(_, expr)| expr)
    }

    /// Returns the errors for the options naming nothing in the trait: an
    /// override of an associated item it does not have, or a `mismatch` for a
    /// method without a `Self` argument. Each is a typo that would otherwise
    /// turn into a type error somewhere else, so the message says what was
    /// probably meant, or what there is to mean.
    pub(crate) fn unknown_names(
        &self,
        item_trait: &ItemTrait,
        trait_ident: &Ident,
        words: Words,
        on_subject: bool,
    ) -> Vec<Error> {
        let mut errors = vec![];
        let associated: Vec<String> = item_trait
            .items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Type(assoc) => Some(assoc.ident.to_string()),
                TraitItem::Const(constant) => Some(constant.ident.to_string()),
                _ => None,
            })
            .collect();
        for fixed in &self.overrides {
            let name = fixed.name.to_string();
            if associated.contains(&name) {
                continue;
            }
            let hint = unknown_override_hint(&name, &associated, words, on_subject);
            errors.push(Error::new(
                fixed.name.span(),
                format!("`{trait_ident}` has no associated type or const `{name}`; {hint}"),
            ));
        }
        let deciding: Vec<String> = item_trait
            .items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Fn(function) if takes_self_argument(&function.sig) => {
                    Some(function.sig.ident.to_string())
                }
                _ => None,
            })
            .collect();
        for (method, _) in &self.mismatch {
            let name = method.to_string();
            if deciding.contains(&name) {
                continue;
            }
            // A method the trait declares under a `#[cfg]` that does not hold
            // is not in this build's trait, and nothing here tells it from a
            // name the trait never had: the sentence names the way around both.
            let elsewhere = "a method under a `#[cfg]` that does not hold in this build takes its mismatch on the trait method, `#[dispatch(mismatch = ..)]`";
            let hint = match closest(&name, deciding.iter().map(String::as_str)) {
                Some(near) => format!("did you mean `{near}`?"),
                None if deciding.is_empty() => {
                    format!("none of its methods takes one; {elsewhere}")
                }
                None => format!(
                    "its methods taking one are {}; {elsewhere}",
                    listed(&deciding)
                ),
            };
            errors.push(Error::new(
                method.span(),
                format!("`{trait_ident}` has no method `{name}` with a `Self`, `&Self` or `&mut Self` argument, so there is no mismatch to decide; {hint}"),
            ));
        }
        errors
    }
}

/// Returns the second half of the message for an override naming nothing in
/// the trait: the item or option probably meant, where the forwarded field
/// or variants are chosen when the name is a field-choosing option of
/// another crate, or else everything there is to name. The conversions and
/// the mismatch are an enum's: a struct has neither to mean.
fn unknown_override_hint(
    name: &str,
    associated: &[String],
    words: Words,
    on_subject: bool,
) -> String {
    if let Some(near) = closest(name, associated.iter().map(String::as_str)) {
        return format!("did you mean `{near}`?");
    }
    let switches = match (words, on_subject) {
        (Words::Enum, true) => &["from", "try_from"][..],
        _ => &[],
    };
    if let Some(near) = closest(name, switches.iter().copied()) {
        return format!("did you mean the enum's option `{near} = ..`?");
    }
    if matches!(name, "target" | "to" | "field" | "delegate" | "via") {
        return match words {
            Words::Struct => "a struct forwards to its one field, to the field marked `#[dispatch]`, or to the one that names the trait, `#[dispatch(Trait)]`".to_owned(),
            Words::Enum => "an enum forwards to every variant, and an inherent method names the variants it forwards to".to_owned(),
        };
    }
    let options = match (words, on_subject) {
        (Words::Struct, _) => {
            return if associated.is_empty() {
                "it has none, and a struct's other option is a route, `via(field = route)`"
                    .to_owned()
            } else {
                format!(
                    "its associated items are {}, and a struct's other option is a route, `via(field = route)`",
                    listed(associated)
                )
            };
        }
        (Words::Enum, true) => "`mismatch(method = expr)`, `from = false` and `try_from = true`",
        (Words::Enum, false) => "`mismatch(method = expr)`",
    };
    if associated.is_empty() {
        return format!("it has none, and the other options are {options}");
    }
    format!(
        "its associated items are {}, and the other options are {options}",
        listed(associated)
    )
}

impl Parse for TraitOptions {
    /// The trait's options alone, as a partial impl block's attribute
    /// holds them: a conversion switch belongs on the subject, and the
    /// trait is the one the block implements.
    fn parse(input: ParseStream) -> Result<Self> {
        let DispatchAttribute {
            path,
            options,
            subject,
        } = DispatchAttribute::parse_for(input, Reader::Block)?;
        if let Some(span) = subject.first_span() {
            return Err(Error::new(
                span,
                "`from`, `try_from`, `into` and `remote` are options of an enum's own `#[dispatch]` attribute; a block's attribute takes the trait's options only",
            ));
        }
        if let Some(path) = path {
            return Err(Error::new_spanned(
                path,
                "`#[dispatch]` on an impl block takes the trait's options only; the trait is the one the block implements",
            ));
        }
        Ok(options)
    }
}

impl ToTokens for TraitOptions {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let mut parts: Vec<TokenStream2> = self
            .overrides
            .iter()
            .map(ToTokens::to_token_stream)
            .collect();
        if !self.mismatch.is_empty() {
            let pairs = self
                .mismatch
                .iter()
                .map(|(method, expr)| quote!(#method = #expr));
            parts.push(quote!(mismatch(#(#pairs),*)));
        }
        if !self.routes.is_empty() {
            let pairs = self
                .routes
                .iter()
                .map(|(variant, route)| quote!(#variant = #route));
            parts.push(quote!(via(#(#pairs),*)));
        }
        tokens.extend(quote!(#(#parts),*));
    }
}

/// `#[dispatch(..)]` on a subject: the trait it dispatches, when it names
/// one, with that trait's options, and the subject's own options, which
/// only `#[dispatch]` on the subject reads.
pub(crate) struct DispatchAttribute {
    pub(crate) path: Option<Path>,
    pub(crate) options: TraitOptions,
    pub(crate) subject: SubjectOptions,
}

impl DispatchAttribute {
    /// Rejects a trait of `std`, `core` or `alloc`, which is never
    /// `#[dispatchable]`; rustc would only say that the macro of that name
    /// is missing.
    pub(crate) fn check_path(&self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let first = &path.segments[0].ident;
        if path.segments.len() > 1 && (first == "std" || first == "core" || first == "alloc") {
            let spelled = spelling::spelled(path);
            let name = &path.segments.last().expect("checked above").ident;
            return Err(Error::new_spanned(
                path,
                format!("`{spelled}` is not `#[dispatchable]`; dispatch `whichever::foreign::{name}` if it is one of the traits `whichever` declares, or restate the trait with `#[whichever::dispatchable(remote = {spelled})]`"),
            ));
        }
        Ok(())
    }
}

/// Who reads a list of a trait's options, which decides the words its
/// sentences speak.
#[derive(Clone, Copy)]
pub(crate) enum Reader {
    /// The subject's own attribute, read where the subject is: in the
    /// subject's words.
    Subject(Words),
    /// A partial impl block's attribute, read before the subject's carrier
    /// brings the subject: its sentences name neither an enum nor a struct.
    Block,
}

impl Parse for DispatchAttribute {
    /// An enum's own attribute; `read` takes the subject's words.
    fn parse(input: ParseStream) -> Result<Self> {
        Self::parse_for(input, Reader::Subject(Words::Enum))
    }
}

impl DispatchAttribute {
    /// Reads the arguments of the subject's own attribute, in `words`.
    pub(crate) fn read(arguments: TokenStream2, words: Words) -> Result<Self> {
        let parser = |input: ParseStream| Self::parse_for(input, Reader::Subject(words));
        parser.parse2(arguments)
    }

    /// Parses the list as `reader` reads it.
    fn parse_for(input: ParseStream, reader: Reader) -> Result<Self> {
        let starts_with_option =
            input.peek(Ident) && (input.peek2(Token![=]) || input.peek2(syn::token::Paren));
        let path = if input.is_empty() || starts_with_option {
            None
        } else {
            Some(input.parse::<Path>()?)
        };
        let mut options = TraitOptions {
            overrides: vec![],
            mismatch: vec![],
            routes: vec![],
        };
        let mut subject = SubjectOptions::default();
        if let Some(path) = &path {
            // `Item<'a> = ..` as the first thing is read as a trait path.
            if input.peek(Token![=]) {
                if let [segment] = path.segments.iter().collect::<Vec<_>>().as_slice() {
                    return Err(gat_override(&segment.ident, None, reader));
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        parse_options(input, |key, input| {
            if key == "mismatch" {
                let content;
                parenthesized!(content in input);
                let pairs = Punctuated::<KeyedExpr, Token![,]>::parse_terminated(&content)?;
                for pair in pairs {
                    if options.mismatch_for(&pair.key).is_some() {
                        return Err(given_twice(&pair.key));
                    }
                    options.mismatch.push((pair.key, pair.value));
                }
                return Ok(());
            }
            if key == "via" && input.peek(syn::token::Paren) {
                let content;
                parenthesized!(content in input);
                let pairs = Punctuated::<KeyedRoute, Token![,]>::parse_terminated(&content)?;
                for pair in pairs {
                    if options
                        .routes
                        .iter()
                        .any(|(variant, _)| *variant == pair.key)
                    {
                        return Err(given_twice(&pair.key));
                    }
                    options.routes.push((pair.key, pair.route));
                }
                return Ok(());
            }
            if input.peek(Token![<]) {
                return Err(gat_override(&key, path.as_ref(), reader));
            }
            // A route written as a variant, a field or a method writes it
            // would otherwise be read as an override named `via`.
            if key == "via" && input.peek(Token![=]) {
                return Err(Error::new(
                    key.span(),
                    "`via = route` goes on a variant, on a field or on a method's `#[dispatch(..)]`; after a trait a route is keyed by what it goes through, `via(name = route)`, and the field of a struct that a trait is forwarded to names the trait, `#[dispatch(Trait)]`",
                ));
            }
            if !input.peek(Token![=]) {
                return Err(Error::new(
                    key.span(),
                    match reader {
                        Reader::Subject(Words::Enum) => format!("unknown option `{key}`; the options are `Name = Type` for an associated type, `Name = value` for an associated const, `mismatch(method = expr)`, `via(Variant = route)`, `from = false`, `try_from = true`, `into = Target` and `remote = path`"),
                        // The conversions and the mismatch are an enum's.
                        Reader::Subject(Words::Struct) => format!("unknown option `{key}`; the options of a struct are `Name = Type` for an associated type, `Name = value` for an associated const, `via(field = route)` and `remote = path`"),
                        Reader::Block => format!("unknown option `{key}`; a block's attribute takes the trait's options: `Name = Type` for an associated type, `Name = value` for an associated const, `mismatch(method = expr)` and a route, `via(name = route)`"),
                    },
                ));
            }
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "from" if subject.from.is_some() => return Err(given_twice(&key)),
                "try_from" if subject.try_from.is_some() => return Err(given_twice(&key)),
                "from" => subject.from = Some(Switch::parse(key, input)?),
                "try_from" => subject.try_from = Some(Switch::parse(key, input)?),
                "into" => subject.into.push(IntoTarget {
                    key,
                    target: input.parse()?,
                }),
                "remote" if subject.remote.is_some() => return Err(given_twice(&key)),
                "remote" => subject.remote = Some((key, input.parse()?)),
                _ if options.is_overridden(&key) => return Err(given_twice(&key)),
                _ => options.overrides.push(Override::parse(key, input)?),
            }
            Ok(())
        })?;
        Ok(Self {
            path,
            options,
            subject,
        })
    }
}

fn given_twice(key: &Ident) -> Error {
    Error::new(key.span(), format!("`{key}` is given twice"))
}

/// The error for `Name<..> = Type`, an attempt to fix a generic associated
/// type with an option: only a partial impl block declares one. On a
/// block's own attribute the block is the place.
fn gat_override(name: &Ident, trait_path: Option<&Path>, reader: Reader) -> Error {
    let trait_name = trait_path
        .map(spelling::spelled)
        .unwrap_or_else(|| "Trait".to_owned());
    Error::new(
        name.span(),
        match reader {
            Reader::Subject(words) => format!("`{name}<..>` is a generic associated type, which only the {} can declare; write `type {name}<..> = ..;` in a `#[whichever::dispatch] impl {trait_name} for ..` block", words.subject()),
            Reader::Block => format!("`{name}<..>` is a generic associated type, which the block declares and no option gives; write `type {name}<..> = ..;` in the block"),
        },
    )
}

/// The subject's own options: its say on its conversions, and its real
/// path when it restates another crate's enum.
#[derive(Default)]
pub(crate) struct SubjectOptions {
    pub(crate) from: Option<Switch>,
    pub(crate) try_from: Option<Switch>,
    /// The types the enum converts into, `into = Target`, each through
    /// every payload's own `Into<Target>`.
    pub(crate) into: Vec<IntoTarget>,
    /// The real path of an enum restated on behalf of another crate,
    /// `remote = path`, with where it was written.
    pub(crate) remote: Option<(Ident, Path)>,
}

impl SubjectOptions {
    /// Returns where the first of them was written, for a message about
    /// any of them.
    pub(crate) fn first_span(&self) -> Option<Span> {
        self.from
            .as_ref()
            .map(Switch::span)
            .or(self.try_from.as_ref().map(Switch::span))
            .or(self.into.first().map(|into| into.key.span()))
            .or(self.remote.as_ref().map(|(key, _)| key.span()))
    }
}

/// One `into = Target`.
pub(crate) struct IntoTarget {
    pub(crate) key: Ident,
    pub(crate) target: Type,
}

/// One `from = ..` or `try_from = ..`, with where it was written.
pub(crate) struct Switch {
    key: Ident,
    value: TryFromForms,
}

impl Switch {
    /// `from` takes `true` or `false`; `try_from` takes those, `ref` and
    /// `ref_mut`.
    fn parse(key: Ident, input: ParseStream) -> Result<Self> {
        let value = TryFromForms::parse(input)?;
        if key == "from" && (value.by_ref || value.by_mut) {
            return Err(Error::new(
                key.span(),
                "`from` is `true` or `false`; the by-reference forms are `try_from`'s",
            ));
        }
        Ok(Self { key, value })
    }

    /// Returns whether the switch is on at all.
    pub(crate) fn value(&self) -> bool {
        self.value.owned
    }

    pub(crate) fn forms(&self) -> TryFromForms {
        self.value
    }

    pub(crate) fn span(&self) -> Span {
        self.key.span()
    }
}

/// Which `TryFrom` impls a payload gets: `true` is `TryFrom<Enum> for
/// Payload`; `ref` adds `TryFrom<&Enum> for &Payload`; `ref_mut` adds
/// `TryFrom<&mut Enum> for &mut Payload` to those; `false` is none. The
/// words are `derive_more`'s.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TryFromForms {
    pub(crate) owned: bool,
    pub(crate) by_ref: bool,
    pub(crate) by_mut: bool,
}

impl TryFromForms {
    pub(crate) const NONE: Self = Self {
        owned: false,
        by_ref: false,
        by_mut: false,
    };

    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitBool) {
            let on = input.parse::<LitBool>()?.value;
            return Ok(Self {
                owned: on,
                by_ref: false,
                by_mut: false,
            });
        }
        if input.peek(Token![ref]) {
            input.parse::<Token![ref]>()?;
            return Ok(Self {
                owned: true,
                by_ref: true,
                by_mut: false,
            });
        }
        let word: Ident = input
            .parse()
            .map_err(|_| input.error("expected `true`, `false`, `ref` or `ref_mut`"))?;
        if word == "ref_mut" {
            return Ok(Self {
                owned: true,
                by_ref: true,
                by_mut: true,
            });
        }
        Err(Error::new(
            word.span(),
            "expected `true`, `false`, `ref` or `ref_mut`",
        ))
    }

    /// The forms either of two switches asks for.
    pub(crate) fn or(self, other: Self) -> Self {
        Self {
            owned: self.owned || other.owned,
            by_ref: self.by_ref || other.by_ref,
            by_mut: self.by_mut || other.by_mut,
        }
    }
}

/// A `Name = ..` option, kept as the tokens written until the trait says
/// whether `Name` is an associated type or an associated const. The tokens
/// reach as far as a type does when a type is what is written, since a
/// value parser would stop at the comma in `Result<A, B>`, and as far as an
/// expression does otherwise.
pub(crate) struct Override {
    pub(crate) name: Ident,
    tokens: TokenStream2,
}

impl Override {
    fn parse(name: Ident, input: ParseStream) -> Result<Self> {
        let fork = input.fork();
        if let Ok(ty) = fork.parse::<Type>() {
            if fork.is_empty() || fork.peek(Token![,]) {
                input.advance_to(&fork);
                return Ok(Self {
                    name,
                    tokens: ty.into_token_stream(),
                });
            }
        }
        let expr: Expr = input.parse()?;
        Ok(Self {
            name,
            tokens: expr.into_token_stream(),
        })
    }

    /// An override written as an associated type in a partial impl block.
    pub(crate) fn of_type(name: Ident, ty: &Type) -> Self {
        Self {
            name,
            tokens: ty.to_token_stream(),
        }
    }

    /// Reads the override as the type of the associated type `name`.
    pub(crate) fn as_type(&self) -> Result<Type> {
        syn::parse2(self.tokens.clone()).map_err(|_| {
            Error::new_spanned(
                &self.tokens,
                format!("`{}` is an associated type; give it a type", self.name),
            )
        })
    }

    /// Reads the override as the value of the associated const `name`.
    pub(crate) fn as_value(&self) -> Result<Expr> {
        syn::parse2(self.tokens.clone()).map_err(|_| {
            Error::new_spanned(
                &self.tokens,
                format!("`{}` is an associated const; give it a value", self.name),
            )
        })
    }
}

impl ToTokens for Override {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Override {
            name,
            tokens: value,
        } = self;
        tokens.extend(quote!(#name = #value));
    }
}

pub(crate) struct KeyedExpr {
    pub(crate) key: Ident,
    pub(crate) value: Expr,
}

/// One `Variant = route` of a `via(..)` option.
struct KeyedRoute {
    key: Ident,
    route: Route,
}

impl Parse for KeyedRoute {
    fn parse(input: ParseStream) -> Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=]>()?;
        let route = input.parse()?;
        Ok(Self { key, route })
    }
}

impl Parse for KeyedExpr {
    fn parse(input: ParseStream) -> Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;
        Ok(Self { key, value })
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

/// The `#[dispatch(..)]` options of a variant or a field.
pub(crate) struct TargetOptions {
    /// How an arm reaches the target from the payload; `deref` is `*`.
    pub(crate) route: Option<Route>,
    pub(crate) from: bool,
    pub(crate) try_from: Option<TryFromForms>,
    /// The traits a struct's field is the target of, `#[dispatch(Rom)]`,
    /// over the field marked bare; a variant names none.
    pub(crate) traits: Vec<Path>,
}

/// Where the `#[dispatch(..)]` of a variant or a field is written.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Position {
    /// On a variant: a route, and the variant's say on its conversions.
    Variant,
    /// On the field of a variant: a route.
    VariantField,
    /// On a struct's field: a route, and the traits the field is the
    /// target of.
    StructField,
}

impl TargetOptions {
    /// Reads every `#[dispatch(..)]` attribute written at `position`:
    /// `deref` and `via = route` anywhere, `from = false` and `try_from =
    /// true` on a variant, and the traits a struct's field is the target
    /// of. A bare `#[dispatch]` marks the field to forward to and carries
    /// nothing.
    pub(crate) fn read(attributes: &[Attribute], position: Position) -> Result<Self> {
        let on_variant = position == Position::Variant;
        let mut options = Self {
            route: None,
            from: true,
            try_from: None,
            traits: vec![],
        };
        for attribute in attributes
            .iter()
            .filter(|attribute| is_dispatch_attribute(attribute))
        {
            if matches!(attribute.meta, Meta::Path(_)) {
                continue;
            }
            attribute.parse_args_with(|input: ParseStream| {
                parse_separated(input, |input| {
                    // A trait's path may begin where no option's name
                    // does, `crate::Rom`, `self::Rom`, `super::Rom` or
                    // `::rom::Rom`: a field names a trait as its home
                    // spells it.
                    let rooted = input.peek(Token![::])
                        || input.peek(Token![crate])
                        || input.peek(Token![self])
                        || input.peek(Token![super]);
                    if rooted {
                        let path: Path = input.parse()?;
                        let first = &path.segments[0].ident;
                        if on_variant {
                            return Err(Error::new(
                                first.span(),
                                format!("unknown option `{first}`; the options are `deref`, `via = route`, `from = false` and `try_from = true`"),
                            ));
                        }
                        if position == Position::VariantField {
                            return Err(variant_field_option(first));
                        }
                        options.traits.push(path);
                        return Ok(());
                    }
                    let key: Ident = input.parse()?;
                    // A name that is no option and takes no value is a
                    // trait the field is the target of.
                    let is_option = key == "deref"
                        || key == "via"
                        || ((key == "from" || key == "try_from") && on_variant)
                        || input.peek(Token![=]);
                    if !is_option {
                        if on_variant {
                            return Err(Error::new(
                                key.span(),
                                format!("unknown option `{key}`; the options are `deref`, `via = route`, `from = false` and `try_from = true`"),
                            ));
                        }
                        // A variant is the target of every trait its enum
                        // dispatches, so its field names none.
                        if position == Position::VariantField {
                            return Err(variant_field_option(&key));
                        }
                        let mut path = Path::from(key);
                        if input.peek(Token![<]) {
                            let segment = path.segments.last_mut().expect("one segment");
                            segment.arguments = syn::PathArguments::AngleBracketed(input.parse()?);
                        }
                        while input.peek(Token![::]) {
                            input.parse::<Token![::]>()?;
                            path.segments.push(input.parse()?);
                        }
                        options.traits.push(path);
                        return Ok(());
                    }
                    if key == "deref" || key == "via" {
                        let route = if key == "deref" {
                            Route::deref()
                        } else {
                            input.parse::<Token![=]>()?;
                            input.parse()?
                        };
                        if options.route.is_some() {
                            return Err(Error::new(
                                key.span(),
                                "`deref` is the route `*`; give one route, `deref` or `via = ..`",
                            ));
                        }
                        options.route = Some(route);
                    } else if key == "from" && on_variant {
                        input.parse::<Token![=]>()?;
                        options.from = input.parse::<LitBool>()?.value;
                    } else if key == "try_from" && on_variant {
                        input.parse::<Token![=]>()?;
                        options.try_from = Some(TryFromForms::parse(input)?);
                    } else if position == Position::VariantField {
                        return Err(variant_field_option(&key));
                    } else {
                        let known = if on_variant {
                            "the options are `deref`, `via = route`, `from = false` and `try_from = true`"
                        } else {
                            "the options are `deref` and `via = route`, and the traits the field is the target of"
                        };
                        return Err(Error::new(
                            key.span(),
                            format!("unknown option `{key}`; {known}"),
                        ));
                    }
                    Ok(())
                })
            })?;
        }
        Ok(options)
    }
}

/// The error for an option a variant's field does not take: a trait's
/// name, which only a struct's field carries, or anything else.
fn variant_field_option(key: &Ident) -> Error {
    Error::new(
        key.span(),
        format!("unknown option `{key}`; the options of a variant's field are `deref` and `via = route`, and a trait takes its own route through the variant on the enum, `via(Variant = route)`"),
    )
}

/// What `#[dispatch(..)]` on a method of a partial impl block says.
pub(crate) enum TraitMark {
    /// `#[dispatch(default)]`: the method is left to the trait's default.
    Default,
    /// `#[dispatch(Variant, ..)]` or `#[dispatch(*)]`: the method is
    /// forwarded to the variants named, each through `via` when given.
    Forward(ForwardingOptions),
}

impl TraitMark {
    /// Reads the mark of a partial block's method: `default`, or the
    /// variants and the options an inherent method takes, of which only the
    /// names and `via` apply to a trait method.
    pub(crate) fn read(attribute: &Attribute, words: Words) -> Result<Self> {
        let default = attribute
            .parse_args::<Ident>()
            .is_ok_and(|word| word == "default");
        if default {
            return Ok(TraitMark::Default);
        }
        let forwarding = ForwardingOptions::read(attribute, words)?;
        let inherent_only = [
            forwarding.call.as_ref().map(|_| "`call`"),
            forwarding.with.as_ref().map(|_| "`with`"),
            forwarding.field.as_ref().map(|_| "`field`"),
            forwarding.value.then_some("`value`"),
        ];
        if let Some(option) = inherent_only.into_iter().flatten().next() {
            return Err(Error::new_spanned(
                attribute,
                format!("{option} is an option of an inherent method; a trait method is called by its own name, and takes `via` only"),
            ));
        }
        Ok(TraitMark::Forward(forwarding))
    }
}

/// Which variants a method of an impl block forwards to, and how.
pub(crate) struct ForwardingOptions {
    pub(crate) all: bool,
    pub(crate) variants: Vec<Ident>,
    pub(crate) call: Option<Ident>,
    pub(crate) with: Option<Path>,
    /// The route to the target for this method, over the variants' own.
    pub(crate) via: Option<Route>,
    /// `field = name`: the arm reads this field of the target instead of
    /// calling a method.
    pub(crate) field: Option<Member>,
    /// `value`: the arm is the target itself, for `with` to wrap or an
    /// unsized coercion to widen.
    pub(crate) value: bool,
}

impl ForwardingOptions {
    /// Reads the mark of an inherent method, in the subject's `words`; a
    /// bare `#[dispatch]` names no variant and no field.
    pub(crate) fn read(attribute: &Attribute, words: Words) -> Result<Self> {
        if matches!(attribute.meta, Meta::Path(_)) {
            return Err(Error::new_spanned(
                attribute,
                match words {
                    Words::Enum => "name the variants to forward to, `#[dispatch(Variant, ..)]`, or `#[dispatch(*)]` for all of them",
                    Words::Struct => "name the field to forward to, `#[dispatch(field)]`, or `#[dispatch(*)]` for the one the struct forwards to",
                },
            ));
        }
        attribute.parse_args_with(|input: ParseStream| Self::parse_for(input, words))
    }
}

impl Parse for ForwardingOptions {
    /// The mark of an enum's method; `read` takes the subject's words.
    fn parse(input: ParseStream) -> Result<Self> {
        Self::parse_for(input, Words::Enum)
    }
}

impl ForwardingOptions {
    /// Parses the mark's arguments, with the sentences in `words`.
    fn parse_for(input: ParseStream, words: Words) -> Result<Self> {
        let mut forwarding = ForwardingOptions {
            all: false,
            variants: vec![],
            call: None,
            with: None,
            via: None,
            field: None,
            value: false,
        };
        while !input.is_empty() {
            if input.peek(Token![*]) {
                input.parse::<Token![*]>()?;
                forwarding.all = true;
            } else {
                let ident: Ident = input.parse()?;
                if input.peek(Token![=]) {
                    input.parse::<Token![=]>()?;
                    match ident.to_string().as_str() {
                        "call" => forwarding.call = Some(input.parse()?),
                        "with" if input.peek(Token![|]) || input.peek(Token![move]) => {
                            return Err(Error::new(
                                ident.span(),
                                "`with` takes the path of a function, called on each arm's result; write a helper function instead of a closure, generic over the result when the payloads' results differ",
                            ))
                        }
                        "with" => forwarding.with = Some(input.parse()?),
                        "via" => forwarding.via = Some(input.parse()?),
                        "field" => {
                            forwarding.field = Some(if input.peek(LitInt) {
                                let index: LitInt = input.parse()?;
                                Member::Unnamed(syn::Index {
                                    index: index.base10_parse()?,
                                    span: index.span(),
                                })
                            } else {
                                Member::Named(input.parse()?)
                            });
                        }
                        other => {
                            return Err(Error::new(
                                ident.span(),
                                format!("unknown option `{other}`; the options are `call = method`, `with = path`, `via = route` and `field = name`"),
                            ))
                        }
                    }
                } else if ident == "value" {
                    forwarding.value = true;
                } else {
                    if forwarding.variants.contains(&ident) {
                        return Err(Error::new(
                            ident.span(),
                            format!("`{ident}` is named twice"),
                        ));
                    }
                    forwarding.variants.push(ident);
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        if forwarding.all && !forwarding.variants.is_empty() {
            return Err(Error::new(
                forwarding.variants[0].span(),
                match words {
                    Words::Enum => "`*` already names every variant",
                    Words::Struct => "`*` already names the field the struct forwards to; name a field or `*`, not both",
                },
            ));
        }
        if !forwarding.all && forwarding.variants.is_empty() {
            return Err(Error::new(
                input.span(),
                match words {
                    Words::Enum => "name the variants to forward to, or `*` for all of them",
                    Words::Struct => {
                        "name the field to forward to, or `*` for the one the struct forwards to"
                    }
                },
            ));
        }
        let reads = match (&forwarding.field, forwarding.value) {
            (Some(_), true) => {
                return Err(Error::new(
                    input.span(),
                    "`field` reads a field and `value` is the target itself; give one",
                ))
            }
            (Some(field), false) => Some(format!("`field = {}`", field.to_token_stream())),
            (None, true) => Some("`value`".to_owned()),
            (None, false) => None,
        };
        if let (Some(reads), Some(call)) = (reads, &forwarding.call) {
            return Err(Error::new(
                call.span(),
                format!("{reads} calls no method, so `call` has nothing to name"),
            ));
        }
        Ok(forwarding)
    }
}

/// The options of `#[dispatchable(..)]`.
pub(crate) struct DispatchableOptions {
    /// The real path of a trait restated on behalf of another crate.
    pub(crate) remote: Option<Path>,
    /// A name for the carrier macro, in place of the trait's text and
    /// position.
    pub(crate) name: Option<Ident>,
}

impl Parse for DispatchableOptions {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut options = Self {
            remote: None,
            name: None,
        };
        parse_options(input, |key, input| {
            if !input.peek(Token![=]) || !(key == "remote" || key == "name") {
                return Err(Error::new(
                    key.span(),
                    format!("unknown option `{key}`; the options are `remote = path` and `name = ident`"),
                ));
            }
            input.parse::<Token![=]>()?;
            if key == "remote" {
                options.remote = Some(input.parse()?);
            } else {
                options.name = Some(input.parse()?);
            }
            Ok(())
        })?;
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    fn attribute(tokens: &str) -> DispatchAttribute {
        syn::parse_str(tokens).expect("a valid attribute")
    }

    #[test]
    fn an_override_reaches_as_far_as_a_type_or_a_value_does() {
        let attribute = attribute(
            "T, N = -1, S = \"s\", R = (A, B), E = Result<A, B>, F = Foo::<T>::bar(), L = [u8; 4]",
        );
        let fixed = |name: &str| {
            let name: Ident = syn::parse_str(name).unwrap();
            attribute.options.override_for(&name).expect("an override")
        };
        assert_eq!(attribute.options.overrides.len(), 6);
        assert!(fixed("N").as_value().is_ok());
        assert!(fixed("N").as_type().is_err());
        assert!(fixed("S").as_value().is_ok());
        assert!(
            fixed("R").as_value().is_ok(),
            "a tuple of paths reads as a value"
        );
        assert!(fixed("R").as_type().is_ok());
        assert!(
            fixed("E").as_type().is_ok(),
            "the comma inside `Result<A, B>` is the type's"
        );
        assert!(fixed("F").as_value().is_ok());
        assert!(fixed("L").as_type().is_ok());
    }

    #[test]
    fn the_path_is_optional_and_the_switches_are_the_subjects() {
        let switches_only = attribute("from = false, try_from = true");
        assert!(switches_only.path.is_none());
        assert_eq!(
            switches_only.subject.from.as_ref().map(Switch::value),
            Some(false)
        );
        assert_eq!(
            switches_only.subject.try_from.as_ref().map(Switch::value),
            Some(true)
        );
        let by_ref = attribute("try_from = ref, into = Target<A, B>, into = Other");
        let forms = by_ref.subject.try_from.as_ref().unwrap().forms();
        assert!(forms.owned && forms.by_ref && !forms.by_mut);
        assert_eq!(by_ref.subject.into.len(), 2);
        assert!(syn::parse_str::<DispatchAttribute>("from = ref").is_err());
        assert!(syn::parse_str::<DispatchAttribute>("try_from = sometimes").is_err());
        let with_path = attribute("a::Trait<X>, mismatch(same = false, other = 1)");
        assert!(with_path.path.is_some());
        assert_eq!(with_path.options.mismatch.len(), 2);
        assert!(
            syn::parse_str::<TraitOptions>("from = false").is_err(),
            "a trait's options alone take no switch"
        );
        assert!(syn::parse_str::<DispatchAttribute>("Trait, Error = A, Error = B").is_err());
        assert!(syn::parse_str::<DispatchAttribute>("Trait, from = false, from = true").is_err());
        assert!(syn::parse_str::<DispatchAttribute>("Trait, bogus").is_err());
        let gat = |tokens: &str| {
            syn::parse_str::<DispatchAttribute>(tokens)
                .err()
                .expect("an error")
                .to_string()
        };
        assert!(gat("Lend, Item<'a> = &'a str").contains("`Item<..>` is a generic associated type"));
        assert!(gat("Item<'a> = &'a str").contains("impl Trait for .."));
    }

    #[test]
    fn forwarding_names_variants_or_everything() {
        let options: ForwardingOptions =
            syn::parse_str("Circle, Square, with = Some, call = r").unwrap();
        assert_eq!(options.variants.len(), 2);
        assert!(options.with.is_some() && options.call.is_some() && !options.all);
        assert!(syn::parse_str::<ForwardingOptions>("*").unwrap().all);
        assert!(syn::parse_str::<ForwardingOptions>("*, Circle").is_err());
        assert!(syn::parse_str::<ForwardingOptions>("Circle, Circle").is_err());
        assert!(syn::parse_str::<ForwardingOptions>("with = Some").is_err());
        let closure = syn::parse_str::<ForwardingOptions>("*, with = |s| Some(s)")
            .err()
            .expect("an error")
            .to_string();
        assert!(
            closure.starts_with("`with` takes the path of a function"),
            "{closure}"
        );
    }

    #[test]
    fn the_closest_name_is_within_one_edit_or_two_for_long_names() {
        let names = ["Unit", "Error", "from", "try_from"];
        assert_eq!(closest("Unti", names), Some("Unit"));
        assert_eq!(closest("form", names), Some("from"));
        assert_eq!(closest("Eror", names), Some("Error"));
        assert_eq!(closest("try_form", names), Some("try_from"));
        assert_eq!(closest("tri_form", names), Some("try_from"));
        assert_eq!(closest("Unix", names), Some("Unit"));
        assert_eq!(
            closest("Un", names),
            None,
            "two edits on a short name say nothing"
        );
        assert_eq!(closest("target", names), None);
        assert_eq!(edit_distance("eqq", "eq"), 1);
        assert_eq!(edit_distance("same", "sane"), 1);
        assert_eq!(listed(&[]), "");
        assert_eq!(listed(&["a".into()]), "`a`");
        assert_eq!(
            listed(&["a".into(), "b".into(), "c".into()]),
            "`a`, `b` and `c`"
        );
    }

    #[test]
    fn target_options_differ_between_a_variant_and_a_field() {
        let attributes: Vec<Attribute> = vec![
            parse_quote!(#[dispatch(deref, from = false)]),
            parse_quote!(#[dispatch(try_from = true)]),
        ];
        let variant = TargetOptions::read(&attributes, Position::Variant).unwrap();
        assert!(variant.route.as_ref().is_some_and(Route::is_deref));
        assert!(
            !variant.from
                && variant
                    .try_from
                    .is_some_and(|forms| forms.owned && !forms.by_ref)
        );
        let by_ref: Vec<Attribute> = vec![parse_quote!(#[dispatch(try_from = ref_mut)])];
        let forms = TargetOptions::read(&by_ref, Position::Variant)
            .unwrap()
            .try_from
            .unwrap();
        assert!(forms.owned && forms.by_ref && forms.by_mut);
        assert!(
            TargetOptions::read(&attributes, Position::StructField).is_err(),
            "a field takes `deref` and `via` only"
        );
        let bare: Vec<Attribute> = vec![parse_quote!(#[dispatch])];
        assert!(
            TargetOptions::read(&bare, Position::StructField)
                .unwrap()
                .from
        );
        let named: Vec<Attribute> =
            vec![parse_quote!(#[dispatch(Rom, io::Write, Encoder<Json>, deref)])];
        let field = TargetOptions::read(&named, Position::StructField).unwrap();
        assert_eq!(field.traits.len(), 3);
        assert!(field.route.is_some_and(|route| route.is_deref()));
        assert!(
            TargetOptions::read(&named, Position::Variant).is_err(),
            "a variant names no trait"
        );
        let routed: Vec<Attribute> = vec![parse_quote!(#[dispatch(via = .base.get_ref())])];
        let route = TargetOptions::read(&routed, Position::StructField)
            .unwrap()
            .route
            .unwrap();
        assert_eq!(route.spelled(), ".base.get_ref()");
        let both: Vec<Attribute> = vec![parse_quote!(#[dispatch(deref, via = .0)])];
        assert!(TargetOptions::read(&both, Position::Variant).is_err());
        let spec: TraitOptions = syn::parse_str("via(VarChar = .1, Decimal = .2)").unwrap();
        assert_eq!(spec.routes.len(), 2);
        assert_eq!(
            spec.to_token_stream().to_string().replace(' ', ""),
            "via(VarChar=.1,Decimal=.2)"
        );
        let forwarding: ForwardingOptions = syn::parse_str("*, field = line_no").unwrap();
        assert!(forwarding.field.is_some() && !forwarding.value);
        let forwarding: ForwardingOptions = syn::parse_str("A, B, value, with = Some").unwrap();
        assert!(forwarding.value && forwarding.with.is_some());
        assert!(syn::parse_str::<ForwardingOptions>("*, field = a, value").is_err());
        assert!(syn::parse_str::<ForwardingOptions>("*, value, call = m").is_err());
        let forwarding: ForwardingOptions =
            syn::parse_str("*, via = .content(), call = refresh").unwrap();
        assert_eq!(forwarding.via.unwrap().spelled(), ".content()");
    }

    /// The message of an error, or a panic for a result that is none.
    fn message<T>(result: Result<T>) -> String {
        match result {
            Ok(_) => panic!("expected an error"),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn a_structs_field_names_a_trait_from_crate_self_super_or_the_root() {
        let rooted: Vec<Attribute> = vec![parse_quote!(#[dispatch(
            crate::Rom,
            self::Rom,
            super::io::Write,
            ::rom::Rom,
            crate::Encoder<Json>,
            deref,
        )])];
        let field = TargetOptions::read(&rooted, Position::StructField).unwrap();
        let traits: Vec<String> = field
            .traits
            .iter()
            .map(|path| path.to_token_stream().to_string().replace(' ', ""))
            .collect();
        assert_eq!(
            traits,
            [
                "crate::Rom",
                "self::Rom",
                "super::io::Write",
                "::rom::Rom",
                "crate::Encoder<Json>"
            ]
        );
        assert!(field.route.is_some_and(|route| route.is_deref()));
        for (attribute, first) in [
            (parse_quote!(#[dispatch(crate::Rom)]), "crate"),
            (parse_quote!(#[dispatch(self::Rom)]), "self"),
            (parse_quote!(#[dispatch(super::io::Write)]), "super"),
            (parse_quote!(#[dispatch(::rom::Rom)]), "rom"),
        ] {
            let attributes: Vec<Attribute> = vec![attribute];
            assert_eq!(
                message(TargetOptions::read(&attributes, Position::Variant)),
                format!("unknown option `{first}`; the options are `deref`, `via = route`, `from = false` and `try_from = true`"),
                "a variant names no trait"
            );
            let on_field = message(TargetOptions::read(&attributes, Position::VariantField));
            assert!(
                on_field.starts_with(&format!("unknown option `{first}`")),
                "a variant's field names no trait: {on_field}"
            );
        }
    }

    #[test]
    fn a_variants_field_takes_a_route_and_nothing_else() {
        let field =
            |attribute: Attribute| TargetOptions::read(&[attribute], Position::VariantField);
        assert!(field(parse_quote!(#[dispatch])).unwrap().route.is_none());
        assert!(field(parse_quote!(#[dispatch(deref)]))
            .unwrap()
            .route
            .is_some_and(|route| route.is_deref()));
        let routed = field(parse_quote!(#[dispatch(via = .base)])).unwrap();
        assert_eq!(routed.route.unwrap().spelled(), ".base");
        let sentence = "the options of a variant's field are `deref` and `via = route`";
        for refused in [
            parse_quote!(#[dispatch(Shape)]),
            parse_quote!(#[dispatch(from = false)]),
            parse_quote!(#[dispatch(try_from = true)]),
            parse_quote!(#[dispatch(bogus = 1)]),
        ] {
            let error = message(field(refused));
            assert!(error.contains(sentence), "{error}");
        }
        assert!(
            message(field(parse_quote!(#[dispatch(Shape)]))).starts_with("unknown option `Shape`")
        );
    }

    #[test]
    fn a_partial_blocks_mark_leaves_to_the_default_or_forwards_by_name() {
        let mark = |attribute: Attribute| TraitMark::read(&attribute, Words::Enum);
        assert!(matches!(
            mark(parse_quote!(#[dispatch(default)])),
            Ok(TraitMark::Default)
        ));
        let Ok(TraitMark::Forward(forwarding)) = mark(parse_quote!(#[dispatch(Square, via = .0)]))
        else {
            panic!("a forward");
        };
        assert_eq!(forwarding.variants.len(), 1);
        assert!(forwarding.via.is_some() && !forwarding.all);
        assert!(matches!(
            mark(parse_quote!(#[dispatch(*)])),
            Ok(TraitMark::Forward(ForwardingOptions { all: true, .. }))
        ));
        for (attribute, option) in [
            (parse_quote!(#[dispatch(*, call = other)]), "`call`"),
            (parse_quote!(#[dispatch(*, with = Some)]), "`with`"),
            (parse_quote!(#[dispatch(*, field = inner)]), "`field`"),
            (parse_quote!(#[dispatch(*, value)]), "`value`"),
        ] {
            let error = message(mark(attribute));
            assert!(
                error.starts_with(&format!("{option} is an option of an inherent method")),
                "{error}"
            );
        }
    }

    #[test]
    fn a_bare_mark_on_a_method_names_no_variant() {
        let error = message(ForwardingOptions::read(
            &parse_quote!(#[dispatch]),
            Words::Enum,
        ));
        assert!(
            error.starts_with("name the variants to forward to, `#[dispatch(Variant, ..)]`"),
            "{error}"
        );
        assert!(
            message(TraitMark::read(&parse_quote!(#[dispatch]), Words::Enum))
                .starts_with("name the variants to forward to")
        );
    }

    #[test]
    fn a_list_speaks_the_words_of_its_reader() {
        let read = |tokens: &str, words: Words| {
            message(DispatchAttribute::read(
                syn::parse_str(tokens).expect("tokens"),
                words,
            ))
        };
        let block = |tokens: &str| message(syn::parse_str::<TraitOptions>(tokens));
        assert!(read("Value, inline", Words::Enum)
            .starts_with("unknown option `inline`; the options are `Name = Type`"));
        assert_eq!(
            read("Value, inline", Words::Struct),
            "unknown option `inline`; the options of a struct are `Name = Type` for an associated type, `Name = value` for an associated const, `via(field = route)` and `remote = path`"
        );
        assert_eq!(
            block("Unit = u8, inline"),
            "unknown option `inline`; a block's attribute takes the trait's options: `Name = Type` for an associated type, `Name = value` for an associated const, `mismatch(method = expr)` and a route, `via(name = route)`"
        );
        assert!(read("Lend, Item<'a> = &'a A", Words::Enum).starts_with(
            "`Item<..>` is a generic associated type, which only the enum can declare"
        ));
        assert_eq!(
            read("Lend, Item<'a> = &'a A", Words::Struct),
            "`Item<..>` is a generic associated type, which only the struct can declare; write `type Item<..> = ..;` in a `#[whichever::dispatch] impl Lend for ..` block"
        );
        assert_eq!(
            block("Item<'a> = &'a A"),
            "`Item<..>` is a generic associated type, which the block declares and no option gives; write `type Item<..> = ..;` in the block"
        );
        assert_eq!(
            block("from = false"),
            "`from`, `try_from`, `into` and `remote` are options of an enum's own `#[dispatch]` attribute; a block's attribute takes the trait's options only"
        );
        assert!(
            DispatchAttribute::read(syn::parse_str("Value").unwrap(), Words::Struct).is_ok(),
            "a struct's attribute reads a trait as an enum's does"
        );
    }

    #[test]
    fn a_structs_mark_names_its_field() {
        let read = |attribute: Attribute| ForwardingOptions::read(&attribute, Words::Struct);
        assert_eq!(
            message(read(parse_quote!(#[dispatch]))),
            "name the field to forward to, `#[dispatch(field)]`, or `#[dispatch(*)]` for the one the struct forwards to"
        );
        assert_eq!(
            message(read(parse_quote!(#[dispatch()]))),
            "name the field to forward to, or `*` for the one the struct forwards to"
        );
        assert_eq!(
            message(read(parse_quote!(#[dispatch(*, square)]))),
            "`*` already names the field the struct forwards to; name a field or `*`, not both"
        );
        assert_eq!(
            message(ForwardingOptions::read(
                &parse_quote!(#[dispatch(*, Square)]),
                Words::Enum
            )),
            "`*` already names every variant"
        );
        let Ok(named) = read(parse_quote!(#[dispatch(square)])) else {
            panic!("a field named");
        };
        assert_eq!(named.variants.len(), 1);
        assert!(!named.all);
        assert!(
            message(TraitMark::read(&parse_quote!(#[dispatch]), Words::Struct))
                .starts_with("name the field to forward to")
        );
    }

    #[test]
    fn an_option_naming_nothing_on_a_struct_offers_a_route_and_no_conversion() {
        let item_trait: ItemTrait = parse_quote! {
            trait Shape {
                type Output;
                fn area(&self) -> u32;
            }
        };
        let trait_ident: Ident = parse_quote!(Shape);
        let unknown = |options: &str, words: Words| {
            syn::parse_str::<TraitOptions>(options)
                .expect("valid options")
                .unknown_names(&item_trait, &trait_ident, words, true)
                .into_iter()
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            unknown("frm = false", Words::Enum),
            ["`Shape` has no associated type or const `frm`; did you mean the enum's option `from = ..`?"]
        );
        assert_eq!(
            unknown("frm = false", Words::Struct),
            ["`Shape` has no associated type or const `frm`; its associated items are `Output`, and a struct's other option is a route, `via(field = route)`"]
        );
        assert_eq!(
            unknown("target = inner", Words::Struct),
            ["`Shape` has no associated type or const `target`; a struct forwards to its one field, to the field marked `#[dispatch]`, or to the one that names the trait, `#[dispatch(Trait)]`"]
        );
        assert_eq!(
            unknown("Outpt = u8", Words::Struct),
            ["`Shape` has no associated type or const `Outpt`; did you mean `Output`?"]
        );
    }

    #[test]
    fn an_option_naming_nothing_in_the_trait_is_reported_with_what_was_meant() {
        let item_trait: ItemTrait = parse_quote! {
            trait Shape {
                type Output;
                fn same(&self, other: &Self) -> bool;
                fn area(&self) -> u32;
            }
        };
        let trait_ident: Ident = parse_quote!(Shape);
        let unknown = |options: &str| {
            syn::parse_str::<TraitOptions>(options)
                .expect("valid options")
                .unknown_names(&item_trait, &trait_ident, Words::Enum, true)
                .into_iter()
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
        };
        assert!(unknown("Output = u8, mismatch(same = false)").is_empty());
        let errors = unknown("Outpt = u8, mismatch(sme = false)");
        assert_eq!(errors.len(), 2);
        assert_eq!(
            errors[0],
            "`Shape` has no associated type or const `Outpt`; did you mean `Output`?"
        );
        assert!(
            errors[1].starts_with("`Shape` has no method `sme` with a `Self`, `&Self` or `&mut Self` argument, so there is no mismatch to decide; did you mean `same`?"),
            "{}",
            errors[1]
        );
        let errors = unknown("mismatch(area = false)");
        assert!(
            errors[0].contains("its methods taking one are `same`"),
            "a method without a `Self` argument has no mismatch: {}",
            errors[0]
        );
    }
}
