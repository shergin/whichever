//! The hand: the subject as a plain enum with the forwarding a careful
//! hand writes, as `docs/what-is-generated.md` and the goldens show it.
//!
//! The borrows are derived from types, not from a table of cases. The
//! receiver fixes how the `match` binds the variant's field; the route
//! gives a place from the binding; the payload method's receiver says how
//! the place is handed over. The return type says how the answer is
//! converted.

use super::context::{
    constructor, is_struct, pattern, target_type, variant, write_struct, Context, Site, HOLE,
};
use super::writer::Writer;
use crate::model::argument::ArgumentKind;
use crate::model::path::Form as PathForm;
use crate::model::receiver::{Binding, Handover};
use crate::model::returns::{Associated, Conversion, FutureKind, Node};
use crate::model::stack::OTHER;
use crate::model::target::{Step, TargetDescriptor};
use crate::model::{Callee, Lift, Receiver, Slot, Target};

/// A place expression, kept as a tree so that a borrow of it can be
/// written the way a hand writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Place {
    /// The binding itself; `reference` says how it borrows the field it
    /// binds, `None` when it binds by value.
    Binding(&'static str, Option<Binding>),
    /// `*place`.
    Deref(Box<Place>),
    /// `place.name`.
    Field(Box<Place>, &'static str),
    /// `place.name()`: a reference the target is behind, or the target
    /// itself as a temporary.
    Call(Box<Place>, &'static str),
}

impl Method {
    /// Returns the name of the payload's method an inherent arm calls.
    fn callee_name(&self) -> &'static str {
        match self.callee {
            Callee::Renamed => "answer",
            Callee::Method | Callee::Value | Callee::Field => "m",
        }
    }

    /// Returns whether the `match` takes the `Self` argument beside `self`,
    /// as a pair, rather than handing the enum over as it is.
    fn pairs(&self) -> bool {
        self.argument.is_some() && !self.hands_enum
    }

    /// Returns whether the `match` reads the enum by place, with `ref`
    /// bindings, for a receiver that binds as `binding`: over an enum that
    /// may be empty, unless the receiver binds by value or the `match`
    /// takes a pair.
    fn matches_by_place(&self, binding: Binding) -> bool {
        self.by_place && binding != Binding::Owned && !self.pairs()
    }
}

impl Place {
    /// Returns the place a route reaches from a binding.
    fn of(binding_name: &'static str, binding: Binding, route: &[Step]) -> Place {
        let mut place = match binding {
            Binding::Owned => Place::Binding(binding_name, None),
            Binding::Shared | Binding::Exclusive => {
                Place::Deref(Box::new(Place::Binding(binding_name, Some(binding))))
            }
        };
        for step in route {
            place = match step {
                Step::Deref => Place::Deref(Box::new(place)),
                Step::Field(name) => Place::Field(Box::new(place), name),
                Step::Method(name) => Place::Deref(Box::new(Place::Call(Box::new(place), name))),
                Step::Yield(name) => Place::Call(Box::new(place), name),
            };
        }
        place
    }

    /// Returns the place as an expression.
    fn text(&self) -> String {
        match self {
            Place::Binding(name, _) => String::from(*name),
            Place::Deref(inner) => format!("*{}", inner.text()),
            Place::Field(inner, name) => format!("{}.{name}", inner.receiver_text()),
            Place::Call(inner, name) => format!("{}.{name}()", inner.receiver_text()),
        }
    }

    /// Returns the place as the receiver of a field access or a method
    /// call, where a reference binding is dereferenced implicitly.
    fn receiver_text(&self) -> String {
        match self {
            Place::Deref(inner) if matches!(**inner, Place::Binding(_, Some(_))) => inner.text(),
            Place::Deref(_) => format!("({})", self.text()),
            Place::Binding(_, _) | Place::Field(_, _) | Place::Call(_, _) => self.text(),
        }
    }

    /// Returns the place borrowed, `exclusive` or shared; a dereference of
    /// a reference that is already the borrow asked for is that reference.
    fn borrowed(&self, exclusive: bool) -> String {
        if let Place::Deref(inner) = self {
            let already = match **inner {
                Place::Binding(_, Some(Binding::Exclusive)) => exclusive,
                Place::Binding(_, Some(Binding::Shared)) | Place::Call(_, _) => !exclusive,
                Place::Binding(_, Some(Binding::Owned) | None)
                | Place::Deref(_)
                | Place::Field(_, _) => false,
            };
            if already {
                return inner.text();
            }
        }
        match exclusive {
            true => format!("&mut {}", self.text()),
            false => format!("&{}", self.text()),
        }
    }

    /// Returns the method call a route ends in: the reference a lending
    /// method returns, or the value a yielding one does.
    fn method_result(&self) -> Option<&Place> {
        match self {
            Place::Call(_, _) => Some(self),
            Place::Deref(inner) if matches!(**inner, Place::Call(_, _)) => Some(inner),
            Place::Binding(_, _) | Place::Deref(_) | Place::Field(_, _) => None,
        }
    }

    /// Returns the place handed over as `handover` asks. A route ending in
    /// a method call hands the method's result over as it is
    /// (docs/decisions/an-arm-reaches-its-target-through-a-route.md).
    fn handed(&self, handover: Handover) -> String {
        if let Some(call) = self.method_result() {
            return match handover {
                Handover::Pin | Handover::PinMutably => format!("Pin::new({})", call.text()),
                Handover::Borrow | Handover::BorrowMutably | Handover::Move | Handover::MoveBox => {
                    call.text()
                }
            };
        }
        match handover {
            Handover::Borrow => self.borrowed(false),
            Handover::BorrowMutably => self.borrowed(true),
            Handover::Move => self.text(),
            Handover::MoveBox => match self {
                Place::Deref(inner) => inner.text(),
                Place::Binding(_, _) | Place::Field(_, _) | Place::Call(_, _) => {
                    unreachable!("the verdict hands a `Box` over only from a `deref` variant")
                }
            },
            Handover::Pin => format!("Pin::new({})", self.borrowed(false)),
            Handover::PinMutably => format!("Pin::new({})", self.borrowed(true)),
        }
    }
}

/// The allowance of a fallback arm that some build leaves unreachable.
const UNREACHABLE: &str = "#[allow(unreachable_patterns)]";

/// What one forwarded method is.
struct Method {
    name: &'static str,
    signature: String,
    receiver: Receiver,
    argument: Option<ArgumentKind>,
    node: Node,
    asyncness: bool,
    unsafety: bool,
    turbofish: &'static str,
    /// The slots the method is forwarded to.
    slots: Vec<usize>,
    /// The arm for every other variant: a mismatch, or a block's body.
    fallback: Option<String>,
    /// Whether the method is inherent, calling the payloads' own by
    /// method-call syntax.
    inherent: bool,
    /// What an inherent arm calls or reads.
    callee: Callee,
    /// What an inherent arm's result is wrapped in.
    lift: Lift,
    /// The type an inherent arm's target answers before the lift.
    target: Node,
    /// Whether the `match` reads the enum by place, every variant being
    /// under `#[cfg]`, so a reference receiver's arms bind with `ref`.
    by_place: bool,
    /// The attributes the method carries before its `#[inline]`.
    attributes: Vec<String>,
    /// Its `#[inline]`, without the `#[..]`.
    inline: &'static str,
    /// The name of its `u64` argument.
    parameter: &'static str,
    /// Whether each arm passes that argument after the target and the
    /// `Self` argument.
    passes: bool,
    /// Whether its `Self` argument is handed over as it is, the enum
    /// itself, rather than matched beside `self`.
    hands_enum: bool,
    /// The trait as its arms name it.
    trait_path: String,
    /// The type the impl gives the subject's parameter, where it is for
    /// one instantiation.
    instance: Option<&'static str>,
}

/// Writes `mod hand`.
pub fn write(writer: &mut Writer, context: &Context) {
    writer.line("/// The same subject with the forwarding written by hand.");
    writer.open("mod hand {");
    // The foreign trait's impl names no payload where no variant holding
    // one is in the build, nor anything else of the cell.
    if context.foreign() && context.live().is_empty() {
        writer.line("#[allow(unused_imports)]");
    }
    writer.line("use super::*;");
    writer.blank();
    let generics = context.generics();
    if is_struct(&context.slots) {
        write_struct(writer, &context.slots[0], &generics, false, false);
    } else {
        writer.open(format!("pub enum E{generics} {{"));
        for slot in &context.declared {
            let (_, declaration) = variant(slot, false);
            if let Some(predicate) = slot.cfg {
                writer.line(format!("#[cfg({predicate})]"));
            }
            if slot.deprecated {
                writer.line("#[deprecated]");
            }
            writer.line(declaration);
        }
        writer.close("}");
    }
    writer.blank();
    super::conversions::write_hand(writer, context);
    // `with = Into::into` rebuilds the enum through the `From` impls the
    // subject's attribute writes, which the hand writes for its own enum:
    // the one from a payload is a variant's that holds it as it is, the
    // others take what their variant stores, which no rebuild from a
    // payload uses; a struct's attribute writes none.
    let rebuilds = context.inherent()
        && context.cell.lift == Lift::Into
        && context.cell.declared_node().holds_this()
        && !is_struct(&context.slots);
    if rebuilds {
        let generics = context.generics();
        for (slot, target, _) in context.cell.method_slots() {
            if target != Target::Plain {
                continue;
            }
            let payload = slot.payload();
            if slot.deprecated {
                writer.line("#[allow(deprecated)]");
            }
            writer.open(format!("impl{generics} From<{payload}> for E{generics} {{"));
            writer.open(format!("fn from(payload: {payload}) -> E{generics} {{"));
            writer.line(format!("E::{}(payload)", slot.name()));
            writer.close("}");
            writer.close("}");
            writer.blank();
        }
    }
    write_other(writer, context);
    if context.foreign() {
        write_foreign(writer, context);
        writer.close("}");
        return;
    }
    allow_unread(writer, context);
    writer.open(format!(
        "{}{} {} for E{} {{",
        context.impl_keyword(),
        context.impl_generics(),
        context.trait_path(),
        context.impl_type_generics()
    ));
    for (associated, overridden) in context.associated() {
        let name = associated.name();
        let ty = match (associated, overridden) {
            (Associated::Out | Associated::Error, true) => String::from("support::Wide"),
            (Associated::Out | Associated::Error, false) => format!(
                "<{} as {}>::{name}",
                context.supplier_payload(),
                context.trait_path()
            ),
            // The enum declares it, as the block does.
            (Associated::Item, _) => String::from(super::context::VIEW),
        };
        writer.line(associated.definition(&ty));
    }
    // An item the enum fixes is the value fixed, and any other is the
    // supplier's (what-is-generated.md).
    for item in context.agreement.items {
        let value = item.overridden.map_or_else(
            || {
                format!(
                    "<{} as {}>::{}",
                    context.supplier_payload(),
                    context.trait_path(),
                    item.name
                )
            },
            String::from,
        );
        writer.line(item.definition(&value));
    }
    let indices = |slots: Vec<(Slot, crate::model::Target, TargetDescriptor)>| -> Vec<usize> {
        slots.iter().map(|(slot, _, _)| slot.index).collect()
    };
    let base = Method {
        name: "base",
        signature: context.base_signature(),
        receiver: context.base_receiver(),
        argument: None,
        node: Node::Tag,
        asyncness: false,
        unsafety: false,
        turbofish: "",
        slots: indices(context.cell.base_slots()),
        fallback: (!context.cell.base_reaches_all()).then(|| String::from("0")),
        inherent: false,
        callee: Callee::Method,
        lift: Lift::Bare,
        target: Node::Tag,
        by_place: context.cell.every_under_cfg() && context.base_receiver() != Receiver::Owned,
        attributes: Vec::new(),
        inline: "inline",
        parameter: "x",
        passes: false,
        hands_enum: false,
        trait_path: context.trait_path(),
        instance: context.instance(),
    };
    write_method(writer, context, &base);
    // The method returning the trait's own type is forwarded as the control
    // method is, its answer kept as it is.
    if context.agreement.returned {
        let kind = Method {
            name: "kind",
            signature: context.kind_signature("Self::Kind"),
            node: Node::Assoc,
            target: Node::Assoc,
            ..base
        };
        write_method(writer, context, &kind);
    }
    // The forwarding of a deprecated method calls it, so it is written
    // under `#[allow(deprecated)]`, as a hand writes it
    // (what-is-generated.md).
    let mut attributes = context.repeated_attributes();
    if context.item.deprecated {
        attributes.push(String::from("#[allow(deprecated)]"));
    }
    // A pair keeps its fallback arm in every build, and in a build that
    // holds one variant the arm is unreachable, so a hand allows it once on
    // the method, as the golden `inherent` allows the catch-all of its
    // `TryFrom` impls: wherever the enum declares one variant under no
    // `#[cfg]`, or none.
    //
    // Open question: where a hand allows an arm that may be unreachable is
    // a spelling the docs do not fix. The model follows the golden for the
    // methods whose signature the attribute writes, and allows a body's
    // arm, a method the user writes or marks, on the arm itself.
    let unconditional = context
        .declared
        .iter()
        .filter(|slot| slot.cfg.is_none())
        .count();
    let paired = context.path.form == PathForm::Forwarded
        && !context.inherent()
        && context.argument.kind.is_some()
        && context.cell.needs_mismatch();
    if paired && !context.path.in_block && unconditional <= 1 {
        attributes.push(String::from(UNREACHABLE));
    }
    // A partial block's or an inherent block's method over an enum whose
    // every variant is under `#[cfg]` allows its arguments to go unused, as
    // a hand's does, since a build may leave no arm to use them; the impl
    // the enum's attribute writes is `#[automatically_derived]` and needs
    // none (CHANGELOG, Unreleased; behaviour test
    // `enums_with_every_variant_under_cfg`). A body for the variants a mark
    // leaves out uses them in every build.
    let user_item = context.path.site != crate::model::path::Site::Attribute;
    let arguments = context.takes_x() || context.argument.kind.is_some();
    let may_be_empty = context.cell.every_under_cfg() && !is_struct(&context.slots);
    let bodiless = context.path.form == PathForm::Forwarded;
    let quiet = user_item && arguments && may_be_empty && bodiless;
    if quiet {
        attributes.push(String::from("#[allow(unused_variables)]"));
    }
    let fallback = match context.path.form {
        // A method in no build has no arm to write.
        PathForm::Forwarded | PathForm::First if !context.cell.method_present() => None,
        // An inherent method's arms are written below, with the body it
        // takes for the variants its mark leaves out.
        PathForm::Forwarded | PathForm::First if context.inherent() => None,
        // The pair of different variants is met in some build wherever the
        // enum declares two, or a variant under `#[cfg]`.
        PathForm::Forwarded => context
            .argument
            .kind
            .filter(|_| context.cell.needs_mismatch())
            .map(|_| context.mismatch("E")),
        PathForm::First => Some(format!(
            "{{ {} {} }}",
            context.unused(Site::Impl, false),
            context.mismatch("E")
        )),
        PathForm::Default | PathForm::Written => None,
    };
    let method = Method {
        name: "m",
        signature: context.signature(Site::Impl),
        receiver: context.cell.recv,
        argument: context.argument.kind,
        node: context.node,
        asyncness: context.qualifier.asyncness,
        unsafety: context.qualifier.unsafety,
        turbofish: match context.qualifier.generic {
            Some(crate::model::qualifier::Generic::Turbofish) => "::<W>",
            Some(crate::model::qualifier::Generic::Fixed) | None => "",
        },
        slots: indices(context.cell.method_slots()),
        fallback,
        inherent: context.inherent(),
        callee: context.cell.callee,
        lift: context.cell.lift,
        target: context.cell.target_node(),
        by_place: context.cell.every_under_cfg() && context.cell.recv != Receiver::Owned,
        attributes,
        inline: context.item.inline,
        parameter: context.parameter(),
        passes: true,
        hands_enum: false,
        trait_path: context.trait_path(),
        instance: context.instance(),
    };
    if method.inherent {
        writer.close("}");
        writer.blank();
        allow_unread(writer, context);
        writer.open(format!("impl{generics} E{generics} {{"));
        let fallback = match context.path.form {
            PathForm::First => Some(format!(
                "{{ {} {} }}",
                context.unused(Site::Inherent, context.hands_the_enum()),
                context.declared_constant()
            )),
            PathForm::Forwarded | PathForm::Default | PathForm::Written => None,
        };
        // An inherent method carries what the user wrote and the allowance
        // over an enum that may be empty, and no `#[inline]`.
        let attributes = match quiet {
            true => vec![String::from("#[allow(unused_variables)]")],
            false => Vec::new(),
        };
        // A function lift's declared type is its own, `u64`, which names
        // no `Self` and no future, so its arms are neither rewrapped nor
        // awaited: the function is handed the payload's answer as it is.
        let node = match context.cell.lift {
            Lift::ByRef | Lift::ByValue => context.cell.declared_node(),
            Lift::Bare | Lift::Some | Lift::Ok | Lift::Into => method.node,
        };
        let method = Method {
            signature: context.declared_signature(),
            node,
            fallback,
            hands_enum: context.hands_the_enum(),
            attributes,
            ..method
        };
        write_method(writer, context, &method);
        writer.close("}");
        writer.close("}");
        return;
    }
    // A method the trait declares under a `#[cfg]` that never holds is in
    // no build, so the hand has nothing of it to write.
    let present = context.cell.method_present();
    match context.path.form {
        PathForm::Forwarded | PathForm::First if present => write_method(writer, context, &method),
        PathForm::Forwarded | PathForm::First | PathForm::Default => {}
        PathForm::Written if !present => {}
        PathForm::Written => {
            writer.blank();
            for attribute in context.repeated_attributes() {
                writer.line(attribute);
            }
            if context.item.inline != "inline" {
                writer.line(format!("#[{}]", context.item.inline));
            }
            writer.open(format!("{} {{", method.signature));
            writer.line(context.unused(Site::Impl, true));
            writer.line(context.mismatch("E"));
            writer.close("}");
        }
    }
    writer.close("}");
    writer.close("}");
}

/// Writes the hand's impl of the second trait: its associated items as the
/// trait under test takes them, and its method forwarded to every variant
/// as the control method is.
fn write_other(writer: &mut Writer, context: &Context) {
    if !context.stack.present {
        return;
    }
    allow_unread(writer, context);
    writer.open(format!(
        "impl{} {OTHER} for E{} {{",
        context.other_impl_generics(),
        context.type_generics()
    ));
    for item in context.agreement.items {
        let value = item.overridden.map_or_else(
            || format!("<{} as {OTHER}>::{}", context.supplier_payload(), item.name),
            String::from,
        );
        writer.line(item.definition(&value));
    }
    let receiver = context.other_receiver();
    let method = Method {
        name: "other",
        signature: context.other_signature(),
        receiver,
        argument: None,
        node: Node::Tag,
        asyncness: false,
        unsafety: false,
        turbofish: "",
        slots: context
            .live()
            .iter()
            .map(|(slot, _, _)| slot.index)
            .collect(),
        fallback: None,
        inherent: false,
        callee: Callee::Method,
        lift: Lift::Bare,
        target: Node::Tag,
        by_place: context.cell.every_under_cfg() && receiver != Receiver::Owned,
        attributes: Vec::new(),
        inline: "inline",
        parameter: "x",
        passes: false,
        hands_enum: false,
        trait_path: String::from(OTHER),
        instance: None,
    };
    write_method(writer, context, &method);
    writer.close("}");
    writer.blank();
}

/// Writes the hand's impl of `core::fmt::Display`, the trait
/// `whichever::foreign::Display` implements: `fmt` forwarded to the
/// variants it reaches, with the body the block writes for the rest, or
/// that body alone.
fn write_foreign(writer: &mut Writer, context: &Context) {
    use super::foreign::{body, SIGNATURE};
    use crate::model::declaration::CORE_DISPLAY;
    allow_unread(writer, context);
    writer.open(format!(
        "impl{} {CORE_DISPLAY} for E{} {{",
        context.impl_generics(),
        context.type_generics()
    ));
    // A partial block's method over an enum whose every variant is under
    // `#[cfg]` allows its arguments to go unused, as `m`'s does.
    let user_item = context.path.site != crate::model::path::Site::Attribute;
    let may_be_empty = context.cell.every_under_cfg() && !is_struct(&context.slots);
    let bodiless = context.path.form == PathForm::Forwarded;
    let attributes = match user_item && may_be_empty && bodiless {
        true => vec![String::from("#[allow(unused_variables)]")],
        false => Vec::new(),
    };
    let method = Method {
        name: "fmt",
        signature: String::from(SIGNATURE),
        receiver: Receiver::Shared,
        argument: None,
        node: Node::Tag,
        asyncness: false,
        unsafety: false,
        turbofish: "",
        slots: context
            .cell
            .method_slots()
            .iter()
            .map(|(slot, _, _)| slot.index)
            .collect(),
        fallback: (context.path.form == PathForm::First).then(body),
        inherent: false,
        callee: Callee::Method,
        lift: Lift::Bare,
        target: Node::Tag,
        by_place: context.cell.every_under_cfg(),
        attributes,
        inline: "inline",
        parameter: "f",
        passes: true,
        hands_enum: false,
        trait_path: String::from(CORE_DISPLAY),
        instance: None,
    };
    match context.path.form {
        PathForm::Forwarded | PathForm::First => write_method(writer, context, &method),
        PathForm::Default => {}
        PathForm::Written => {
            writer.blank();
            writer.open(format!("{SIGNATURE} {{"));
            writer.line(body());
            writer.close("}");
        }
    }
    writer.close("}");
}

/// Writes the `#[allow(unused_variables)]` a hand's impl over an enum
/// that is empty in this build needs: no arm reads the arguments.
fn allow_unread(writer: &mut Writer, context: &Context) {
    if !is_struct(&context.slots) && context.slots.is_empty() {
        writer.line("#[allow(unused_variables)]");
    }
}

/// How the arms of a method meet its return type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wrapping {
    /// The `match` is the body.
    Plain,
    /// The `match` is in one `async move` block, every arm awaited.
    AsyncBlock,
    /// The `match` is in `Box::pin(async move { .. })`, every arm awaited.
    BoxedBlock,
}

