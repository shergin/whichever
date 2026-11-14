//! What every part of a rendered cell reads: the cell's descriptors, and
//! the spellings the trait, the payloads, the subject and the hand share.

use crate::cell::Cell;
use crate::model::argument::{ArgumentDescriptor, ArgumentKind, MismatchSite};
use crate::model::path::{Form as PathForm, PathDescriptor};
use crate::model::qualifier::{Generic, QualifierDescriptor};
use crate::model::receiver::{Lends, ReceiverDescriptor};
use crate::model::returns::{Associated, FutureKind, Node};
use crate::model::target::{Form, Pointer, Storage, TargetDescriptor};
use crate::model::{Receiver, Slot, SlotVariant, Target, Verdict};

/// The trait every cell dispatches.
pub const TRAIT: &str = "Probe";

/// The seed of a slot's fresh payload.
pub fn seed(slot: usize) -> u64 {
    [11, 23][slot]
}

/// The seed of the payload a `Self` argument holds.
pub const OTHER_SEED: u64 = 5;

/// The label every wrapper and marked variant carries beside its payload.
pub const LABEL: u64 = 7;

/// The arguments the law calls the method with: one even, one odd, so an
/// `Option` or a `Result` answer takes both branches.
pub const SAMPLES: [u64; 2] = [2, 3];

/// The cell's descriptors.
#[derive(Clone, Debug)]
pub struct Context {
    /// The cell.
    pub cell: Cell,
    /// Its verdict.
    pub verdict: Verdict,
    /// The return type of the method under test.
    pub node: Node,
    /// Its receiver.
    pub receiver: ReceiverDescriptor,
    /// Its `Self` argument.
    pub argument: ArgumentDescriptor,
    /// Its qualifier.
    pub qualifier: QualifierDescriptor,
    /// The subject's variants.
    pub slots: Vec<Slot>,
    /// How the method reaches the subject.
    pub path: PathDescriptor,
}

/// How an associated type is spelled where a signature is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Associates {
    /// `Self::Out`, in the trait and in its impls.
    Projection,
    /// `<Self as Probe>::Out`, in an inherent impl.
    Qualified,
}

/// Who a signature is written for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Site {
    /// The trait's declaration.
    Trait,
    /// An impl of the trait.
    Impl,
    /// A payload's inherent impl.
    Inherent,
}

impl Context {
    /// Returns the context of a cell.
    pub fn new(cell: &Cell) -> Context {
        Context {
            cell: *cell,
            verdict: cell.verdict(),
            node: cell.ret.node(),
            receiver: cell.recv.descriptor(),
            argument: cell.arg.descriptor(),
            qualifier: cell.qual.descriptor(),
            slots: cell.slots(),
            path: cell.path.descriptor(),
        }
    }

    /// Returns the live slots with their target descriptions.
    pub fn live(&self) -> Vec<(Slot, Target, TargetDescriptor)> {
        self.cell.live_slots()
    }

    /// Returns the control method's receiver: `&self`, or `self` when every
    /// live target is yielded by a route's method, which serves `self` by
    /// value only.
    pub fn base_receiver(&self) -> Receiver {
        let live = self.cell.base_slots();
        let yielded =
            !live.is_empty() && live.iter().all(|(_, _, target)| target.ends_in_temporary());
        match yielded {
            true => Receiver::Owned,
            false => Receiver::Shared,
        }
    }

    /// Returns the control method's signature.
    pub fn base_signature(&self) -> String {
        format!(
            "fn base({}) -> u64",
            self.base_receiver().descriptor().spelling
        )
    }

    /// Returns a call of the control method through the trait as `path`
    /// implements it, on `value`, which it borrows or takes.
    pub fn base_call(&self, path: &str, value: &str) -> String {
        let argument = self.base_receiver().descriptor().call.replace("{}", value);
        format!("<{path} as {TRAIT}>::base({argument})")
    }

    /// Returns whether the subject borrows, so it takes `'a`.
    pub fn borrows(&self) -> bool {
        self.live().iter().any(|(_, _, target)| target.borrows())
    }

