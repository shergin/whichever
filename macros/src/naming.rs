//! How the carrier macros are named.
//!
//! A `pub` trait's carrier is exported at the crate root, so its name has
//! to be unique across the crate: it carries the trait's name, a hash of
//! the trait's text and of the position of every one of its tokens, and the
//! attribute's call site. A subject's carrier stays in its module and is
//! named the same way, for one rule. The hash is a fixed FNV-1a, so the
//! names come out the same on every compiler release.
use std::hash::{Hash, Hasher};

use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::format_ident;
use syn::Ident;

/// FNV-1a over 64 bits, written out so that a carrier macro's name comes
/// out the same on every compiler release; `std`'s `DefaultHasher` is
/// documented as unstable across releases.
struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Returns the name of the carrier of the trait `ident`, whose tokens are
/// `tokens`. `name` stands in for the hash and the position when given, for
/// a trait that one `macro_rules!` emits several times with identical
/// tokens.
pub(crate) fn trait_carrier(
    ident: &Ident,
    tokens: &TokenStream2,
    name: Option<&Ident>,
    line: usize,
    column: usize,
) -> Ident {
    match name {
        Some(name) => format_ident!("__whichever_{}_{}", ident, name),
        None => {
            let mut hasher = Fnv::default();
            tokens.to_string().hash(&mut hasher);
            hash_token_positions(tokens.clone(), &mut hasher);
            format_ident!(
                "__whichever_{}_{:016x}_{}_{}",
                ident,
                hasher.finish(),
                line,
                column
            )
        }
    }
}

/// Returns the name of the carrier of the subject `ident`, whose tokens are
/// `tokens`.
pub(crate) fn subject_carrier(
    ident: &Ident,
    tokens: &TokenStream2,
    line: usize,
    column: usize,
) -> Ident {
    let mut hasher = Fnv::default();
    tokens.to_string().hash(&mut hasher);
    format_ident!(
        "__whichever_dispatch_{}_{:016x}_{}_{}",
        ident,
        hasher.finish(),
        line,
        column
    )
}

/// Hashes the file and position of every token in `tokens`, groups included.
///
/// The text alone does not tell apart the same trait emitted twice by one
/// `macro_rules!` in two modules, and the call site of `#[dispatchable]`
/// then lies inside the macro's body for both expansions. The tokens the
/// invocation supplied, usually the trait's name, keep the file and position
/// they were written at, so hashing those keeps the two apart. A trait
/// written out in full in the macro's body is the one case this leaves.
fn hash_token_positions(tokens: TokenStream2, hasher: &mut Fnv) {
    for tree in tokens {
        let span = tree.span().unwrap();
        (file_key(&span.file()), span.line(), span.column()).hash(hasher);
        if let TokenTree::Group(group) = tree {
            hash_token_positions(group.stream(), hasher);
        }
    }
}

/// Returns the last two components of a source file's path. They tell
/// `tokio/mod.rs` from `smol/mod.rs` without depending on where the checkout
/// lives, so the carrier macros' names come out the same on every machine.
fn file_key(path: &str) -> String {
    let mut components = path.rsplit(['/', '\\']);
    let file = components.next().unwrap_or_default();
    match components.next() {
        Some(directory) => format!("{directory}/{file}"),
        None => file.to_owned(),
    }
}
