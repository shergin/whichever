//! The law: forwarding commutes with the variant's constructor. Calling
//! the method on the subject holding a payload answers what calling it on
//! a twin of that payload answers, converted as the return type asks, and
//! leaves the payload in the twin's state. The same body is checked
//! against the subject and against the hand.

use super::context::{seed, spell, Associates, Context, LABEL, OTHER_SEED, SAMPLES, TRAIT};
use super::payload::answer_type;
use super::writer::Writer;
use crate::model::argument::{ArgumentKind, MismatchSite};
use crate::model::qualifier::Generic;
use crate::model::returns::Node;
use crate::model::target::{Form, Pointer, Step, Storage, TargetDescriptor};
use crate::model::Slot;

/// Who gave an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// The subject, whose `Self` is the enum.
    Subject,
    /// A payload, by slot.
    Payload(usize),
}

/// Writes `mod law`.
pub fn write(writer: &mut Writer, context: &Context) {
    writer.line("#[cfg(test)]");
    writer.open("mod law {");
    writer.line("use super::*;");
    writer.blank();
    writer.line("/// The law, the same against the subject and the hand: `S` is the one");
    writer.line("/// under test.");
    writer.open("macro_rules! law {");
    writer.open("() => {");
    write_seen(writer, context);
    write_observers(writer, context);
    for (slot, _, target) in context.live() {
        write_slot(writer, context, &slot, &target);
    }
    write_mismatch(writer, context);
    writer.close("};");
    writer.close("}");
    writer.blank();
    writer.line("#[test]");
    writer.open("fn subject() {");
    writer.line("use super::E as S;");
    writer.anchor("law-subject");
    writer.line("law!();");
    writer.close("}");
    writer.blank();
    writer.line("#[test]");
    writer.open("fn hand() {");
    writer.line("use super::hand::E as S;");
    writer.anchor("law-hand");
    writer.line("law!();");
    writer.close("}");
    writer.close("}");
}

fn write_seen(writer: &mut Writer, context: &Context) {
    writer.line("/// Returns the variant a subject holds, by slot, and its payload's state.");
    writer.open("fn seen(subject: &S) -> String {");
    writer.open("match subject {");
    for (slot, _, target) in context.live() {
        let pattern = super::context::pattern("S", &slot, &target, "inner");
        writer.line(format!(
            "{pattern} => format!(\"{}:{{}}\", {}.state()),",
            slot.index,
            reach("inner", target.route)
        ));
    }
    writer.close("}");
    writer.close("}");
    writer.blank();
}

/// Returns the payload reached from a binding by method-call syntax,
/// which dereferences on its own.
fn reach(binding: &str, route: &[Step]) -> String {
    let mut text = String::from(binding);
    for step in route {
        match step {
            Step::Deref => {}
            Step::Field(name) => text.push_str(&format!(".{name}")),
            Step::Method(name) | Step::Yield(name) => text.push_str(&format!(".{name}()")),
        }
    }
    text
}

fn write_observers(writer: &mut Writer, context: &Context) {
    let output = context.output();
    let subject_type = spell(&output, "S", Associates::Qualified);
    writer.line("/// Returns what the subject's answer shows.");
    writer.open(format!("fn observe(answer: {subject_type}) -> String {{"));
    writer.line(observation(&output, "answer", Side::Subject, 0));
    writer.close("}");
    writer.blank();
    for (slot, _, _) in context.live() {
        let index = slot.index;
        writer.line(format!("/// Returns what the answer of `P{index}` shows."));
        writer.open(format!(
            "fn observe_p{index}(answer: {}) -> String {{",
            answer_type(&output, index)
        ));
        writer.line(observation(&output, "answer", Side::Payload(index), 0));
        writer.close("}");
        writer.blank();
    }
}

