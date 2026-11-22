//! What a target kind is: the type a variant stores, how the variant is
//! written, and the route from the variant's binding to the payload.

use super::axes::{Second, Target, Third};

/// What a variant stores around its payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Storage {
    /// The payload itself.
    Payload,
    /// A pointer to the payload.
    Pointer(Pointer),
    /// A wrapper struct `Wk { label: u64, payload: Pk }` with an inherent
    /// `fn lend(&self) -> &Pk`.
    Wrapper,
    /// A tuple `(u64, Pk)`.
    Pair,
    /// A pointer to a wrapper, `Box<Wk>`.
    WrapperBehind(Pointer),
    /// A tuple `(u64, Wk)` holding a wrapper.
    WrapperInPair,
    /// A `Box` holding the payload as a trait object, `Box<dyn Probe>`.
    Object,
}

/// A pointer a `deref` variant holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointer {
    /// `Box<Pk>`.
    Box,
    /// `Rc<Pk>`.
    Rc,
    /// `Arc<Pk>`.
    Arc,
    /// `&'a Pk`.
    Shared,
    /// `&'a mut Pk`.
    Exclusive,
}

impl Pointer {
    /// Returns whether a value can be moved out of the pointer.
    pub fn owns(self) -> bool {
        match self {
            Pointer::Box => true,
            Pointer::Rc | Pointer::Arc | Pointer::Shared | Pointer::Exclusive => false,
        }
    }

    /// Returns whether the pointer lends its target mutably.
    pub fn lends_mutably(self) -> bool {
        match self {
            Pointer::Box | Pointer::Exclusive => true,
            Pointer::Rc | Pointer::Arc | Pointer::Shared => false,
        }
    }
}

/// How the variant is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// One unnamed field, `A(storage)`.
    Single,
    /// The marked first field of two, `A(#[dispatch] storage, u64)`.
    MarkedTuple,
    /// The marked field of a struct variant, `A { label: u64, #[dispatch]
    /// held: storage }`.
    MarkedStruct,
    /// The subject is a struct of one field, `E(storage)`.
    Struct,
    /// The subject is a struct with a marked field among several, `E {
    /// label: u64, #[dispatch] held: storage }`.
    StructMarked,
    /// `pub struct E { held: P0 }`, one named field.
    StructNamed,
    /// The one field of a struct variant, named and marked, `A {
    /// #[dispatch] held: storage }`.
    MarkedNamed,
    /// The marked last field of two, `A(u64, #[dispatch] storage)`.
    MarkedLast,
    /// The subject is a tuple struct whose marked field is the last of
    /// two, `E(u64, #[dispatch] storage)`.
    StructMarkedLast,
}

