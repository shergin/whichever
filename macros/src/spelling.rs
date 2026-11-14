//! How a type, a path or a route is spelled in a sentence.
//!
//! A token stream's own `Display` puts a space between most tokens, `Rc <
//! Account >`, and taking every space out glues the tokens Rust needs
//! apart, `&'a Name` into `&'aName`. A sentence spells its tokens as a hand
//! writes them: `Rc<Account>`, `&'a mut Name`, `Result<A, B>`, `dyn Named +
//! 'a`, `<Vec<T> as IntoIterator>::Item`, and a route as it is written,
//! `*.base.get_ref()`.
use proc_macro2::{Delimiter, Spacing, TokenStream as TokenStream2, TokenTree};
use quote::ToTokens;

/// One piece of a spelling, as far as the space around it goes.
enum Piece {
    /// An identifier, a keyword, a lifetime or a literal.
    Word(String),
    /// One operator, whole: `::`, `->`, `<`, `&` or `,`.
    Operator(String),
    /// An opening `(`, `[` or `{`.
    Open(char),
    /// A closing `)`, `]` or `}`.
    Close(char),
}

/// Spells `tokens` for a sentence.
pub(crate) fn spelled(tokens: &impl ToTokens) -> String {
    let mut pieces = vec![];
    collect(tokens.to_token_stream(), &mut pieces);
    let mut text = String::new();
    for (index, piece) in pieces.iter().enumerate() {
        if index > 0 && spaced(&pieces[index - 1], piece) {
            text.push(' ');
        }
        match piece {
            Piece::Word(word) | Piece::Operator(word) => text.push_str(word),
            Piece::Open(delimiter) | Piece::Close(delimiter) => text.push(*delimiter),
        }
    }
    text
}

/// Reads `tokens` into `pieces`, in order. A punctuation mark joined to the
/// next one is one operator with it, `::` or `->`; the `'` joined to an
/// identifier is a lifetime with it; and a group without delimiters, which
/// a `macro_rules!` fragment leaves, is its contents.
fn collect(tokens: TokenStream2, pieces: &mut Vec<Piece>) {
    let mut joined = String::new();
    for tree in tokens {
        match tree {
            TokenTree::Punct(punct) => {
                joined.push(punct.as_char());
                if punct.spacing() == Spacing::Alone {
                    pieces.push(Piece::Operator(std::mem::take(&mut joined)));
                }
            }
            TokenTree::Ident(ident) => {
                let lifetime = joined.ends_with('\'');
                if lifetime {
                    joined.pop();
                }
                if !joined.is_empty() {
                    pieces.push(Piece::Operator(std::mem::take(&mut joined)));
                }
                let quote = if lifetime { "'" } else { "" };
                pieces.push(Piece::Word(format!("{quote}{ident}")));
            }
            TokenTree::Literal(literal) => {
                if !joined.is_empty() {
                    pieces.push(Piece::Operator(std::mem::take(&mut joined)));
                }
                pieces.push(Piece::Word(literal.to_string()));
            }
            TokenTree::Group(group) => {
                if !joined.is_empty() {
                    pieces.push(Piece::Operator(std::mem::take(&mut joined)));
                }
                let delimiters = match group.delimiter() {
                    Delimiter::Parenthesis => Some(('(', ')')),
                    Delimiter::Bracket => Some(('[', ']')),
                    Delimiter::Brace => Some(('{', '}')),
                    Delimiter::None => None,
                };
                if let Some((open, _)) = delimiters {
                    pieces.push(Piece::Open(open));
                }
                collect(group.stream(), pieces);
                if let Some((_, close)) = delimiters {
                    pieces.push(Piece::Close(close));
                }
            }
        }
    }
    if !joined.is_empty() {
        pieces.push(Piece::Operator(joined));
    }
}

