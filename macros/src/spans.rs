//! The mechanism of the span rule in `ARCHITECTURE.md` 3.8: every token the
//! engine writes carries a user span, the subject's, a method's or a
//! payload's, never the call site. Inside `implement!` the call site is the
//! carrier's expansion, which `#[dispatchable]` wrote, so a token written
//! there puts rustc's errors on the trait's attribute, in another crate's
//! file when the trait is foreign. The test reads the modules that write
//! generated code and names every bare `quote!` or `parse_quote!` it finds.
use std::fs;
use std::path::Path;

use syn::visit::Visit;
use syn::Macro;

/// The modules that write generated code inside `implement!` or
/// `implement_block!`. The others write at a call site that is the user's
/// own attribute: `dispatch.rs`, `dispatchable.rs` and `conversions.rs`
/// expand at `#[dispatch]` or `#[dispatchable]`, `protocol.rs` writes the
/// messages between the macros, and `route.rs` spells a route back into a
/// message.
const ENGINE: &[&str] = &[
    "agreement.rs",
    "block.rs",
    "forwarding.rs",
    "implement.rs",
    "selection.rs",
    "shape.rs",
    "subject.rs",
    "substitution.rs",
];

/// Every macro invocation that writes tokens at the call site.
struct CallSiteWriters(Vec<String>);

impl<'ast> Visit<'ast> for CallSiteWriters {
    fn visit_macro(&mut self, mac: &'ast Macro) {
        let name = mac
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        if matches!(name.as_deref(), Some("quote") | Some("parse_quote")) {
            let tokens = mac.tokens.to_string();
            let shown: String = tokens.chars().take(60).collect();
            self.0
                .push(format!("{}!({shown}..)", name.unwrap_or_default()));
        }
    }
}

#[test]
fn the_engine_writes_every_token_at_a_user_site() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut unspanned = vec![];
    for name in ENGINE {
        let source = fs::read_to_string(root.join(name)).expect("a source file");
        let file = syn::parse_file(&source).expect("a Rust file");
        let mut writers = CallSiteWriters(vec![]);
        // Unit tests of the modules write nothing a user compiles.
        for item in &file.items {
            if let syn::Item::Mod(module) = item {
                if module.ident == "tests" {
                    continue;
                }
            }
            writers.visit_item(item);
        }
        unspanned.extend(writers.0.into_iter().map(|site| format!("{name}: {site}")));
    }
    assert!(
        unspanned.is_empty(),
        "{} tokens are written at the call site, which inside `implement!` is the trait's attribute; write them with `quote_spanned!` or `parse_quote_spanned!` at the subject's, the method's or the payload's span:\n  {}",
        unspanned.len(),
        unspanned.join("\n  ")
    );
}
