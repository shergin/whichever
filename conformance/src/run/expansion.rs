//! The shape step: every pass shard's library is expanded once with
//! `cargo expand`, the tool the repository's golden test uses, and each
//! passing cell's forwarded methods are compared with its hand's through
//! [`crate::shape`].
//!
//! Beside the operations, the step compares what the generated items
//! carry (what-is-generated.md): every method of the subject's impls
//! carries the attributes the hand's carries, so a forwarded trait method
//! has `#[inline]`, or the trait method's own `#[inline(..)]`, beside the
//! attributes the trait method passes on, and an inherent method keeps the
//! attributes the user wrote and gains none; and the trait's impl written
//! through the subject's attribute is `#[automatically_derived]`.

use std::collections::BTreeMap;
use std::process::Stdio;

use super::workspace::{Shard, Workspace};
use super::{cargo, harness, RunError};
use crate::shape;

/// What the shape step found for one cell: per forwarded method, the
/// differences from the hand, the first of them the method's.
pub type Differences = BTreeMap<String, Vec<String>>;

/// Checks that `cargo expand` is installed.
pub fn available(workspace: &Workspace) -> Result<(), RunError> {
    let output = cargo::command(&workspace.root, &workspace.target)
        .args(["expand", "--version"])
        .stderr(Stdio::null())
        .output()
        .map_err(harness)?;
    if output.status.success() {
        return Ok(());
    }
    Err(RunError::Harness(String::from(
        "`--shape` expands the shards with `cargo expand`, which is not installed; \
         install it with `cargo install cargo-expand`",
    )))
}

/// Returns the expanded library of a shard, parsed.
fn expand(workspace: &Workspace, shard: &Shard) -> Result<syn::File, RunError> {
    let output = cargo::command(&workspace.root, &workspace.target)
        .args(["expand", "--lib", "--color=never", "-p", &shard.name])
        .stderr(Stdio::null())
        .output()
        .map_err(harness)?;
    if !output.status.success() {
        return Err(RunError::Harness(format!(
            "`cargo expand -p {}` failed after a clean check",
            shard.name
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    syn::parse_file(&text).map_err(|error| {
        RunError::Harness(format!(
            "the expansion of `{}` does not parse: {error}",
            shard.name
        ))
    })
}

/// What the shape step checks of one cell.
#[derive(Clone, Copy, Debug)]
pub struct Expected<'a> {
    /// The cell's module.
    pub module: &'a str,
    /// Whether the subject's attribute writes the trait's impl, which is
    /// then `#[automatically_derived]`.
    pub derived: bool,
}

/// Returns, for each of `cells` in the shard, how its subject's
/// forwarded methods differ from its hand's; a cell with no difference is
/// absent.
pub fn compare(
    workspace: &Workspace,
    shard: &Shard,
    cells: &[Expected<'_>],
    trait_name: &str,
) -> Result<BTreeMap<String, Differences>, RunError> {
    let file = expand(workspace, shard)?;
    let mut found = BTreeMap::new();
    for cell in cells {
        let differences = compare_cell(&file, cell, trait_name);
        if !differences.is_empty() {
            found.insert(String::from(cell.module), differences);
        }
    }
    Ok(found)
}

/// Returns an attribute as it is written, without the `#[..]`.
fn spell_attribute(attribute: &syn::Attribute) -> String {
    let path: Vec<String> = attribute
        .path()
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let path = path.join("::");
    match &attribute.meta {
        syn::Meta::Path(_) => path,
        syn::Meta::List(list) => format!("{path}({})", list.tokens),
        syn::Meta::NameValue(_) => format!("{path} = .."),
    }
}

/// Returns the attributes, spelled and sorted, so that their order makes
/// no difference.
fn spell_attributes(attributes: &[syn::Attribute]) -> Vec<String> {
    let mut spelled: Vec<String> = attributes.iter().map(spell_attribute).collect();
    spelled.sort();
    spelled
}

/// Returns a list of attributes for a message.
fn list_attributes(attributes: &[String]) -> String {
    if attributes.is_empty() {
        return String::from("none");
    }
    let spelled: Vec<String> = attributes
        .iter()
        .map(|attribute| format!("`#[{attribute}]`"))
        .collect();
    spelled.join(" ")
}

fn compare_cell(file: &syn::File, cell: &Expected<'_>, trait_name: &str) -> Differences {
    let module = cell.module;
    let mut found = Differences::new();
    let missing = |what: &str| vec![format!("the expansion holds no {what}")];
    let Some(items) = shape::module(file, &[module]) else {
        found.insert(String::from("module"), missing("module of the cell"));
        return found;
    };
    let Some(hand_items) = shape::module(file, &[module, "hand"]) else {
        found.insert(String::from("module"), missing("hand"));
        return found;
    };
    let mut subject = shape::impls(items, "E", Some(trait_name));
    let derived = subject.iter().all(|block| {
        block
            .attrs
            .iter()
            .any(|attribute| attribute.path().is_ident("automatically_derived"))
    });
    if cell.derived && !derived {
        found.insert(
            String::from("impl"),
            vec![String::from(
                "the impl the subject's attribute writes is not `#[automatically_derived]`",
            )],
        );
    }
    subject.extend(shape::impls(items, "E", None));
    let mut hand = shape::impls(hand_items, "E", Some(trait_name));
    hand.extend(shape::impls(hand_items, "E", None));
    for block in &hand {
        for item in &block.items {
            let syn::ImplItem::Fn(hand_method) = item else {
                continue;
            };
            let name = hand_method.sig.ident.to_string();
            let Some(subject_method) = shape::method(&subject, &name) else {
                found.insert(name, missing("forwarded method of this name"));
                continue;
            };
            let mut differences = shape::differences(
                &shape::method_shape(&subject_method.block),
                &shape::method_shape(&hand_method.block),
            );
            let carried = spell_attributes(&subject_method.attrs);
            let written = spell_attributes(&hand_method.attrs);
            if carried != written {
                differences.push(format!(
                    "attributes: the subject {}, the hand {}",
                    list_attributes(&carried),
                    list_attributes(&written)
                ));
            }
            if !differences.is_empty() {
                found.insert(name, differences);
            }
        }
    }
    found
}
