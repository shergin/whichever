//! Procedural macros of `whichever`.
//!
//! `#[dispatchable]` on a trait re-emits the trait and exports its carrier
//! macro, a hidden `macro_rules!` that carries the trait's tokens; the macro
//! is re-exported under the trait's own name, so importing the trait imports
//! the macro. `#[dispatch(Trait)]` on an enum invokes that macro, which
//! forwards the trait tokens, the enum tokens and the options into the
//! implementation macro, `implement!`, which writes `impl Trait for Enum`
//! with one `match` per method. No state is shared between expansions, so
//! the trait and the enum may live in different crates.
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Error, Item, ItemTrait};

use crate::block::{expand_inherent, expand_partial};
use crate::dispatch::{expand_block, expand_subject};
use crate::dispatchable::expand_dispatchable;
use crate::implement::expand_implement;
use crate::options::DispatchableOptions;
use crate::protocol::{BlockInput, ImplementInput};
use crate::subject::Subject;

/// Returns the span a generated token carries: located at `span`, a token
/// the user wrote, so that an error rustc finds in the generated code lands
/// there, and resolved at the expansion's own site, so that lints treat the
/// token as the expansion's and not as the user's.
pub(crate) fn at(span: Span) -> Span {
    Span::mixed_site().located_at(span)
}

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
mod block;
mod conversions;
mod dispatch;
mod dispatchable;
mod forwarding;
mod implement;
#[cfg(test)]
mod limits;
mod naming;
mod options;
mod protocol;
mod route;
mod shape;
#[cfg(test)]
mod spans;
mod subject;
mod substitution;

/// Makes a trait available to `#[dispatch]`, from this crate or any other.
///
/// The trait is emitted unchanged, except that `#[dispatch(..)]` attributes
/// on its methods are consumed. The only method option is `mismatch`, which
/// gives the result of a method taking `Self` arguments when the arguments
/// hold a different variant than `self`:
///
/// ```rust,ignore
/// #[whichever::dispatchable]
/// pub trait Shape {
///     fn area(&self) -> u32;
///     #[dispatch(mismatch = false)]
///     fn congruent(&self, other: &Self) -> bool;
/// }
/// ```
///
/// Options:
/// - `remote = path` declares a trait of another crate: the item is a
///   restatement of that trait's signatures, nothing is emitted for it but
///   the carrier macro, and `#[dispatch]` of the declaration implements the
///   trait at `path`. Only the methods restated are forwarded.
/// - `name = ident` names the carrier macro after `ident` instead of the
///   trait's text and position, for a trait that one `macro_rules!` emits
///   several times with identical tokens.
///
/// ```rust,ignore
/// #[whichever::dispatchable(remote = std::io::Write)]
/// trait Write {
///     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
///     fn flush(&mut self) -> std::io::Result<()>;
///     fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
/// }
/// ```
///
/// The trait's signature types are later written into the dispatching
/// enum's module, so they must be nameable there; `crate::` paths are
/// rewritten to this crate, `super::` and `self::` paths are not and must
/// not be used.
#[proc_macro_attribute]
pub fn dispatchable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let options = parse_macro_input!(attr as DispatchableOptions);
    let item_trait = parse_macro_input!(item as ItemTrait);
    let site = proc_macro::Span::call_site();
    expand_dispatchable(item_trait, options, site.line(), site.column())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Implements a `#[dispatchable]` trait for an enum by forwarding every
/// method to whichever variant the value holds, or forwards the methods of
/// an inherent `impl` block of such an enum.
///
/// On an enum, each attribute names one trait, with its generic arguments
/// when it has any, followed by options; several attributes dispatch several
/// traits, and a bare `#[dispatch]` only registers the enum. Every variant
/// must hold exactly one unnamed field whose type implements the trait. The
/// enum also gets a `From<Payload>` impl for every payload type that appears
/// in exactly one variant.
///
/// ```rust,ignore
/// #[whichever::dispatch(Shape)]
/// #[whichever::dispatch(Encoder<Json>, Error = EncodeError)]
/// pub enum AnyShape {
///     Circle(Circle),
///     #[dispatch(deref)]
///     Square(Box<Square>),
/// }
/// ```
///
/// Options, after the trait path:
/// - `Name = Type`: the associated type `Name` of the enum's impl, instead
///   of the first variant's. Methods returning `Self::Name` are converted
///   with `Into`, methods returning `Result<_, Self::Name>` with
///   `map_err(Into::into)`.
/// - `mismatch(method = expr, ..)`: the result of `method` when its `Self`
///   arguments hold a different variant than `self`.
/// - `from = false`: no `From<Payload>` impls.
/// - `try_from = true`: a `TryFrom<Enum> for Payload` impl per payload type,
///   returning the enum unchanged when it holds another variant.
///
/// Three options go on a variant. `#[dispatch(deref)]` calls the trait on what
/// the payload dereferences to, for `Box<T>`, `Rc<T>`, `Arc<T>`, `&T` and
/// `&mut T` payloads; such a variant cannot take part in a method that
/// returns `Self`. `#[dispatch(from = false)]` leaves the variant out of the
/// conversions, and `#[dispatch(try_from = true)]` adds the `TryFrom` impl
/// for that payload alone.
///
/// On a struct, the trait is forwarded to its one field, or to the field
/// marked `#[dispatch]`, with `#[dispatch(deref)]` for a field behind a
/// pointer. A struct gets no conversions, needs no `mismatch`, and forwards
/// a return-position `impl Trait` as it is, since its `match` has one arm.
///
/// On an inherent `impl` block of a dispatch enum, in the same crate, every
/// method carrying `#[dispatch(..)]` is forwarded to the named variants'
/// own method of the same name, and the body is what the other variants
/// return. `*` names every variant; the method then has no fallback and is
/// written without a body:
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
///     pub fn perimeter(&self) -> f64;
/// }
/// ```
///
/// Method options: `with = path` wraps each forwarded result in a call to
/// `path`, and `call = name` forwards to a payload method of another name.
/// Methods without the attribute are left as they are.
///
/// On a trait impl block of a dispatch enum, in the same crate, the block
/// holds the items the enum implements itself, a generic associated type
/// among them, and the rest of the trait is forwarded around them. The enum
/// then does not name that trait in its own attribute; the trait's options
/// go on the block's attribute:
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
    let input = parse_macro_input!(input as BlockInput);
    let expanded = match input.partial {
        Some(spec) => expand_partial(
            input.rejected,
            &input.facade,
            &input.dispatched,
            input.remote.as_ref(),
            input.subject,
            spec,
            input.item_impl,
        ),
        None => expand_inherent(
            input.rejected,
            input.remote.as_ref(),
            input.subject,
            input.item_impl,
        ),
    };
    expanded
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}
