//! Why a method cannot be forwarded to a target, and the sentence that
//! says so.
//!
//! The engine judges before it writes: a [`Refusal`] is the reason one
//! target cannot take one method, as a value, and its sentence is worded
//! here, on the payload, with the way around it that the method's path
//! offers. A refused target's arm holds the sentence, under the variant's
//! own `cfg`, so a build without the variant forwards the rest.
use std::fmt;

use syn::{Error, Ident, Signature};

use crate::shape::Pointer;
use crate::spelling;
use crate::subject::{Holder, Target};

/// The ways out a sentence offers a method that cannot be forwarded as it
/// is written: an impl by hand on the trait path, a body on the inherent
/// path, and the body itself where the method has one for the variants it
/// does not name.
#[derive(Clone, Copy)]
pub(crate) struct WayOut {
    /// Whether the method is a trait's, forwarded by the subject's
    /// attribute or in a partial impl block, and not an inherent block's.
    pub(crate) on_trait: bool,
    /// Whether the method names its variants and has a body for the rest.
    pub(crate) has_body: bool,
}

impl WayOut {
    /// The way out a sentence about the whole method offers: where the
    /// method names its variants and has a body for the rest, the body
    /// answers for every variant once the mark is gone.
    pub(crate) fn of_method(self) -> &'static str {
        if self.has_body {
            return "drop its `#[dispatch(..)]`, and the body answers for every variant";
        }
        self.by_hand()
    }

    /// The way out a sentence about `variant` offers: the body, where the
    /// method has one for the variants it does not name.
    pub(crate) fn of_variant(self, variant: &Ident) -> String {
        if self.has_body {
            return format!("leave `{variant}` to the body");
        }
        String::from(self.by_hand())
    }

    /// The way out when the value cannot be built around `variant`'s
    /// payload again.
    fn of_rebuilding(self, variant: &Ident) -> String {
        if self.has_body {
            return format!("leave `{variant}` to the body");
        }
        String::from(if self.on_trait {
            "write the method in a `#[whichever::dispatch] impl` block"
        } else {
            "give it a body instead"
        })
    }

    /// What the method's path offers one that cannot be forwarded.
    fn by_hand(self) -> &'static str {
        if self.on_trait {
            "implement the method by hand"
        } else {
            "give it a body"
        }
    }
}

/// What a method takes that a target cannot give, as the signature has it.
#[derive(Clone, Copy)]
pub(crate) enum Taken<'a> {
    /// `&mut self`.
    MutSelf,
    /// `self: Pin<&mut Self>`.
    PinnedMutSelf,
    /// A `&mut Self` argument.
    MutArgument(&'a Ident),
    /// `self` by value.
    SelfByValue,
    /// `self: Box<Self>`.
    BoxedSelf,
    /// A `Self` argument by value.
    ArgumentByValue(&'a Ident),
}

impl fmt::Display for Taken<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Taken::MutSelf => formatter.write_str("`&mut self`"),
            Taken::PinnedMutSelf => formatter.write_str("`self: Pin<&mut Self>`"),
            Taken::MutArgument(argument) => write!(formatter, "`{argument}: &mut Self`"),
            Taken::SelfByValue => formatter.write_str("`self` by value"),
            Taken::BoxedSelf => formatter.write_str("`self: Box<Self>`"),
            Taken::ArgumentByValue(argument) => write!(formatter, "`{argument}` by value"),
        }
    }
}

/// How a method borrows its target mutably.
#[derive(Clone, Copy)]
pub(crate) enum MutableBorrow<'a> {
    /// By what it takes.
    Takes(Taken<'a>),
    /// By the place it returns as `&mut`.
    Returns,
}

impl fmt::Display for MutableBorrow<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MutableBorrow::Takes(taken) => write!(formatter, "takes {taken}"),
            MutableBorrow::Returns => formatter.write_str("returns a `&mut`"),
        }
    }
}

