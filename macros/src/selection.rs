//! Which targets a method's `#[dispatch(..)]` names, and the checks on a
//! method's attribute and body that go with the choice: the engine's,
//! shared by the trait path and the inherent path.
use quote::quote_spanned;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Block, Error, Ident, Meta, Path, Result, Signature, Token, Visibility};

use crate::forwarding::Arm;
use crate::options::{is_dispatch_attribute, ForwardingOptions};
use crate::site::Site;
use crate::subject::{Entries, Subject, Target};

/// A method declared without a body, which `syn` leaves as verbatim tokens.
pub(crate) struct BodylessMethod {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) vis: Visibility,
    pub(crate) sig: Signature,
}

impl Parse for BodylessMethod {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let sig = input.parse()?;
        input.parse::<Token![;]>()?;
        Ok(Self { attrs, vis, sig })
    }
}

/// Parses the options of a method's `#[dispatch(..)]`; a bare `#[dispatch]`
/// names no variant.
fn forwarding_options(attribute: &Attribute) -> Result<ForwardingOptions> {
    if matches!(attribute.meta, Meta::Path(_)) {
        return Err(Error::new_spanned(
            attribute,
            "name the variants to forward to, `#[dispatch(Variant, ..)]`, or `#[dispatch(*)]` for all of them",
        ));
    }
    attribute.parse_args()
}

/// Parses the one `#[dispatch(..)]` of a method, `attribute`, taken off
/// `rest`; a second one left in `rest` is reported on itself and taken off
/// too, so that rustc does not report it as an unknown attribute on top.
pub(crate) fn one_attribute(
    rest: &mut Vec<Attribute>,
    attribute: &Attribute,
) -> Result<ForwardingOptions> {
    let Some(position) = rest.iter().position(is_dispatch_attribute) else {
        return forwarding_options(attribute);
    };
    let second = rest.remove(position);
    rest.retain(|attribute| !is_dispatch_attribute(attribute));
    Err(Error::new_spanned(
        second,
        "one `#[dispatch(..)]` per method; name every variant it forwards to in that one",
    ))
}

/// What a method's `#[dispatch(..)]` selects: the targets, with the
/// method's own route when it gives one; the arms for the variants `*`
/// cannot reach; and whether every variant is covered, so that a body
/// would never run.
pub(crate) struct Selection<'a> {
    pub(crate) targets: Vec<Target<'a>>,
    pub(crate) unforwardable_arms: Vec<Arm>,
    pub(crate) covers_all: bool,
}

/// Selects the targets `forwarding` names among `entries`, or every target
/// under `*`. Under `*`, a variant that holds no payload cannot be
/// forwarded to and the method has no body to fall back on, so the arm for
/// it is an error, which fires only when the variant is compiled in.
pub(crate) fn select_targets<'a>(
    forwarding: &'a ForwardingOptions,
    entries: &'a Entries,
    prefix: &Path,
    method: &Ident,
    subject: &Subject,
) -> Result<Selection<'a>> {
    let subject_ident = subject.ident();
    let noun = subject.target_noun();
    let mut unforwardable_arms = vec![];
    let selected: Vec<Target> = if forwarding.all {
        for entry in entries.iter() {
            if let Some(unforwardable) = entry.unforwardable() {
                let (ident, reason) = (&entry.ident, &unforwardable.reason);
                let message = format!("`{method}` is forwarded to every variant, but `{ident}` {reason}; name the variants with a payload and give the method a body for the rest");
                unforwardable_arms.push(Arm::unforwardable(
                    prefix,
                    entry,
                    quote_spanned! { method.span() => ::core::compile_error!(#message) },
                    Site::at(method.span()),
                ));
            }
        }
        // On an enum `*` is every variant; on a struct, the field the trait
        // is forwarded to, or in an inherent block the field marked bare or
        // the one field.
        entries.star()?
    } else {
        forwarding
            .variants
            .iter()
            .map(|name| {
                let entry = entries.iter().find(|entry| entry.ident == *name);
                if let Some(target) = entry.and_then(|entry| entry.target()) {
                    return Ok(target);
                }
                let message = match entry.and_then(|entry| entry.unforwardable()) {
                    Some(unforwardable) => format!(
                        "`{name}` {}, so nothing can be forwarded to it",
                        unforwardable.reason
                    ),
                    None => format!("`{subject_ident}` has no {noun} `{name}`"),
                };
                Err(Error::new(name.span(), message))
            })
            .collect::<Result<_>>()?
    };
    // A struct always holds every field it names, so one field covers it.
    let covers_all = forwarding.all
        || subject.has_one_target()
        || entries
            .iter()
            .all(|entry| entry.target().is_some() && forwarding.variants.contains(&entry.ident));
    // The method's own route, when it gives one, over the variants'.
    let targets = selected
        .into_iter()
        .map(|target| match &forwarding.via {
            Some(route) => target.through(route),
            None => target,
        })
        .collect();
    Ok(Selection {
        targets,
        unforwardable_arms,
        covers_all,
    })
}

/// Checks that a method has a body exactly when some variant is left to it.
pub(crate) fn check_body(
    selection: &Selection,
    body: Option<&Block>,
    sig: &Signature,
    noun: &str,
) -> Result<()> {
    let method = &sig.ident;
    match (selection.covers_all, body) {
        (true, Some(body)) => Err(Error::new_spanned(
            body,
            format!("`{method}` is forwarded to every {noun}, so its body would never run; end the signature with `;` instead"),
        )),
        (false, None) => Err(Error::new_spanned(
            sig,
            format!("`{method}` is forwarded to some variants only; give it a body, which is what the other variants return"),
        )),
        _ => Ok(()),
    }
}
