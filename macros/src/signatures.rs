//! The trait as its carrier holds it.
//!
//! A carrier holds the trait's signatures and names the methods the trait
//! gives a default body; a carrier of the first prefix held the bodies
//! themselves. `implement!` reads either into [`Signatures`], which answers
//! for a method what the generators ask of it, so that the difference
//! between the two prefixes stops here.
use syn::{Attribute, Expr, Ident, ItemTrait, Path, Result, TraitItemFn};

use crate::options::method_mismatch;
use crate::subject::is_deprecated;

/// The trait as carried: its declaration, the path it is implemented at
/// when the declaration restates another crate's trait, and the methods it
/// gives a default body.
pub(crate) struct Signatures {
    /// The trait's declaration: its name, its generics and its items, each
    /// with the attributes the carrier kept.
    pub(crate) item: ItemTrait,
    /// The real path of a trait restated with `remote = ..`, which the impl
    /// is written for.
    pub(crate) remote: Option<Path>,
    /// The methods the trait gives a default body, as a carrier that leaves
    /// the bodies behind names them.
    pub(crate) provided: Vec<Ident>,
}

impl Signatures {
    /// Returns the path the impl is written for: a restated trait's real
    /// path, with the generic arguments the subject named on the
    /// declaration's path, or the path named.
    pub(crate) fn implemented_path(&self, named: &Path) -> Path {
        match &self.remote {
            Some(remote) => {
                let mut remote = remote.clone();
                if let (Some(last), Some(named)) =
                    (remote.segments.last_mut(), named.segments.last())
                {
                    last.arguments = named.arguments.clone();
                }
                remote
            }
            None => named.clone(),
        }
    }

    /// Returns `function`, a method of the trait, with what the trait says
    /// of its default body.
    pub(crate) fn method<'a>(&'a self, function: &'a TraitItemFn) -> Method<'a> {
        let given = function.default.is_some() || self.provided.contains(&function.sig.ident);
        let default = match (given, &self.remote) {
            (true, _) => DefaultBody::Given,
            (false, None) => DefaultBody::Required,
            // A restatement names the methods it forwards and nothing
            // else, so a method it lists without a body may still have one
            // in the real trait, which the restatement cannot see.
            (false, Some(_)) => DefaultBody::Unseen,
        };
        Method { function, default }
    }
}

/// What a trait says of a method's default body.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DefaultBody {
    /// The trait gives the method a body.
    Given,
    /// The trait gives it none, so the impl has to write it.
    Required,
    /// The declaration restates another crate's trait and gives the method
    /// no body, which says nothing of the real trait.
    Unseen,
}

/// A method of the trait as carried.
#[derive(Clone, Copy)]
pub(crate) struct Method<'a> {
    /// The signature, with the attributes the carrier kept.
    pub(crate) function: &'a TraitItemFn,
    pub(crate) default: DefaultBody,
}

impl<'a> Method<'a> {
    pub(crate) fn ident(&self) -> &'a Ident {
        &self.function.sig.ident
    }

    /// Returns whether the trait is seen to give the method a body, which
    /// a method that cannot be forwarded can be left to.
    pub(crate) fn has_default(&self) -> bool {
        self.default == DefaultBody::Given
    }

    /// Returns whether `#[dispatch(default)]` may leave the method to the
    /// trait: it may unless the trait is seen to give no body. Where the
    /// declaration cannot see, rustc reports a method the real trait does
    /// not provide.
    pub(crate) fn may_be_left_to_default(&self) -> bool {
        self.default != DefaultBody::Required
    }

    /// Returns the `mismatch` expression given on the trait method itself.
    pub(crate) fn mismatch(&self) -> Result<Option<Expr>> {
        method_mismatch(self.function)
    }

    /// Returns the attributes of the method that the generated one carries.
    pub(crate) fn carried_attributes(&self) -> Vec<Attribute> {
        carried_attributes(&self.function.attrs)
    }

    /// Returns whether the trait deprecates the method, which is then
    /// forwarded under `#[allow(deprecated)]`, as a hand writes it; the
    /// deprecation reaches the callers through the trait's item.
    pub(crate) fn is_deprecated(&self) -> bool {
        is_deprecated(&self.function.attrs)
    }
}

/// Returns the attributes of a trait item that the generated item carries:
/// `#[cfg]`, since the item exists under the same condition, and the
/// `#[allow]` the trait's author wrote, since the signature the lint is
/// allowed on is the one the generated item repeats. A level that only
/// silences travels; one that raises the level, `warn`, `deny` or `expect`,
/// is the trait crate's policy and stays there. An `#[allow]` another macro
/// added, as `#[async_trait]` adds to every method, stays too: rustc exempts
/// it from the lints about lint names where it was written, and would not
/// on the way through the carrier.
pub(crate) fn carried_attributes(attributes: &[Attribute]) -> Vec<Attribute> {
    attributes
        .iter()
        .filter(|attribute| {
            let path = attribute.path();
            path.is_ident("cfg") || (path.is_ident("allow") && written_by_hand(attribute))
        })
        .cloned()
        .collect()
}

/// Returns whether `attribute` was written in the source rather than added
/// by a macro: a token's span is then exactly the token's own text.
fn written_by_hand(attribute: &Attribute) -> bool {
    attribute
        .path()
        .get_ident()
        .and_then(|ident| ident.span().source_text())
        .is_some_and(|text| text == "allow")
}
