//! The payloads: one struct per slot with a state, its impl of the trait,
//! whose every answer shows which payload gave it, from which state and
//! for which arguments, and an inherent method of the trait method's name
//! that answers a poison value.

use super::context::{spell, Associates, Context, Site, TRAIT};
use super::writer::Writer;
use crate::model::argument::ArgumentKind;
use crate::model::qualifier::Generic;
use crate::model::returns::{Associated, FutureKind, Node};
use crate::model::target::Storage;
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
        writer.line("    base: u64,");
        writer.line("}");
        writer.blank();
        writer.open(format!("impl {TRAIT} for Defaulted {{"));
        writer.open(format!("{} {{", context.base_signature()));
        writer.line("self.base");
        writer.close("}");
        writer.close("}");
        writer.blank();
    }
    for slot in &context.slots {
        let index = slot.index;
        let payload = slot.payload();
        match slot.variant {
            SlotVariant::Live(_) | SlotVariant::Pair => {}
            SlotVariant::Unit => continue,
        }
        if context.inherent() && context.cell.callee == Callee::Value {
            writer.line("#[derive(Clone, Copy)]");
        }
        writer.line(format!("pub struct {payload} {{"));
        writer.line("    state: u64,");
        writer.line("    spare: u64,");
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
        let calls = matches!(callee, Callee::Method | Callee::Renamed);
        if context.inherent() && calls {
            writer.line("/// The method the subject's inherent method forwards to.");
            writer.open(format!(
                "pub {} {{",
                context.signature_with(Site::Inherent, context.node, context.callee_name())
            ));
            answer_body(writer, context, index);
            writer.close("}");
        }
        if !context.inherent() || callee != Callee::Method {
            writer.blank();
            writer.line("/// Answers what no forwarding may: an arm that calls it instead of the");
            writer.line("/// method meant shows in the law.");
            writer.open(format!("pub {} {{", context.signature(Site::Inherent)));
            poison_body(writer, context, index);
            writer.close("}");
        }
        writer.close("}");
        writer.blank();
        write_impl(writer, context, index);
        writer.blank();
        if matches!(context.cell.lift, Lift::ByRef | Lift::ByValue) {
            writer.open(format!("impl support::Counted for {payload} {{"));
            writer.open("fn count(&self) -> u64 {");
            writer.line("self.state");
            writer.close("}");
            writer.close("}");
            writer.blank();
        }
        let wrapped = slot
            .target()
            .is_some_and(|(_, target)| target.storage == Storage::Wrapper);
        if wrapped {
            let wrapper = slot.wrapper();
            writer.line(format!("pub struct {wrapper} {{"));
            writer.line("    label: u64,");
            writer.line(format!("    payload: {payload},"));
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
            writer.close("}");
            writer.blank();
        }
    }
}

fn write_impl(writer: &mut Writer, context: &Context, index: usize) {
    writer.open(format!("impl {TRAIT} for P{index} {{"));
    for (associated, overridden) in context.associated() {
        let ty = match (associated, overridden) {
            (Associated::Out, false) => String::from("u64"),
            (Associated::Out | Associated::Error, true) => format!("support::Narrow{index}"),
            (Associated::Error, false) => unreachable!("an error type is only ever overridden"),
        };
        writer.line(format!("type {} = {ty};", associated.name()));
    }
    writer.blank();
    writer.open(format!("{} {{", context.base_signature()));
    match index {
        0 => writer.line("self.state * 10"),
        _ => writer.line(format!("self.state * 10 + {index}")),
    }
    writer.close("}");
    let overrides = !context.inherent() && (!context.qualifier.provided || index == 0);
    if overrides {
        writer.blank();
        writer.open(format!("{} {{", context.signature(Site::Impl)));
        answer_body(writer, context, index);
        writer.close("}");
    }
    writer.close("}");
}

/// Returns how the body names the payload: `self`, or `this` after a
/// pinned receiver is unwrapped.
fn this(context: &Context) -> &'static str {
    if context.receiver.this == "self" {
        return "self";
    }
    "this"
}

fn answer_body(writer: &mut Writer, context: &Context, index: usize) {
    let this = this(context);
    if this != "self" {
        writer.line(format!("let this = {};", context.receiver.this));
    }
    if context.qualifier.generic == Some(Generic::Fixed) {
        writer.line("let x: u64 = x.into();");
    }
    if context.receiver.mutates {
        writer.line(format!("{this}.state += x + 1;"));
    }
    if context.argument.kind == Some(ArgumentKind::Exclusive) {
        writer.line("other.state += 2;");
    }
    let mut number = format!("{}_000_000 + {this}.state * 100 + x", index + 1);
    if context.argument.kind.is_some() {
        number.push_str(" + other.state * 7");
    }
    if context.qualifier.generic == Some(Generic::Turbofish) {
        number.push_str(" + W::VALUE * 10_000");
    }
    writer.line(format!("let n = {number};"));
    if !uses_number(&context.node) {
        writer.line("let _ = n;");
    }
    if context.node != Node::Unit {
        writer.line(value(&context.node, this, index, Flavour::Answer));
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
    let mut number = String::from("3_000_000 + self.base() * 100 + x");
    if context.argument.kind.is_some() {
        number.push_str(" + other.base() * 7");
    }
    writer.line(format!("let n = {number};"));
    if !uses_number(&context.node) {
        writer.line("let _ = n;");
    }
    if context.node != Node::Unit {
        writer.line(value(&context.node, "self", 3, Flavour::Answer));
    }
}

/// Returns whether a value of the type reads the answer's number.
fn uses_number(node: &Node) -> bool {
    match node {
        Node::Unit => false,
        Node::Payload | Node::PayloadShared | Node::PayloadExclusive => true,
        Node::Tuple(elements) => elements.iter().any(uses_number),
        Node::Tag
        | Node::Shared
        | Node::Exclusive
        | Node::This
        | Node::Assoc
        | Node::Overridden(_)
        | Node::Fault
        | Node::Option(_)
        | Node::Result(_, _)
        | Node::Future(_, _)
        | Node::Iterator
        | Node::Vec(_) => true,
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
        Node::Payload | Node::PayloadShared | Node::PayloadExclusive => {
            unreachable!("no payload answers with a payload; an inherent place reads it")
        }
        Node::Unit => String::from("()"),
        Node::Tag | Node::Assoc => String::from(number),
        Node::Shared => match flavour {
            Flavour::Answer => branch(format!("&{this}.state"), format!("&{this}.spare")),
            Flavour::Poison => String::from("&support::POISON_PLACE"),
        },
        Node::Exclusive => match flavour {
            Flavour::Answer => branch(format!("&mut {this}.state"), format!("&mut {this}.spare")),
            Flavour::Poison => format!("{{ {this}.spare = support::POISON; &mut {this}.spare }}"),
        },
        Node::This => format!("Self::new({number})"),
        Node::Overridden(_) => format!("support::Narrow{index}({number})"),
        Node::Fault => format!("support::Fault({number})"),
        Node::Option(inner_node) => {
            branch(format!("Some({})", inner(inner_node)), String::from("None"))
        }
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