/// Why a method cannot be forwarded to one target.
#[derive(Clone, Copy)]
pub(crate) enum Refusal<'a> {
    /// The method requires `Self: Sized`, and the target is a trait
    /// object. A hand leaves such a method to the trait's default, and the
    /// partial block named here says so with `#[dispatch(default)]`.
    TraitObject {
        declared: &'a Signature,
        block_header: &'a str,
    },
    /// The method returns `Self`, which cannot be wrapped back through the
    /// target's route.
    RewrapThroughRoute,
    /// The method borrows mutably, and the route ends in a method call:
    /// what a method returns is a value, which cannot be borrowed through;
    /// a field is a place, which can.
    MethodRouteBorrowedMutably(Taken<'a>),
    /// The method borrows mutably, and the route dereferences a pointer
    /// that lends what it points at shared and never exclusively.
    SharedPointerBorrowedMutably(MutableBorrow<'a>),
    /// The method moves its target out, and the route dereferences a
    /// pointer that is no `Box`: a value is moved out of a `Box` by
    /// dereferencing it, and out of nothing else.
    MovedOutOfPointer(Taken<'a>),
    /// The method takes `self: Box<Self>`, and the target is not a
    /// dereferenced `Box` to hand over.
    NoBoxToHandOver,
    /// The method returns `Self`, and the subject cannot be built around
    /// the payload's result: other fields would be missing, or a path has
    /// to build it and none does.
    NotRebuilt,
}

impl Refusal<'_> {
    /// Returns the sentence: on the payload, or for a trait object on the
    /// trait's method, naming the way around it.
    pub(crate) fn error(
        &self,
        method: &Ident,
        target: &Target,
        subject: &Ident,
        way_out: WayOut,
    ) -> Error {
        let Target { payload, route, .. } = *target;
        let ident = target.spoken(subject);
        let spelled = spelling::spelled(payload);
        let message = match self {
            Refusal::TraitObject {
                declared,
                block_header,
            } => {
                return Error::new_spanned(
                    declared,
                    format!("`{method}` requires `Self: Sized`, and `{}` holds a trait object, which is not; leave it to the trait's default with `#[dispatch(default)] fn {method}(..);` in a `#[whichever::dispatch] {block_header}` block, or implement the method by hand", target.ident()),
                );
            }
            Refusal::RewrapThroughRoute if route.is_deref() => {
                format!("`{method}` returns `Self`, which cannot be wrapped back into the dereferenced payload of `{ident}`; drop `deref` on it or {}", way_out.of_variant(ident))
            }
            Refusal::RewrapThroughRoute => {
                format!("`{method}` returns `Self`, which cannot be wrapped back through the route `{}` of `{ident}`, whose other fields are gone; drop `via` on it or {}", route.spelled(), way_out.of_variant(ident))
            }
            Refusal::MethodRouteBorrowedMutably(taken) => {
                format!("`{method}` takes {taken}, and the route `{}` of `{ident}` ends in a method call, whose result cannot be borrowed mutably; route to a field, or {}", route.spelled(), way_out.of_variant(ident))
            }
            Refusal::SharedPointerBorrowedMutably(does) => {
                format!("`{method}` {does}, and `{ident}` dereferences a `{spelled}`, which cannot be borrowed mutably; hold a `Box` or a `&mut`, or {}", way_out.of_variant(ident))
            }
            Refusal::MovedOutOfPointer(taken) => {
                format!("`{method}` takes {taken}, and `{ident}` dereferences a `{spelled}`, which a value cannot be moved out of; hold a `Box`, or {}", way_out.of_variant(ident))
            }
            // A `deref` variant over another pointer has no box to hand
            // over, and boxing the pointer would not make one of what it
            // points at.
            Refusal::NoBoxToHandOver if route.starts_with_deref() => {
                format!("`{method}` takes `self: Box<Self>`, and `{ident}` dereferences a `{spelled}`, which is no `Box` to hand over; hold a `Box`, or {}", way_out.of_variant(ident))
            }
            Refusal::NoBoxToHandOver => {
                // A payload that already is a `Box` only lacks `deref`.
                let advice = if Pointer::of(payload) == Pointer::Box {
                    // The variant by its name; a struct's one field is
                    // what the span points at.
                    let to = match target.place.holder {
                        Holder::Variant(_) => format!("`{ident}`"),
                        Holder::Struct => String::from("the field"),
                    };
                    format!("add `#[dispatch(deref)]` to {to}")
                } else {
                    let noun = target.noun();
                    format!("hold `Box<{spelled}>` in a `#[dispatch(deref)]` {noun}")
                };
                format!("`{method}` takes `self: Box<Self>`, and `{ident}` holds a plain `{spelled}`; {advice}, or {}", way_out.of_variant(ident))
            }
            // A struct whose one field is named is built around a value by
            // its braces and by no path, and a `Self` under an `Option` or
            // a `Result` is mapped with a path: the closure a hand writes
            // there is what the attribute does not.
            Refusal::NotRebuilt if target.is_built_by_no_path() => {
                format!("`{method}` returns `Self` inside an `Option` or a `Result`, where a path rebuilds the value, and `{subject}` has a named field, which no path builds; make it a tuple struct, `struct {subject}({spelled});`, or {}", way_out.of_rebuilding(ident))
            }
            Refusal::NotRebuilt => {
                format!("`{method}` returns `Self`, which cannot be built around the other fields of `{ident}`; {}", way_out.of_rebuilding(ident))
            }
        };
        Error::new_spanned(payload, message)
    }
}