    /// Returns the subject's generics: `<'a>` or nothing.
    pub fn generics(&self) -> &'static str {
        if self.borrows() {
            return "<'a>";
        }
        ""
    }

    /// Returns whether the trait method needs `where Self: Sized`: `Self`
    /// is a type argument in the return type, or the body of an `async fn`
    /// or a default holds a `Self` by value.
    pub fn needs_sized(&self) -> bool {
        if self.node.holds_this_below() {
            return true;
        }
        let by_value =
            self.cell.recv == Receiver::Owned || self.argument.kind == Some(ArgumentKind::Owned);
        if self.qualifier.asyncness && (by_value || self.node.holds_this()) {
            return true;
        }
        self.qualifier.provided && by_value
    }

    /// Returns whether a call of the method yields a future to await.
    pub fn awaited(&self) -> bool {
        self.qualifier.asyncness || self.node.future().is_some()
    }

    /// Returns what an awaited call yields, or the answer itself.
    pub fn output(&self) -> Node {
        match self.node.future() {
            Some((_, output)) => *output,
            None => self.node,
        }
    }

    /// Returns the signature of the method under test, without a body or
    /// a semicolon.
    pub fn signature(&self, site: Site) -> String {
        let associates = match site {
            Site::Trait | Site::Impl => Associates::Projection,
            Site::Inherent => Associates::Qualified,
        };
        let mut text = String::new();
        if self.qualifier.asyncness {
            text.push_str("async ");
        }
        if self.qualifier.unsafety {
            text.push_str("unsafe ");
        }
        text.push_str("fn m");
        match self.qualifier.generic {
            Some(Generic::Fixed) => text.push_str("<X: Into<u64>>"),
            Some(Generic::Turbofish) => text.push_str("<W: support::Weight>"),
            None => {}
        }
        text.push('(');
        text.push_str(self.receiver.spelling);
        if let Some(kind) = self.argument.kind {
            text.push_str(", other: ");
            text.push_str(kind.spelling());
        }
        match self.qualifier.generic {
            Some(Generic::Fixed) => text.push_str(", x: X"),
            Some(Generic::Turbofish) | None => text.push_str(", x: u64"),
        }
        text.push(')');
        if self.node != Node::Unit {
            text.push_str(" -> ");
            text.push_str(&spell(&self.node, "Self", associates));
            // An inherent method's opaque return captures no lifetime it
            // does not name, so a borrowing receiver's is named, as the
            // golden `async_returns` writes it.
            let opaque = matches!(self.node.future(), Some((FutureKind::Opaque, _)));
            if site == Site::Inherent && opaque && self.receiver.lends != Lends::Nothing {
                text.push_str(" + '_");
            }
        }
        if self.needs_sized() && site != Site::Inherent {
            text.push_str(" where Self: Sized");
        }
        text
    }

    /// Returns the associated types the trait declares, with whether the
    /// enum overrides each.
    pub fn associated(&self) -> Vec<(Associated, bool)> {
        self.node.associated()
    }

    /// Returns the subject's attribute options after the trait's path.
    pub fn options(&self) -> Vec<String> {
        let mut options = Vec::new();
        for (associated, overridden) in self.associated() {
            if overridden {
                options.push(format!("{} = support::Wide", associated.name()));
            }
        }
        let forwarded = self.path.form == PathForm::Forwarded;
        if forwarded && self.argument.kind.is_some() && self.argument.mismatch == MismatchSite::Enum
        {
            options.push(format!("mismatch(m = {})", self.mismatch("E")));
        }
        options
    }

    /// Returns whether the method under test is an inherent method of the
    /// subject, forwarded to the payloads' own.
    pub fn inherent(&self) -> bool {
        self.path.site == crate::model::path::Site::Inherent
    }

    /// Returns the statement a written body opens with, so that the
    /// arguments it ignores are used: `x`, and the `Self` argument when
    /// `with_other`, which a fallback arm cannot name since the `match`
    /// takes it.
    pub fn unused(&self, with_other: bool) -> &'static str {
        match self.argument.kind.is_some() && with_other {
            true => "let _ = (other, x);",
            false => "let _ = x;",
        }
    }

    /// Returns the mismatch expression, the method's answer for two
    /// different variants, naming the enum by `path`.
    pub fn mismatch(&self, path: &str) -> String {
        let output = self.mismatched_output(path);
        let Some((FutureKind::Boxed, inner)) = self.node.future() else {
            return output;
        };
        let converted = inner
            .conversion()
            .is_some_and(|conversion| !conversion.is_keep());
        match converted {
            true => output,
            false => format!("Box::pin(async move {{ {output} }})"),
        }
    }

    /// Returns the output the mismatch gives once a future is awaited, or
    /// the mismatch itself for any other return type.
    pub fn mismatched_output(&self, path: &str) -> String {
        let payload = "P0::new(424_242)";
        let this = match self.cell.s0.descriptor().form {
            Form::Single => format!("{path}::A({payload})"),
            Form::MarkedTuple => format!("{path}::A({payload}, {LABEL})"),
            Form::MarkedStruct => format!("{path}::A {{ label: {LABEL}, held: {payload} }}"),
        };
        constant(&self.output(), &this)
    }
}

