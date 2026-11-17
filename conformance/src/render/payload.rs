//! The payloads: one struct per slot with a state, its impl of the trait,
//! whose every answer shows which payload gave it, from which state and
//! for which arguments, and an inherent method of the trait method's name
//! that answers a poison value.

use super::context::{spell, Associates, Context, Site};
use super::writer::Writer;
use crate::model::argument::ArgumentKind;
use crate::model::qualifier::Generic;
use crate::model::returns::{Associated, FutureKind, Node};
use crate::model::stack::OTHER;
use crate::model::{Callee, Lift, SlotVariant};

/// Whether a value is a payload's true answer or the poison an inherent
/// method answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flavour {
    Answer,
    Poison,
}

/// Writes the payloads, their wrappers and their impls, and the twin a
/// default is called on when the block leaves the method to it.
pub fn write(writer: &mut Writer, context: &Context) {
    if context.path.form == crate::model::path::Form::Default && context.qualifier.provided {
        writer.line("/// A type that keeps the trait's default, to call it directly.");
        writer.line("pub struct Defaulted {");
        writer.line("    pub base: u64,");
        writer.line("}");
        writer.blank();
        writer.open(format!(
            "{}{} {} for Defaulted {{",
            context.impl_keyword(),
            context.payload_generics(),
            context.trait_path()
        ));
        // The types the return names are the enum's, so that what the
        // default answers on either is one type.
        for (associated, overridden) in context.associated() {
            let ty = match (associated, overridden) {
                (Associated::Out | Associated::Error, true) => "support::Wide",
                (Associated::Out | Associated::Error, false) => "u64",
                (Associated::Item, _) => super::context::VIEW,
            };
            writer.line(associated.definition(ty));
        }
        for item in context.agreement.items {
            writer.line(item.definition(item.values[0]));
        }
        writer.open(format!("{} {{", context.base_signature()));
        writer.line("self.base");
        writer.close("}");
        if context.agreement.returned {
            writer.open(format!("{} {{", context.kind_signature("Self::Kind")));
            writer.line("0");
            writer.close("}");
        }
        writer.close("}");
        writer.blank();
        // A trait that requires the second trait is implemented only beside
        // it.
        if context.stack.required {
            writer.open(format!("impl {OTHER} for Defaulted {{"));
            for item in context.agreement.items {
                writer.line(item.definition(item.values[0]));
            }
            writer.open(format!("{} {{", context.other_signature()));
            writer.line("self.base");
            writer.close("}");
            writer.close("}");
            writer.blank();
        }
    }
    for slot in &context.declared {
        let index = slot.index;
        let payload = slot.payload();
        match slot.variant {
            SlotVariant::Live(_) | SlotVariant::Pair => {}
            SlotVariant::Unit => continue,
        }
        // A variant that holds another slot's payload type declares none.
        if !slot.holds_another() {
            write_payload(writer, context, index);
        }
        let wrapped = slot.target().is_some_and(|(_, target)| target.wraps());
        if wrapped {
            let wrapper = slot.wrapper();
            writer.line(format!("pub struct {wrapper} {{"));
            writer.line("    pub label: u64,");
            writer.line(format!("    pub payload: {payload},"));
            writer.line("}");
            writer.blank();
            writer.open(format!("impl {wrapper} {{"));
            writer.open(format!("pub fn lend(&self) -> &{payload} {{"));
            writer.line("&self.payload");
            writer.close("}");
            writer.blank();
            writer.open(format!("pub fn copied(&self) -> {payload} {{"));
            writer.line(format!(
                "{payload} {{ state: self.payload.state, spare: self.payload.spare }}"
            ));
            writer.close("}");
            writer.blank();
            writer.line("/// Lends the wrapper itself, which a route reads a field of.");
            writer.open(format!("pub fn get(&self) -> &{wrapper} {{"));
            writer.line("self");
            writer.close("}");
            writer.close("}");
            writer.blank();
        }
    }
}