/// Returns an expression that maps the answer `value` to plain text.
fn observation(node: &Node, value: &str, side: Side, depth: usize) -> String {
    let inner = |node: &Node, value: &str| observation(node, value, side, depth + 1);
    let binding = format!("value{depth}");
    match node {
        Node::Unit => format!("{{ let () = {value}; String::from(\"()\") }}"),
        Node::Tag | Node::Assoc => format!("{value}.to_string()"),
        Node::Shared => format!("format!(\"&{{}}\", {value})"),
        Node::Exclusive => format!("format!(\"&mut {{}}\", {value})"),
        Node::This => match side {
            Side::Subject => format!("seen(&{value})"),
            Side::Payload(index) => format!("format!(\"{index}:{{}}\", {value}.state())"),
        },
        Node::Overridden(_) => match side {
            Side::Subject => format!("format!(\"{{:?}}\", {value})"),
            Side::Payload(_) => format!("format!(\"{{:?}}\", support::Wide::from({value}))"),
        },
        Node::Fault => format!("format!(\"{{:?}}\", {value})"),
        Node::Option(element) => format!(
            "match {value} {{ Some({binding}) => format!(\"Some({{}})\", {}), None => String::from(\"None\") }}",
            inner(element, &binding)
        ),
        Node::Result(ok, error) => format!(
            "match {value} {{ Ok({binding}) => format!(\"Ok({{}})\", {}), Err({binding}) => format!(\"Err({{}})\", {}) }}",
            inner(ok, &binding),
            inner(error, &binding)
        ),
        Node::Tuple(elements) => {
            let names: Vec<String> = (0..elements.len())
                .map(|index| format!("{binding}_{index}"))
                .collect();
            let shown: Vec<String> = elements
                .iter()
                .zip(&names)
                .map(|(element, name)| inner(element, name))
                .collect();
            let holes = vec!["{}"; elements.len()].join(", ");
            format!(
                "{{ let ({}) = {value}; format!(\"({holes})\", {}) }}",
                names.join(", "),
                shown.join(", ")
            )
        }
        Node::Future(_, _) | Node::Iterator | Node::Vec(_) => {
            unreachable!("the law observes a future's output, and a passing cell holds no iterator")
        }
    }
}

/// Writes the lines that build `name`, a subject holding a fresh payload
/// of `slot` made from `seed`, and returns nothing: the subject is bound.
fn write_subject(
    writer: &mut Writer,
    path: &str,
    slot: &Slot,
    target: &TargetDescriptor,
    name: &str,
    seed: u64,
    mutable: bool,
) {
    let payload = format!("{}::new({seed})", slot.payload());
    let stored = match target.storage {
        Storage::Payload => payload,
        Storage::Pointer(Pointer::Box) => format!("Box::new({payload})"),
        Storage::Pointer(Pointer::Rc) => format!("Rc::new({payload})"),
        Storage::Pointer(Pointer::Arc) => format!("Arc::new({payload})"),
        Storage::Pointer(Pointer::Shared) => {
            writer.line(format!("let {name}_held = {payload};"));
            format!("&{name}_held")
        }
        Storage::Pointer(Pointer::Exclusive) => {
            writer.line(format!("let mut {name}_held = {payload};"));
            format!("&mut {name}_held")
        }
        Storage::Wrapper => format!(
            "{} {{ label: {LABEL}, payload: {payload} }}",
            slot.wrapper()
        ),
        Storage::Pair => format!("({LABEL}, {payload})"),
    };
    let variant = slot.name();
    let value = match target.form {
        Form::Single => format!("{path}::{variant}({stored})"),
        Form::MarkedTuple => format!("{path}::{variant}({stored}, {LABEL})"),
        Form::MarkedStruct => format!("{path}::{variant} {{ label: {LABEL}, held: {stored} }}"),
    };
    let binding = if mutable { "let mut" } else { "let" };
    writer.line(format!("{binding} {name} = {value};"));
}

/// Returns a call of the method under test on `receiver`, with `other`
/// when it takes a `Self` argument, through the trait as `path` implements
/// it.
fn call(context: &Context, path: &str, receiver: &str, other: Option<&str>, x: &str) -> String {
    let mut arguments = vec![context.receiver.call.replace("{}", receiver)];
    if let (Some(kind), Some(other)) = (context.argument.kind, other) {
        arguments.push(match kind {
            ArgumentKind::Shared => format!("&{other}"),
            ArgumentKind::Exclusive => format!("&mut {other}"),
            ArgumentKind::Owned => String::from(other),
        });
    }
    arguments.push(String::from(x));
    let turbofish = match context.qualifier.generic {
        Some(Generic::Turbofish) => "::<support::Three>",
        Some(Generic::Fixed) | None => "",
    };
    let mut text = format!(
        "<{path} as {TRAIT}>::m{turbofish}({})",
        arguments.join(", ")
    );
    if context.qualifier.unsafety {
        text = format!("unsafe {{ {text} }}");
    }
    if context.awaited() {
        text = format!("support::block_on({text})");
    }
    text
}

