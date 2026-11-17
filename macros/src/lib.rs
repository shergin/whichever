//! Procedural macros of `whichever`.
//!
//! This crate is an implementation detail of `whichever`. Depend on
//! `whichever` and use the two attributes from there, where they are
//! documented: `whichever::dispatchable` and `whichever::dispatch`.
//!
//! How the pieces fit together:
//!
//! - `#[dispatchable]` on a trait emits the trait, and next to it the
//!   carrier macro: a hidden `macro_rules!` that holds the trait's
//!   signatures. The carrier macro is re-exported under the trait's own
//!   name, so importing the trait imports the macro too.
//! - `#[dispatch(Trait)]` on an enum calls the trait's carrier macro. The
//!   carrier macro hands the trait's tokens, the enum's tokens and the
//!   options to the hidden `implement!` macro, which writes
//!   `impl Trait for Enum` with one `match` per method.
//! - `#[dispatch]` on an enum or a struct also leaves a crate-local carrier
//!   macro that holds the item's definition. `#[dispatch]` on an impl block
//!   of the item calls it, and it hands the block and the definition to the
//!   hidden `implement_block!` macro, which rewrites the block.
//!
//! Nothing is kept between expansions. Everything an expansion needs
//! travels in the tokens of these macros, so the trait and the enum may
//! live in different crates.
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Error, Item, ItemTrait};

use crate::block::{expand_inherent, expand_partial};
use crate::dispatch::{expand_block, expand_subject};
use crate::dispatchable::expand_dispatchable;
use crate::implement::expand_implement;
use crate::options::DispatchableOptions;
use crate::protocol::{BlockInput, ImplementInput};
use crate::subject::Subject;

/// Folds `errors` into the first, so that all of them are reported at once.
pub(crate) fn combined(errors: Vec<Error>) -> Option<Error> {
    let mut errors = errors.into_iter();
    let mut first = errors.next()?;
    for error in errors {
        first.combine(error);
    }
    Some(first)
}

mod agreement;
mod arm;
mod block;
mod builds;
mod conversions;
mod dispatch;
mod dispatchable;
mod forwarding;
mod implement;
mod items;
#[cfg(test)]
mod limits;
mod marked;
mod naming;
mod options;
mod partial;
mod protocol;
mod refusal;
mod route;
mod selection;
mod shape;
mod signatures;
mod site;
#[cfg(test)]
mod spans;
mod spelling;
mod subject;
mod substitution;
mod words;

