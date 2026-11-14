//! What a target kind is: the type a variant stores, how the variant is
//! written, and the route from the variant's binding to the payload.

use super::axes::{Second, Target};

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
}

/// One step of a route from the stored value to the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// What the value dereferences to, the route `*`.
    Deref,
    /// A named or numbered field, `.payload` or `.1`.
    Field(&'static str),
    /// What a method lends, `.lend()`: the method returns `&Pk`.
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
    /// The option written in `#[dispatch(..)]` on the variant, or on the
    /// marked field: `deref`, `via = .payload`.
    pub option: Option<&'static str>,
}

impl TargetDescriptor {
    /// Returns whether the target is reached through a route, `deref`
    /// included, rather than being the stored value.
    pub fn is_routed(&self) -> bool {
        !self.route.is_empty()
    }

    /// Returns whether the variant has fields beside the one forwarded to.
    pub fn has_other_fields(&self) -> bool {
        match self.form {
            Form::Single => false,
            Form::MarkedTuple | Form::MarkedStruct => true,
        }
    }

    /// Returns the pointer a `deref` variant holds.
    pub fn pointer(&self) -> Option<Pointer> {
        match self.storage {
            Storage::Pointer(pointer) => Some(pointer),
            Storage::Payload | Storage::Wrapper | Storage::Pair => None,
        }
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

    /// Returns whether the stored type borrows, so the enum needs `'a`.
    pub fn borrows(&self) -> bool {
        matches!(
            self.storage,
            Storage::Pointer(Pointer::Shared | Pointer::Exclusive)
        )
    }
}

const DEREF: &[Step] = &[Step::Deref];
const FIELD: &[Step] = &[Step::Field("payload")];
const INDEX: &[Step] = &[Step::Field("1")];
const METHOD: &[Step] = &[Step::Method("lend")];
const YIELD: &[Step] = &[Step::Yield("copied")];

impl Target {
    /// Returns the target kind's description.
    pub fn descriptor(self) -> TargetDescriptor {
        let single = |storage, route, option| TargetDescriptor {
            storage,
            form: Form::Single,
            route,
            option,
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
            },
            Target::MarkedStruct => TargetDescriptor {
                storage: Storage::Payload,
                form: Form::MarkedStruct,
                route: &[],
                option: None,
            },
            Target::MarkedVia => TargetDescriptor {
                storage: Storage::Wrapper,
                form: Form::MarkedStruct,
                route: FIELD,
                option: Some("via = .payload"),
            },
        }
    }
}

/// What the second variant is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecondSlot {
    /// There is none.
    Absent,
    /// A variant over `P1` with this target kind.
    Live(Target),
    /// A unit variant.
    Unit,
    /// A variant of two unmarked fields.
    Pair,
}

impl Second {
    /// Returns what the second variant is.
    pub fn slot(self) -> SecondSlot {
        match self {
            Second::Plain => SecondSlot::Live(Target::Plain),
            Second::Absent => SecondSlot::Absent,
            Second::DerefBox => SecondSlot::Live(Target::DerefBox),
            Second::DerefRc => SecondSlot::Live(Target::DerefRc),
            Second::DerefArc => SecondSlot::Live(Target::DerefArc),
            Second::DerefShared => SecondSlot::Live(Target::DerefShared),
            Second::DerefExclusive => SecondSlot::Live(Target::DerefExclusive),
            Second::ViaField => SecondSlot::Live(Target::ViaField),
            Second::ViaIndex => SecondSlot::Live(Target::ViaIndex),
            Second::ViaMethod => SecondSlot::Live(Target::ViaMethod),
            Second::MarkedTuple => SecondSlot::Live(Target::MarkedTuple),
            Second::MarkedStruct => SecondSlot::Live(Target::MarkedStruct),
            Second::MarkedVia => SecondSlot::Live(Target::MarkedVia),
            Second::Unit => SecondSlot::Unit,
            Second::Pair => SecondSlot::Pair,
        }
    }
}