fn write_slot(writer: &mut Writer, context: &Context, slot: &Slot, target: &TargetDescriptor) {
    let index = slot.index;
    let name = slot.name();
    let payload = slot.payload();
    let seed = seed(index);
    let exclusive = context.receiver.call.contains("&mut");
    let other_exclusive = context.argument.kind == Some(ArgumentKind::Exclusive);
    writer.line(format!(
        "// `{name}` holds a fresh `{payload}`, and a twin of it is called directly."
    ));
    let samples: Vec<String> = SAMPLES.iter().map(u64::to_string).collect();
    writer.open(format!("for x in [{}u64, {}] {{", samples[0], samples[1]));
    write_subject(writer, "S", slot, target, "subject", seed, exclusive);
    let binding = if exclusive { "let mut" } else { "let" };
    writer.line(format!("{binding} twin = {payload}::new({seed});"));
    if context.argument.kind.is_some() {
        write_subject(
            writer,
            "S",
            slot,
            target,
            "other",
            OTHER_SEED,
            other_exclusive,
        );
        let binding = if other_exclusive { "let mut" } else { "let" };
        writer.line(format!(
            "{binding} other_twin = {payload}::new({OTHER_SEED});"
        ));
    }
    let other = context.argument.kind.map(|_| "other");
    let other_twin = context.argument.kind.map(|_| "other_twin");
    writer.line(format!(
        "let answer = observe({});",
        call(context, "S", "subject", other, "x")
    ));
    writer.line(format!(
        "let expected = observe_p{index}({});",
        call(context, &payload, "twin", other_twin, "x")
    ));
    writer.line(format!(
        "assert_eq!(answer, expected, \"m on {name} with x = {{x}}\");"
    ));
    if !context.receiver.consumes {
        writer.line(format!(
            "assert_eq!(seen(&subject), format!(\"{index}:{{}}\", twin.state()), \"{name} after m with x = {{x}}\");"
        ));
        writer.line(format!(
            "assert_eq!(<S as {TRAIT}>::base(&subject), <{payload} as {TRAIT}>::base(&twin), \"base on {name} after m\");"
        ));
    }
    if other_exclusive {
        writer.line(format!(
            "assert_eq!(seen(&other), format!(\"{index}:{{}}\", other_twin.state()), \"the argument after m with x = {{x}}\");"
        ));
    }
    writer.close("}");
    writer.open("{");
    write_subject(writer, "S", slot, target, "subject", seed, false);
    writer.line(format!("let twin = {payload}::new({seed});"));
    writer.line(format!(
        "assert_eq!(<S as {TRAIT}>::base(&subject), <{payload} as {TRAIT}>::base(&twin), \"base on {name}\");"
    ));
    writer.close("}");
}

fn write_mismatch(writer: &mut Writer, context: &Context) {
    if context.argument.kind.is_none() || context.argument.mismatch == MismatchSite::Missing {
        return;
    }
    let live = context.live();
    let [(first, _, first_target), (second, _, second_target)] = live.as_slice() else {
        return;
    };
    let exclusive = context.receiver.call.contains("&mut");
    let other_exclusive = context.argument.kind == Some(ArgumentKind::Exclusive);
    writer.line(format!(
        "// `{}` against `{}`: two different variants answer the mismatch.",
        first.name(),
        second.name()
    ));
    writer.open("{");
    write_subject(
        writer,
        "S",
        first,
        first_target,
        "subject",
        seed(0),
        exclusive,
    );
    write_subject(
        writer,
        "S",
        second,
        second_target,
        "other",
        OTHER_SEED,
        other_exclusive,
    );
    let output = context.output();
    writer.line(format!(
        "let mismatch: {} = {};",
        spell(&output, "S", Associates::Qualified),
        context.mismatched_output("S")
    ));
    writer.line(format!(
        "let answer = observe({});",
        call(context, "S", "subject", Some("other"), "2u64")
    ));
    writer.line("assert_eq!(answer, observe(mismatch), \"m on two different variants\");");
    writer.close("}");
}