/// Makes a trait available to `#[dispatch]`, from this crate or any other.
///
/// The trait is emitted as you wrote it. The one change is that the
/// `#[dispatch(..)]` attributes on its methods are consumed. Next to the
/// trait, the attribute leaves a hidden carrier macro that holds the
/// trait's signatures. `#[dispatch]` on an enum reads the trait from the
/// carrier macro, so the enum may be in another crate, and nothing is
/// cached between expansions. The carrier macro is re-exported under the
/// trait's name, so importing the trait imports it too.
///
/// ```rust,ignore
/// #[whichever::dispatchable]
/// pub trait Shape {
///     fn area(&self) -> u32;
/// }
///
/// #[whichever::dispatch(Shape)]
/// pub enum AnyShape {
///     Circle(Circle),
///     Square(Square),
/// }
/// ```
///
/// # On a trait method
///
/// `#[dispatch(mismatch = expr)]` says what the method returns when `self`
/// and a `Self` argument hold different variants:
///
/// ```rust,ignore
/// #[whichever::dispatchable]
/// pub trait Shape {
///     /// Two different kinds of shape are never congruent.
///     #[dispatch(mismatch = false)]
///     fn congruent(&self, other: &Self) -> bool;
/// }
/// ```
///
/// An enum can decide it instead, with `mismatch(congruent = ..)` on its
/// own attribute. A method with a `Self` argument that has neither is an
/// error, unless the enum has only one variant.
///
/// `#[inline(..)]` on a method without a body says how its forwarding
/// methods are inlined. The carrier macro keeps it for them, and the trait's
/// own method is emitted without it, since rustc rejects `#[inline]` on a
/// method that has no body. One written under `cfg_attr` is not carried.
///
/// # Options
///
/// - `remote = path`: the item is a restatement of another crate's trait,
///   the one at `path`. Nothing is emitted for it but the carrier macro.
///   `#[dispatch]` of the restatement implements the real trait at `path`,
///   and forwards exactly the methods restated. A method left out keeps
///   the real trait's default.
/// - `name = ident`: names the carrier macro after `ident`, instead of
///   after the trait's tokens and their positions. Use it when one
///   `macro_rules!` emits the same trait several times.
///
/// ```rust,ignore
/// #[whichever::dispatchable(remote = std::io::Write)]
/// trait Write {
///     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
///     fn flush(&mut self) -> std::io::Result<()>;
///     fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
/// }
///
/// #[whichever::dispatch(Write)] // implements std::io::Write
/// pub enum Stream {
///     Tcp(TcpStream),
///     Buffer(Vec<u8>),
/// }
/// ```
///
/// ```rust,ignore
/// macro_rules! runtime {
///     ($module:ident) => {
///         pub mod $module {
///             #[whichever::dispatchable(name = $module)]
///             pub trait Runtime {
///                 fn spawn(&self) -> u32;
///             }
///         }
///     };
/// }
///
/// runtime!(tokio);
/// runtime!(smol);
/// ```
///
/// # Notes
///
/// - The types in the trait's signatures are written into the enum's
///   module, so they must be in scope and nameable there, as for a
///   hand-written impl. Paths that start with `crate::` are rewritten to
///   the trait's crate. `super::` and `self::` paths are not, so do not
///   use them.
/// - With `#[async_trait]`, put `#[dispatchable]` below it, so that it
///   reads the signatures after `#[async_trait]` has rewritten them.
/// - With `#[typetag::serde]`, put `#[dispatchable]` above it, so that
///   `typetag`'s own methods are never forwarded. Then
///   `#[typetag::serde(name = "..")]` goes on the enum's partial impl
///   block.
/// - The other order gives you an error that says where the attribute
///   goes.
///
/// The crate-level documentation of `whichever` explains all of this in
/// full, with runnable examples: <https://docs.rs/whichever>.
#[proc_macro_attribute]
pub fn dispatchable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let options = parse_macro_input!(attr as DispatchableOptions);
    let item_trait = parse_macro_input!(item as ItemTrait);
    let site = proc_macro::Span::call_site();
    expand_dispatchable(item_trait, options, site.line(), site.column())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Implements a `#[dispatchable]` trait for an enum by forwarding each
