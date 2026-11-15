//! What every part of a rendered cell reads: the cell's descriptors, and
//! the spellings the trait, the payloads, the subject and the hand share.

use crate::cell::Cell;
use crate::model::agreement::AgreementDescriptor;
use crate::model::argument::{ArgumentDescriptor, ArgumentKind, MismatchSite};
use crate::model::generics::GenericsDescriptor;
use crate::model::home::HomeDescriptor;
use crate::model::item::ItemDescriptor;
use crate::model::path::{Form as PathForm, PathDescriptor};
use crate::model::qualifier::{Generic, QualifierDescriptor};
use crate::model::receiver::{Lends, ReceiverDescriptor};
use crate::model::returns::{Associated, FutureKind, Node};
use crate::model::stack::{StackDescriptor, OTHER};
use crate::model::target::{Form, Pointer, Storage, TargetDescriptor};
use crate::model::{Anchor, Callee, Lift, Receiver, Slot, SlotVariant, Target, Verdict};

use super::writer::Writer;

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
    /// The subject's variants in this build.
    pub slots: Vec<Slot>,
    /// Every variant the subject declares, those compiled out included.
    pub declared: Vec<Slot>,
    /// How the method reaches the subject.
    pub path: PathDescriptor,
    /// The attributes on the trait's method under test.
    pub item: ItemDescriptor,
    /// Where the trait and the payloads live.
    pub home: HomeDescriptor,
    /// The generics of the trait or the subject.
    pub generics: GenericsDescriptor,
    /// The trait's own associated items.
    pub agreement: AgreementDescriptor,
    /// The second trait dispatched on the subject.
    pub stack: StackDescriptor,
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
    /// The subject's inherent impl.
    Inherent,
    /// A payload's inherent impl. On an inherent path over an enum, its
    /// `Self` argument is the enum, handed over as it is, which the payload
    /// takes as any implementor of the trait.
    Payload,
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
            declared: cell.declared_slots(),
            path: cell.path.descriptor(),
            item: cell.item.descriptor(),
            home: cell.home.descriptor(),
            generics: cell.generic.descriptor(),
            agreement: cell.agree.descriptor(),
            stack: cell.stack.descriptor(),
        }
    }

    /// Returns the name the method's `u64` argument has where the trait's
    /// signature is repeated: in the trait and in its impls.
    pub fn parameter(&self) -> &'static str {
        self.item.parameter
    }

    /// Returns the attributes a block writes on its method under test
    /// after its mark: the lint the trait allows and the trait method's own
    /// `#[inline(..)]`, as a hand repeating the signature writes them.
    pub fn block_attributes(&self) -> Vec<String> {
        let mut attributes = Vec::new();
        if let Some(lint) = self.item.allowed {
            attributes.push(format!("#[allow({lint})]"));
        }
        if self.item.inline != "inline" {
            attributes.push(format!("#[{}]", self.item.inline));
        }
        attributes
    }

    /// Returns the attributes an item that repeats the trait method's
    /// signature carries: its `#[cfg]`, and the lint the trait allows on
    /// it.
    pub fn repeated_attributes(&self) -> Vec<String> {
        let mut attributes = Vec::new();
        if let Some(predicate) = self.item.cfg {
            attributes.push(format!("#[cfg({predicate})]"));
        }
        if let Some(lint) = self.item.allowed {
            attributes.push(format!("#[allow({lint})]"));
        }
        attributes
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

    /// Returns the second trait's method's receiver: `&self`, or `self`
    /// when every live target is yielded by a route's method, as for the
    /// control method, over every variant.
    pub fn other_receiver(&self) -> Receiver {
        let live = self.live();
        let yielded =
            !live.is_empty() && live.iter().all(|(_, _, target)| target.ends_in_temporary());
        match yielded {
            true => Receiver::Owned,
            false => Receiver::Shared,
        }
    }

    /// Returns the second trait's method's signature.
    pub fn other_signature(&self) -> String {
        format!(
            "fn other({}) -> u64",
            self.other_receiver().descriptor().spelling
        )
    }

    /// Returns a call of the second trait's method as `path` implements it,
    /// on `value`, which it borrows or takes.
    pub fn other_call(&self, path: &str, value: &str) -> String {
        let argument = self.other_receiver().descriptor().call.replace("{}", value);
        format!("<{path} as {OTHER}>::other({argument})")
    }

    /// Returns the second trait's bound on the trait under test, as its
    /// declaration spells it: ` : Probe`, `: Probe<support::Json>`.
    pub fn supertrait(&self) -> String {
        match self.stack.supertrait {
            true => format!(": {}", self.trait_path()),
            false => String::new(),
        }
    }

    /// Returns the second trait's attribute options: the associated items
    /// it fixes as the trait under test does.
    pub fn other_options(&self) -> Vec<String> {
        self.agreement
            .items
            .iter()
            .filter_map(|item| {
                item.overridden
                    .map(|value| format!("{} = {value}", item.name))
            })
            .collect()
    }

    /// Returns the second trait's attribute on the subject.
    pub fn other_attribute(&self) -> String {
        let mut arguments = vec![String::from(OTHER)];
        arguments.extend(self.other_options());
        format!("#[whichever::dispatch({})]", arguments.join(", "))
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
        format!("<{path} as {}>::base({argument})", self.trait_instance())
    }

    /// Returns the trait's parameters as it declares them. A bound fixes
    /// the projection to `u64` where a default body counts the argument.
    pub fn trait_parameters(&self) -> &'static str {
        match (self.generics.fixed, self.qualifier.provided) {
            (true, true) => "<F: support::Format<Value = u64>>",
            (true, false) | (false, _) => self.generics.declared,
        }
    }

    /// Returns the trait as an impl names it, its arguments included:
    /// `Probe<support::Json>`, `Probe<N>`.
    pub fn trait_path(&self) -> String {
        format!("{TRAIT}{}", self.generics.argument)
    }

    /// Returns the trait as the law names it on a value: `Probe<3>`.
    pub fn trait_instance(&self) -> String {
        format!("{TRAIT}{}", self.generics.instance)
    }

    /// Returns the subject at the argument the law takes it at, `E<P0>`,
    /// by `path`.
    pub fn subject_instance(&self, path: &str) -> String {
        match self.generics.parameter {
            Some(parameter) => format!("{path}<{}>", parameter.instance),
            None => String::from(path),
        }
    }

    /// Returns the generics an impl of the trait for a payload declares:
    /// the trait's const parameter, which every payload implements it at.
    pub fn payload_generics(&self) -> &'static str {
        match self.generics.constant {
            true => "<const N: u64>",
            false => "",
        }
    }

    /// Returns the subject's generics as an impl declares them, with the
    /// bound a hand writes: `<'a>`, `<T: Probe>`.
    pub fn impl_generics(&self) -> String {
        self.generics_with(|parameter| parameter.bounded)
    }

    /// Returns the subject's generics as its type names them: `<'a>`,
    /// `<T>`.
    pub fn type_generics(&self) -> String {
        self.generics_with(|parameter| parameter.named)
    }

    fn generics_with(
        &self,
        spell: impl Fn(&crate::model::generics::Parameter) -> &'static str,
    ) -> String {
        let mut parameters: Vec<&str> = Vec::new();
        if self.borrows() {
            parameters.push("'a");
        }
        if let Some(parameter) = &self.generics.parameter {
            parameters.push(spell(parameter));
        }
        match parameters.is_empty() {
            true => String::new(),
            false => format!("<{}>", parameters.join(", ")),
        }
    }

    /// Returns whether the subject borrows, so it takes `'a`.
    pub fn borrows(&self) -> bool {
        self.live().iter().any(|(_, _, target)| target.borrows())
    }

    /// Returns the subject's generics as it declares them: `<'a>`, `<T>`,
    /// `<const N: u64>`, or nothing.
    pub fn generics(&self) -> String {
        self.generics_with(|parameter| parameter.declared)
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
        self.signature_with(site, self.node, "m")
    }

    /// Returns whether the method takes the argument `x`: an inherent
    /// method that reads a place takes none, since nothing would use it.
    pub fn takes_x(&self) -> bool {
        !(self.inherent() && matches!(self.cell.callee, Callee::Value | Callee::Field))
    }

    /// Returns the name of the payload's method an inherent arm calls.
    pub fn callee_name(&self) -> &'static str {
        match self.cell.callee {
            Callee::Renamed => "answer",
            Callee::Method | Callee::Value | Callee::Field => "m",
        }
    }

    /// Returns the signature the subject's inherent method declares: the
    /// declared type, after the lift.
    pub fn declared_signature(&self) -> String {
        format!(
            "pub {}",
            self.signature_as(
                Site::Inherent,
                self.cell.declared_node(),
                "m",
                self.declared_this()
            )
        )
    }

    /// Returns how the subject's inherent method spells `Self` in its return
    /// type: by the enum's name where `with = Into::into` rebuilds the enum
    /// from the payload's own type, as `fn into_static(self) ->
    /// PlayableId<'static>` does (inherent-methods.md), and `Self`
    /// otherwise.
    ///
    /// Open question: under `-> Self` the rule that "`with` lifts the
    /// wrapped variant" would rewrap the payload's answer before converting
    /// it, which is no rebuild; the docs' example names the enum, and so
    /// does the model.
    pub fn declared_this(&self) -> &'static str {
        match self.cell.lift {
            Lift::Into => "E",
            Lift::Bare | Lift::Some | Lift::Ok | Lift::ByRef | Lift::ByValue => "Self",
        }
    }

    /// Returns whether the method's `Self` argument is the enum handed over
    /// as it is: an inherent method's over an enum (inherent-methods.md).
    pub fn hands_the_enum(&self) -> bool {
        self.inherent() && self.argument.kind.is_some() && !is_struct(&self.slots)
    }

    /// Returns the constant of the declared type an inherent method's body
    /// answers for the variants its mark leaves out.
    pub fn declared_constant(&self) -> String {
        constant(&self.cell.declared_node(), "E::A(P0::new(424_242))")
    }

    /// Returns the options an inherent method's `#[dispatch(..)]` takes
    /// after the variants it names.
    pub fn inherent_options(&self) -> String {
        let callee = match self.cell.callee {
            Callee::Method => "",
            Callee::Renamed => ", call = answer",
            Callee::Value => ", value",
            Callee::Field => ", field = state",
        };
        let lift = match self.cell.lift {
            Lift::Bare => "",
            Lift::Some => ", with = Some",
            Lift::Ok => ", with = Ok",
            Lift::ByRef => ", with = support::by_ref",
            Lift::ByValue => ", with = support::by_value",
            Lift::Into => ", with = Into::into",
        };
        format!("{callee}{lift}")
    }

    /// Returns a signature of the method under test, named `name` and
    /// returning `node`.
    pub fn signature_with(&self, site: Site, node: Node, name: &str) -> String {
        self.signature_as(site, node, name, "Self")
    }

    /// Returns a signature of the method under test, named `name` and
    /// returning `node`, with `Self` in the return type spelled `this`.
    pub fn signature_as(&self, site: Site, node: Node, name: &str, this: &str) -> String {
        let associates = match site {
            Site::Trait | Site::Impl => Associates::Projection,
            Site::Inherent | Site::Payload => Associates::Qualified,
        };
        let mut text = String::new();
        if self.qualifier.asyncness {
            text.push_str("async ");
        }
        if self.qualifier.unsafety {
            text.push_str("unsafe ");
        }
        text.push_str("fn ");
        text.push_str(name);
        let mut parameters: Vec<&str> = Vec::new();
        match self.qualifier.generic {
            Some(Generic::Fixed) => parameters.push("X: Into<u64>"),
            Some(Generic::Turbofish) => parameters.push("W: support::Weight"),
            None => {}
        }
        let enum_argument = site == Site::Payload && self.hands_the_enum();
        if enum_argument {
            parameters.push("O: Probe");
        }
        if !parameters.is_empty() {
            text.push_str(&format!("<{}>", parameters.join(", ")));
        }
        text.push('(');
        text.push_str(self.receiver.spelling);
        if let Some(kind) = self.argument.kind {
            text.push_str(", other: ");
            match enum_argument {
                true => text.push_str(&kind.spelling().replace("Self", "O")),
                false => text.push_str(kind.spelling()),
            }
        }
        if self.takes_x() {
            let name = match site {
                Site::Trait | Site::Impl => self.parameter(),
                Site::Inherent | Site::Payload => "x",
            };
            // The trait names the argument's type as its home spells it, and
            // an impl by the alias its module imports.
            let ty = match (site, self.home.argument) {
                (Site::Trait, _) if self.generics.projected => "F::Value",
                (Site::Trait, argument) => argument,
                (Site::Impl, "u64") | (Site::Inherent | Site::Payload, _) => "u64",
                (Site::Impl, _) => "support::Count",
            };
            match self.qualifier.generic {
                Some(Generic::Fixed) => text.push_str(&format!(", {name}: X")),
                Some(Generic::Turbofish) | None => text.push_str(&format!(", {name}: {ty}")),
            }
        }
        text.push(')');
        if node != Node::Unit {
            text.push_str(" -> ");
            text.push_str(&spell(&node, this, associates));
            // An inherent method's opaque return captures no lifetime it
            // does not name, so a borrowing receiver's is named, as the
            // golden `async_returns` writes it.
            let opaque = matches!(node.future(), Some((FutureKind::Opaque, _)));
            let inherent = matches!(site, Site::Inherent | Site::Payload);
            if inherent && opaque && self.receiver.lends != Lends::Nothing {
                text.push_str(" + '_");
            }
        }
        let declared = matches!(site, Site::Trait | Site::Impl);
        if self.needs_sized() && declared {
            text.push_str(" where Self: Sized");
        }
        text
    }

    /// Returns the associated types the trait declares, with whether the
    /// enum overrides each.
    pub fn associated(&self) -> Vec<(Associated, bool)> {
        self.node.associated()
    }

    /// Returns the payload type an associated item is taken from, as the
    /// subject declares it: the supplier's, or the first's where no variant
    /// supplies it and the verdict reports so.
    pub fn supplier_payload(&self) -> String {
        self.cell
            .supplier()
            .map_or_else(|| String::from("P0"), |slot| slot.declared_payload())
    }

    /// Returns the subject's attribute options after the trait's path.
    pub fn options(&self) -> Vec<String> {
        let mut options = Vec::new();
        for (associated, overridden) in self.associated() {
            if overridden {
                options.push(format!("{} = support::Wide", associated.name()));
            }
        }
        for item in self.agreement.items {
            if let Some(value) = item.overridden {
                options.push(format!("{} = {value}", item.name));
            }
        }
        // An inherent method has no mismatch: its `Self` argument is the
        // enum, handed over as it is.
        // Two variants of which one holds the subject's type parameter
        // would get overlapping `From` impls, so the cell leaves them out,
        // as the golden `generic_payloads` does.
        //
        // Open question: whether a type-parameter payload beside another
        // gets `From` impls is not documented.
        let live = self
            .declared
            .iter()
            .filter(|slot| slot.target().is_some())
            .count();
        if self.generics.payload && live > 1 {
            options.push(String::from("from = false"));
        }
        if let Some(option) = self.cell.conv.descriptor().option {
            options.push(String::from(option));
        }
        let forwarded = self.path.form == PathForm::Forwarded && !self.inherent();
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
    /// arguments it ignores are used: `x`, by the name `site` gives it,
    /// and the `Self` argument when `with_other`, which a fallback arm
    /// cannot name since the `match` takes it.
    pub fn unused(&self, site: Site, with_other: bool) -> String {
        if !self.takes_x() {
            return String::new();
        }
        let name = match site {
            Site::Trait | Site::Impl => self.parameter(),
            Site::Inherent | Site::Payload => "x",
        };
        match self.argument.kind.is_some() && with_other {
            true => format!("let _ = (other, {name});"),
            false => format!("let _ = {name};"),
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
            Form::Struct => format!("{path}({payload})"),
            Form::StructMarked => format!("{path} {{ label: {LABEL}, held: {payload} }}"),
        };
        constant(&self.output(), &this)
    }
}

