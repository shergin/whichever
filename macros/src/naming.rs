//! How the carrier macros are named.
//!
//! A `pub` trait's carrier is exported at the crate root, so its name has
//! to be unique across the crate: it carries the trait's name, a hash of
//! the trait's text and of the position of every one of its tokens, and the
//! attribute's call site. A subject's carrier stays in its module and is
//! named the same way, for one rule. The hash is a fixed FNV-1a, so the
//! names come out the same on every compiler release.
use std::hash::{Hash, Hasher};

use proc_macro2::{Span, TokenStream as TokenStream2, TokenTree};
use quote::format_ident;
use syn::Ident;

/// Where a token was written: the key of its file, its line and its
/// column.
pub(crate) type Position = (String, usize, usize);

/// Returns where `span` was written, from the compiler. Only an expansion
/// can ask; a unit test hands [`trait_carrier`] a stand-in instead.
pub(crate) fn written_at(span: Span) -> Position {
    let span = span.unwrap();
    (file_key(&span.file()), span.line(), span.column())
}

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
/// tokens. `position` says where a token was written.
pub(crate) fn trait_carrier(
    ident: &Ident,
    tokens: &TokenStream2,
    name: Option<&Ident>,
    line: usize,
    column: usize,
    position: &dyn Fn(Span) -> Position,
) -> Ident {
    match name {
        Some(name) => format_ident!("__whichever_{}_{}", ident, name),
        None => {
            let mut hasher = Fnv::default();
            tokens.to_string().hash(&mut hasher);
            hash_token_positions(tokens.clone(), &mut hasher, position);
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
fn hash_token_positions(
    tokens: TokenStream2,
    hasher: &mut Fnv,
    position: &dyn Fn(Span) -> Position,
) {
    for tree in tokens {
        position(tree.span()).hash(hasher);
        if let TokenTree::Group(group) = tree {
            hash_token_positions(group.stream(), hasher, position);
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use quote::quote;
    use syn::parse_quote;

    use super::*;

    /// Every token at one place, or each at its own.
    fn at(file: &str, line: usize) -> impl Fn(Span) -> Position {
        let file = file.to_owned();
        move |_| (file.clone(), line, 1)
    }

    #[test]
    fn the_name_is_a_function_of_the_text_and_the_positions() {
        let ident: Ident = parse_quote!(Shape);
        let tokens = quote!(
            pub trait Shape {
                fn area(&self) -> u32;
            }
        );
        let same = |line| trait_carrier(&ident, &tokens, None, 3, 1, &at("src/lib.rs", line));
        assert_eq!(same(10), same(10), "the same text at the same place");
        assert_ne!(same(10), same(11), "the same text at another place");
        let other = quote!(
            pub trait Shape {
                fn area(&self) -> u64;
            }
        );
        assert_ne!(
            trait_carrier(&ident, &other, None, 3, 1, &at("src/lib.rs", 10)),
            same(10),
            "other text at the same place"
        );
        assert_ne!(
            trait_carrier(&ident, &tokens, None, 3, 1, &at("tokio/mod.rs", 10)),
            trait_carrier(&ident, &tokens, None, 3, 1, &at("smol/mod.rs", 10)),
            "another file"
        );
        assert!(same(10).to_string().starts_with("__whichever_Shape_"));
    }

    #[test]
    fn a_name_given_stands_in_for_the_hash() {
        let ident: Ident = parse_quote!(Shape);
        let name: Ident = parse_quote!(tokio);
        let counter = Cell::new(0usize);
        let counting = |_: Span| {
            counter.set(counter.get() + 1);
            ("src/lib.rs".to_owned(), 1, 1)
        };
        let named = trait_carrier(
            &ident,
            &quote!(
                trait Shape {}
            ),
            Some(&name),
            3,
            1,
            &counting,
        );
        assert_eq!(named, "__whichever_Shape_tokio");
        assert_eq!(counter.get(), 0, "no position is asked for");
    }

    #[test]
    fn the_file_key_is_the_last_two_components() {
        assert_eq!(file_key("/home/me/src/tokio/mod.rs"), "tokio/mod.rs");
        assert_eq!(file_key("C:\\src\\smol\\mod.rs"), "smol/mod.rs");
        assert_eq!(file_key("lib.rs"), "lib.rs");
    }
}
