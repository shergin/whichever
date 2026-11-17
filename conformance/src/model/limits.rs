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
    /// `Self` inside an `Option` or a `Result` through a variant whose one
    /// field is named and marked, which no path builds.
    NamedVariantUnderWrapper,
    /// An option on a variant's marked field other than `deref` and `via =
    /// route`: a trait's name, or a conversion option.
    VariantFieldOption,
    /// A macro invocation in a partial impl block, which is not read.
    MacroInBlock,
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
    /// A body for the rest on a method whose mark names every variant.
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
    /// A generic associated type the enum does not declare in a partial
    /// block.
    GenericAssociatedType,
    /// An arm of a method under `*` that names no variant.
    ArmNamingNoVariant,
    /// Arms of its own beside a `Self` argument a trait method matches
    /// beside `self`.
    ArmsBesideSelfArgument,
    /// Arms of its own in a method matched by place, over an enum whose
    /// every variant is under `#[cfg]`.
    ArmsMatchedByPlace,
    /// A struct's field naming the trait its attribute names, spelled
    /// another way.
    FieldTraitSpelt,
    /// A struct's field naming the trait a partial block's header names,
    /// spelled another way.
    BlockTraitSpelt,
    /// A partial block's method naming a field of the struct other than
    /// the one its trait is forwarded to.
    FieldElsewhere,
    /// A struct's method naming two of its fields.
    TwoFieldsNamed,
    /// A method of a partial block without a body and without a mark.
    BodilessInBlock,
    /// `from` given the other value by a later option.
    FromContradicted,
    /// An `into` target given a second time.
    IntoRepeated,
}

/// The way out of [`Limit::StarOverUnforwardable`] for a method that holds
/// no arms of its own under `*`: a trait method with a `Self` argument,
/// matched beside `self`, and a method matched by place over an enum whose
/// every variant is under `#[cfg]` (`docs/limits.md`), as the snapshot
/// `star_over_variants_without_a_payload_beside_a_self_argument` spells it.
pub const STAR_OVER_UNFORWARDABLE_BY_NAMED_LIST: &str =
    "name the variants with a payload and give the method a body for the rest";

/// The way out of [`Limit::BodyNeverRuns`] for a body under `*` on an enum,
/// which may instead hold the arms the enum answers itself, as the snapshot
/// `inherent_all_with_body` spells it; a named list that names every
/// variant, and a struct, keep the entry's.
pub const BODY_NEVER_RUNS_UNDER_STAR: &str = "make the body one `match self` holding the arms of the variants the enum answers itself, or end the signature with `;`";