/// One step of a route from the stored value to the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// What the value dereferences to, the route `*`.
    Deref,
    /// A named or numbered field, `.payload` or `.1`.
    Field(&'static str),
    /// What a method lends, `.lend()` or `.get()`: the method returns a
    /// reference, to the payload or to the wrapper.
    Method(&'static str),
    /// What a method returns by value, `.copied()`: the method returns a
    /// fresh `Pk` with the same state.
    Yield(&'static str),
}

/// A target kind's description, which the renderers interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetDescriptor {
    /// What the variant stores.
    pub storage: Storage,
    /// How the variant is written.
    pub form: Form,
    /// The steps from the stored value to the payload.
    pub route: &'static [Step],
    /// The steps a method that borrows its target mutably takes instead,
    /// where the variant or its field names them with `via_mut`: "a second
    /// route for the methods that borrow the target mutably" (routes.md).
    pub route_mut: Option<&'static [Step]>,
    /// The option written in `#[dispatch(..)]` on the variant, or on the
    /// marked field: `deref`, `via = .payload`.
    pub option: Option<&'static str>,
    /// Whether the option is one a variant's field does not take, a
    /// trait's name or a conversion option, which is the subject's one
    /// sentence (limits.md).
    pub refused: bool,
    /// Whether the mark is written under `cfg_attr`, `#[cfg_attr(all(),
    /// dispatch)]`: "A mark cannot go under `cfg_attr`. [..] You get an
    /// error on the mark, and the item is read as if the mark were plain,
    /// so nothing else follows" (limits.md).
    pub under_cfg_attr: bool,
}

impl TargetDescriptor {
    /// Returns the steps a method takes to the target: those named for a
    /// mutable borrow where it borrows `exclusive`ly and the variant names
    /// them, and the route otherwise. "A method that borrows the target
    /// mutably follows `via_mut`. That is a method taking `&mut self` or
    /// `self: Pin<&mut Self>`, and a `&mut Self` argument. Every other
    /// method follows `via`" (routes.md).
    pub fn route_for(&self, exclusive: bool) -> &'static [Step] {
        match (exclusive, self.route_mut) {
            (true, Some(route)) => route,
            (true, None) | (false, _) => self.route,
        }
    }

    /// Returns whether a mutable borrow would go through a method call
    /// that no route names for one: the route ends in a call, and the
    /// variant names no `via_mut`.
    pub fn borrows_mutably_through_a_call(&self) -> bool {
        self.route_mut.is_none() && self.ends_in_method()
    }

    /// Returns whether the target is reached through a route, `deref`
    /// included, rather than being the stored value.
    pub fn is_routed(&self) -> bool {
        !self.route.is_empty()
    }

    /// Returns whether the variant has fields beside the one forwarded to.
    pub fn has_other_fields(&self) -> bool {
        match self.form {
            Form::Single | Form::Struct | Form::StructNamed | Form::MarkedNamed => false,
            Form::MarkedTuple
            | Form::MarkedStruct
            | Form::StructMarked
            | Form::MarkedLast
            | Form::StructMarkedLast => true,
        }
    }

    /// Returns the pointer the route dereferences: a `deref` variant's, or
    /// the one a route through `*` and a field starts from.
    pub fn pointer(&self) -> Option<Pointer> {
        match self.storage {
            Storage::Pointer(pointer) | Storage::WrapperBehind(pointer) => Some(pointer),
            Storage::Object => Some(Pointer::Box),
            Storage::Payload | Storage::Wrapper | Storage::Pair | Storage::WrapperInPair => None,
        }
    }

    /// Returns whether the variant holds its payload as a trait object.
    pub fn is_object(&self) -> bool {
        self.storage == Storage::Object
    }

    /// Returns whether the route is `deref` alone, `*`.
    pub fn is_deref(&self) -> bool {
        self.route == [Step::Deref]
    }

    /// Returns whether the variant stores a wrapper `Wk`, as it is, behind
    /// a pointer or in a tuple.
    pub fn wraps(&self) -> bool {
        match self.storage {
            Storage::Wrapper | Storage::WrapperBehind(_) | Storage::WrapperInPair => true,
            Storage::Payload | Storage::Pointer(_) | Storage::Pair | Storage::Object => false,
        }
    }

    /// Returns whether the route reads a field of what a method lends: a
    /// method call followed by more steps, so that the target is a place
    /// behind a shared reference the call returned.
    pub fn reads_through_a_loan(&self) -> bool {
        let Some(call) = self
            .route
            .iter()
            .position(|step| matches!(step, Step::Method(_)))
        else {
            return false;
        };
        call + 1 < self.route.len()
    }

    /// Returns whether the route ends in a method call.
    pub fn ends_in_method(&self) -> bool {
        matches!(self.route.last(), Some(Step::Method(_) | Step::Yield(_)))
    }

    /// Returns whether the route ends in a method that lends its target.
    pub fn ends_in_loan(&self) -> bool {
        matches!(self.route.last(), Some(Step::Method(_)))
    }

    /// Returns whether the route ends in a method that returns a value, so
    /// the target is a temporary.
    pub fn ends_in_temporary(&self) -> bool {
        matches!(self.route.last(), Some(Step::Yield(_)))
    }

    /// Returns the trait the struct's held field names, as the field
    /// spells it, beside a bare label: `None` where the field names none.
    pub fn trait_on_field(&self) -> Option<&'static str> {
        match self.form {
            Form::StructMarked => self.option,
            _ => None,
        }
    }

    /// Returns whether the subject is a struct rather than an enum.
    pub fn is_struct(&self) -> bool {
        matches!(
            self.form,
            Form::Struct | Form::StructMarked | Form::StructNamed | Form::StructMarkedLast
        )
    }

    /// Returns whether the stored type borrows, so the enum needs `'a`.
    pub fn borrows(&self) -> bool {
        matches!(
            self.storage,
            Storage::Pointer(Pointer::Shared | Pointer::Exclusive)
        )
    }
}

/// The trait under test as a struct's field names it, as its home spells
/// it.
pub const TRAIT_ON_FIELD: &str = "Probe";

/// The trait under test as a struct's field names it by a path from
/// `self`, as its home spells it too.
pub const ROOTED_TRAIT_ON_FIELD: &str = "self::Probe";

