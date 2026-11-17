//! What every part of a rendered cell reads: the cell's descriptors, and
//! the spellings the trait, the payloads, the subject and the hand share.

use crate::cell::Cell;
use crate::model::agreement::AgreementDescriptor;
use crate::model::argument::{ArgumentDescriptor, ArgumentKind, MismatchSite};
use crate::model::declaration::{DeclarationDescriptor, Foreign, REAL};
use crate::model::generics::GenericsDescriptor;
use crate::model::home::HomeDescriptor;
use crate::model::item::ItemDescriptor;
use crate::model::path::{Form as PathForm, PathDescriptor};
use crate::model::qualifier::{Generic, QualifierDescriptor};
use crate::model::receiver::{Lends, ReceiverDescriptor};
use crate::model::returns::{Associated, FutureKind, Node};
use crate::model::stack::{StackDescriptor, OTHER};
use crate::model::target::{Form, Pointer, Storage, TargetDescriptor};
use crate::model::{
    Anchor, Callee, Lift, OwnArm, Owner, Receiver, Slot, SlotVariant, Target, Verdict,
};

use super::writer::Writer;

/// The trait every cell dispatches.
pub const TRAIT: &str = "Probe";

/// The lifetime `#[async_trait]` names a borrowing receiver's by.
pub const RECEIVER_LIFETIME: &str = "'life0";

/// The lifetime `#[async_trait]` names a borrowed `Self` argument's by.
pub const ARGUMENT_LIFETIME: &str = "'life1";

/// The lifetime of a boxed future, as `#[async_trait]` names it.
pub const FUTURE_LIFETIME: &str = "'async_trait";

/// The seed of a slot's fresh payload.
pub fn seed(slot: usize) -> u64 {
    [11, 23, 37][slot]
}

/// The seed of the payload a `Self` argument holds.
pub const OTHER_SEED: u64 = 5;