/// Returns a type's spelling, `this` standing for `Self`.
pub fn spell(node: &Node, this: &str, associates: Associates) -> String {
    match node {
        Node::Payload => String::from("P0"),
        Node::PayloadShared => String::from("&P0"),
        Node::PayloadExclusive => String::from("&mut P0"),
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
        Node::Exclusive
        | Node::Payload
        | Node::PayloadShared
        | Node::PayloadExclusive
        | Node::Future(_, _)
        | Node::Iterator
        | Node::Vec(_) => {
            unreachable!("the verdict leaves no mismatch for this type")
        }
    }
}

/// Returns the type a slot's variant stores.
pub fn storage(slot: &Slot, target: &TargetDescriptor) -> String {
    let payload = slot.declared_payload();
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
        Form::Struct | Form::StructMarked => {
            unreachable!("a struct subject is written by `write_struct`")
        }
    }
}

/// Writes a struct subject of one slot, its fields public so a sibling
/// module can build it. The forwarded field has a line of its own, the
/// anchor of the slot when `anchored` holds, since a sentence about the
/// field sits on its type.
pub fn write_struct(
    writer: &mut Writer,
    slot: &Slot,
    generics: &str,
    attributes: bool,
    anchored: bool,
) {
    let Some((_, target)) = slot.target() else {
        unreachable!("a struct subject holds a live target");
    };
    let stored = storage(slot, &target);
    if anchored {
        writer.anchor(Anchor::Enum.name());
    }
    match target.form {
        Form::Struct => {
            writer.open(format!("pub struct E{generics}("));
            if anchored {
                writer.anchor(Anchor::Variant(slot.index).name());
            }
            writer.line(format!("pub {stored},"));
            writer.close(");");
        }
        Form::StructMarked => {
            let mark = match (attributes, target.option) {
                (false, _) => "",
                (true, Some(_)) => unreachable!("the marked struct field takes no option"),
                (true, None) => "#[dispatch] ",
            };
            writer.open(format!("pub struct E{generics} {{"));
            writer.line("pub label: u64,");
            if anchored {
                writer.anchor(Anchor::Variant(slot.index).name());
            }
            writer.line(format!("{mark}pub held: {stored},"));
            writer.close("}");
        }
        Form::Single | Form::MarkedTuple | Form::MarkedStruct => {
            unreachable!("an enum subject is written variant by variant")
        }
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
        Form::Struct => format!("{path}({binding}, ..)"),
        Form::StructMarked => format!("{path} {{ held: {binding}, .. }}"),
    }
}

/// Returns the constructor a rewrap calls: the variant's path, or the
/// struct's name.
pub fn constructor(path: &str, slot: &Slot) -> String {
    let is_struct = slot.target().is_some_and(|(_, target)| target.is_struct());
    match is_struct {
        true => String::from(path),
        false => format!("{path}::{}", slot.name()),
    }
}

/// Returns whether the subject is a struct.
pub fn is_struct(slots: &[Slot]) -> bool {
    slots
        .first()
        .and_then(Slot::target)
        .is_some_and(|(_, target)| target.is_struct())
}