/// Returns whether a space goes between `before` and `after`. Two words
/// need one to stay two tokens, `&'a Name` and `dyn Named`; a separator
/// takes one after it, `Result<A, B>` and `[u8; 4]`, unless it trails; `+`,
/// `->` and `=` stand between two things, `dyn Named + 'a` and `Item = u8`;
/// and a closed list of generic arguments takes one before a word, `<Vec<T>
/// as IntoIterator>`. Nothing else does: `Rc<Account>`, `&mut T`,
/// `core::fmt::Display`, `.base.get_ref()`.
fn spaced(before: &Piece, after: &Piece) -> bool {
    let between = |mark: &str| matches!(mark, "+" | "->" | "=");
    match (before, after) {
        (Piece::Word(_), Piece::Word(_)) => true,
        (Piece::Operator(mark), next) if matches!(mark.as_str(), "," | ";" | ":") => {
            !matches!(next, Piece::Close(_))
                && !matches!(next, Piece::Operator(next) if next.starts_with('>'))
        }
        (Piece::Operator(mark), _) if between(mark) => true,
        (_, Piece::Operator(mark)) if between(mark) => true,
        (Piece::Operator(mark), Piece::Word(_)) => mark.ends_with('>'),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route::Route;
    use proc_macro2::Group;

    fn ty(text: &str) -> String {
        spelled(&syn::parse_str::<syn::Type>(text).expect("a valid type"))
    }

    fn path(text: &str) -> String {
        spelled(&syn::parse_str::<syn::Path>(text).expect("a valid path"))
    }

    fn route(text: &str) -> String {
        spelled(&syn::parse_str::<Route>(text).expect("a valid route"))
    }

    #[test]
    fn two_words_keep_their_space() {
        assert_eq!(ty("&'a Name"), "&'a Name");
        assert_eq!(ty("&'a mut Name"), "&'a mut Name");
        assert_eq!(ty("&mut T"), "&mut T");
        assert_eq!(ty("&'static str"), "&'static str");
        assert_eq!(ty("*const u8"), "*const u8");
        assert_eq!(ty("*mut T"), "*mut T");
        assert_eq!(ty("Option<&'a str>"), "Option<&'a str>");
    }

    #[test]
    fn punctuation_takes_no_space_of_its_own() {
        assert_eq!(ty("Rc<Account>"), "Rc<Account>");
        assert_eq!(ty("&[u8]"), "&[u8]");
        assert_eq!(ty("Vec<Vec<T>>"), "Vec<Vec<T>>");
        assert_eq!(ty("!"), "!");
        assert_eq!(ty("()"), "()");
        assert_eq!(path("core::fmt::Display"), "core::fmt::Display");
        assert_eq!(path("::core::fmt::Display"), "::core::fmt::Display");
    }

    #[test]
    fn a_separator_takes_a_space_after_it_unless_it_trails() {
        assert_eq!(ty("Result<A, B>"), "Result<A, B>");
        assert_eq!(ty("(A, B)"), "(A, B)");
        assert_eq!(ty("(A,)"), "(A,)");
        assert_eq!(ty("[u8; 4]"), "[u8; 4]");
        assert_eq!(path("Trait<'a, T>"), "Trait<'a, T>");
    }

    #[test]
    fn bounds_arrows_and_bindings_stand_between_spaces() {
        assert_eq!(ty("Box<dyn Named + 'a>"), "Box<dyn Named + 'a>");
        assert_eq!(ty("impl Named + Send"), "impl Named + Send");
        assert_eq!(
            ty("dyn for<'a> Fn(&'a u8) -> u8"),
            "dyn for<'a> Fn(&'a u8) -> u8"
        );
        assert_eq!(
            ty("Box<dyn Fn(u8) -> u8 + Send>"),
            "Box<dyn Fn(u8) -> u8 + Send>"
        );
        assert_eq!(ty("fn(u8) -> u8"), "fn(u8) -> u8");
        assert_eq!(path("Iterator<Item = u8>"), "Iterator<Item = u8>");
    }

    #[test]
    fn a_closed_list_of_arguments_takes_a_space_before_a_word() {
        assert_eq!(
            ty("<Vec<T> as IntoIterator>::Item"),
            "<Vec<T> as IntoIterator>::Item"
        );
    }

    #[test]
    fn a_route_is_spelled_as_it_is_written() {
        for text in ["*", ".base", ".0.1", ".base.get_ref()", "*.a*"] {
            assert_eq!(route(text), text, "{text}");
        }
    }

    #[test]
    fn a_group_without_delimiters_is_its_contents() {
        let payload: TokenStream2 = syn::parse_str("&'a Name").expect("valid tokens");
        let group = Group::new(Delimiter::None, payload);
        let mut tokens = TokenStream2::new();
        tokens.extend([TokenTree::Group(group)]);
        assert_eq!(spelled(&tokens), "&'a Name");
    }
}
