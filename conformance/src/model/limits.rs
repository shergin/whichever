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
    /// A value moved out of a `deref` variant holding anything but a
    /// `Box`.
    MovesThroughSharedPointer,
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
        Limit::MovesThroughSharedPointer,
        Limit::MethodRouteOnMutSelf,
        Limit::UnsuppliedType,
        Limit::DefaultWithoutDefault,
        Limit::BodyNeverRuns,
        Limit::StarOverUnforwardable,
    ];

    /// Returns the sentence's catalogue entry.
    pub fn entry(self) -> Entry {
        let entry = |key, needle, case| Entry {
            key,
            needle,
            case,
            keeps_impl: true,
        };
        match self {
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
            Limit::MovesThroughSharedPointer => entry(
                "moves-through-shared-pointer",
                "which a value cannot be moved out of",
                "route_moves_through_a_shared_pointer",
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
            Limit::UnsuppliedType => Entry {
                keeps_impl: false,
                ..entry(
                    "unsupplied-type",
                    "so none can supply",
                    "all_variants_conditional",
                )
            },
        }
    }

    /// Returns whether the sentence is the body of the method under test,
    /// which then reports nothing else, rather than a sentence on a variant
    /// that leaves the method's other arms written.
    pub fn is_of_method(self) -> bool {
        match self {
            Limit::UnitVariant | Limit::TwoFields | Limit::UnsuppliedType => false,
            Limit::Receiver
            | Limit::SelfArgumentWithoutMismatch
            | Limit::OpaqueReturn
            | Limit::SelfInReturn
            | Limit::DerefSelfReturn
            | Limit::RouteSelfReturn
            | Limit::SelfReturnOtherFields
            | Limit::BoxSelfPlain
            | Limit::MovesThroughSharedPointer
            | Limit::MethodRouteOnMutSelf
            | Limit::DefaultWithoutDefault
            | Limit::BodyNeverRuns
            | Limit::StarOverUnforwardable => true,
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
            assert_eq!(Limit::of_message(&stderr), Some(*limit), "{}", entry.key);
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
