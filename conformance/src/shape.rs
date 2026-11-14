//! The shape check: the expansion performs the operations the hand
//! performs, arm by arm and in the same order, and nothing else.
//!
//! Token equality with the hand would pin spelling that carries no
//! meaning: `&**inner` against `inner`, `Self::A` against `Enum::A`,
//! `::core::convert::Into::into(x)` against `x.into()`. The type checker
//! and the law already hold the borrows and the paths to account. What
//! neither sees is an operation that type-checks and behaves and that a
//! careful hand would not write: a clone, a boxing, a closure, a
//! conversion nobody asked for, an inherent method called where the
//! trait's was meant. So a method body is read as its operations, in
//! evaluation order, with spelling erased, and compared with the hand's.
//!
//! # Operations
//!
//! A method body is one `match`, possibly inside `async move { .. }`,
//! `Box::pin(async move { .. })` or `unsafe { .. }`. Its shape is the
//! operations around the `match`, then per arm the variant the pattern
//! names and the operations of the arm's value. An operation is a short
//! canonical string:
//!
//! | Source | Operation |
//! | --- | --- |
//! | `<X as Trait>::method(..)`, for any `X`, `_` included | `call Trait::method` |
//! | `receiver.name(..)` | `method name` |
//! | `path::to::function(..)` | `fn function`, by the last segment, or the last two when the one before it is a type: `fn Pin::new`, `fn Box::pin` |
//! | `Enum::Variant(..)`, `Self::Variant { .. }`, `Struct(..)`, and the same as a path handed to `map` | `construct Variant` |
//! | `Into::into(x)`, `x.into()`, and `Into::into` as a path handed to `map` or `map_err` | `into` |
//! | `.map(..)`, `.map_err(..)` | `map`, `map_err`, followed by the operation of the path they are handed |
//! | `Some(..)`, `Ok(..)`, `Err(..)` | `some`, `ok`, `err` |
//! | `.await` | `await` |
//! | `unsafe { .. }` | `unsafe` |
//! | `async { .. }`, `async move { .. }` | `async` |
//! | `expr.field`, `expr.0` | `field name` |
//! | a closure | `closure`, which is also a violation |
//! | a macro invocation | `macro name` |
//! | a nested `match` | `match`, followed by its arms' operations |
//!
//! References, dereferences, parentheses, blocks around one expression,
//! `let` bindings of a tuple taken apart, literals and plain paths read as
//! values leave no operation. Leading `::core::..` and `::std::..` segments
//! are dropped before a path is read.
//!
//! A pattern names its variant by the last segment of its path: `A` for
//! `Enum::A(inner)`, `Self::A { field: inner, .. }` and `Enum::A(ref mut
//! inner, ..)`; `A A` for the tuple `(Enum::A(x), Enum::A(y))`; `_` for a
//! wildcard; the struct's name for a struct pattern.
//!
//! # Violations
//!
//! Independent of the hand, a generated body may not hold a closure, may
//! not name a path that starts with `std` or `alloc`, and may name a path
//! with a leading `::` only when it starts `::core::`.

use syn::{Block, File, ImplItemFn, Item, ItemImpl};

/// The operations of one arm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmShape {
    /// The variant the pattern names, as the module docs spell it.
    pub variant: String,
    /// The operations of the arm's value, in evaluation order.
    pub operations: Vec<String>,
}

/// The shape of one method body.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MethodShape {
    /// The operations around the `match`, outermost first.
    pub around: Vec<String>,
    /// The arms, in the order written; empty when the body is no `match`.
    pub arms: Vec<ArmShape>,
    /// The operations of a body that is no `match`: a struct's one arm is
    /// often the bare call.
    pub bare: Vec<String>,
    /// What the body holds that no generated body may.
    pub violations: Vec<String>,
}

/// Reads the shape of a method body.
pub fn method_shape(body: &Block) -> MethodShape {
    let _ = body;
    todo!("the reading of a method body")
}

/// Returns how the subject's method differs from the hand's, one line per
/// difference, each naming the arm; empty when the two perform the same
/// operations. The subject's own violations are reported first.
pub fn differences(subject: &MethodShape, hand: &MethodShape) -> Vec<String> {
    let _ = (subject, hand);
    todo!("the comparison of two shapes")
}

/// Returns the items of the module at `path` below the file's root, or
/// `None` when a segment names no inline module.
pub fn module<'a>(file: &'a File, path: &[&str]) -> Option<&'a [Item]> {
    let _ = (file, path);
    todo!("the walk to a module")
}

/// Returns the impl blocks among `items` whose self type's last segment is
/// `self_type`: with `trait_name`, the impls of the trait whose path ends
/// in it; without, the inherent impls.
pub fn impls<'a>(
    items: &'a [Item],
    self_type: &str,
    trait_name: Option<&str>,
) -> Vec<&'a ItemImpl> {
    let _ = (items, self_type, trait_name);
    todo!("the search for impl blocks")
}

/// Returns the method named `name` of the first of `blocks` that has one.
pub fn method<'a>(blocks: &[&'a ItemImpl], name: &str) -> Option<&'a ImplItemFn> {
    let _ = (blocks, name);
    todo!("the search for a method")
}
