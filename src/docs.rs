//! The long-form pages of the documentation.
//!
//! These modules hold no items. Each one is a page that ships with the
//! crate, so that it reads the same here as the version you depend on:
//!
//! - [`tutorial`] is a tour of the crate, one feature at a time.
//! - [`glossary`] has the words the documentation uses, one per thing.
//! - [`troubleshooting`] goes from a compiler error to its cause and the
//!   fix.
//!
//! The reference for every option and limit is the
//! [crate documentation](crate).
//!
//! The modules exist only while rustdoc builds the documentation, so
//! nothing in your code can name them.

// Each page is a Markdown file under `docs/`, written for GitHub: its links
// are reference-style, and the definitions at the foot of the file point at
// the other pages by relative path. Those paths do not exist in the crate
// docs. The definitions written here come first, and Markdown keeps the
// first definition of a label, so in the crate docs every label goes to the
// matching section of the crate documentation, to another of these modules,
// or to the repository. A label a page uses must have a line here;
// `scripts/check_doc_links.py` fails on one that does not.

/// A guided tour of the crate, one feature at a time.
///
/// [comparison]: https://github.com/shergin/whichever/blob/main/docs/comparison.md
/// [compatibility]: crate#across-crates
/// [compatibility-footprint]: crate#footprint
/// [glossary]: crate::docs::glossary
/// [glossary-carrier-macro]: crate::docs::glossary#carrier-macro
/// [inherent-methods]: crate#inherent-methods
/// [limits]: crate#requirements-and-limits
/// [moving-from-enum-dispatch]: https://github.com/shergin/whichever/blob/main/docs/moving-from-enum-dispatch.md
/// [options]: crate#options
/// [partial-impl-blocks]: crate#partial-impl-blocks-write-some-methods-yourself
/// [restatements]: crate#traits-from-std-and-other-crates
/// [routes]: crate#reaching-the-target-pointers-fields-and-methods
/// [structs]: crate#forwarding-from-a-struct-field
/// [tour-rs]: https://github.com/shergin/whichever/blob/main/examples/tour.rs
/// [troubleshooting]: crate::docs::troubleshooting
/// [what-is-generated]: crate#what-is-generated
/// [what-is-generated-items]: crate#what-each-trait-item-becomes
/// [when-to-use-enum-dispatch]: https://github.com/shergin/whichever/blob/main/docs/when-to-use-enum-dispatch.md
///
#[doc = include_str!("../docs/tutorial.md")]
pub mod tutorial {}

/// The words the documentation uses, one per thing.
///
/// [compatibility]: crate#across-crates
/// [inherent-methods]: crate#inherent-methods
/// [limits]: crate#requirements-and-limits
/// [options]: crate#options
/// [partial-impl-blocks]: crate#partial-impl-blocks-write-some-methods-yourself
/// [partial-impl-blocks-mark]: crate#write-a-method-or-mark-it
/// [restatements]: crate#traits-from-std-and-other-crates
/// [routes]: crate#reaching-the-target-pointers-fields-and-methods
/// [structs]: crate#forwarding-from-a-struct-field
/// [tutorial]: crate::docs::tutorial
/// [what-is-generated]: crate#what-is-generated
/// [what-is-generated-consts]: crate#associated-types-and-consts
///
#[doc = include_str!("../docs/glossary.md")]
pub mod glossary {}

/// From a compiler error to its cause and the fix.
///
/// [compatibility-renamed-dependency]: crate#footprint
/// [compatibility-versions]: crate#across-crates
/// [glossary-agreement-check]: crate::docs::glossary#agreement-check
/// [glossary-carrier-macro]: crate::docs::glossary#carrier-macro
/// [glossary-restatement]: crate::docs::glossary#restatement
/// [glossary-target]: crate::docs::glossary#target
/// [inherent-methods]: crate#inherent-methods
/// [limits]: crate#requirements-and-limits
/// [limits-conversions]: crate#conversions
/// [limits-receivers]: crate#receivers
/// [options-enum-inside-enum]: crate#conversions
/// [options-from-false]: crate#conversions
/// [options-mismatch]: crate#self-arguments-and-mismatch
/// [options-name-type]: crate#associated-types-and-consts
/// [options-name-value]: crate#associated-types-and-consts
/// [partial-impl-blocks]: crate#partial-impl-blocks-write-some-methods-yourself
/// [partial-impl-blocks-header]: crate#the-impl-header
/// [partial-impl-blocks-names]: crate#names-attributes-and-what-the-block-cannot-hold
/// [restatements]: crate#traits-from-std-and-other-crates
/// [restatements-any-trait]: crate#restate-any-other-trait
/// [restatements-boxed-iterators]: crate::foreign#self-sized-and-boxed-iterators
/// [restatements-enums]: crate#enums-from-other-crates
/// [restatements-provided-methods]: crate#restate-any-other-trait
/// [routes]: crate#reaching-the-target-pointers-fields-and-methods
/// [structs-cannot-be-asked]: crate#forwarding-from-a-struct-field
/// [structs-some-builds]: crate#forwarding-from-a-struct-field
/// [routes-mutable-borrow]: crate#where-you-write-a-route
/// [what-is-generated]: crate#what-is-generated
///
#[doc = include_str!("../docs/troubleshooting.md")]
pub mod troubleshooting {}