fn write_method(writer: &mut Writer, context: &Context, method: &Method) {
    let receiver = method.receiver.descriptor();
    let unwrap = receiver
        .unwrap
        .expect("the verdict writes a hand only for a forwarded receiver");
    // "On a struct the one arm returns the field's future as it is, unless
    // its output is rewrapped or converted" (what-is-generated.md).
    let one_arm = is_struct(&context.slots);
    let (wrapping, awaited, conversion) = match method.node.future() {
        Some((FutureKind::Opaque, output))
            if one_arm
                && output
                    .conversion()
                    .as_ref()
                    .is_some_and(Conversion::is_keep) =>
        {
            (Wrapping::Plain, false, output.conversion())
        }
        Some((FutureKind::Opaque, output)) => (Wrapping::AsyncBlock, true, output.conversion()),
        Some((FutureKind::Boxed, output)) => {
            let conversion = output.conversion();
            match conversion.as_ref().is_some_and(Conversion::is_keep) {
                true => (Wrapping::Plain, false, conversion),
                false => (Wrapping::BoxedBlock, true, conversion),
            }
        }
        None if one_arm => (
            Wrapping::Plain,
            method.asyncness,
            method.node.conversion_of_one(),
        ),
        None => (Wrapping::Plain, method.asyncness, method.node.conversion()),
    };
    let conversion =
        conversion.expect("the verdict writes a hand only for a conversion that exists");
    writer.blank();
    for attribute in &method.attributes {
        writer.line(attribute);
    }
    if !method.inherent {
        writer.line(format!("#[{}]", method.inline));
    }
    writer.open(format!("{} {{", method.signature));
    match wrapping {
        Wrapping::Plain => {}
        Wrapping::AsyncBlock => writer.open("async move {"),
        Wrapping::BoxedBlock => writer.open("Box::pin(async move {"),
    }
    // An enum whose every variant is under `#[cfg]` may be empty, and is
    // matched by place, with `ref` bindings, where the receiver binds by
    // reference (golden `every_variant_under_cfg`); a method with a `Self`
    // argument matches the pair, "whose tuple scrutinee has the fallback
    // arm in every build"
    // (docs/decisions/an-enum-that-may-be-empty-is-matched-by-place.md).
    let by_place = method.matches_by_place(unwrap.binding);
    let paired = method.pairs();
    let scrutinee = match (paired, by_place) {
        (true, _) => format!("({}, other)", unwrap.scrutinee),
        (false, true) => format!("*{}", unwrap.scrutinee),
        (false, false) => String::from(unwrap.scrutinee),
    };
    writer.open(format!("match {scrutinee} {{"));
    for (slot, _, target) in context.live() {
        if !method.slots.contains(&slot.index) {
            continue;
        }
        let arm = arm(
            &slot,
            &target,
            method,
            unwrap.binding,
            receiver.handover,
            awaited,
            &conversion,
        );
        writer.line(arm);
    }
    // Beside a variant under `#[cfg]` the fallback arm is unreachable in a
    // build that leaves only variants the arms name, so a hand writing the
    // arm for every build allows it, on the arm where the method does not
    // allow it already.
    let conditional = context.declared.iter().any(|slot| slot.cfg.is_some())
        && !method
            .attributes
            .iter()
            .any(|attribute| attribute == UNREACHABLE);
    if let Some(fallback) = &method.fallback {
        match conditional {
            true => writer.line(format!("{UNREACHABLE} _ => {fallback},")),
            false => writer.line(format!("_ => {fallback},")),
        }
    }
    writer.close("}");
    match wrapping {
        Wrapping::Plain => {}
        Wrapping::AsyncBlock => writer.close("}"),
        Wrapping::BoxedBlock => writer.close("})"),
    }
    writer.close("}");
}

