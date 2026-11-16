//! The mechanism of the span rule in `ARCHITECTURE.md` 3.8: every token the
//! engine writes carries a user span, the subject's, a method's or a
//! payload's, never the call site. Inside `implement!` the call site is the
//! carrier's expansion, which `#[dispatchable]` wrote, so a token written
//! there puts rustc's errors on the trait's attribute, in another crate's
//! file when the trait is foreign. The test reads the modules that write
//! generated code and names every bare `quote!` or `parse_quote!` it finds,
//! and every identifier or lifetime built at `Span::mixed_site()` or
//! `Span::call_site()` without a location. Such a token is no less written
//! at the call site for being built outside a `quote!`: an expression that
//! holds it beside the arm's own tokens has a span joined from the two, and
//! rustc reports it from the attribute to the subject. `Site::at` in
//! `site.rs` is the way a span is made: the hygiene of `Span::mixed_site()`,
//! located where the user's token is.
use std::fs;
use std::path::Path;

use syn::visit::Visit;
use syn::{Expr, ExprCall, Macro};

/// The modules that write generated code inside `implement!` or
/// `implement_block!`, or read and judge what it is written from. The
/// others write at a call site that is the user's own attribute or write
/// no code: `dispatch.rs`, `dispatchable.rs` and `conversions.rs` expand at
/// `#[dispatch]` or `#[dispatchable]`, `naming.rs` names the carrier macro
/// `#[dispatchable]` leaves, `protocol.rs` writes the messages between the
/// macros, `route.rs` spells a route back into a message, `options.rs`
/// reads the attributes, and `spelling.rs` spells a type in a sentence.
const ENGINE: &[&str] = &[
    "agreement.rs",
    "arm.rs",
    "block.rs",
    "builds.rs",
    "forwarding.rs",
    "implement.rs",
    "items.rs",
    "marked.rs",
    "partial.rs",
    "refusal.rs",
    "selection.rs",
    "shape.rs",
    "signatures.rs",
    "site.rs",
    "subject.rs",
    "substitution.rs",
];

/// Every macro invocation that writes tokens at the call site, and every
/// identifier or lifetime built there.
struct CallSiteWriters(Vec<String>);

/// Returns the last two segments of `expr` when it is a path, `Ident::new`
/// for `proc_macro2::Ident::new`.
fn last_two(expr: &Expr) -> Option<(String, String)> {
    let Expr::Path(path) = expr else {
        return None;
    };
    let mut segments = path.path.segments.iter().rev();
    let last = segments.next()?.ident.to_string();
    let before = segments.next()?.ident.to_string();
    Some((before, last))
}

/// Returns whether `expr` is a span with no location of its own,
/// `Span::mixed_site()` or `Span::call_site()` as it stands; one that a
/// method call locates, `Span::mixed_site().located_at(span)`, is not.
fn unlocated(expr: &Expr) -> bool {
    let Expr::Call(call) = expr else {
        return false;
    };
    matches!(
        last_two(&call.func),
        Some((ty, constructor)) if ty == "Span"
            && (constructor == "mixed_site" || constructor == "call_site")
    )
}

impl<'ast> Visit<'ast> for CallSiteWriters {
    fn visit_macro(&mut self, mac: &'ast Macro) {
        let name = mac
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        let tokens = mac.tokens.to_string();
        let at_call_site = match name.as_deref() {
            Some("quote") | Some("parse_quote") => true,
            // `format_ident!` locates its identifier at the call site unless
            // it is given a `span`.
            Some("format_ident") => !tokens.contains("span ="),
            _ => false,
        };
        if at_call_site {
            let shown: String = tokens.chars().take(60).collect();
            self.0
                .push(format!("{}!({shown}..)", name.unwrap_or_default()));
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        let built = matches!(
            last_two(&call.func),
            Some((ty, constructor)) if (ty == "Ident" || ty == "Lifetime") && constructor == "new"
        );
        if built && call.args.iter().any(unlocated) {
            let shown: String = quote::ToTokens::to_token_stream(call)
                .to_string()
                .chars()
                .take(60)
                .collect();
            self.0.push(shown);
        }
        syn::visit::visit_expr_call(self, call);
    }
}

/// Returns what `source`, outside its unit tests, writes at the call site.
fn call_site_writers(source: &str) -> Vec<String> {
    let file = syn::parse_file(source).expect("a Rust file");
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
    writers.0
}

#[test]
fn the_engine_writes_every_token_at_a_user_site() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut unspanned = vec![];
    for name in ENGINE {
        let source = fs::read_to_string(root.join(name)).expect("a source file");
        unspanned.extend(
            call_site_writers(&source)
                .into_iter()
                .map(|site| format!("{name}: {site}")),
        );
    }
    assert!(
        unspanned.is_empty(),
        "{} tokens are written at the call site, which inside `implement!` is the trait's attribute; write them with `quote_spanned!` or `parse_quote_spanned!` at the subject's, the method's or the payload's span, and build an identifier or a lifetime at `at(span)`:\n  {}",
        unspanned.len(),
        unspanned.join("\n  ")
    );
}

#[test]
fn an_identifier_or_a_lifetime_built_at_the_call_site_is_named() {
    let named = |source: &str| call_site_writers(source).len();
    assert_eq!(
        named("fn f() -> Ident { Ident::new(\"__inner\", Span::mixed_site()) }"),
        1
    );
    assert_eq!(
        named("fn f() -> Ident { proc_macro2::Ident::new(\"__inner\", proc_macro2::Span::call_site()) }"),
        1
    );
    assert_eq!(
        named("fn f() -> Lifetime { Lifetime::new(\"'__enum\", Span::mixed_site()) }"),
        1
    );
    assert_eq!(
        named("fn f() -> Ident { format_ident!(\"__element{}\", 0) }"),
        1
    );
    // Located where the user's token is, each is the arm's own.
    assert_eq!(
        named("fn f(site: Span) -> Ident { Ident::new(\"__inner\", at(site)) }"),
        0
    );
    assert_eq!(
        named("fn f(site: Span) -> Ident { Ident::new(\"__inner\", Span::mixed_site().located_at(site)) }"),
        0
    );
    assert_eq!(
        named("fn f(site: Span) -> Ident { format_ident!(\"__element{}\", 0, span = site) }"),
        0
    );
    assert_eq!(
        named("mod tests { fn f() -> Ident { Ident::new(\"Kind\", Span::call_site()) } }"),
        0
    );
}