/// Writes the payload of slot `index`: its struct, its inherent methods,
/// its impl of the trait and, for a function lift, its count.
fn write_payload(writer: &mut Writer, context: &Context, index: usize) {
    if let Some(foreign) = context.foreign_trait() {
        super::foreign::write_payload(writer, context, index, foreign);
        return;
    }
    let payload = format!("P{index}");
    if context.inherent() && context.cell.callee == Callee::Value {
        writer.line("#[derive(Clone, Copy)]");
    }
    writer.line(format!("pub struct {payload} {{"));
    writer.line("    pub state: u64,");
    writer.line("    pub spare: u64,");
    writer.line("}");
    writer.blank();
    writer.open(format!("impl {payload} {{"));
    writer.open("pub fn new(state: u64) -> Self {");
    writer.line(format!("{payload} {{ state, spare: 0 }}"));
    writer.close("}");
    writer.blank();
    writer.open("pub fn state(&self) -> u64 {");
    writer.line("self.state");
    writer.close("}");
    writer.blank();
    let callee = context.cell.callee;
    let calls = matches!(callee, Callee::Method | Callee::Renamed | Callee::Borrowing);
    if context.inherent() && calls {
        let own = context.payload_side();
        writer.line("/// The method the subject's inherent method forwards to.");
        // It goes with the inherent method, under its `#[cfg]`.
        if let Some(predicate) = context.item.cfg {
            writer.line(format!("#[cfg({predicate})]"));
        }
        writer.open(format!(
            "pub {} {{",
            own.signature_with(Site::Payload, own.node, own.callee_name())
        ));
        answer_body(writer, &own, index, "x");
        writer.close("}");
    }
    // A poison returning for the subject's lifetime could not name it in
    // the payload; nothing calls it there, since only a place answers so.
    let lent = matches!(context.node, Node::Lent);
    let own_name = matches!(callee, Callee::Method | Callee::Borrowing);
    if (!context.inherent() || !own_name) && !lent {
        writer.blank();
        writer.line("/// Answers what no forwarding may: an arm that calls it instead of the");
        writer.line("/// method meant shows in the law.");
        writer.open(format!("pub {} {{", context.signature(Site::Payload)));
        poison_body(writer, context, index);
        writer.close("}");
    }
    writer.close("}");
    writer.blank();
    write_impl(writer, context, index);
    writer.blank();
    write_other(writer, context, index);
    super::conversions::write_payload_into(writer, context, index);
    if matches!(context.cell.lift, Lift::ByRef | Lift::ByValue) {
        writer.open(format!("impl support::Counted for {payload} {{"));
        writer.open("fn count(&self) -> u64 {");
        writer.line("self.state");
        writer.close("}");
        writer.close("}");
        writer.blank();
    }
}

fn write_impl(writer: &mut Writer, context: &Context, index: usize) {
    writer.open(format!(
        "{}{} {} for P{index} {{",
        context.impl_keyword(),
        context.payload_generics(),
        context.trait_path()
    ));
    for (associated, overridden) in context.associated() {
        let line = match (associated, overridden) {
            (Associated::Out, false) => String::from("type Out = u64;"),
            (Associated::Out | Associated::Error, true) => {
                format!("type {} = support::Narrow{index};", associated.name())
            }
            (Associated::Error, false) => unreachable!("an error type is only ever overridden"),
            // A loan of the payload's state, which needs no bound of its
            // own, as the golden `gat_in_a_partial_block` writes its
            // payloads' items.
            (Associated::Item, _) => format!("type Item<'v> = support::Loan{index}<'v>;"),
        };
        writer.line(line);
    }
    for item in context.agreement.items {
        writer.line(item.definition(item.values[index]));
    }
    writer.blank();
    writer.open(format!("{} {{", context.base_signature()));
    match index {
        0 => writer.line("self.state * 10"),
        _ => writer.line(format!("self.state * 10 + {index}")),
    }
    writer.close("}");
    // The trait's own type, in the type the payload gives it.
    if context.agreement.returned {
        let given = context
            .agreement
            .items
            .iter()
            .find(|item| item.name == "Kind")
            .map_or("u32", |item| item.values[index]);
        writer.blank();
        writer.open(format!("{} {{", context.kind_signature(given)));
        writer.line(format!("self.state as {given} * 10 + {index}"));
        writer.close("}");
    }
    let overrides = !context.inherent() && (!context.qualifier.provided || index == 0);
    if overrides {
        writer.blank();
        for attribute in context.repeated_attributes() {
            writer.line(attribute);
        }
        writer.open(format!("{} {{", context.signature(Site::Impl)));
        answer_body(writer, context, index, context.parameter());
        writer.close("}");
    }
    writer.close("}");
}

/// Writes the payload's impl of the second trait, whose answer shows the
/// payload and its state.
fn write_other(writer: &mut Writer, context: &Context, index: usize) {
    if !context.stack.present {
        return;
    }
    writer.open(format!("impl {OTHER} for P{index} {{"));
    for item in context.agreement.items {
        writer.line(item.definition(item.values[index]));
    }
    writer.open(format!("{} {{", context.other_signature()));
    writer.line(format!("self.state * 1_000 + 900 + {index}"));
    writer.close("}");
    writer.close("}");
    writer.blank();
}