fn arm(
    slot: &Slot,
    target: &TargetDescriptor,
    method: &Method,
    binding: Binding,
    handover: Handover,
    awaited: bool,
    conversion: &Conversion,
) -> String {
    let own = Place::of("inner", binding, target.route);
    let by_place = method.matches_by_place(binding);
    let bound = match (by_place, binding) {
        (true, Binding::Shared) => "ref inner",
        (true, Binding::Exclusive) => "ref mut inner",
        (true, Binding::Owned) | (false, _) => "inner",
    };
    let mut pattern_text = pattern("E", slot, target, bound);
    // The `Self` argument: the enum handed over as it is, or the same
    // variant's target, matched beside `self`.
    let mut other_argument = None;
    if let Some(kind) = method.argument {
        let (other_binding, other_handover) = match kind {
            ArgumentKind::Shared => (Binding::Shared, Handover::Borrow),
            ArgumentKind::Exclusive => (Binding::Exclusive, Handover::BorrowMutably),
            ArgumentKind::Owned => (Binding::Owned, Handover::Move),
        };
        match method.hands_enum {
            true => other_argument = Some(String::from("other")),
            false => {
                let other = Place::of("same", other_binding, target.route);
                pattern_text = format!("({pattern_text}, {})", pattern("E", slot, target, "same"));
                other_argument = Some(other.handed(other_handover));
            }
        }
    }
    // The arms of a deprecated variant are under `#[allow(deprecated)]`
    // (what-is-generated.md, golden `deprecated`), and a variant's `#[cfg]`
    // is carried onto its arm; both are the arm's, outside its pattern.
    if slot.deprecated {
        pattern_text = format!("#[allow(deprecated)] {pattern_text}");
    }
    if let Some(predicate) = slot.cfg {
        pattern_text = format!("#[cfg({predicate})] {pattern_text}");
    }
    if method.inherent {
        let value = inherent_value(
            &own,
            method,
            handover,
            other_argument,
            awaited,
            conversion,
            slot,
        );
        return format!("{pattern_text} => {value},");
    }
    let mut arguments = vec![own.handed(handover)];
    arguments.extend(other_argument);
    if method.passes {
        arguments.push(String::from(method.parameter));
    }
    // "Where the route names the target's type, the payload or what it
    // dereferences to, the call is `<Square as Shape>::area(&inner.base)`;
    // a field's or a method's type cannot be named, and the call is `<_ as
    // Shape>::area(&inner.base)`, qualified by the trait alone" (routes.md).
    // What a `Box<dyn Probe>` dereferences to is the trait object.
    let named = target.route.iter().all(|step| *step == Step::Deref);
    let qualifier = match named {
        true => target_type(slot, target, method.instance),
        false => String::from("_"),
    };
    let mut call = format!(
        "<{} as {}>::{}{}({})",
        qualifier,
        method.trait_path,
        method.name,
        method.turbofish,
        arguments.join(", ")
    );
    if method.unsafety {
        call = format!("unsafe {{ {call} }}");
    }
    if awaited {
        call = format!("{call}.await");
    }
    let variant = constructor("E", slot);
    format!(
        "{pattern_text} => {},",
        convert(conversion, &call, &variant)
    )
}