/// The way out of the sentences that name a variant a method under `*`
/// cannot reach and offer its arm, as the snapshot
/// `own_arms_over_a_variant_the_method_refuses` spells it.
pub const ARM_IN_BODY_CASE: &str = "own_arms_over_a_variant_the_method_refuses";

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
        Limit::NamedVariantUnderWrapper,
        Limit::VariantFieldOption,
        Limit::MacroInBlock,
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
        Limit::GenericAssociatedType,
        Limit::ArmNamingNoVariant,
        Limit::ArmsBesideSelfArgument,
        Limit::ArmsMatchedByPlace,
        Limit::FieldTraitSpelt,
        Limit::BlockTraitSpelt,
        Limit::FieldElsewhere,
        Limit::TwoFieldsNamed,
        Limit::BodilessInBlock,
        Limit::FromContradicted,
        Limit::IntoRepeated,
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
            Limit::NamedVariantUnderWrapper => entry(
                "named-variant-under-wrapper",
                "holds its payload in a named field, which no path builds",
                "self_in_an_option_over_a_variants_named_field",
            ),
            // The subject is not read, so no impl is written beside the
            // sentence; the enum is emitted as it is.
            Limit::VariantFieldOption => Entry {
                keeps_impl: false,
                ..entry(
                    "variant-field-option",
                    "the options of a variant's field are `deref` and `via = route`",
                    "trait_named_on_a_variants_field",
                )
            },
            // The block is not read, so the impl it would hold is not
            // written.
            Limit::MacroInBlock => Entry {
                keeps_impl: false,
                ..entry(
                    "macro-in-block",
                    "a macro invocation in a partial impl block is not read",
                    "macro_in_a_partial_block",
                )
            },
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
                "inherent_list_of_every_variant_with_body",
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
            // `gat_undeclared_in_block` holds the same sentence for a block
            // that leaves the type out.
            Limit::GenericAssociatedType => entry(
                "generic-associated-type",
                "is a generic associated type, which only the",
                "generic_associated_type",
            ),
            Limit::ArmNamingNoVariant => entry(
                "arm-naming-no-variant",
                "names the variants it answers, as `Self::Name(..)`, and every other variant is forwarded; this one names none",
                "own_arm_naming_no_variant",
            ),
            Limit::ArmsBesideSelfArgument => entry(
                "arms-beside-self-argument",
                "which are matched beside `self`, and an arm of its own is written for `self` alone",
                "own_arms_beside_a_self_argument",
            ),
            // The field's sentence; the impl the attribute writes is the
            // bare field's, beside it (a-fields-trait-may-be-a-partial-blocks.md).
            Limit::FieldTraitSpelt => entry(
                "field-trait-spelt",
                "on the struct spell one trait's name two ways",
                "struct_trait_spelt_two_ways",
            ),
            // The block is refused before it is read, so no impl is written.
            Limit::BlockTraitSpelt => Entry {
                keeps_impl: false,
                ..entry(
                    "block-trait-spelt",
                    "here spell one trait's name two ways",
                    "block_trait_spelt_two_ways_by_path",
                )
            },
            // The block is not read, so the impl it would hold is not
            // written.
            Limit::BodilessInBlock => Entry {
                keeps_impl: false,
                ..entry(
                    "bodiless-in-block",
                    "has no body; forward it",
                    "block_bodiless_without_attribute",
                )
            },
            Limit::FromContradicted => entry(
                "from-contradicted",
                "on another attribute of",
                "from_contradicts_another_attribute",
            ),
            Limit::IntoRepeated => entry(
                "into-repeated",
                "is given twice on",
                "into_target_twice_on_two_attributes",
            ),
            Limit::TwoFieldsNamed => entry(
                "two-fields-named",
                "and a struct forwards a method to one, since every value holds them all",
                "struct_method_naming_two_fields",
            ),
            Limit::FieldElsewhere => entry(
                "field-elsewhere",
                "but this trait is forwarded to",
                "block_method_naming_another_field",
            ),
            Limit::ArmsMatchedByPlace => entry(
                "arms-matched-by-place",
                "with `ref` bindings, which arms written for `match self` are not",
                "own_arms_over_variants_all_under_cfg",
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
            Limit::NamedVariantUnderWrapper => ("write the variant as", false),
            Limit::VariantFieldOption => (
                "a trait takes its own route through the variant on the enum, `via(Variant = route)`",
                false,
            ),
            Limit::MacroInBlock => ("write the items out in the block", false),
            Limit::BoxSelfOnSharedPointer => ("which is no `Box` to hand over; hold a `Box`", true),
            Limit::MovesThroughSharedPointer => ("which a value cannot be moved out of; hold a `Box`", true),
            Limit::MutableBorrowThroughSharedPointer => ("hold a `Box` or a `&mut`", true),
            Limit::MismatchForAbsentMethod => ("takes its mismatch on the trait method, `#[dispatch(mismatch = ..)]`", false),
            Limit::MethodRouteOnMutSelf => ("route to a field", true),
            Limit::DefaultWithoutDefault => ("give it a body, or forward it", false),
            Limit::BodyNeverRuns => ("end the signature with `;` instead", false),
            // Where the method holds no arms of its own, its way out is
            // `STAR_OVER_UNFORWARDABLE_BY_NAMED_LIST` instead (`wording`).
            Limit::StarOverUnforwardable => ("write its arm in the method's body, `match self { Self::", false),
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
            Limit::GenericAssociatedType => (" = ..;` in a `#[whichever::dispatch] impl", false),
            Limit::ArmNamingNoVariant => (
                "name the variants it answers, or name the forwarded ones in `#[dispatch(..)]` and make this the method's body",
                false,
            ),
            Limit::ArmsBesideSelfArgument | Limit::ArmsMatchedByPlace => (
                "name the variants in `#[dispatch(..)]` and give the method a body for the rest",
                false,
            ),
            Limit::FieldTraitSpelt => ("a field names a trait as the struct's attribute spells it", false),
            Limit::BlockTraitSpelt => ("a field names a trait as the block's header spells it", false),
            Limit::FieldElsewhere => ("`#[dispatch(Trait)]`, to forward it there", false),
            Limit::TwoFieldsNamed => ("name one, and write a method that needs both by hand", false),
            Limit::FromContradicted => ("so give `from` one value", false),
            Limit::IntoRepeated => ("so name a target once", false),
            Limit::BodilessInBlock => (
                "or leave it to the trait's default with `#[dispatch(default)]`",
                false,
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
            | Limit::MismatchForAbsentMethod
            | Limit::GenericAssociatedType
            | Limit::VariantFieldOption
            | Limit::MacroInBlock
            | Limit::FieldTraitSpelt
            | Limit::BlockTraitSpelt
            | Limit::BodilessInBlock
            | Limit::FromContradicted
            | Limit::IntoRepeated => false,
            Limit::ArmNamingNoVariant
            | Limit::FieldElsewhere
            | Limit::TwoFieldsNamed
            | Limit::ArmsBesideSelfArgument
            | Limit::ArmsMatchedByPlace
            | Limit::MutableBorrowThroughSharedPointer
            | Limit::Receiver
            | Limit::SelfArgumentWithoutMismatch
            | Limit::OpaqueReturn
            | Limit::SelfInReturn
            | Limit::DerefSelfReturn
            | Limit::RouteSelfReturn
            | Limit::SelfReturnOtherFields
            | Limit::BoxSelfPlain
            | Limit::NamedFieldUnderWrapper
            | Limit::NamedVariantUnderWrapper
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
            | Limit::BodyNeverRuns
            | Limit::ArmNamingNoVariant
            | Limit::ArmsBesideSelfArgument
            | Limit::ArmsMatchedByPlace
            | Limit::FieldElsewhere
            | Limit::TwoFieldsNamed => true,
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
            | Limit::NamedVariantUnderWrapper
            | Limit::VariantFieldOption
            | Limit::MacroInBlock
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
            | Limit::SizedOnObject
            | Limit::GenericAssociatedType
            | Limit::FieldTraitSpelt
            | Limit::BlockTraitSpelt
            | Limit::BodilessInBlock
            | Limit::FromContradicted
            | Limit::IntoRepeated => false,
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
    /// A method under `*` whose body holds arms of its own: "write the arm
    /// of `A` in the method's body".
    ArmInBody,
    /// A sentence on the whole of a method under `*` whose body holds arms
    /// of its own: "drop its `#[dispatch(*)]`".
    DropStar,
}

impl Tail {
    /// Every tail.
    pub const ALL: &'static [Tail] = &[
        Tail::ByHand,
        Tail::Body,
        Tail::LeaveToBody,
        Tail::DropMark,
        Tail::ArmInBody,
        Tail::DropStar,
    ];

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
            Tail::ArmInBody => format!("write the arm of {variant} in the method's body"),
            Tail::DropStar => {
                String::from("drop its `#[dispatch(*)]` and write every variant's arm in the body")
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
    /// The second trait's `#[whichever::dispatch(..)]` on the subject.
    OtherAttribute,
    /// The `#[whichever::dispatch(..)]` on a partial impl block.
    BlockAttribute,
    /// The header of a partial impl block, `impl Probe for E {`, whose
    /// trait path a sentence about the block's trait is spanned on.
    BlockHeader,
    /// The macro invocation in a partial impl block.
    BlockInvocation,
    /// The arm of the method's own body under `*` that names no variant.
    Arm,
    /// The declaration of the trait's generic associated type, in the
    /// trait or in its restatement.
    Associated,
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
            Anchor::OtherAttribute => String::from("other-attribute"),
            Anchor::BlockAttribute => String::from("block-attribute"),
            Anchor::BlockHeader => String::from("block-header"),
            Anchor::BlockInvocation => String::from("block-invocation"),
            Anchor::Arm => String::from("arm"),
            Anchor::Associated => String::from("associated"),
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
                        Tail::ArmInBody => stderr.contains(" in the method's body"),
                        Tail::ByHand | Tail::Body | Tail::DropMark | Tail::DropStar => {
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
    fn the_named_list_way_out_is_pinned_by_its_case() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/ui/star_over_variants_without_a_payload_beside_a_self_argument.stderr");
        let stderr = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let line = stderr
            .lines()
            .find(|line| line.contains(STAR_OVER_UNFORWARDABLE_BY_NAMED_LIST))
            .expect("the way out is on a line of its snapshot");
        assert_eq!(Limit::of_message(line), Some(Limit::StarOverUnforwardable));
    }

    #[test]
    fn the_ways_out_under_star_are_pinned_by_their_cases() {
        let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/ui");
        let read = |case: &str| {
            let path = ui.join(format!("{case}.stderr"));
            std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        };
        let body = read("inherent_all_with_body");
        let line = body
            .lines()
            .find(|line| line.contains(BODY_NEVER_RUNS_UNDER_STAR))
            .expect("the way out is on a line of its snapshot");
        assert_eq!(Limit::of_message(line), Some(Limit::BodyNeverRuns));
        let refused = read(ARM_IN_BODY_CASE);
        assert!(refused.contains(&Tail::ArmInBody.fragment("`Counted`")));
        let lifted = read("own_arms_beside_a_self_return_the_lift_cannot_wrap");
        assert!(lifted.contains(&Tail::DropStar.fragment("")));
        let rebuilt = read("own_arms_over_a_variant_that_cannot_be_rebuilt");
        assert!(rebuilt.contains(&Tail::ArmInBody.fragment("`Tagged`")));
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