/// Returns how the body names the payload: `self`, or `this` after a
/// pinned receiver is unwrapped.
fn this(context: &Context) -> &'static str {
    if context.receiver.this == "self" {
        return "self";
    }
    "this"
}

/// Writes the body of a payload's answer, `x` being the name the method's
/// `u64` argument has.
fn answer_body(writer: &mut Writer, context: &Context, index: usize, x: &str) {
    let this = this(context);
    if this != "self" {
        writer.line(format!("let this = {};", context.receiver.this));
    }
    if context.qualifier.generic == Some(Generic::Fixed) {
        writer.line(format!("let {x}: u64 = {x}.into();"));
    }
    if context.receiver.mutates {
        writer.line(format!("{this}.state += {x} + 1;"));
    }
    // The enum an inherent method hands over shows only what the trait's
    // control method answers of it.
    let enum_argument = context.hands_the_enum();
    if context.argument.kind == Some(ArgumentKind::Exclusive) && !enum_argument {
        writer.line("other.state += 2;");
    }
    let mut number = format!("{}_000_000 + {this}.state * 100 + {x}", index + 1);
    // The const parameter the trait is implemented at shows in the answer.
    if context.generics.constant {
        number.push_str(" + N * 1_000");
    }
    match (context.argument.kind, enum_argument) {
        (Some(_), true) => number.push_str(" + other.base() * 7"),
        (Some(_), false) => number.push_str(" + other.state * 7"),
        (None, _) => {}
    }
    if context.qualifier.generic == Some(Generic::Turbofish) {
        number.push_str(" + W::VALUE * 10_000");
    }
    writer.line(format!("let n = {number};"));
    note_or_answer(writer, &context.node, this, index);
}

/// Writes the answer made from `n`, or, for a method answering `()`, which
/// shows nothing, the note of `n` that the law reads back.
fn note_or_answer(writer: &mut Writer, node: &Node, this: &str, index: usize) {
    match node {
        Node::Unit => writer.line("support::note(n);"),
        other => writer.line(value(other, this, index, Flavour::Answer)),
    }
}

fn poison_body(writer: &mut Writer, context: &Context, index: usize) {
    let this = this(context);
    if this != "self" && context.node.borrows_mutably() {
        writer.line(format!("let this = {};", context.receiver.this));
    }
    match (context.argument.kind, context.takes_x()) {
        (Some(_), true) => writer.line("let _ = (other, x);"),
        (Some(_), false) => writer.line("let _ = other;"),
        (None, true) => writer.line("let _ = x;"),
        (None, false) => {}
    }
    if context.node != Node::Unit {
        writer.line(value(&context.node, this, index, Flavour::Poison));
    }
}

/// Writes the default body of a provided method, which knows no payload.
pub fn default_body(writer: &mut Writer, context: &Context) {
    // A default body knows the projected argument's type only where the
    // trait's bound fixes it.
    let x = context.parameter();
    let counted = !context.generics.projected || context.generics.fixed;
    if !counted {
        writer.line(format!("let _ = {x};"));
    }
    // The default's answer starts at nine millions, where a payload's
    // starts at its slot's number plus one, so no payload's is mistaken
    // for it.
    let mut number = match counted {
        true => format!("9_000_000 + self.base() * 100 + {x}"),
        false => String::from("9_000_000 + self.base() * 100"),
    };
    if context.generics.constant {
        number.push_str(" + N * 1_000");
    }
    if context.argument.kind.is_some() {
        number.push_str(" + other.base() * 7");
    }
    writer.line(format!("let n = {number};"));
    if context.node == Node::Unit {
        writer.line("support::note(n);");
        return;
    }
    // An answer that is no number, `self` or `None`, still uses it.
    let value = default_value(&context.node, context.owned_self());
    let uses = value
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|word| word == "n");
    if !uses {
        writer.line("let _ = n;");
    }
    writer.line(value);
}