/// Returns the value of an inherent arm: the payload's method called by
/// method-call syntax, or the target or its field read as the declared
/// type asks, then wrapped in the lift. A function lift is handed the
/// target as it stands: the binding, or a place reached through a field,
/// by value (inherent-methods.md).
fn inherent_value(
    own: &Place,
    method: &Method,
    handover: Handover,
    other: Option<String>,
    awaited: bool,
    conversion: &Conversion,
    slot: &Slot,
) -> String {
    let read = |place: &Place| match method.target {
        Node::PayloadShared | Node::PayloadLent | Node::Shared | Node::Lent => {
            place.borrowed(false)
        }
        Node::PayloadExclusive | Node::Exclusive => place.borrowed(true),
        Node::Payload
        | Node::Tag
        | Node::Unit
        | Node::This
        | Node::Assoc
        | Node::Overridden(_)
        | Node::Gat
        | Node::Fault
        | Node::Option(_)
        | Node::Result(_, _)
        | Node::Fallible(_)
        | Node::Tuple(_)
        | Node::Future(_, _)
        | Node::Iterator
        | Node::Vec(_) => place.text(),
    };
    let field = Place::Field(Box::new(own.clone()), "state");
    let standing = match method.callee {
        Callee::Value => match own {
            Place::Deref(binding) => binding.text(),
            Place::Binding(_, _) | Place::Field(_, _) | Place::Call(_, _) => own.text(),
        },
        Callee::Field => field.text(),
        Callee::Method | Callee::Renamed => String::new(),
    };
    let value = match method.callee {
        Callee::Method | Callee::Renamed => {
            // A receiver that is a pinned reference or the box is built as
            // the trait's forwarding hands it over; any other is the target
            // the call autorefs.
            let receiver = match handover {
                Handover::Pin | Handover::PinMutably | Handover::MoveBox => own.handed(handover),
                Handover::Borrow | Handover::BorrowMutably | Handover::Move => {
                    match own.method_result() {
                        Some(call) => call.text(),
                        None => own.receiver_text(),
                    }
                }
            };
            let mut arguments: Vec<String> = other.into_iter().collect();
            arguments.push(String::from("x"));
            let mut call = format!(
                "{receiver}.{}{}({})",
                method.callee_name(),
                method.turbofish,
                arguments.join(", ")
            );
            if method.unsafety {
                call = format!("unsafe {{ {call} }}");
            }
            if awaited {
                call = format!("{call}.await");
            }
            // `with = Into::into` converts what the payload answers, its
            // own type included, into the declared type through the enum's
            // `From` impls, where a return of `Self` would rewrap it.
            match method.lift {
                Lift::Into => format!("Into::into({call})"),
                Lift::Bare | Lift::Some | Lift::Ok | Lift::ByRef | Lift::ByValue => {
                    let variant = constructor("E", slot);
                    convert(conversion, &call, &variant)
                }
            }
        }
        // A route ending in a method call hands the method's result over as
        // it is, a place read through it included.
        Callee::Value => match own.method_result() {
            Some(call) => call.text(),
            None => read(own),
        },
        Callee::Field => read(&field),
    };
    let handed = match method.callee {
        Callee::Method | Callee::Renamed => value.clone(),
        Callee::Value | Callee::Field => standing,
    };
    match method.lift {
        Lift::Bare | Lift::Into => value,
        Lift::Some => format!("Some({value})"),
        Lift::Ok => format!("Ok({value})"),
        Lift::ByRef => format!("support::by_ref({handed})"),
        Lift::ByValue => format!("support::by_value({handed})"),
    }
}