/// method to whichever variant the value holds.
///
/// The same attribute also goes on a struct and on the enum's impl blocks.
///
/// # Where it goes
///
/// ## On an enum
///
/// ```rust,ignore
/// #[whichever::dispatch(Shape)]
/// #[whichever::dispatch(Named)]
/// pub enum AnyShape {
///     Circle(Circle),
///     #[dispatch(deref)]
///     Boxed(Box<Square>),
/// }
/// ```
///
/// Each attribute lists one trait, with its generic arguments if it has
/// any (`Encoder<Json>`), and then options. Repeat the attribute for each
/// trait, a supertrait included. Every variant holds one unnamed field,
/// the payload, or marks the field to forward to with `#[dispatch]`. The
/// payload, or what a route reaches from it, implements the trait.
///
/// A bare `#[dispatch]`, with no trait, forwards nothing. It registers the
/// enum for its inherent blocks and partial impl blocks.
///
/// `#[dispatch(remote = path)]`, as an attribute of its own, makes the
/// enum a restatement of another crate's enum at `path`. The impls are
/// written for the real enum, with no conversions. A struct is restated
/// the same way; it lists only the fields its impls forward to, and the
/// pattern ends in `..`.
///
/// ## On a struct
///
/// ```rust,ignore
/// #[whichever::dispatch(Shape)]
/// pub struct Labelled {
///     #[dispatch]
///     shape: Circle,
///     label: String,
/// }
/// ```
///
/// The trait is forwarded to the struct's one field, or to the field
/// marked `#[dispatch]`. The field may be any type that implements the
/// trait. A struct gets no `From` impls and needs no `mismatch`, and it
/// forwards a method that returns `impl Trait` as it is, because its
/// `match` has one arm. What needs no value comes from the field's type,
/// where the struct names it: a trait method without a receiver,
/// `<Square as Shape>::kind()`, a generic associated type, and an inherent
/// method without a receiver marked in a block.
///
/// ## On an inherent block
///
/// ```rust,ignore
/// #[whichever::dispatch]
/// impl AnyShape {
///     /// Only circles have a radius.
///     #[dispatch(Circle, with = Some)]
///     pub fn radius(&self) -> Option<f64> {
///         None
///     }
///
///     #[dispatch(*)]
///     pub fn describe(&self) -> String;
/// }
/// ```
///
/// A method marked `#[dispatch(..)]` is forwarded to the method of the
/// same name on the listed variants' payloads. Its body is the fallback:
/// what every other variant returns. `*` lists every variant, so
/// `describe` has no body. Methods without the attribute are left alone.
/// The block must be in the enum's crate, and the enum needs `#[dispatch]`
/// on it.
///
/// ## On a trait impl block
///
/// ```rust,ignore
/// #[whichever::dispatch]
/// pub enum Handler {
///     Text(TextHandler),
///     Map(MapHandler),
/// }
///
/// #[whichever::dispatch]
/// impl HandlerTrait for Handler {
///     fn from_handler(handler: Handler) -> Option<Self> {
///         Some(handler)
///     }
/// }
/// ```
///
/// This is a partial impl block. You write the items the enum handles
/// itself, and the rest of the trait is forwarded around them.
///
/// - The enum does not list the trait in its own attribute: the block is
///   the impl. The trait's options go on the block's attribute.
/// - An associated type written in the block is an override. An enum's
///   generic associated type is declared here.
/// - The block's header, generics and bounds included, becomes the impl's
///   header. An empty block gives the impl a header the enum's attribute
///   cannot write.
/// - The block must be in the enum's crate. A macro invocation in the
///   block is an error, because its items cannot be told apart from the
///   ones to forward.
///
/// # Options
///
/// ## On the enum, after the trait path
///
/// | Option | What it does |
/// | --- | --- |
/// | `Name = Type` | Fixes the associated type `Name`; results are converted into it. |
/// | `Name = value` | Fixes the associated const `Name`, instead of the first variant's. |
/// | `mismatch(method = expr, ..)` | What `method` returns when `self` and a `Self` argument hold different variants. |
/// | `from = false` | No `From<Payload>` impls. |
/// | `try_from = true` | Adds `TryFrom<Enum> for Payload`; the error is the enum itself. |
/// | `try_from = ref` | Also adds `TryFrom<&Enum> for &Payload`. |
/// | `try_from = ref_mut` | Also adds `TryFrom<&mut Enum> for &mut Payload`. |
/// | `into = Target` | Adds `From<Enum> for Target`, through each payload's `Into<Target>`. |
/// | `via(Variant = route, ..)` | A route through `Variant`, for this trait only; it replaces both of the variant's routes. |
/// | `via_mut(Variant = route, ..)` | The route through `Variant` for the methods that borrow the target mutably, for this trait only. |
///
/// The conversion options (`from`, `try_from`, `into`) belong to the enum,
/// not to one trait, so they apply whichever attribute they are on. The
/// other options belong to the trait they follow: on an attribute that
/// lists no trait, they are an error. They go on a partial impl block's
/// attribute when the block is the impl.
///
/// ## On a variant
///
/// | Option | What it does |
/// | --- | --- |
/// | `from = false` | Leaves the variant out of `From` and `TryFrom`. |
/// | `try_from = true` | Adds `TryFrom` for this variant's payload alone. |
/// | `deref` | Calls the trait on what the payload dereferences to (`Box`, `Rc`, `Arc`, `&`, `&mut`). |
/// | `via = route` | Calls the trait on what the route reaches: `.field`, `.0`, `.method()`, `*`, chained. |
/// | `via_mut = route` | The route the methods that borrow the target mutably take instead: `&mut self`, `self: Pin<&mut Self>`, a `&mut Self` argument. |
///
/// A variant takes `deref` or `via`, not both. A variant with several
/// fields, or with one named field, marks the field to forward to:
/// `Tagged(#[dispatch] Square, u32)`. The marked field takes `deref`,
/// `via = route` or `via_mut = route`, and nothing else.
///
/// ## On a struct's field
///
/// | Option | What it does |
/// | --- | --- |
/// | `#[dispatch]` | The field that takes every trait no other field lists. |
/// | `#[dispatch(Trait)]` | The field this trait goes to, spelled as the trait's impl spells it. |
/// | `#[dispatch(deref)]` | Calls the trait on what the field dereferences to. |
/// | `#[dispatch(via = route)]` | Calls the trait on what the route reaches from the field. |
/// | `#[dispatch(via_mut = route)]` | The route the methods that borrow the target mutably take instead. |
///
/// ## On a method of an inherent block
///
/// | Option | What it does |
/// | --- | --- |
/// | `Variant, ..` | Forwards to these variants; the body is what the others return. |
/// | `*` | Forwards to every variant; the method has no body. |
/// | `*` with a body | The body is one `match self` with the enum's own arms; every other variant is forwarded. |
/// | `with = path` | Wraps each forwarded result: `path(inner.method(args))`. |
/// | `call = name` | Calls the payload's method `name` instead. |
/// | `field = name` | The arm is the field `inner.name`, borrowed or read. |
/// | `value` | The arm is the target itself, borrowed or read. |
/// | `via = route` | Reaches the target by a route, for this method alone, whatever it borrows. |
///
/// On a struct, a field's name takes the place of the variant list, and
/// `*` is the field marked `#[dispatch]`, or the only field.
///
/// ## On a method of a partial impl block
///
/// | Form | What it does |
/// | --- | --- |
/// | no attribute, with a body | Your method; it replaces the forwarding for every variant. |
/// | `Variant, ..` with a body | Forwards to these variants; the body is the arm for the others. |
/// | `*` without a body | The plain forwarding, written out. |
/// | `*` with a body | The body is one `match self` with the enum's own arms; every other variant is forwarded. |
/// | `default` without a body | Leaves the method to the trait's default. |
///
/// # What you get
///
/// - The forwarding impl you would write by hand: one `match` per method,
///   one fully qualified call per arm, such as
///   `<Circle as Shape>::area(inner)`, and `#[inline]` on every method.
/// - On an enum, `From<Payload>` for every payload type that exactly one
///   variant holds.
///
/// The crate-level documentation of `whichever` explains every form and
/// option in full, with runnable examples: <https://docs.rs/whichever>.
#[proc_macro_attribute]
pub fn dispatch(attr: TokenStream, item: TokenStream) -> TokenStream {
    let site = proc_macro::Span::call_site();
    let (line, column) = (site.line(), site.column());
    let item = parse_macro_input!(item as Item);
    let expanded = match item {
        Item::Enum(item_enum) => {
            expand_subject(attr.into(), Subject::Enum(item_enum), line, column)
        }
        Item::Struct(item_struct) => {
            expand_subject(attr.into(), Subject::Struct(item_struct), line, column)
        }
        Item::Impl(item_impl) => expand_block(attr.into(), item_impl),
        other => {
            let error = Error::new_spanned(
                &other,
                "`#[dispatch]` goes on an enum or a struct, or on an impl block of one",
            )
            .to_compile_error();
            quote! { #other #error }
        }
    };
    expanded.into()
}

/// Writes the trait impl; called by the macro that `#[dispatchable]` exports.
#[doc(hidden)]
#[proc_macro]
pub fn implement(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ImplementInput);
    expand_implement(input)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Rewrites an impl block of a dispatch enum or struct; called by the macro
/// that `#[dispatch]` defines for the item.
#[doc(hidden)]
#[proc_macro]
pub fn implement_block(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as BlockInput);
    let expanded = match input.partial.take() {
        Some(options) => expand_partial(input, options),
        None => expand_inherent(input),
    };
    expanded
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}
