//! The shape step: every pass shard's library is expanded once with
//! `cargo expand`, the tool the repository's golden test uses, and each
//! passing cell's forwarded methods are compared with its hand's through
//! [`crate::shape`].

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

/// Returns, for each of `modules` in the shard, how its subject's
/// forwarded methods differ from its hand's; a cell with no difference is
/// absent.
pub fn compare(
    workspace: &Workspace,
    shard: &Shard,
    modules: &[&str],
    trait_name: &str,
) -> Result<BTreeMap<String, Differences>, RunError> {
    let file = expand(workspace, shard)?;
    let mut found = BTreeMap::new();
    for module in modules {
        let differences = compare_cell(&file, module, trait_name);
        if !differences.is_empty() {
            found.insert(String::from(*module), differences);
        }
    }
    Ok(found)
}

fn compare_cell(file: &syn::File, module: &str, trait_name: &str) -> Differences {
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
            let differences = shape::differences(
                &shape::method_shape(&subject_method.block),
                &shape::method_shape(&hand_method.block),
            );
            if !differences.is_empty() {
                found.insert(name, differences);
            }
        }
    }
    found
}