/// Returns a type's spelling, `this` standing for `Self`.
pub fn spell(node: &Node, this: &str, associates: Associates) -> String {
    match node {
        Node::Unit => String::from("()"),
        Node::Tag => String::from("u64"),
        Node::Shared => String::from("&u64"),
        Node::Exclusive => String::from("&mut u64"),
        Node::This => String::from(this),
        Node::Assoc => associated(Associated::Out, this, associates),
        Node::Overridden(name) => associated(*name, this, associates),
        Node::Fault => String::from("support::Fault"),
        Node::Option(inner) => format!("Option<{}>", spell(inner, this, associates)),
        Node::Result(ok, error) => format!(
            "Result<{}, {}>",
            spell(ok, this, associates),
            spell(error, this, associates)
        ),
        Node::Tuple(elements) => {
            let elements: Vec<String> = elements
                .iter()
                .map(|element| spell(element, this, associates))
                .collect();
            format!("({})", elements.join(", "))
        }
        Node::Future(FutureKind::Opaque, output) => {
            format!("impl Future<Output = {}>", spell(output, this, associates))
        }
        Node::Future(FutureKind::Boxed, output) => format!(
            "Pin<Box<dyn Future<Output = {}> + '_>>",
            spell(output, this, associates)
        ),
        Node::Iterator => String::from("impl Iterator<Item = u64>"),
        Node::Vec(inner) => format!("Vec<{}>", spell(inner, this, associates)),
    }
}

fn associated(name: Associated, this: &str, associates: Associates) -> String {
    match associates {
        Associates::Projection => format!("{this}::{}", name.name()),
        Associates::Qualified => format!("<{this} as {TRAIT}>::{}", name.name()),
    }
}

/// Returns a value of the type that needs no payload of the subject: what
/// a mismatch expression is, `this` being a value of `Self`.
pub fn constant(node: &Node, this: &str) -> String {
    match node {
        Node::This => String::from(this),
        Node::Unit => String::from("()"),
        Node::Tag | Node::Assoc => String::from("424_242"),
        Node::Shared => String::from("&support::MISMATCH"),
        Node::Overridden(_) => String::from("support::Wide::MISMATCH"),
        Node::Fault => String::from("support::Fault(424_242)"),
        Node::Option(_) => String::from("None"),
        Node::Result(_, error) => format!("Err({})", constant(error, this)),
        Node::Tuple(elements) => {
            let elements: Vec<String> = elements
                .iter()
                .map(|element| constant(element, this))
                .collect();
            format!("({})", elements.join(", "))
        }
        Node::Exclusive | Node::Future(_, _) | Node::Iterator | Node::Vec(_) => {
            unreachable!("the verdict leaves no mismatch for this type")
        }
    }
}

/// Returns the type a slot's variant stores.
pub fn storage(slot: &Slot, target: &TargetDescriptor) -> String {
    let payload = slot.payload();
    match target.storage {
        Storage::Payload => payload,
        Storage::Pointer(Pointer::Box) => format!("Box<{payload}>"),
        Storage::Pointer(Pointer::Rc) => format!("Rc<{payload}>"),
        Storage::Pointer(Pointer::Arc) => format!("Arc<{payload}>"),
        Storage::Pointer(Pointer::Shared) => format!("&'a {payload}"),
        Storage::Pointer(Pointer::Exclusive) => format!("&'a mut {payload}"),
        Storage::Wrapper => slot.wrapper(),
        Storage::Pair => format!("(u64, {payload})"),
    }
}

/// Returns a variant's declaration, with the attributes the subject
/// spells when `attributes` holds.
pub fn variant(slot: &Slot, attributes: bool) -> (Option<String>, String) {
    let name = slot.name();
    let target = match slot.variant {
        SlotVariant::Unit => return (None, format!("{name},")),
        SlotVariant::Pair => return (None, format!("{name}({}, u64),", slot.payload())),
        SlotVariant::Live(target) => target.descriptor(),
    };
    let stored = storage(slot, &target);
    let field_option = match (attributes, target.option) {
        (false, _) => String::new(),
        (true, Some(option)) => format!("#[dispatch({option})] "),
        (true, None) => String::from("#[dispatch] "),
    };
    match target.form {
        Form::Single => {
            let option = match (attributes, target.option) {
                (true, Some(option)) => Some(format!("#[dispatch({option})]")),
                (true, None) | (false, _) => None,
            };
            (option, format!("{name}({stored}),"))
        }
        Form::MarkedTuple => (None, format!("{name}({field_option}{stored}, u64),")),
        Form::MarkedStruct => (
            None,
            format!("{name} {{ label: u64, {field_option}held: {stored} }},"),
        ),
    }
}

/// Returns the pattern that binds a slot's forwarded field to `binding`,
/// with the subject's path `path`.
pub fn pattern(path: &str, slot: &Slot, target: &TargetDescriptor, binding: &str) -> String {
    let name = slot.name();
    match target.form {
        Form::Single => format!("{path}::{name}({binding})"),
        Form::MarkedTuple => format!("{path}::{name}({binding}, ..)"),
        Form::MarkedStruct => format!("{path}::{name} {{ held: {binding}, .. }}"),
    }
}
