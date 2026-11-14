//! The law: forwarding commutes with the variant's constructor. Calling
//! the method on the subject holding a payload answers what calling it on
//! a twin of that payload answers, converted as the return type asks, and
//! leaves the payload in the twin's state. The same body is checked
//! against the subject and against the hand.

use super::context::{seed, spell, Associates, Context, LABEL, OTHER_SEED, SAMPLES, TRAIT};
use super::payload::answer_type;
use super::writer::Writer;
use crate::model::argument::{ArgumentKind, MismatchSite};
use crate::model::path::Form as PathForm;
use crate::model::qualifier::Generic;
use crate::model::returns::Node;
use crate::model::target::{Form, Pointer, Step, Storage};
use crate::model::{Slot, SlotVariant};

/// Who gave an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// The subject, whose `Self` is the enum.
    Subject,
    /// A payload, by slot.
    Payload(usize),
}

/// Writes `mod law` with the tests named, `subject` against `E` and
/// `hand` against `hand::E`.
pub fn write(writer: &mut Writer, context: &Context, tests: &[&str]) {
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
    for slot in &context.slots {
        write_slot(writer, context, slot);
    }
    write_mismatch(writer, context);
    writer.close("};");
    writer.close("}");
    for test in tests {
        let path = match *test {
            "subject" => "super::E",
            _ => "super::hand::E",
        };
        writer.blank();
        writer.line("#[test]");
        writer.open(format!("fn {test}() {{"));
        writer.line(format!("use {path} as S;"));
        writer.anchor(format!("law-{test}"));
        writer.line("law!();");
        writer.close("}");
    }
    writer.close("}");
}

