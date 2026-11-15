//! The catalogue of the macro's sentences the model predicts, and where
//! each is expected.

/// A sentence of the macro the model predicts, one per limit of
/// `docs/limits.md` it reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Limit {
    /// A receiver that would need a refcount or an allocation.
    Receiver,
    /// A unit variant, which holds no payload.
    UnitVariant,
    /// A variant of several unmarked fields.
    TwoFields,
    /// A `Self` argument with no mismatch on a enum of several variants.
    SelfArgumentWithoutMismatch,
    /// A return-position `impl Trait` other than a future.
    OpaqueReturn,
    /// `Self` where no rewrap is a path: in a `Vec`, in a tuple under an
    /// `Option`.
    SelfInReturn,
    /// A `Self` return on a `deref` variant.
    DerefSelfReturn,
    /// A `Self` return on a variant reached through a field or a method.
    RouteSelfReturn,
    /// A `Self` return on the marked field of a variant with other fields.
    SelfReturnOtherFields,
    /// `self: Box<Self>` on a variant that holds no `Box` under `deref`.
    BoxSelfPlain,
    /// `Self` inside an `Option` or a `Result` through a struct with one
    /// named field, which no path builds.
    NamedFieldUnderWrapper,
    /// `self: Box<Self>` on a `deref` variant over a pointer that is no
    /// `Box`.
    BoxSelfOnSharedPointer,
    /// A value moved out of a `deref` variant holding anything but a
    /// `Box`.
    MovesThroughSharedPointer,
    /// A mutable borrow through a `deref` variant over an `Rc`, an `Arc`
    /// or a `&T`.
    MutableBorrowThroughSharedPointer,
    /// A `mismatch` on the enum for a method a `#[cfg]` compiled out.
    MismatchForAbsentMethod,

    /// A `&mut self` method on a route ending in a method call.
    MethodRouteOnMutSelf,
    /// An associated type taken from the first variant, where no variant's
    /// target has a type to take it from.
    UnsuppliedType,
    /// `#[dispatch(default)]` on a method the trait gives no default body.
    DefaultWithoutDefault,
    /// A body on a method whose mark names every variant.
    BodyNeverRuns,
    /// `#[dispatch(*)]` over a variant that holds no payload.
    StarOverUnforwardable,
    /// A conversion option on a struct.
    StructConversion,
    /// `into` beside a variant with other fields.
    IntoOtherFields,
    /// `into` beside a variant that holds no payload.
    IntoWithoutPayload,
    /// A conversion option on a restated enum, which gets no conversions.
    RestatedConversions,
    /// An associated type a variant gives otherwise than the first, with
    /// no override.
    TypeDisagreement,
    /// An associated const of a checked type a variant gives otherwise
    /// than the first, with no override.
    ConstDisagreement,
    /// `From` for a variant holding the enum's type parameter beside
    /// another variant's `From`.
    FromOverlapTypeParameter,
    /// `TryFrom` for a variant holding the enum's type parameter.
    TryFromTypeParameter,
    /// rustc's E0308 on the arms of a method returning an associated type
    /// the payloads give differently.
    ArmsDisagree,
    /// A method that requires `Self: Sized` forwarded to a variant that
    /// holds a trait object.
    SizedOnObject,
}

/// A catalogue entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The key an outcome names the sentence by.
    pub key: &'static str,
    /// A fragment that tells the sentence from every other, without the
    /// names the sentence interpolates.
    pub needle: &'static str,
    /// The `tests/ui` case that pins the sentence.
    pub case: &'static str,
    /// The way out the sentence names, as its snapshot spells it, without
    /// the names it interpolates.
    pub way_out: &'static str,
    /// Whether the sentence ends in the method's own way out: implementing
    /// it by hand, giving it a body, or leaving the variant to the body it
    /// has ([`Tail`]).
    pub tail: bool,
    /// Whether the impl still stands when the sentence is reported, so a
    /// call of another method through the trait compiles; see
    /// `macros/ARCHITECTURE.md` section 3.8.
    pub keeps_impl: bool,
}