/// Returns the value a default body answers, made from the number `n`: a
/// place leaked for it where the type borrows, the `Self` the method owns,
/// spelled `owned`, and `None`, an `Err` or an empty `Vec` where the inner
/// type has no default.
fn default_value(node: &Node, owned: Option<&str>) -> String {
    let inner = |node: &Node| default_value(node, owned);
    let has = |node: &Node| node.has_default(owned.is_some());
    match node {
        Node::Unit => String::from("()"),
        Node::Tag => String::from("n"),
        Node::Shared | Node::Exclusive => String::from("support::leak(n)"),
        Node::This => {
            String::from(owned.expect("the verdict answers `Self` only where it is owned"))
        }
        Node::Fault => String::from("support::Fault(n)"),
        Node::Option(element) if has(element) => format!(
            "if n % 2 == 0 {{ Some({}) }} else {{ None }}",
            inner(element)
        ),
        Node::Option(_) => String::from("None"),
        Node::Fallible(ok) => default_value(&Node::Result(ok, &Node::Fault), owned),
        Node::Result(ok, error) => match (has(ok), has(error)) {
            (true, true) => format!(
                "if n % 2 == 0 {{ Ok({}) }} else {{ Err({}) }}",
                inner(ok),
                inner(error)
            ),
            (true, false) => format!("Ok({})", inner(ok)),
            (false, _) => format!("Err({})", inner(error)),
        },
        Node::Tuple(elements) => {
            let elements: Vec<String> = elements.iter().map(inner).collect();
            format!("({})", elements.join(", "))
        }
        Node::Future(FutureKind::Opaque, output) => format!("async move {{ {} }}", inner(output)),
        Node::Future(FutureKind::Boxed, output) => {
            format!("Box::pin(async move {{ {} }})", inner(output))
        }
        Node::Iterator => String::from("core::iter::once(n)"),
        Node::Vec(element) if has(element) => format!("vec![{}]", inner(element)),
        Node::Vec(_) => String::from("Vec::new()"),
        Node::Lent
        | Node::Payload
        | Node::PayloadShared
        | Node::PayloadLent
        | Node::PayloadExclusive
        | Node::Assoc
        | Node::Overridden(_)
        | Node::Gat => unreachable!("the verdict leaves no default body for this type"),
    }
}

/// Returns a value of the type: a true answer made from the number `n`
/// and the payload's state, or the poison.
fn value(node: &Node, this: &str, index: usize, flavour: Flavour) -> String {
    let number = match flavour {
        Flavour::Answer => "n",
        Flavour::Poison => "support::POISON",
    };
    let inner = |node: &Node| value(node, this, index, flavour);
    let branch = |yes: String, no: String| match flavour {
        Flavour::Answer => format!("if n % 2 == 0 {{ {yes} }} else {{ {no} }}"),
        Flavour::Poison => yes,
    };
    match node {
        Node::Payload | Node::PayloadShared | Node::PayloadLent | Node::PayloadExclusive => {
            unreachable!("no payload answers with a payload; an inherent place reads it")
        }
        Node::Unit => String::from("()"),
        Node::Tag | Node::Assoc => String::from(number),
        Node::Shared | Node::Lent => match flavour {
            Flavour::Answer => branch(format!("&{this}.state"), format!("&{this}.spare")),
            Flavour::Poison => String::from("&support::POISON_PLACE"),
        },
        Node::Exclusive => match flavour {
            Flavour::Answer => branch(format!("&mut {this}.state"), format!("&mut {this}.spare")),
            Flavour::Poison => format!("{{ {this}.spare = support::POISON; &mut {this}.spare }}"),
        },
        Node::This => format!("Self::new({number})"),
        Node::Overridden(_) => format!("support::Narrow{index}({number})"),
        Node::Gat => match flavour {
            Flavour::Answer => branch(
                format!("support::Loan{index}(&{this}.state)"),
                format!("support::Loan{index}(&{this}.spare)"),
            ),
            Flavour::Poison => format!("support::Loan{index}(&support::POISON_PLACE)"),
        },
        Node::Fault => format!("support::Fault({number})"),
        Node::Option(inner_node) => {
            branch(format!("Some({})", inner(inner_node)), String::from("None"))
        }
        Node::Fallible(ok) => value(&Node::Result(ok, &Node::Fault), this, index, flavour),
        Node::Result(ok, error) => branch(
            format!("Ok({})", inner(ok)),
            format!("Err({})", inner(error)),
        ),
        Node::Tuple(elements) => {
            let elements: Vec<String> = elements.iter().map(inner).collect();
            format!("({})", elements.join(", "))
        }
        Node::Future(FutureKind::Opaque, output) => format!("async move {{ {} }}", inner(output)),
        Node::Future(FutureKind::Boxed, output) => {
            format!("Box::pin(async move {{ {} }})", inner(output))
        }
        Node::Iterator => format!("core::iter::once({number})"),
        Node::Vec(element) => format!("vec![{}]", inner(element)),
    }
}

/// Returns the type of the answer of slot `index`'s payload, spelled for
/// the law: `Self` is the payload.
pub fn answer_type(node: &Node, index: usize) -> String {
    spell(node, &format!("P{index}"), Associates::Qualified)
}
