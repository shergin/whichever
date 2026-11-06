//! The mechanism of rule 4, a limit is one sentence on the item: every
//! sentence the macros write is pinned by a `tests/ui` snapshot, so that a
//! sentence cannot land without its case and a case cannot drift from its
//! sentence. The test reads this crate's sources and the snapshots next
//! to the facade's tests, and names every sentence it finds unpinned.
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMacro, ExprMethodCall, Lit, Macro};

/// The sentences that are not limits: internal errors of the protocol,
/// which no source can reach; a shape stable Rust cannot write; and the
/// pieces of a sentence that are composed into one the snapshots pin.
const NOT_LIMITS: &[&str] = &[
    "unsupported `whichever` protocol version",
    "the request has no `",
    "expected an enum or a struct",
    "expected a trait impl",
    "the trait to implement is missing",
    "` must be a dependency of this crate: ",
    "a forwarded method needs a receiver",
    // Negative impls are not stable Rust.
    "a negative impl has nothing to dispatch",
    // `syn` leaves only a bodiless method verbatim in an impl block.
    "only methods, with or without a body, are forwarded",
    // Pieces of the override and mismatch sentences, which `{hint}` holds.
    "did you mean `",
    "it has none, and the other options are",
    "its methods taking one are",
];

/// Every sentence a source file writes, as the template it is written from.
struct Sentences(BTreeSet<String>);

impl Sentences {
    fn first_literal(tokens: proc_macro2::TokenStream) -> Option<String> {
        tokens.into_iter().find_map(|tree| match tree {
            proc_macro2::TokenTree::Literal(literal) => match Lit::new(literal) {
                Lit::Str(text) => Some(text.value()),
                _ => None,
            },
            _ => None,
        })
    }

    fn record(&mut self, text: Option<String>) {
        if let Some(text) = text {
            if text.len() >= 20 {
                self.0.insert(text);
            }
        }
    }

    /// Returns the runs of the template between its placeholders, in order.
    fn pieces(template: &str) -> Vec<String> {
        let mut pieces = vec![];
        let mut current = String::new();
        let mut depth = 0usize;
        let mut characters = template.chars().peekable();
        while let Some(character) = characters.next() {
            match character {
                '{' if depth == 0 && characters.peek() == Some(&'{') => {
                    characters.next();
                    current.push('{');
                }
                '}' if depth == 0 && characters.peek() == Some(&'}') => {
                    characters.next();
                    current.push('}');
                }
                '{' => {
                    if depth == 0 {
                        pieces.push(std::mem::take(&mut current));
                    }
                    depth += 1;
                }
                '}' => depth = depth.saturating_sub(1),
                other if depth == 0 => current.push(other),
                _ => {}
            }
        }
        pieces.push(current);
        pieces
            .into_iter()
            .filter(|piece| !piece.is_empty())
            .collect()
    }

    /// Returns whether `line` is an instance of the template: its pieces
    /// appear in order, with anything where the placeholders are.
    fn matches(template: &str, line: &str) -> bool {
        let mut rest = line;
        for piece in Self::pieces(template) {
            let Some(at) = rest.find(piece.as_str()) else {
                return false;
            };
            rest = &rest[at + piece.len()..];
        }
        true
    }
}

impl<'ast> Visit<'ast> for Sentences {
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = &*call.func {
            let name = path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string());
            if matches!(name.as_deref(), Some("new") | Some("new_spanned")) {
                if let Some(Expr::Lit(literal)) = call.args.iter().nth(1) {
                    if let Lit::Str(text) = &literal.lit {
                        self.record(Some(text.value()));
                    }
                }
            }
        }
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if call.method == "error" {
            if let Some(Expr::Lit(literal)) = call.args.first() {
                if let Lit::Str(text) = &literal.lit {
                    self.record(Some(text.value()));
                }
            }
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_macro(&mut self, expr: &'ast ExprMacro) {
        self.visit_macro(&expr.mac);
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        let name = mac
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        if matches!(name.as_deref(), Some("format") | Some("compile_error")) {
            self.record(Self::first_literal(mac.tokens.clone()));
        }
    }
}

#[test]
fn every_sentence_is_pinned_by_a_snapshot() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sentences = Sentences(BTreeSet::new());
    for entry in fs::read_dir(root.join("src")).expect("the sources") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|extension| extension == "rs")
            && path.file_name().is_some_and(|name| name != "limits.rs")
        {
            let source = fs::read_to_string(&path).expect("a source file");
            let file = syn::parse_file(&source).expect("a Rust file");
            // Unit tests of the modules write no sentence of the crate's.
            for item in &file.items {
                if let syn::Item::Mod(module) = item {
                    if module.ident == "tests" {
                        continue;
                    }
                }
                sentences.visit_item(item);
            }
        }
    }
    let mut snapshots = String::new();
    for entry in fs::read_dir(root.join("../tests/ui")).expect("the ui cases") {
        let path = entry.expect("an entry").path();
        if path
            .extension()
            .is_some_and(|extension| extension == "stderr")
        {
            snapshots.push_str(&fs::read_to_string(&path).expect("a snapshot"));
        }
    }
    // Every line of every snapshot: a sentence is the message of an error,
    // or, for a check rustc evaluates, the text of its note.
    let lines: Vec<&str> = snapshots.lines().collect();
    let unpinned: Vec<String> = sentences
        .0
        .iter()
        .filter(|template| {
            !NOT_LIMITS
                .iter()
                .any(|internal| template.contains(internal))
        })
        .filter(|template| !lines.iter().any(|line| Sentences::matches(template, line)))
        .cloned()
        .collect();
    assert!(
        unpinned.is_empty(),
        "{} sentences have no `tests/ui` snapshot pinning them:\n  {}",
        unpinned.len(),
        unpinned.join("\n  ")
    );
}