impl Limit {
    /// Every catalogued sentence, in the order a message is matched
    /// against them.
    pub const ALL: &'static [Limit] = &[
        Limit::Receiver,
        Limit::UnitVariant,
        Limit::TwoFields,
        Limit::SelfArgumentWithoutMismatch,
        Limit::OpaqueReturn,
        Limit::SelfInReturn,
        Limit::DerefSelfReturn,
        Limit::RouteSelfReturn,
        Limit::SelfReturnOtherFields,
        Limit::BoxSelfPlain,
        Limit::NamedFieldUnderWrapper,
        Limit::BoxSelfOnSharedPointer,
        Limit::MovesThroughSharedPointer,
        Limit::MutableBorrowThroughSharedPointer,
        Limit::MismatchForAbsentMethod,
        Limit::MethodRouteOnMutSelf,
        Limit::UnsuppliedType,
        Limit::DefaultWithoutDefault,
        Limit::BodyNeverRuns,
        Limit::StarOverUnforwardable,
        Limit::StructConversion,
        Limit::IntoOtherFields,
        Limit::IntoWithoutPayload,
        Limit::RestatedConversions,
        Limit::TypeDisagreement,
        Limit::ConstDisagreement,
        Limit::FromOverlapTypeParameter,
        Limit::TryFromTypeParameter,
        Limit::ArmsDisagree,
        Limit::SizedOnObject,
    ];

    /// Returns the sentence's catalogue entry.
    pub fn entry(self) -> Entry {
        let entry = |key, needle, case| Entry {
            key,
            needle,
            case,
            way_out: "",
            tail: false,
            keeps_impl: true,
        };
        let entry = match self {
            Limit::Receiver => entry(
                "receiver",
                "this receiver type is not supported",
                "rc_self_receiver",
            ),
            Limit::UnitVariant => entry(
                "unit-variant",
                "holds no payload, so no trait method can be forwarded to it",
                "unit_variant",
            ),
            Limit::TwoFields => entry(
                "two-fields",
                "fields, not one payload, so no trait method can be forwarded to it",
                "two_field_variant",
            ),
            Limit::SelfArgumentWithoutMismatch => entry(
                "self-argument-without-mismatch",
                "takes `Self` arguments; give the result for two different variants",
                "self_argument_without_mismatch",
            ),
            Limit::OpaqueReturn => entry(
                "opaque-return",
                "cannot be dispatched by a `match` because every variant returns a different opaque type",
                "rpitit",
            ),
            Limit::SelfInReturn => entry(
                "self-in-return",
                "mentions `Self` where `whichever` cannot wrap the payload's result back into",
                "self_in_a_tuple_under_option",
            ),
            Limit::DerefSelfReturn => entry(
                "deref-self-return",
                "which cannot be wrapped back into the dereferenced payload of",
                "deref_with_self_return",
            ),
            Limit::RouteSelfReturn => entry(
                "route-self-return",
                "which cannot be wrapped back through the route",
                "self_return_through_a_route",
            ),
            Limit::SelfReturnOtherFields => entry(
                "self-return-other-fields",
                "which cannot be built around the other fields of",
                "struct_self_return_other_fields",
            ),
            Limit::BoxSelfPlain => entry(
                "box-self-plain",
                "holds a plain",
                "box_self_plain_variant",
            ),
            Limit::NamedFieldUnderWrapper => entry(
                "named-field-under-wrapper",
                "has a named field, which no path builds",
                "named_field_under_a_wrapper",
            ),
            Limit::BoxSelfOnSharedPointer => entry(
                "box-self-on-shared-pointer",
                "which is no `Box` to hand over",
                "box_self_on_a_shared_pointer",
            ),
            Limit::MovesThroughSharedPointer => entry(
                "moves-through-shared-pointer",
                "which a value cannot be moved out of",
                "route_moves_through_a_shared_pointer",
            ),
            Limit::MutableBorrowThroughSharedPointer => entry(
                "mutable-borrow-through-shared-pointer",
                "which cannot be borrowed mutably; hold a `Box` or a `&mut`",
                "mutable_borrow_through_a_shared_pointer",
            ),
            Limit::MismatchForAbsentMethod => entry(
                "mismatch-for-absent-method",
                "so there is no mismatch to decide",
                "mismatch_for_a_method_under_a_cfg",
            ),
            Limit::MethodRouteOnMutSelf => entry(
                "method-route-on-mut-self",
                "ends in a method call, whose result cannot be borrowed mutably",
                "route_method_on_mut_self",
            ),
            Limit::DefaultWithoutDefault => entry(
                "default-without-default",
                "has no default body in",
                "block_default_without_a_default",
            ),
            Limit::BodyNeverRuns => entry(
                "body-never-runs",
                "so its body would never run",
                "inherent_all_with_body",
            ),
            Limit::StarOverUnforwardable => entry(
                "star-over-unforwardable",
                "is forwarded to every variant, but",
                "inherent_all_with_unforwardable",
            ),
            Limit::StructConversion => entry(
                "struct-conversion",
                "are options of an enum; a struct gets no conversions",
                "struct_from_option",
            ),
            Limit::IntoOtherFields => entry(
                "into-other-fields",
                "has other fields beside its payload, or is left out",
                "into_with_a_marked_variant",
            ),
            Limit::IntoWithoutPayload => entry(
                "into-without-payload",
                "`into` converts every variant's payload",
                "into_with_a_unit_variant",
            ),
            Limit::RestatedConversions => entry(
                "restated-conversions",
                "a restated enum gets no conversions",
                "restated_enum_conversions",
            ),
            Limit::TypeDisagreement => entry(
                "type-disagreement",
                "type mismatch resolving",
                "associated_type_disagreement",
            ),
            Limit::ConstDisagreement => entry(
                "const-disagreement",
                "differs from the first variant's; set it on the enum with",
                "associated_const_disagreement",
            ),
            Limit::FromOverlapTypeParameter => entry(
                "from-overlap-type-parameter",
                "would overlap the other variants' `From` impls",
                "from_overlap_type_parameter",
            ),
            Limit::TryFromTypeParameter => entry(
                "try-from-type-parameter",
                "would implement a foreign trait for a type parameter, which the orphan rules forbid",
                "try_from_type_parameter",
            ),
            // rustc's, not the macro's: each arm of a method returning the
            // type hands over the payload's own, and no expansion can make
            // `u32` and `u64` one type while the payloads give them; the
            // snapshot holds it beside the agreement check's E0271.
            Limit::ArmsDisagree => entry(
                "arms-disagree",
                "`match` arms have incompatible types",
                "associated_type_disagreement",
            ),
            // The item holds its sentence and the impl is written beside it
            // (`unsupplied_items_keep_the_impl`).
            Limit::UnsuppliedType => entry(
                "unsupplied-type",
                "so none can supply",
                "all_variants_conditional",
            ),
            Limit::SizedOnObject => entry(
                "sized-on-object",
                "holds a trait object, which is not; leave it to the trait's default with",
                "dyn_payload_under_sized_method",
            ),
        };
        let (way_out, tail) = match self {
            Limit::Receiver => ("a receiver is `self`, `&self`, `&mut self`, `self: Box<Self>`, `self: Pin<&Self>` or `self: Pin<&mut Self>`", false),
            Limit::UnitVariant => ("or forward inherent methods instead, whose body answers for it", false),
            Limit::TwoFields => ("mark the field to forward to with `#[dispatch]`, or forward inherent methods instead", false),
            Limit::SelfArgumentWithoutMismatch => ("give the result for two different variants with `#[dispatch(mismatch = <expr>)]` on the trait method or", false),
            Limit::OpaqueReturn => ("return a concrete type or a boxed trait object, or `impl Future<Output = T>`", false),
            Limit::SelfInReturn => ("return `Self`, `Option<Self>`, `Result<Self, E>` or a tuple of those, spelled with those names", true),
            Limit::DerefSelfReturn => ("drop `deref` on it", true),
            Limit::RouteSelfReturn => ("drop `via` on it", true),
            Limit::SelfReturnOtherFields => ("write the method in a `#[whichever::dispatch] impl` block", false),
            // The noun is the variant's or, on a struct, the field's.
            Limit::BoxSelfPlain => ("in a `#[dispatch(deref)]`", true),
            Limit::NamedFieldUnderWrapper => ("make it a tuple struct", false),
            Limit::BoxSelfOnSharedPointer => ("which is no `Box` to hand over; hold a `Box`", true),
            Limit::MovesThroughSharedPointer => ("which a value cannot be moved out of; hold a `Box`", true),
            Limit::MutableBorrowThroughSharedPointer => ("hold a `Box` or a `&mut`", true),
            Limit::MismatchForAbsentMethod => ("takes its mismatch on the trait method, `#[dispatch(mismatch = ..)]`", false),
            Limit::MethodRouteOnMutSelf => ("route to a field", true),
            Limit::DefaultWithoutDefault => ("give it a body, or forward it", false),
            Limit::BodyNeverRuns => ("end the signature with `;` instead", false),
            Limit::StarOverUnforwardable => ("name the variants with a payload and give the method a body for the rest", false),
            Limit::StructConversion => ("a struct gets no conversions", false),
            Limit::IntoOtherFields => ("or is left out with `from = false`", false),
            Limit::IntoWithoutPayload => ("holds no payload", false),
            Limit::RestatedConversions => (
                "would implement a foreign trait for another crate's type",
                false,
            ),
            Limit::TypeDisagreement => ("type mismatch resolving", false),
            Limit::ConstDisagreement => ("set it on the enum with", false),
            Limit::FromOverlapTypeParameter => (
                "add `from = false` on the enum, or `#[dispatch(from = false)]` on this variant",
                false,
            ),
            Limit::TryFromTypeParameter => ("add `#[dispatch(try_from = false)]` on this variant", false),
            Limit::ArmsDisagree => ("`match` arms have incompatible types", false),
            Limit::UnsuppliedType => ("fix it on the enum with", false),
            Limit::SizedOnObject => (
                "leave it to the trait's default with `#[dispatch(default)] fn",
                true,
            ),
        };
        Entry {
            way_out,
            tail,
            ..entry
        }
    }

    /// Returns whether the sentence concerns the method under test, on
    /// the method or on one of the variants it is forwarded to, rather than
    /// the subject as a whole.
    pub fn is_of_method(self) -> bool {
        match self {
            Limit::UnitVariant
            | Limit::TwoFields
            | Limit::UnsuppliedType
            | Limit::StructConversion
            | Limit::IntoOtherFields
            | Limit::IntoWithoutPayload
            | Limit::RestatedConversions
            | Limit::TypeDisagreement
            | Limit::ConstDisagreement
            | Limit::FromOverlapTypeParameter
            | Limit::TryFromTypeParameter
            | Limit::ArmsDisagree
            | Limit::MismatchForAbsentMethod => false,
            Limit::MutableBorrowThroughSharedPointer
            | Limit::Receiver
            | Limit::SelfArgumentWithoutMismatch
            | Limit::OpaqueReturn
            | Limit::SelfInReturn
            | Limit::DerefSelfReturn
            | Limit::RouteSelfReturn
            | Limit::SelfReturnOtherFields
            | Limit::BoxSelfPlain
            | Limit::NamedFieldUnderWrapper
            | Limit::BoxSelfOnSharedPointer
            | Limit::MovesThroughSharedPointer
            | Limit::MethodRouteOnMutSelf
            | Limit::DefaultWithoutDefault
            | Limit::BodyNeverRuns
            | Limit::StarOverUnforwardable
            | Limit::SizedOnObject => true,
        }
    }

    /// Returns whether the sentence replaces the body of the method under
    /// test, which then reports nothing else. A sentence about one variant
    /// is that variant's arm instead, and the other arms are written and
    /// type-checked beside it (macros/ARCHITECTURE.md 3.8).
    pub fn replaces_method(self) -> bool {
        match self {
            Limit::Receiver
            | Limit::SelfArgumentWithoutMismatch
            | Limit::OpaqueReturn
            | Limit::SelfInReturn
            | Limit::DefaultWithoutDefault
            | Limit::BodyNeverRuns => true,
            // The variants a mark of `*` names are forwarded, each arm
            // carrying a sentence of its own, beside the one on the mark.
            Limit::StarOverUnforwardable
            | Limit::UnitVariant
            | Limit::TwoFields
            | Limit::UnsuppliedType
            | Limit::StructConversion
            | Limit::IntoOtherFields
            | Limit::IntoWithoutPayload
            | Limit::RestatedConversions
            | Limit::DerefSelfReturn
            | Limit::RouteSelfReturn
            | Limit::SelfReturnOtherFields
            | Limit::BoxSelfPlain
            | Limit::NamedFieldUnderWrapper
            | Limit::BoxSelfOnSharedPointer
            | Limit::MovesThroughSharedPointer
            | Limit::MutableBorrowThroughSharedPointer
            | Limit::MismatchForAbsentMethod
            | Limit::MethodRouteOnMutSelf
            | Limit::TypeDisagreement
            | Limit::ConstDisagreement
            | Limit::FromOverlapTypeParameter
            | Limit::TryFromTypeParameter
            | Limit::ArmsDisagree
            | Limit::SizedOnObject => false,
        }
    }

    /// Returns the sentence a message is, by its needle.
    pub fn of_message(message: &str) -> Option<Limit> {
        Limit::ALL
            .iter()
            .copied()
            .find(|limit| message.contains(limit.entry().needle))
    }

    /// Returns the sentence a catalogue key names.
    pub fn from_key(key: &str) -> Option<Limit> {
        Limit::ALL
            .iter()
            .copied()
            .find(|limit| limit.entry().key == key)
    }
}