/// What the enum declares its generic associated type as, which every
/// payload's own converts into. Its parameter is `'v`, which no subject
/// declares, since an enum over `&'a P0` names `'a` already.
pub const VIEW: &str = "support::View<'v>";

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
    /// How the trait under test is declared.
    pub declaration: DeclarationDescriptor,
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
            path: cell.path_descriptor(),
            item: cell.item.descriptor(),
            home: cell.home.descriptor(),
            generics: cell.generic.descriptor(),
            agreement: cell.agree.descriptor(),
            stack: cell.stack.descriptor(),
            declaration: cell.decl.descriptor(),
        }
    }

    /// Returns the name the method's `u64` argument has where the trait's
    /// signature is repeated: in the trait and in its impls.
    pub fn parameter(&self) -> &'static str {
        self.item.parameter
    }

    /// Returns the attributes a block writes on its method under test
    /// after its mark: the lint the trait allows and the trait method's own
    /// `#[inline(..)]`, as a hand repeating the signature writes them. "A
    /// method written or marked in the block is yours, and so are its
    /// attributes: it carries what you write on it, with `#[inline]` added
    /// where you write no `#[inline(..)]`" (partial-impl-blocks.md).
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
        let mut options: Vec<String> = self
            .agreement
            .items
            .iter()
            .filter_map(|item| {
                item.overridden
                    .map(|value| format!("{} = {value}", item.name))
            })
            .collect();
        options.extend(self.cell.conv.descriptor().other_option.map(String::from));
        options
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

    /// Returns the signature of the method that returns the trait's own
    /// associated type, with the control method's receiver, where the
    /// trait declares it: `fn kind(&self) -> Self::Kind`, or the type a
    /// payload gives it.
    pub fn kind_signature(&self, ty: &str) -> String {
        format!(
            "fn kind({}) -> {ty}",
            self.base_receiver().descriptor().spelling
        )
    }

    /// Returns a call of the method returning the trait's own associated
    /// type through the trait as `path` implements it, on `value`.
    pub fn kind_call(&self, path: &str, value: &str) -> String {
        let argument = self.base_receiver().descriptor().call.replace("{}", value);
        format!("<{path} as {}>::kind({argument})", self.trait_instance())
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
        if let Some(foreign) = self.declaration.foreign {
            return String::from(foreign.path());
        }
        format!("{TRAIT}{}", self.generics.argument)
    }

    /// Returns the trait as its impl's home spells it, the subject's
    /// attribute or a partial block's header: as an impl names it, or with
    /// one more path segment, `self::Probe`, where the struct's field
    /// spells it as an impl names it, one name spelled two ways.
    pub fn home_path(&self) -> String {
        match self.cell.s0.home_prefix() {
            Some(prefix) => format!("{prefix}::{}", self.trait_path()),
            None => self.trait_path(),
        }
    }

    /// Returns how an impl of the trait under test begins: `unsafe impl`
    /// for an `unsafe trait` (what-is-generated.md), `impl` otherwise.
    pub fn impl_keyword(&self) -> &'static str {
        match self.declaration.unsafety {
            true => "unsafe impl",
            false => "impl",
        }
    }

    /// Returns the bound the trait under test declares on itself: `:
    /// Other` where it requires the second trait.
    pub fn required(&self) -> &'static str {
        match self.stack.required {
            true => ": Other",
            false => "",
        }
    }

    /// Returns the `#[dispatch(..)]` on the method under test in a partial
    /// block: the path's, or, where the method names its target and the
    /// struct's field names the trait, the struct's other field, the label,
    /// which a sentence then says the trait is not forwarded to
    /// (a-fields-trait-may-be-a-partial-blocks.md).
    pub fn block_mark(&self) -> Option<&'static str> {
        let named = self.cell.s0.descriptor().trait_on_field().is_some();
        match (self.path.form, named) {
            (crate::model::path::Form::First, true) => Some("label"),
            _ => self.path.mark,
        }
    }

    /// Returns whether the trait under test is a restatement
    /// `whichever::foreign` ships.
    pub fn foreign(&self) -> bool {
        self.declaration.foreign.is_some()
    }

    /// Returns the restatement `whichever::foreign` ships that the trait
    /// under test is, if it is one.
    pub fn foreign_trait(&self) -> Option<Foreign> {
        self.declaration.foreign
    }

    /// Returns the module of the cell the trait is declared in, beside the
    /// subject: a sibling module, or, for a trait restated beside the
    /// subject, the module the real trait is declared in without the
    /// attribute; `None` where it is the cell's own or another crate's.
    pub fn trait_module(&self) -> Option<&'static str> {
        if self.home.module.is_some() {
            return self.home.module;
        }
        (self.declaration.restated && !self.home.foreign).then_some(REAL)
    }

    /// Returns the path a restatement names the real trait by: in a module
    /// of the cell, or at the cell's module in the trait's crate.
    pub fn real_trait(&self) -> String {
        if let Some(module) = self.trait_module() {
            return format!("{module}::{TRAIT}");
        }
        format!("{}::{}::{TRAIT}", self.trait_crate(), self.cell.module())
    }

    /// Returns the name of the crate the trait and the payloads live in
    /// where they live in a crate of their own.
    pub fn trait_crate(&self) -> String {
        crate::run::workspace::crate_name(
            &crate::run::workspace::ShardKind::Traits.shard(self.cell.hash()),
        )
    }

    /// Returns the signature of the method under test as a restatement
    /// spells it: the trait's, with a type the trait's crate names by
    /// `crate::` spelled by that crate's name, since the restatement's
    /// `crate::` is the subject's (restatements.md: "write the signature
    /// types in full").
    pub fn restated_signature(&self) -> String {
        let signature = self.signature(Site::Trait);
        match self.home.foreign {
            true => signature.replace("crate::", &format!("{}::", self.trait_crate())),
            false => signature,
        }
    }

    /// Returns the trait as the law names it on a value: `Probe<3>`.
    pub fn trait_instance(&self) -> String {
        format!("{TRAIT}{}", self.generics.instance)
    }

    /// Returns the subject at the argument the law takes it at, `E<P0>`,
    /// by `path`.
    pub fn subject_instance(&self, path: &str) -> String {
        if self.cell.owner == Owner::Remote {
            let payloads: Vec<String> = self
                .declared
                .iter()
                .filter(|slot| slot.parameter.is_some())
                .map(Slot::payload)
                .collect();
            return format!("{path}<{}>", payloads.join(", "));
        }
        match self.generics.parameter {
            Some(parameter) => format!("{path}<{}>", parameter.instance),
            None => String::from(path),
        }
    }

    /// Returns the generics an impl of the trait for a payload declares:
    /// the trait's const parameter, which every payload implements it at.
    pub fn payload_generics(&self) -> String {
        if let Some(header) = self.generics.header {
            return format!("<{header}>");
        }
        match self.generics.constant {
            true => String::from("<const N: u64>"),
            false => String::new(),
        }
    }

    /// Returns the subject's generics as an impl of the trait under test
    /// declares them, with the bound a hand writes, `<'a>`, `<T: Probe>`,
    /// and the parameter a block's own header declares beside them; an
    /// impl for one instantiation declares none of the subject's.
    pub fn impl_generics(&self) -> String {
        if self.generics.instantiated {
            return String::new();
        }
        let parameters =
            self.generics_with(|parameter| parameter.bounded, Some(&self.trait_path()));
        let Some(header) = self.generics.header else {
            return parameters;
        };
        match parameters.strip_suffix('>') {
            Some(open) => format!("{open}, {header}>"),
            None => format!("<{header}>"),
        }
    }

    /// Returns the subject's type as an impl of the trait under test names
    /// it: with its parameters, or at the instantiation a block's header
    /// implements, `E<P0>`.
    pub fn impl_type_generics(&self) -> String {
        match self.instance() {
            Some(instance) => format!("<{instance}>"),
            None => self.type_generics(),
        }
    }

    /// Returns the type an arm names a payload by where the route names
    /// it: the instantiation's own where the impl is for one.
    pub fn instance(&self) -> Option<&'static str> {
        self.generics
            .parameter
            .filter(|_| self.generics.instantiated)
            .map(|parameter| parameter.instance)
    }

    /// Returns the subject's generics as an impl of the second trait
    /// declares them: a restated enum's payload parameters bounded by it.
    pub fn other_impl_generics(&self) -> String {
        self.generics_with(|parameter| parameter.bounded, Some(OTHER))
    }

    /// Returns the subject's generics as its type names them: `<'a>`,
    /// `<T>`.
    pub fn type_generics(&self) -> String {
        self.generics_with(|parameter| parameter.named, None)
    }

    /// Returns the subject's generics, each parameter spelled by `spell`,
    /// and a restated enum's payload parameters bounded by `bound` where it
    /// is given, as the golden `restated_enum` bounds them.
    fn generics_with(
        &self,
        spell: impl Fn(&crate::model::generics::Parameter) -> &'static str,
        bound: Option<&str>,
    ) -> String {
        let mut parameters: Vec<String> = Vec::new();
        if self.borrows() {
            parameters.push(String::from("'a"));
        }
        if let Some(parameter) = &self.generics.parameter {
            parameters.push(String::from(spell(parameter)));
        }
        if self.cell.owner == Owner::Remote {
            for name in self.declared.iter().filter_map(|slot| slot.parameter) {
                match bound {
                    Some(bound) => parameters.push(format!("{name}: {bound}")),
                    None => parameters.push(String::from(name)),
                }
            }
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
        self.generics_with(|parameter| parameter.declared, None)
    }

    /// Returns whether the trait method needs `where Self: Sized`: the
    /// method declares it, `Self` is a type argument in the return type, or
    /// the body of an `async fn` or a default holds a `Self` by value.
    pub fn needs_sized(&self) -> bool {
        self.cell.requires_sized()
    }

    /// Returns how a default body names the `Self` it owns and may answer:
    /// `self` taken by value, or what the box it is taken in holds; `None`
    /// where it borrows its receiver.
    pub fn owned_self(&self) -> Option<&'static str> {
        match self.cell.recv {
            Receiver::Owned => Some("self"),
            Receiver::Boxed => Some("*self"),
            Receiver::Shared
            | Receiver::Exclusive
            | Receiver::PinShared
            | Receiver::PinExclusive
            | Receiver::Rc
            | Receiver::Arc
            | Receiver::PinBoxed => None,
        }
    }

    /// Returns whether a call of the method yields a future to await.
    pub fn awaited(&self) -> bool {
        self.qualifier.asyncness || self.node.future().is_some()
    }

    /// Returns whether a call of the subject's inherent method yields a
    /// future to await: its declared type is one, which under a function
    /// lift it is not although the payload's method returns one.
    pub fn declared_awaited(&self) -> bool {
        self.qualifier.asyncness || self.cell.declared_node().future().is_some()
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
            Callee::Method | Callee::Value | Callee::Field | Callee::Borrowing => "m",
        }
    }

    /// Returns the context the payload's own method is written and called
    /// in: the cell's, except that a method that borrows its receiver takes
    /// `&self`, and answers for its own borrow where the subject's method
    /// answers for the subject's lifetime, which the payload cannot name.
    pub fn payload_side(&self) -> Context {
        let mut context = self.clone();
        if self.cell.callee != Callee::Borrowing {
            return context;
        }
        context.receiver = Receiver::Shared.descriptor();
        if self.node == Node::Lent {
            context.node = Node::Shared;
        }
        context
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
                &self.declared_this()
            )
        )
    }

    /// Returns how the subject's inherent method spells `Self` in its return
    /// type: by the enum's name and its generics where `with = Into::into`
    /// rebuilds the enum from the payload's own type, as `fn
    /// into_static(self) -> PlayableId<'static>` does (inherent-methods.md),
    /// and `Self` otherwise.
    ///
    /// Open question: under `-> Self` the rule that "`with` lifts the
    /// wrapped variant" would rewrap the payload's answer before converting
    /// it, which is no rebuild; the docs' example names the enum, and so
    /// does the model.
    pub fn declared_this(&self) -> String {
        match self.cell.lift {
            Lift::Into => format!("E{}", self.generics()),
            Lift::Bare | Lift::Some | Lift::Ok | Lift::ByRef | Lift::ByValue => {
                String::from("Self")
            }
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

    /// Returns whether an arm of the method's own over a slot reads the
    /// variant's field: a variant of two fields, where the method answers a
    /// number, answers its label, so that the law tells it from the
    /// constant and from what the payload answers.
    pub fn own_arm_reads_field(&self, slot: &Slot) -> bool {
        let node = match self.inherent() {
            true => self.cell.declared_node(),
            false => self.node,
        };
        slot.variant == SlotVariant::Pair && node == Node::Tag
    }

    /// Returns an arm of the method's own, as the body under `*` writes it
    /// and the hand writes it after the forwarding arms: the variant's and
    /// the arm's `#[cfg]`, the variant's allowance of its deprecation, the
    /// pattern by `path`, and the answer, a constant of the declared type or
    /// the label a variant of two fields holds.
    pub fn own_arm(&self, arm: &OwnArm, path: &str) -> String {
        let slot = &arm.slot;
        let name = slot.name();
        let reads = self.own_arm_reads_field(slot);
        let pattern = match slot.variant {
            SlotVariant::Unit => format!("{path}::{name}"),
            SlotVariant::Pair if reads => format!("{path}::{name}(_, label)"),
            SlotVariant::Pair => format!("{path}::{name}(..)"),
            SlotVariant::Live(target) => match target.descriptor().form {
                Form::MarkedStruct | Form::MarkedNamed => format!("{path}::{name} {{ .. }}"),
                Form::Single
                | Form::MarkedTuple
                | Form::MarkedLast
                | Form::Struct
                | Form::StructMarked
                | Form::StructNamed
                | Form::StructMarkedLast => format!("{path}::{name}(..)"),
            },
        };
        let mut attributes = String::new();
        if let Some(predicate) = slot.cfg {
            attributes.push_str(&format!("#[cfg({predicate})] "));
        }
        if let Some(predicate) = arm.cfg {
            attributes.push_str(&format!("#[cfg({predicate})] "));
        }
        if slot.deprecated {
            attributes.push_str("#[allow(deprecated)] ");
        }
        let (unused, constant) = match self.inherent() {
            true => (
                self.unused(Site::Inherent, self.hands_the_enum()),
                self.declared_constant(),
            ),
            false => (self.unused(Site::Impl, false), self.mismatch("E")),
        };
        // A `u64` label is read the same under every binding mode, by
        // value or through a reference, by `clone`. An arm compiled out of
        // this build is never type-checked, and answers what names nothing
        // the cell imports only for it.
        let compiled_out = slot.cfg == Some("any()") || arm.cfg == Some("any()");
        let answer = match (compiled_out, reads) {
            (true, _) => String::from("::core::unreachable!()"),
            (false, true) => String::from("label.clone()"),
            (false, false) => constant,
        };
        format!("{attributes}{pattern} => {{ {unused} {answer} }},")
    }

    /// Returns the options an inherent method's `#[dispatch(..)]` takes
    /// after the variants it names.
    pub fn inherent_options(&self) -> String {
        let callee = match self.cell.callee {
            Callee::Method | Callee::Borrowing => "",
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
    ///
    /// A boxed future is spelled as `#[async_trait]` writes it, and as the
    /// golden `boxed_futures` does: `Pin<Box<dyn Future<Output = T> + Send
    /// + 'async_trait>>`, every reference the method takes named, `'life0`
    /// for the receiver and `'life1` for a `Self` argument, each bound to
    /// outlive the future, as are `Self` and the method's type parameters.
    pub fn signature_as(&self, site: Site, node: Node, name: &str, this: &str) -> String {
        let associates = match site {
            Site::Trait | Site::Impl => Associates::Projection,
            Site::Inherent | Site::Payload => Associates::Qualified,
        };
        let boxed = matches!(node.future(), Some((FutureKind::Boxed, _)));
        let mut text = String::new();
        if self.qualifier.asyncness {
            text.push_str("async ");
        }
        if self.qualifier.unsafety {
            text.push_str("unsafe ");
        }
        text.push_str("fn ");
        text.push_str(name);
        let receiver = match (boxed, self.receiver.lifetime) {
            (true, Some(named)) => named,
            (true, None) | (false, _) => self.receiver.spelling,
        };
        let enum_argument = site == Site::Payload && self.hands_the_enum();
        let argument = self.argument.kind.map(|kind| {
            let spelled = match boxed {
                true => kind.lifetime_spelling(),
                false => kind.spelling(),
            };
            match enum_argument {
                true => spelled.replace("Self", "O"),
                false => String::from(spelled),
            }
        });
        let mut parameters: Vec<String> = Vec::new();
        let mut bounds: Vec<String> = Vec::new();
        if boxed {
            if self.receiver.lifetime.is_some() {
                parameters.push(String::from(RECEIVER_LIFETIME));
                bounds.push(format!("{RECEIVER_LIFETIME}: {FUTURE_LIFETIME}"));
            }
            if self.argument.kind.is_some_and(ArgumentKind::borrows) {
                parameters.push(String::from(ARGUMENT_LIFETIME));
                bounds.push(format!("{ARGUMENT_LIFETIME}: {FUTURE_LIFETIME}"));
            }
            parameters.push(String::from(FUTURE_LIFETIME));
        }
        // A converted boxed future holds the arguments in its `async move`
        // block, so a type parameter it holds is `Send`, as a trait under
        // `#[async_trait]` declares it.
        let (generic, generic_bound) = match self.qualifier.generic {
            Some(Generic::Fixed) if boxed => (Some("X"), "Into<u64> + Send"),
            Some(Generic::Fixed) => (Some("X"), "Into<u64>"),
            Some(Generic::Turbofish) => (Some("W"), "support::Weight"),
            None => (None, ""),
        };
        if let Some(generic) = generic {
            parameters.push(format!("{generic}: {generic_bound}"));
            if boxed {
                bounds.push(format!("{generic}: {FUTURE_LIFETIME}"));
            }
        }
        if enum_argument {
            parameters.push(format!("O: {TRAIT}"));
            if boxed {
                bounds.push(format!("O: {FUTURE_LIFETIME}"));
            }
        }
        if boxed {
            bounds.push(format!("Self: {FUTURE_LIFETIME}"));
        }
        if !parameters.is_empty() {
            text.push_str(&format!("<{}>", parameters.join(", ")));
        }
        text.push('(');
        text.push_str(receiver);
        if let Some(argument) = argument {
            text.push_str(", other: ");
            text.push_str(&argument);
        }
        if self.takes_x() {
            let name = match site {
                Site::Trait | Site::Impl => self.parameter(),
                Site::Inherent | Site::Payload => "x",
            };
            // The trait names the argument's type as its home spells it, and
            // an impl by the alias its module imports.
            let ty = match (site, self.home.argument) {
                (Site::Trait, _) if self.generics.projected => self.generics.projection,
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
            bounds.push(String::from("Self: Sized"));
        }
        if !bounds.is_empty() {
            text.push_str(&format!(" where {}", bounds.join(", ")));
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
        let Some(slot) = self.cell.supplier() else {
            return String::from("P0");
        };
        match (slot.parameter, self.instance()) {
            (Some(_), Some(instance)) => String::from(instance),
            (Some(_), None) | (None, _) => slot.declared_payload(),
        }
    }

    /// Returns the options of the enum's own, which go on its attribute
    /// even where a partial block is the impl (partial-impl-blocks.md):
    /// the conversions.
    pub fn enum_options(&self) -> Vec<String> {
        let mut options = Vec::new();
        let live = self
            .declared
            .iter()
            .filter(|slot| slot.target().is_some())
            .count();
        let unchecked = !self.cell.conv.descriptor().checked;
        if self.generics.payload && live > 1 && unchecked {
            options.push(String::from("from = false"));
        }
        if let Some(option) = self.cell.conv.descriptor().option {
            options.push(String::from(option));
        }
        options
    }

    /// Returns the options of the trait's, which a partial block's
    /// attribute takes where the block is the impl: the overrides and the
    /// mismatch.
    pub fn trait_options(&self) -> Vec<String> {
        let enum_options = self.enum_options();
        self.options()
            .into_iter()
            .filter(|option| !enum_options.contains(option))
            .collect()
    }

    /// Returns the subject's attribute options after the trait's path.
    pub fn options(&self) -> Vec<String> {
        let mut options = Vec::new();
        // A generic associated type is no option: only a block declares it
        // (`gat_override_option`).
        for (associated, overridden) in self.associated() {
            if overridden && associated != Associated::Item {
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
        // would get overlapping `From` impls, the sentence of
        // `from_overlap_type_parameter`, so a cell that does not check its
        // conversions leaves them out, as the golden `generic_payloads`
        // does; one that checks them spells its own.
        let live = self
            .declared
            .iter()
            .filter(|slot| slot.target().is_some())
            .count();
        let unchecked = !self.cell.conv.descriptor().checked;
        if self.generics.payload && live > 1 && unchecked {
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
        constant(&self.output(), &self.fresh(path))
    }

    /// Returns the first variant built around a fresh payload, by `path`,
    /// which a mismatch or a body answers for `Self`: what the variant
    /// stores is built around the payload, a pointer by its constructor and
    /// a reference by leaking a box, which code with no place of its own
    /// may do.
    pub fn fresh(&self, path: &str) -> String {
        let slot = &self.declared[0];
        let target = self.cell.s0.descriptor();
        let payload = String::from("P0::new(424_242)");
        let wrapper = format!(
            "{} {{ label: {LABEL}, payload: {payload} }}",
            slot.wrapper()
        );
        let pointed = |pointer: Pointer, value: String| match pointer {
            Pointer::Box => format!("Box::new({value})"),
            Pointer::Rc => format!("Rc::new({value})"),
            Pointer::Arc => format!("Arc::new({value})"),
            Pointer::Shared | Pointer::Exclusive => format!("support::leak({value})"),
        };
        let stored = match target.storage {
            Storage::Payload => payload,
            Storage::Pointer(pointer) => pointed(pointer, payload),
            Storage::Wrapper => wrapper,
            Storage::Pair => format!("({LABEL}, {payload})"),
            Storage::WrapperBehind(pointer) => pointed(pointer, wrapper),
            Storage::WrapperInPair => format!("({LABEL}, {wrapper})"),
            Storage::Object => pointed(Pointer::Box, payload),
        };
        match target.form {
            Form::Single => format!("{path}::A({stored})"),
            Form::MarkedTuple => format!("{path}::A({stored}, {LABEL})"),
            Form::MarkedStruct => format!("{path}::A {{ label: {LABEL}, held: {stored} }}"),
            Form::MarkedNamed => format!("{path}::A {{ held: {stored} }}"),
            Form::Struct => format!("{path}({stored})"),
            Form::StructMarked => format!("{path} {{ label: {LABEL}, held: {stored} }}"),
            Form::StructNamed => format!("{path} {{ held: {stored} }}"),
            Form::MarkedLast => format!("{path}::A({LABEL}, {stored})"),
            Form::StructMarkedLast => format!("{path}({LABEL}, {stored})"),
        }
    }
}

/// Returns a type's spelling, `this` standing for `Self`.
pub fn spell(node: &Node, this: &str, associates: Associates) -> String {
    match node {
        Node::Payload => String::from("P0"),
        Node::PayloadShared => String::from("&P0"),
        Node::PayloadLent => String::from("&'a P0"),
        Node::PayloadExclusive => String::from("&mut P0"),
        Node::Unit => String::from("()"),
        Node::Tag => String::from("u64"),
        Node::Shared => String::from("&u64"),
        Node::Lent => String::from("&'a u64"),
        Node::Exclusive => String::from("&mut u64"),
        Node::This => String::from(this),
        Node::Assoc => associated(Associated::Out, this, associates),
        Node::Overridden(name) => associated(*name, this, associates),
        // At the lifetime of the call, which the receiver lends.
        Node::Gat => format!("{}<'_>", associated(Associated::Item, this, associates)),
        Node::Fault => String::from("support::Fault"),
        Node::Option(inner) => format!("Option<{}>", spell(inner, this, associates)),
        Node::Result(ok, error) => format!(
            "Result<{}, {}>",
            spell(ok, this, associates),
            spell(error, this, associates)
        ),
        Node::Fallible(ok) => format!("support::fallible::Result<{}>", spell(ok, this, associates)),
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
            "Pin<Box<dyn Future<Output = {}> + Send + {FUTURE_LIFETIME}>>",
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
        Node::Shared | Node::Lent => String::from("&support::MISMATCH"),
        Node::Overridden(_) => String::from("support::Wide::MISMATCH"),
        Node::Gat => String::from("support::View::MISMATCH"),
        Node::Fault => String::from("support::Fault(424_242)"),
        Node::Option(_) => String::from("None"),
        Node::Result(_, error) => format!("Err({})", constant(error, this)),
        Node::Fallible(_) => format!("Err({})", constant(&Node::Fault, this)),
        Node::Tuple(elements) => {
            let elements: Vec<String> = elements
                .iter()
                .map(|element| constant(element, this))
                .collect();
            format!("({})", elements.join(", "))
        }
        // A place no payload holds, leaked, and a vector of no `Self`.
        Node::Exclusive => String::from("support::leak(424_242)"),
        Node::Vec(_) => String::from("Vec::new()"),
        Node::Payload
        | Node::PayloadShared
        | Node::PayloadLent
        | Node::PayloadExclusive
        | Node::Future(_, _)
        | Node::Iterator => {
            unreachable!("the verdict leaves no mismatch for this type")
        }
    }
}

/// Returns the type a slot's variant stores.
pub fn storage(slot: &Slot, target: &TargetDescriptor) -> String {
    let payload = slot.declared_payload();
    match target.storage {
        Storage::Payload => payload,
        Storage::Pointer(pointer) => pointed(pointer, &payload),
        Storage::Wrapper => slot.wrapper(),
        Storage::Pair => format!("(u64, {payload})"),
        Storage::WrapperBehind(pointer) => pointed(pointer, &slot.wrapper()),
        Storage::WrapperInPair => format!("(u64, {})", slot.wrapper()),
        Storage::Object => pointed(Pointer::Box, &format!("dyn {TRAIT}")),
    }
}

/// Returns the type of a slot's target where a route names it, the
/// payload or what it dereferences to (routes.md): the payload as the
/// subject declares it, the type an impl for one instantiation gives its
/// parameter, `instance`, or the trait object a `Box<dyn Probe>` holds.
pub fn target_type(slot: &Slot, target: &TargetDescriptor, instance: Option<&str>) -> String {
    if target.is_object() {
        return format!("dyn {TRAIT}");
    }
    match (slot.parameter, instance) {
        (Some(_), Some(instance)) => String::from(instance),
        (Some(_), None) | (None, _) => slot.declared_payload(),
    }
}

/// Returns the type of a pointer to `inner`.
fn pointed(pointer: Pointer, inner: &str) -> String {
    match pointer {
        Pointer::Box => format!("Box<{inner}>"),
        Pointer::Rc => format!("Rc<{inner}>"),
        Pointer::Arc => format!("Arc<{inner}>"),
        Pointer::Shared => format!("&'a {inner}"),
        Pointer::Exclusive => format!("&'a mut {inner}"),
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
        Form::MarkedNamed => (None, format!("{name} {{ {field_option}held: {stored} }},")),
        Form::MarkedLast => (None, format!("{name}(u64, {field_option}{stored}),")),
        Form::Struct | Form::StructMarked | Form::StructNamed | Form::StructMarkedLast => {
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
            // A field that names the trait stands beside a bare label, the
            // field every other trait goes to.
            let (label, mark) = match (attributes, target.trait_on_field()) {
                (false, _) => ("", String::new()),
                (true, Some(named)) => ("#[dispatch] ", format!("#[dispatch({named})] ")),
                (true, None) => ("", String::from("#[dispatch] ")),
            };
            writer.open(format!("pub struct E{generics} {{"));
            writer.line(format!("{label}pub label: u64,"));
            if anchored {
                writer.anchor(Anchor::Variant(slot.index).name());
            }
            writer.line(format!("{mark}pub held: {stored},"));
            writer.close("}");
        }
        Form::StructNamed => {
            writer.open(format!("pub struct E{generics} {{"));
            if anchored {
                writer.anchor(Anchor::Variant(slot.index).name());
            }
            writer.line(format!("pub held: {stored},"));
            writer.close("}");
        }
        Form::StructMarkedLast => {
            let mark = match (attributes, target.option) {
                (false, _) => "",
                (true, Some(_)) => unreachable!("the marked struct field takes no option"),
                (true, None) => "#[dispatch] ",
            };
            writer.open(format!("pub struct E{generics}("));
            writer.line("pub u64,");
            if anchored {
                writer.anchor(Anchor::Variant(slot.index).name());
            }
            writer.line(format!("{mark}pub {stored},"));
            writer.close(");");
        }
        Form::Single
        | Form::MarkedTuple
        | Form::MarkedStruct
        | Form::MarkedNamed
        | Form::MarkedLast => {
            unreachable!("an enum subject is written variant by variant")
        }
    }
}

/// Returns the pattern that binds a slot's forwarded field to `binding`,
/// with the subject's path `path`. A sole field, of a variant or a struct,
/// named or not, is bound with no rest pattern, which would skip nothing;
/// a marked field beside others skips the fields after it with `..`, as
/// the goldens `marked_field_by_value` and `routes` write
/// `Self::Tagged(__inner, ..)`, and a marked field that is the last has
/// none after it to skip, so a hand writes `A(_, inner)` (rule 1 of
/// `docs/vision.md`; no document shows the form).
pub fn pattern(path: &str, slot: &Slot, target: &TargetDescriptor, binding: &str) -> String {
    let name = slot.name();
    match target.form {
        Form::Single => format!("{path}::{name}({binding})"),
        Form::MarkedTuple => format!("{path}::{name}({binding}, ..)"),
        Form::MarkedStruct => format!("{path}::{name} {{ held: {binding}, .. }}"),
        Form::MarkedNamed => format!("{path}::{name} {{ held: {binding} }}"),
        Form::Struct => format!("{path}({binding})"),
        Form::StructMarked => format!("{path} {{ held: {binding}, .. }}"),
        Form::StructNamed => format!("{path} {{ held: {binding} }}"),
        Form::MarkedLast => format!("{path}::{name}(_, {binding})"),
        Form::StructMarkedLast => format!("{path}(_, {binding})"),
    }
}

/// Returns the constructor a rewrap calls: the variant's path, the
/// struct's name, or for a struct or a variant whose one field is named
/// its braces, with
/// [`HOLE`] where the value goes, since no path builds it.
pub fn constructor(path: &str, slot: &Slot) -> String {
    let form = slot.target().map(|(_, target)| target.form);
    match form {
        Some(Form::StructNamed) => format!("{path} {{ held: {HOLE} }}"),
        Some(Form::MarkedNamed) => format!("{path}::{} {{ held: {HOLE} }}", slot.name()),
        Some(Form::Struct | Form::StructMarked | Form::StructMarkedLast) => String::from(path),
        Some(Form::Single | Form::MarkedTuple | Form::MarkedStruct | Form::MarkedLast) | None => {
            format!("{path}::{}", slot.name())
        }
    }
}

/// Where a rebuild by braces takes its value.
pub const HOLE: &str = "@";

/// Returns whether the subject is a struct.
pub fn is_struct(slots: &[Slot]) -> bool {
    slots
        .first()
        .and_then(Slot::target)
        .is_some_and(|(_, target)| target.is_struct())
}
