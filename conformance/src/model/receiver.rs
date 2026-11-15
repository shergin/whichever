//! What a receiver is: its spelling, how a forwarding gets at the enum,
//! and how a target is handed to the payload's method.

use super::axes::Receiver;

/// How a `match` binds the variant's fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    /// By shared reference: the binding is `&T`.
    Shared,
    /// By exclusive reference: the binding is `&mut T`.
    Exclusive,
    /// By value: the binding is `T`.
    Owned,
}

/// How the forwarding gets at the enum before it matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unwrap {
    /// The scrutinee, written in terms of `self`.
    pub scrutinee: &'static str,
    /// How the `match` binds.
    pub binding: Binding,
}

/// How the target's place is handed to the payload's method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handover {
    /// `&place`.
    Borrow,
    /// `&mut place`.
    BorrowMutably,
    /// `place`, moved.
    Move,
    /// The `Box` the place is the target of, moved.
    MoveBox,
    /// `Pin::new(&place)`.
    Pin,
    /// `Pin::new(&mut place)`.
    PinMutably,
}

/// What a receiver can lend to a reference in the return type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lends {
    /// Nothing: the receiver holds no lifetime.
    Nothing,
    /// A shared reference.
    Shared,
    /// A shared or an exclusive reference.
    Exclusive,
}

/// A receiver's description, which the renderers interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiverDescriptor {
    /// The receiver as the trait spells it.
    pub spelling: &'static str,
    /// How the forwarding unwraps the enum; `None` for a receiver the
    /// docs reject.
    pub unwrap: Option<Unwrap>,
    /// How a target is handed over.
    pub handover: Handover,
    /// What the receiver can lend.
    pub lends: Lends,
    /// How a payload's method reaches its own fields: `self` reaches them
    /// through `Deref`, a pinned reference through `Pin::get_ref` or
    /// `Pin::get_mut`.
    pub this: &'static str,
    /// Whether the method changes the payload's state.
    pub mutates: bool,
    /// How the law hands a value `{}` as this receiver.
    pub call: &'static str,
    /// Whether the call consumes the value.
    pub consumes: bool,
}

impl Receiver {
    /// Returns the receiver's description.
    pub fn descriptor(self) -> ReceiverDescriptor {
        let shared = Unwrap {
            scrutinee: "self",
            binding: Binding::Shared,
        };
        match self {
            Receiver::Shared => ReceiverDescriptor {
                spelling: "&self",
                unwrap: Some(shared),
                handover: Handover::Borrow,
                lends: Lends::Shared,
                this: "self",
                mutates: false,
                call: "&{}",
                consumes: false,
            },
            Receiver::Exclusive => ReceiverDescriptor {
                spelling: "&mut self",
                unwrap: Some(Unwrap {
                    scrutinee: "self",
                    binding: Binding::Exclusive,
                }),
                handover: Handover::BorrowMutably,
                lends: Lends::Exclusive,
                this: "self",
                mutates: true,
                call: "&mut {}",
                consumes: false,
            },
            Receiver::Owned => ReceiverDescriptor {
                spelling: "self",
                unwrap: Some(Unwrap {
                    scrutinee: "self",
                    binding: Binding::Owned,
                }),
                handover: Handover::Move,
                lends: Lends::Nothing,
                this: "self",
                mutates: false,
                call: "{}",
                consumes: true,
            },
            Receiver::Boxed => ReceiverDescriptor {
                spelling: "self: Box<Self>",
                unwrap: Some(Unwrap {
                    scrutinee: "*self",
                    binding: Binding::Owned,
                }),
                handover: Handover::MoveBox,
                lends: Lends::Nothing,
                this: "self",
                mutates: false,
                call: "Box::new({})",
                consumes: true,
            },
            Receiver::PinShared => ReceiverDescriptor {
                spelling: "self: Pin<&Self>",
                unwrap: Some(Unwrap {
                    scrutinee: "Pin::get_ref(self)",
                    binding: Binding::Shared,
                }),
                handover: Handover::Pin,
                lends: Lends::Shared,
                this: "Pin::get_ref(self)",
                mutates: false,
                call: "Pin::new(&{})",
                consumes: false,
            },
            Receiver::PinExclusive => ReceiverDescriptor {
                spelling: "self: Pin<&mut Self>",
                unwrap: Some(Unwrap {
                    scrutinee: "Pin::get_mut(self)",
                    binding: Binding::Exclusive,
                }),
                handover: Handover::PinMutably,
                lends: Lends::Exclusive,
                this: "Pin::get_mut(self)",
                mutates: true,
                call: "Pin::new(&mut {})",
                consumes: false,
            },
            Receiver::Rc => ReceiverDescriptor {
                spelling: "self: Rc<Self>",
                unwrap: None,
                handover: Handover::Move,
                lends: Lends::Nothing,
                this: "self",
                mutates: false,
                call: "Rc::new({})",
                consumes: true,
            },
            Receiver::Arc => ReceiverDescriptor {
                spelling: "self: Arc<Self>",
                unwrap: None,
                handover: Handover::Move,
                lends: Lends::Nothing,
                this: "self",
                mutates: false,
                call: "Arc::new({})",
                consumes: true,
            },
            Receiver::PinBoxed => ReceiverDescriptor {
                spelling: "self: Pin<Box<Self>>",
                unwrap: None,
                handover: Handover::Move,
                lends: Lends::Nothing,
                this: "self",
                mutates: false,
                call: "Box::pin({})",
                consumes: true,
            },
        }
    }
}