/// How a sentence that ends in the method's own way out ends: what the
/// method can still be given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    /// A trait's method, forwarded through the enum's attribute or a
    /// block: "implement the method by hand".
    ByHand,
    /// An inherent method with no body: "give it a body".
    Body,
    /// A method that names its variants and has a body for the rest:
    /// "leave `A` to the body".
    LeaveToBody,
    /// A sentence on the whole of a method that names its variants and has
    /// a body for the rest: "drop its `#[dispatch(..)]`".
    DropMark,
}

impl Tail {
    /// Every tail.
    pub const ALL: &'static [Tail] = &[Tail::ByHand, Tail::Body, Tail::LeaveToBody, Tail::DropMark];

    /// Returns the tail as a sentence spells it, `variant` being the
    /// variant it is about, in backticks.
    pub fn fragment(self, variant: &str) -> String {
        match self {
            Tail::ByHand => String::from("implement the method by hand"),
            Tail::Body => String::from("give it a body"),
            Tail::LeaveToBody => format!("leave {variant} to the body"),
            Tail::DropMark => {
                String::from("drop its `#[dispatch(..)]`, and the body answers for every variant")
            }
        }
    }
}

/// Where a sentence is to be reported: the line of the primary span.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Anchor {
    /// The signature of the method under test in the trait.
    Method,
    /// The variant of a slot, by its index: `A` is 0, `B` is 1.
    Variant(usize),
    /// The subject's `#[whichever::dispatch(..)]` attribute.
    Attribute,
    /// The subject's name, `pub enum E`.
    Enum,
    /// The method under test in a block: its `#[dispatch(..)]` line, or its
    /// signature when it carries none.
    BlockMethod,
    /// The signature of the method under test in a block.
    BlockSignature,
    /// The `#[whichever::dispatch(..)]` on a partial impl block.
    BlockAttribute,
    /// Anywhere in the cell: the span is not pinned yet.
    Anywhere,
}