/// Returns `value` converted as `conversion` says, `variant` being the
/// constructor a rewrap calls.
fn convert(conversion: &Conversion, value: &str, variant: &str) -> String {
    match conversion {
        Conversion::Keep => String::from(value),
        // A struct with a named field is rebuilt by its braces.
        Conversion::Rewrap if variant.contains(HOLE) => variant.replace(HOLE, value),
        Conversion::Rewrap => format!("{variant}({value})"),
        Conversion::Into => format!("Into::into({value})"),
        Conversion::Map(inner) => format!("{value}.map({})", path(inner, variant)),
        Conversion::Result(ok, error) => {
            let mut text = String::from(value);
            if !ok.is_keep() {
                text.push_str(&format!(".map({})", path(ok, variant)));
            }
            if !error.is_keep() {
                text.push_str(&format!(".map_err({})", path(error, variant)));
            }
            text
        }
        Conversion::Tuple(elements) => {
            let names: Vec<String> = (0..elements.len())
                .map(|index| format!("element{index}"))
                .collect();
            let rebuilt: Vec<String> = elements
                .iter()
                .zip(&names)
                .map(|(element, name)| convert(element, name, variant))
                .collect();
            format!(
                "{{ let ({}) = {value}; ({}) }}",
                names.join(", "),
                rebuilt.join(", ")
            )
        }
    }
}