/// The module of the cell that re-exports the trait under test, so that a
/// home can spell it with one more segment than the field.
pub const SPELT_MODULE: &str = "spelt";

const DEREF: &[Step] = &[Step::Deref];
const FIELD: &[Step] = &[Step::Field("payload")];
const INDEX: &[Step] = &[Step::Field("1")];
const METHOD: &[Step] = &[Step::Method("lend")];
const YIELD: &[Step] = &[Step::Yield("copied")];
const DEREF_FIELD: &[Step] = &[Step::Deref, Step::Field("payload")];
const INDEX_FIELD: &[Step] = &[Step::Field("1"), Step::Field("payload")];
const METHOD_FIELD: &[Step] = &[Step::Method("get"), Step::Field("payload")];
const METHOD_MUT: &[Step] = &[Step::Method("lend_mut")];

/// The option of a variant or a field that reaches its target through a
/// getter pair: "A type with a getter pair therefore needs two routes.
/// Write the second with `via_mut`" (routes.md).
const GETTERS: &str = "via = .lend(), via_mut = .lend_mut()";

impl Target {
    /// Returns whether the struct's field names the trait under test as an
    /// impl names it while its home spells it with one more path segment,
    /// `spelt::Probe`: one name spelled two ways.
    pub fn spells_trait_otherwise(self) -> bool {
        self == Target::StructTraitSpelt
    }

    /// Returns whether the struct has several fields and marks none, so
    /// that no field is the trait's.
    pub fn marks_no_field(self) -> bool {
        self == Target::StructUnmarked
    }

    /// Returns the segment the home spells the trait under test with in
    /// front of its name: a module of the cell, where the field spells it
    /// otherwise, or `self`, as the field spells it too.
    pub fn home_prefix(self) -> Option<&'static str> {
        match self {
            Target::StructTraitSpelt => Some(SPELT_MODULE),
            Target::StructTraitSelf => Some("self"),
            _ => None,
        }
    }

    /// Returns the target kind's description.
    pub fn descriptor(self) -> TargetDescriptor {
        let single = |storage, route, option| TargetDescriptor {
            storage,
            form: Form::Single,
            route,
            option,
            refused: false,
            under_cfg_attr: false,
            route_mut: None,
        };
        let deref = |pointer| single(Storage::Pointer(pointer), DEREF, Some("deref"));
        match self {
            Target::Plain => single(Storage::Payload, &[], None),
            Target::DerefBox => deref(Pointer::Box),
            Target::DerefRc => deref(Pointer::Rc),
            Target::DerefArc => deref(Pointer::Arc),
            Target::DerefShared => deref(Pointer::Shared),
            Target::DerefExclusive => deref(Pointer::Exclusive),
            Target::ViaField => single(Storage::Wrapper, FIELD, Some("via = .payload")),
            Target::ViaIndex => single(Storage::Pair, INDEX, Some("via = .1")),
            Target::ViaMethod => single(Storage::Wrapper, METHOD, Some("via = .lend()")),
            Target::ViaValueMethod => single(Storage::Wrapper, YIELD, Some("via = .copied()")),
            Target::MarkedTuple => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::MarkedTuple,
                route: &[],
                option: None,
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::MarkedStruct => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::MarkedStruct,
                route: &[],
                option: None,
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::Struct => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::Struct,
                route: &[],
                option: None,
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::StructNamed => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::StructNamed,
                route: &[],
                option: None,
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::StructMarked => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::StructMarked,
                route: &[],
                option: None,
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::MarkedVia => TargetDescriptor {
                storage: Storage::Wrapper,
                form: Form::MarkedStruct,
                route: FIELD,
                option: Some("via = .payload"),
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::DerefBoxField => single(
                Storage::WrapperBehind(Pointer::Box),
                DEREF_FIELD,
                Some("via = *.payload"),
            ),
            Target::DerefRcField => single(
                Storage::WrapperBehind(Pointer::Rc),
                DEREF_FIELD,
                Some("via = *.payload"),
            ),
            Target::ViaIndexField => single(
                Storage::WrapperInPair,
                INDEX_FIELD,
                Some("via = .1.payload"),
            ),
            Target::ViaMethodField => {
                single(Storage::Wrapper, METHOD_FIELD, Some("via = .get().payload"))
            }
            Target::DerefObject => single(Storage::Object, DEREF, Some("deref")),
            Target::MarkedNamed => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::MarkedNamed,
                route: &[],
                option: None,
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::MarkedDeref => TargetDescriptor {
                storage: Storage::Pointer(Pointer::Box),
                form: Form::MarkedStruct,
                route: DEREF,
                option: Some("deref"),
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::MarkedTrait => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::MarkedStruct,
                route: &[],
                option: Some("Probe"),
                refused: true,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::MarkedConversion => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::MarkedStruct,
                route: &[],
                option: Some("from = false"),
                refused: true,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::MarkedLast => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::MarkedLast,
                route: &[],
                option: None,
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::StructMarkedLast => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::StructMarkedLast,
                route: &[],
                option: None,
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::StructTrait => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::StructMarked,
                route: &[],
                option: Some(TRAIT_ON_FIELD),
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::StructTraitSpelt => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::StructMarked,
                route: &[],
                option: Some(TRAIT_ON_FIELD),
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
            Target::ViaGetters => TargetDescriptor {
                route_mut: Some(METHOD_MUT),
                ..single(Storage::Wrapper, METHOD, Some(GETTERS))
            },
            Target::MarkedGetters => TargetDescriptor {
                route: METHOD,
                route_mut: Some(METHOD_MUT),
                option: Some(GETTERS),
                ..Target::MarkedVia.descriptor()
            },
            Target::MarkedCfgAttr => TargetDescriptor {
                under_cfg_attr: true,
                ..Target::MarkedTuple.descriptor()
            },
            Target::StructMarkedCfgAttr => TargetDescriptor {
                under_cfg_attr: true,
                ..Target::StructMarked.descriptor()
            },
            // The struct with a marked field beside a label, its mark left
            // out, which `write_struct` asks of the target.
            Target::StructUnmarked => Target::StructMarked.descriptor(),
            Target::StructTraitSelf => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::StructMarked,
                route: &[],
                option: Some(ROOTED_TRAIT_ON_FIELD),
                refused: false,
                under_cfg_attr: false,
                route_mut: None,
            },
        }
    }
}