impl Anchor {
    /// Returns the anchor's name, as the rendered cell records it.
    pub fn name(self) -> String {
        match self {
            Anchor::Method => String::from("method"),
            Anchor::Variant(slot) => format!("variant-{slot}"),
            Anchor::Attribute => String::from("attribute"),
            Anchor::Enum => String::from("enum"),
            Anchor::BlockMethod => String::from("block-method"),
            Anchor::BlockSignature => String::from("block-signature"),
            Anchor::BlockAttribute => String::from("block-attribute"),
            Anchor::Anywhere => String::from("anywhere"),
        }
    }
}

/// A sentence the model predicts, and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Expected {
    /// The sentence.
    pub limit: Limit,
    /// Where its primary span is.
    pub anchor: Anchor,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn every_sentence_is_pinned_by_its_case() {
        let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/ui");
        for limit in Limit::ALL {
            let entry = limit.entry();
            let path = ui.join(format!("{}.stderr", entry.case));
            let stderr = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert!(
                stderr.contains(entry.needle),
                "the needle of `{}` is not in {}",
                entry.key,
                path.display()
            );
            assert!(
                stderr.contains(entry.way_out),
                "the way out of `{}` is not in {}",
                entry.key,
                path.display()
            );
            if entry.tail {
                assert!(
                    Tail::ALL.iter().any(|tail| match tail {
                        Tail::LeaveToBody => stderr.contains(" to the body"),
                        Tail::ByHand | Tail::Body | Tail::DropMark => {
                            stderr.contains(tail.fragment("").as_str())
                        }
                    }),
                    "the tail of `{}` is not in {}",
                    entry.key,
                    path.display()
                );
            }
            // A snapshot may hold several sentences, as
            // `associated_type_disagreement` holds rustc's E0308 beside its
            // E0271, so the line that holds the needle is the one read.
            let line = stderr
                .lines()
                .find(|line| line.contains(entry.needle))
                .expect("the needle is on a line of its snapshot");
            assert_eq!(Limit::of_message(line), Some(*limit), "{}", entry.key);
            assert!(ui.join(format!("{}.rs", entry.case)).exists());
        }
    }

    #[test]
    fn keys_name_their_sentence() {
        for limit in Limit::ALL {
            assert_eq!(Limit::from_key(limit.entry().key), Some(*limit));
        }
        assert_eq!(Limit::ALL.len(), {
            let mut keys: Vec<_> = Limit::ALL.iter().map(|limit| limit.entry().key).collect();
            keys.sort();
            keys.dedup();
            keys.len()
        });
    }
}