/// Returns a conversion as the path a `.map(..)` takes.
fn path(conversion: &Conversion, variant: &str) -> String {
    match conversion {
        Conversion::Rewrap if variant.contains(HOLE) => {
            unreachable!("the verdict maps no struct with a named field")
        }
        Conversion::Rewrap => String::from(variant),
        Conversion::Into => String::from("Into::into"),
        Conversion::Keep | Conversion::Map(_) | Conversion::Result(_, _) | Conversion::Tuple(_) => {
            unreachable!("the verdict maps only a rewrap or an `Into`")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_are_borrowed_as_a_hand_writes_them() {
        let shared = Place::of("inner", Binding::Shared, &[]);
        assert_eq!(shared.handed(Handover::Borrow), "inner");
        assert_eq!(shared.handed(Handover::Pin), "Pin::new(inner)");
        let exclusive = Place::of("inner", Binding::Exclusive, &[]);
        assert_eq!(exclusive.handed(Handover::BorrowMutably), "inner");
        assert_eq!(exclusive.handed(Handover::Borrow), "&*inner");
        let deref = Place::of("inner", Binding::Shared, &[Step::Deref]);
        assert_eq!(deref.handed(Handover::Borrow), "&**inner");
        let owned = Place::of("inner", Binding::Owned, &[Step::Deref]);
        assert_eq!(owned.handed(Handover::Move), "*inner");
        assert_eq!(owned.handed(Handover::MoveBox), "inner");
        let field = Place::of("inner", Binding::Shared, &[Step::Field("payload")]);
        assert_eq!(field.handed(Handover::Borrow), "&inner.payload");
        let field = Place::of("inner", Binding::Owned, &[Step::Field("1")]);
        assert_eq!(field.handed(Handover::Move), "inner.1");
        let method = Place::of("inner", Binding::Shared, &[Step::Method("lend")]);
        assert_eq!(method.handed(Handover::Borrow), "inner.lend()");
        assert_eq!(method.handed(Handover::Pin), "Pin::new(inner.lend())");
        let value = Place::of("inner", Binding::Shared, &[Step::Yield("copied")]);
        assert_eq!(value.handed(Handover::Borrow), "inner.copied()");
        assert_eq!(value.handed(Handover::Move), "inner.copied()");
    }

    #[test]
    fn conversions_are_paths_under_a_map() {
        let tuple = Conversion::Tuple(vec![
            Conversion::Rewrap,
            Conversion::Map(Box::new(Conversion::Rewrap)),
        ]);
        assert_eq!(
            convert(&tuple, "call", "E::A"),
            "{ let (element0, element1) = call; (E::A(element0), element1.map(E::A)) }"
        );
        let result = Conversion::Result(Box::new(Conversion::Rewrap), Box::new(Conversion::Into));
        assert_eq!(
            convert(&result, "call", "E::A"),
            "call.map(E::A).map_err(Into::into)"
        );
    }
}