/// What a variant after the first is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaterSlot {
    /// There is none.
    Absent,
    /// A variant over the slot's payload with this target kind.
    Live(Target),
    /// A unit variant.
    Unit,
    /// A variant of two unmarked fields.
    Pair,
}

impl Second {
    /// Returns what the second variant is.
    pub fn slot(self) -> LaterSlot {
        match self {
            Second::Plain => LaterSlot::Live(Target::Plain),
            Second::Absent => LaterSlot::Absent,
            Second::DerefBox => LaterSlot::Live(Target::DerefBox),
            Second::DerefRc => LaterSlot::Live(Target::DerefRc),
            Second::DerefArc => LaterSlot::Live(Target::DerefArc),
            Second::DerefShared => LaterSlot::Live(Target::DerefShared),
            Second::DerefExclusive => LaterSlot::Live(Target::DerefExclusive),
            Second::ViaField => LaterSlot::Live(Target::ViaField),
            Second::ViaIndex => LaterSlot::Live(Target::ViaIndex),
            Second::ViaMethod => LaterSlot::Live(Target::ViaMethod),
            Second::MarkedTuple => LaterSlot::Live(Target::MarkedTuple),
            Second::MarkedStruct => LaterSlot::Live(Target::MarkedStruct),
            Second::MarkedVia => LaterSlot::Live(Target::MarkedVia),
            Second::Unit => LaterSlot::Unit,
            Second::Pair => LaterSlot::Pair,
            Second::DerefBoxField => LaterSlot::Live(Target::DerefBoxField),
            Second::DerefRcField => LaterSlot::Live(Target::DerefRcField),
            Second::ViaIndexField => LaterSlot::Live(Target::ViaIndexField),
            Second::ViaMethodField => LaterSlot::Live(Target::ViaMethodField),
            Second::DerefObject => LaterSlot::Live(Target::DerefObject),
            Second::ViaGetters => LaterSlot::Live(Target::ViaGetters),
        }
    }
}

impl Third {
    /// Returns what the third variant is.
    pub fn slot(self) -> LaterSlot {
        match self {
            Third::Absent => LaterSlot::Absent,
            Third::Plain => LaterSlot::Live(Target::Plain),
            Third::DerefBox => LaterSlot::Live(Target::DerefBox),
            Third::ViaField => LaterSlot::Live(Target::ViaField),
            Third::Unit => LaterSlot::Unit,
        }
    }
}