fn write_seen(writer: &mut Writer, context: &Context) {
    writer.line("/// Returns the variant a subject holds, by slot, and its payload's state.");
    writer.open("fn seen(subject: &S) -> String {");
    writer.open("match subject {");
    for slot in &context.slots {
        let index = slot.index;
        let name = slot.name();
        let arm = match slot.variant {
            SlotVariant::Live(target) => {
                let target = target.descriptor();
                let pattern = super::context::pattern("S", slot, &target, "inner");
                format!(
                    "{pattern} => format!(\"{index}:{{}}\", {}.state()),",
                    reach("inner", target.route)
                )
            }
            SlotVariant::Unit => format!("S::{name} => String::from(\"{index}:unit\"),"),
            SlotVariant::Pair => {
                format!("S::{name}(inner, _) => format!(\"{index}:{{}}\", inner.state()),")
            }
        };
        writer.line(arm);
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
    for (slot, _, _) in context.cell.method_slots() {
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
        Node::Iterator => format!("format!(\"{{:?}}\", {value}.collect::<Vec<u64>>())"),
        Node::Vec(element) => format!(
            "{value}.into_iter().map(|{binding}| {}).collect::<Vec<String>>().join(\", \")",
            inner(element, &binding)
        ),
        Node::Future(_, _) => {
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
    name: &str,
    seed: u64,
    mutable: bool,
) {
    let payload = format!("{}::new({seed})", slot.payload());
    let variant = slot.name();
    let binding = if mutable { "let mut" } else { "let" };
    let target = match slot.variant {
        SlotVariant::Live(target) => target.descriptor(),
        SlotVariant::Unit => {
            writer.line(format!("{binding} {name} = {path}::{variant};"));
            return;
        }
        SlotVariant::Pair => {
            writer.line(format!(
                "{binding} {name} = {path}::{variant}({payload}, {LABEL});"
            ));
            return;
        }
    };
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
    let value = match target.form {
        Form::Single => format!("{path}::{variant}({stored})"),
        Form::MarkedTuple => format!("{path}::{variant}({stored}, {LABEL})"),
        Form::MarkedStruct => format!("{path}::{variant} {{ label: {LABEL}, held: {stored} }}"),
    };
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
    let mut text = match context.inherent() {
        true => format!("{path}::m({})", arguments.join(", ")),
        false => format!(
            "<{path} as {TRAIT}>::m{turbofish}({})",
            arguments.join(", ")
        ),
    };
    if context.qualifier.unsafety {
        text = format!("unsafe {{ {text} }}");
    }
    if context.awaited() {
        text = format!("support::block_on({text})");
    }
    text
}

/// What the law expects of a call on a slot.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Expect {
    /// What a twin of the slot's payload answers.
    Twin,
    /// A constant: the body a block writes, by its expression.
    Constant(String),
    /// What the trait's default body answers.
    Default,
}

fn expectations(context: &Context, slot: &Slot) -> (Expect, Expect) {
    let forwarded =
        |slots: Vec<(Slot, _, _)>| slots.iter().any(|(other, _, _)| other.index == slot.index);
    let method = match context.path.form {
        PathForm::Forwarded | PathForm::First if forwarded(context.cell.method_slots()) => {
            Expect::Twin
        }
        PathForm::Default => Expect::Default,
        PathForm::Forwarded | PathForm::First | PathForm::Written => {
            Expect::Constant(context.mismatched_output("S"))
        }
    };
    let base = match forwarded(context.cell.base_slots()) {
        true => Expect::Twin,
        false => Expect::Constant(String::from("0")),
    };
    (method, base)
}

fn write_slot(writer: &mut Writer, context: &Context, slot: &Slot) {
    let index = slot.index;
    let name = slot.name();
    let payload = slot.payload();
    let seed = seed(index);
    let (method, base) = expectations(context, slot);
    let exclusive = context.receiver.call.contains("&mut");
    let other_exclusive = context.argument.kind == Some(ArgumentKind::Exclusive);
    let comment = match method {
        Expect::Twin => {
            format!("`{name}` holds a fresh `{payload}`, and a twin of it is called directly.")
        }
        Expect::Constant(_) => format!("`{name}` answers the body the block writes."),
        Expect::Default => format!("`{name}` answers what the trait's default body answers."),
    };
    writer.line(format!("// {comment}"));
    let samples: Vec<String> = SAMPLES.iter().map(u64::to_string).collect();
    writer.open(format!("for x in [{}u64, {}] {{", samples[0], samples[1]));
    write_subject(writer, "S", slot, "subject", seed, exclusive);
    let twin_needed = method == Expect::Twin;
    if twin_needed {
        let binding = if exclusive { "let mut" } else { "let" };
        writer.line(format!("{binding} twin = {payload}::new({seed});"));
    }
    if context.argument.kind.is_some() {
        write_subject(writer, "S", slot, "other", OTHER_SEED, other_exclusive);
        if twin_needed {
            let binding = if other_exclusive { "let mut" } else { "let" };
            writer.line(format!(
                "{binding} other_twin = {payload}::new({OTHER_SEED});"
            ));
        }
    }
    let other = context.argument.kind.map(|_| "other");
    writer.line(format!(
        "let answer = observe({});",
        call(context, "S", "subject", other, "x")
    ));
    match &method {
        Expect::Twin => {
            let other_twin = context.argument.kind.map(|_| "other_twin");
            writer.line(format!(
                "let expected = observe_p{index}({});",
                call(context, &payload, "twin", other_twin, "x")
            ));
        }
        Expect::Constant(constant) => {
            writer.line(format!(
                "let written: {} = {constant};",
                spell(&context.output(), "S", Associates::Qualified)
            ));
            writer.line("let expected = observe(written);");
        }
        Expect::Default => {
            let base_of =
                |seed: u64| context.base_call(&payload, &format!("{payload}::new({seed})"));
            let binding = if exclusive { "let mut" } else { "let" };
            writer.line(format!(
                "{binding} defaulted = Defaulted {{ base: {} }};",
                base_of(seed)
            ));
            if context.argument.kind.is_some() {
                let binding = if other_exclusive { "let mut" } else { "let" };
                writer.line(format!(
                    "{binding} other_defaulted = Defaulted {{ base: {} }};",
                    base_of(OTHER_SEED)
                ));
            }
            let other_defaulted = context.argument.kind.map(|_| "other_defaulted");
            writer.line(format!(
                "let expected = observe({});",
                call(context, "Defaulted", "defaulted", other_defaulted, "x")
            ));
        }
    }
    writer.line(format!(
        "assert_eq!(answer, expected, \"m on {name} with x = {{x}}\");"
    ));
    if !context.receiver.consumes {
        let state = match (&method, slot.variant) {
            (Expect::Twin, _) => String::from("twin.state()"),
            (Expect::Constant(_) | Expect::Default, SlotVariant::Unit) => String::from("\"unit\""),
            (Expect::Constant(_) | Expect::Default, SlotVariant::Live(_) | SlotVariant::Pair) => {
                seed.to_string()
            }
        };
        writer.line(format!(
            "assert_eq!(seen(&subject), format!(\"{index}:{{}}\", {state}), \"{name} after m with x = {{x}}\");"
        ));
        if twin_needed && base == Expect::Twin && !context.base_receiver().descriptor().consumes {
            writer.line(format!(
                "assert_eq!({}, {}, \"base on {name} after m\");",
                context.base_call("S", "subject"),
                context.base_call(&payload, "twin")
            ));
        }
    }
    if other_exclusive && twin_needed {
        writer.line(format!(
            "assert_eq!(seen(&other), format!(\"{index}:{{}}\", other_twin.state()), \"the argument after m with x = {{x}}\");"
        ));
    }
    writer.close("}");
    writer.open("{");
    write_subject(writer, "S", slot, "subject", seed, false);
    match base {
        Expect::Twin => {
            writer.line(format!("let twin = {payload}::new({seed});"));
            writer.line(format!(
                "assert_eq!({}, {}, \"base on {name}\");",
                context.base_call("S", "subject"),
                context.base_call(&payload, "twin")
            ));
        }
        Expect::Constant(constant) => writer.line(format!(
            "assert_eq!({}, {constant}, \"base on {name}\");",
            context.base_call("S", "subject")
        )),
        Expect::Default => unreachable!("the control method is never left to a default"),
    }
    writer.close("}");
}

fn write_mismatch(writer: &mut Writer, context: &Context) {
    if context.argument.kind.is_none() {
        return;
    }
    let checked = match context.path.form {
        PathForm::Forwarded => context.argument.mismatch != MismatchSite::Missing,
        PathForm::First => true,
        PathForm::Default | PathForm::Written => false,
    };
    let [first, second] = context.slots.as_slice() else {
        return;
    };
    if !checked {
        return;
    }
    let exclusive = context.receiver.call.contains("&mut");
    let other_exclusive = context.argument.kind == Some(ArgumentKind::Exclusive);
    writer.line(format!(
        "// `{}` against `{}`: two different variants answer the mismatch.",
        first.name(),
        second.name()
    ));
    writer.open("{");
    write_subject(writer, "S", first, "subject", seed(0), exclusive);
    write_subject(writer, "S", second, "other", OTHER_SEED, other_exclusive);
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
