//! The law: forwarding commutes with the variant's constructor. Calling
//! the method on the subject holding a payload answers what calling it on
//! a twin of that payload answers, converted as the return type asks, and
//! leaves the payload in the twin's state. The same body is checked
//! against the subject and against the hand.

use super::context::{seed, spell, Associates, Context, LABEL, OTHER_SEED, SAMPLES};
use super::payload::answer_type;
use super::writer::Writer;
use crate::model::agreement::ItemKind;
use crate::model::argument::{ArgumentKind, MismatchSite};
use crate::model::path::Form as PathForm;
use crate::model::qualifier::Generic;
use crate::model::returns::Node;
use crate::model::stack::OTHER;
use crate::model::target::{Form, Pointer, Step, Storage, TargetDescriptor};
use crate::model::{Callee, Lift, Slot, SlotVariant, Target};

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
    if context.foreign() {
        write_foreign(writer, context, tests);
        return;
    }
    writer.line("#[cfg(test)]");
    // The law calls a deprecated method, or names a deprecated variant,
    // which the deprecation reaches.
    let deprecated = context.declared.iter().any(|slot| slot.deprecated);
    if context.item.deprecated || deprecated {
        writer.line("#[allow(deprecated)]");
    }
    writer.open("mod law {");
    // Over an enum empty in this build the law names the subject's type
    // alone.
    if context.slots.is_empty() {
        writer.line("#[allow(unused_imports)]");
    }
    writer.line("use super::*;");
    writer.blank();
    writer.line("/// The law, the same against the subject and the hand: `S` is the one");
    writer.line("/// under test.");
    writer.open("macro_rules! law {");
    writer.open("() => {");
    write_seen(writer, context);
    write_associated(writer, context);
    // A method in no build is not called; the control method still is.
    let present = context.cell.method_present() && !context.slots.is_empty();
    if present {
        write_observers(writer, context);
    }
    for slot in &context.slots {
        write_slot(writer, context, slot, present);
    }
    if present {
        write_mismatch(writer, context);
    }
    super::conversions::write_law(writer, context);
    writer.close("};");
    writer.close("}");
    write_tests(writer, context, tests);
    writer.close("}");
}

/// Writes the law of a cell whose trait is `whichever::foreign::Display`:
/// what the subject prints is what a twin of its payload prints, or the
/// body the block writes, and printing leaves the payload as it was.
fn write_foreign(writer: &mut Writer, context: &Context, tests: &[&str]) {
    writer.line("#[cfg(test)]");
    if context.declared.iter().any(|slot| slot.deprecated) {
        writer.line("#[allow(deprecated)]");
    }
    writer.open("mod law {");
    // Where no variant holding a payload is in the build, the law names
    // the subject's type alone.
    if context.live().is_empty() {
        writer.line("#[allow(unused_imports)]");
    }
    writer.line("use super::*;");
    writer.blank();
    writer.line("/// The law, the same against the subject and the hand: `S` is the one");
    writer.line("/// under test.");
    writer.open("macro_rules! law {");
    writer.open("() => {");
    write_seen(writer, context);
    let forwarded = context.cell.method_slots();
    for slot in &context.slots {
        let index = slot.index;
        let name = slot.name();
        let seed = seed(index);
        let twin = forwarded_to(slot, forwarded.clone());
        match twin {
            true => writer.line(format!(
                "// `{name}` prints what a twin of its `{}` prints.",
                slot.payload()
            )),
            false => writer.line(format!("// `{name}` prints the body the block writes.")),
        }
        writer.open("{");
        write_subject(writer, &value_path(context), slot, "subject", seed, false);
        let expected = match twin {
            true => {
                writer.line(format!("let twin = {}::new({seed});", slot.payload()));
                String::from("format!(\"{}\", twin)")
            }
            false => format!("String::from(\"{}\")", super::foreign::WRITTEN),
        };
        writer.line(format!(
            "assert_eq!(format!(\"{{}}\", subject), {expected}, \"fmt on {name}\");"
        ));
        let state = match slot.variant {
            SlotVariant::Unit => String::from("\"unit\""),
            SlotVariant::Live(_) | SlotVariant::Pair => seed.to_string(),
        };
        writer.line(format!(
            "assert_eq!(seen(&subject), format!(\"{index}:{{}}\", {state}), \"{name} after fmt\");"
        ));
        writer.close("}");
    }
    super::conversions::write_law(writer, context);
    writer.close("};");
    writer.close("}");
    write_tests(writer, context, tests);
    writer.close("}");
}

/// The name a test imports the subject's own path by, beside the alias `S`.
const BUILT: &str = "Built";

/// Returns whether the subject is a generic tuple struct, whose constructor
/// and pattern an alias cannot spell.
fn generic_tuple_struct(context: &Context) -> bool {
    let tuple = context
        .slots
        .first()
        .and_then(Slot::target)
        .is_some_and(|(_, target)| target.form == Form::Struct);
    tuple && context.generics.parameter.is_some()
}

/// Returns the path the law builds the subject by and takes it apart by:
/// `S`, or, for a generic tuple struct, the subject's own path with its
/// argument, `Built::<P0>`.
fn value_path(context: &Context) -> String {
    match (generic_tuple_struct(context), context.generics.parameter) {
        (true, Some(parameter)) => format!("{BUILT}::<{}>", parameter.instance),
        (true, None) | (false, _) => String::from("S"),
    }
}

/// Writes the tests that run the law, `subject` against `E` and `hand`
/// against `hand::E`.
fn write_tests(writer: &mut Writer, context: &Context, tests: &[&str]) {
    for test in tests {
        let path = match *test {
            "subject" => "super::E",
            _ => "super::hand::E",
        };
        writer.blank();
        writer.line("#[test]");
        writer.open(format!("fn {test}() {{"));
        // A generic subject is named at the argument the law takes it at.
        // A restated enum is generic over what its variants hold.
        let generic = context.generics.parameter.is_some()
            || context.cell.owner == crate::model::Owner::Remote;
        match generic {
            true => writer.line(format!("type S = {};", context.subject_instance(path))),
            false => writer.line(format!("use {path} as S;")),
        }
        if generic_tuple_struct(context) {
            writer.line(format!("use {path} as {BUILT};"));
        }
        writer.anchor(format!("law-{test}"));
        writer.line("law!();");
        writer.close("}");
    }
}

fn write_seen(writer: &mut Writer, context: &Context) {
    writer.line("/// Returns the variant a subject holds, by slot, and its payload's state.");
    writer.open("fn seen(subject: &S) -> String {");
    if context.slots.is_empty() {
        // Every variant is compiled out: the enum is empty, there is no
        // value to call anything on, and the law only names its type.
        writer.line("match *subject {}");
        writer.close("}");
        writer.line("let _ = seen;");
        writer.blank();
        return;
    }
    writer.open("match subject {");
    for slot in &context.slots {
        let index = slot.index;
        let name = slot.name();
        let arm = match slot.variant {
            SlotVariant::Live(target) => {
                let target = target.descriptor();
                let pattern = super::context::pattern(&value_path(context), slot, &target, "inner");
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

/// Writes the checks of the trait's own associated items: each is the
/// value the enum fixes, or the supplier's.
fn write_associated(writer: &mut Writer, context: &Context) {
    let items = context.agreement.items;
    if items.is_empty() {
        return;
    }
    // The law names a payload by its own type, a type parameter's at the
    // argument the law takes it at.
    let supplier = context
        .cell
        .supplier()
        .map_or_else(|| String::from("P0"), |slot| slot.payload());
    let instance = context.trait_instance();
    writer.line(format!(
        "// The associated items: fixed on the enum, or `{supplier}`'s."
    ));
    let mut traits = vec![instance.clone()];
    if context.stack.present {
        traits.push(String::from(OTHER));
    }
    for (item, instance) in traits
        .iter()
        .flat_map(|instance| items.iter().map(move |item| (item, instance)))
    {
        let name = item.name;
        match (item.kind, item.overridden) {
            (ItemKind::Type, _) => {
                // A value of the supplier's type is one of the subject's.
                writer.open("{");
                writer.line(format!("let kind: <{supplier} as {instance}>::{name} = 1;"));
                writer.line(format!("let _: <S as {instance}>::{name} = kind;"));
                writer.close("}");
            }
            (ItemKind::Const, Some(value)) => writer.line(format!(
                "assert_eq!(<S as {instance}>::{name}, {value}, \"{name}\");"
            )),
            (ItemKind::Const, None) => writer.line(format!(
                "assert_eq!(<S as {instance}>::{name}, <{supplier} as {instance}>::{name}, \"{name}\");"
            )),
        }
    }
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
    let declared = match context.inherent() {
        true => {
            let declared = context.cell.declared_node();
            match declared.future() {
                Some((_, inner)) => *inner,
                None => declared,
            }
        }
        false => output,
    };
    // The law has no lifetime of its own to name, so a return for the
    // subject's lifetime is observed as an elided reference.
    let subject_type = spell(&declared, "S", Associates::Qualified).replace("&'a ", "&");
    writer.line("/// Returns what the subject's answer shows.");
    writer.open(format!("fn observe(answer: {subject_type}) -> String {{"));
    writer.line(observation(&declared, "answer", Side::Subject, 0));
    writer.close("}");
    writer.blank();
    for (slot, _, _) in context.cell.method_slots() {
        let index = slot.index;
        writer.line(format!("/// Returns what the answer of `P{index}` shows."));
        writer.open(format!(
            "fn observe_p{index}(answer: {}) -> String {{",
            answer_type(&output, index).replace("&'a ", "&")
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
        // An answer of `()` shows nothing, so what the call noted is read
        // back, right after the call: a dropped call, a call of the poison
        // or a wrong argument notes otherwise than the twin's.
        Node::Unit => format!("{{ let () = {value}; format!(\"() noted {{}}\", support::take_note()) }}"),
        Node::Tag | Node::Assoc => format!("{value}.to_string()"),
        Node::Shared | Node::Lent => format!("format!(\"&{{}}\", {value})"),
        // A mutable reference is written through, so that the state checked
        // afterwards shows whether it was the payload's own.
        Node::Exclusive => format!("{{ *{value} += 100; format!(\"&mut {{}}\", {value}) }}"),
        Node::Payload => format!("{value}.state.to_string()"),
        Node::PayloadShared | Node::PayloadLent => format!("format!(\"&{{}}\", {value}.state)"),
        Node::PayloadExclusive => {
            format!("{{ {value}.state += 100; format!(\"&mut {{}}\", {value}.state) }}")
        }
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
        Node::Fallible(ok) => observation(&Node::Result(ok, &Node::Fault), value, side, depth),
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
    let wrapper = format!(
        "{} {{ label: {LABEL}, payload: {payload} }}",
        slot.wrapper()
    );
    let stored = match target.storage {
        Storage::Payload => payload,
        Storage::Pointer(pointer) => point(writer, pointer, name, payload),
        Storage::Wrapper => wrapper,
        Storage::Pair => format!("({LABEL}, {payload})"),
        Storage::WrapperBehind(pointer) => point(writer, pointer, name, wrapper),
        Storage::WrapperInPair => format!("({LABEL}, {wrapper})"),
    };
    let value = match target.form {
        Form::Single => format!("{path}::{variant}({stored})"),
        Form::MarkedTuple => format!("{path}::{variant}({stored}, {LABEL})"),
        Form::MarkedStruct => format!("{path}::{variant} {{ label: {LABEL}, held: {stored} }}"),
        Form::Struct => format!("{path}({stored})"),
        Form::StructMarked => format!("{path} {{ label: {LABEL}, held: {stored} }}"),
        Form::StructNamed => format!("{path} {{ held: {stored} }}"),
    };
    writer.line(format!("{binding} {name} = {value};"));
}

/// Returns a pointer to `value`, which the binding `name` is built around:
/// a reference points at a binding of its own, written before it.
fn point(writer: &mut Writer, pointer: Pointer, name: &str, value: String) -> String {
    match pointer {
        Pointer::Box => format!("Box::new({value})"),
        Pointer::Rc => format!("Rc::new({value})"),
        Pointer::Arc => format!("Arc::new({value})"),
        Pointer::Shared => {
            writer.line(format!("let {name}_held = {value};"));
            format!("&{name}_held")
        }
        Pointer::Exclusive => {
            writer.line(format!("let mut {name}_held = {value};"));
            format!("&mut {name}_held")
        }
    }
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
    if context.takes_x() {
        arguments.push(String::from(x));
    }
    let turbofish = match context.qualifier.generic {
        Some(Generic::Turbofish) => "::<support::Three>",
        Some(Generic::Fixed) | None => "",
    };
    let mut text = match context.inherent() {
        true if path == "S" => format!("{path}::m{turbofish}({})", arguments.join(", ")),
        true => format!(
            "{path}::{}{turbofish}({})",
            context.callee_name(),
            arguments.join(", ")
        ),
        false => format!(
            "<{path} as {}>::m{turbofish}({})",
            context.trait_instance(),
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

/// Returns whether the slot is among `slots`.
fn forwarded_to(slot: &Slot, slots: Vec<(Slot, Target, TargetDescriptor)>) -> bool {
    slots.iter().any(|(other, _, _)| other.index == slot.index)
}

/// Returns what the law expects of the method under test on a slot.
fn method_expectation(context: &Context, slot: &Slot) -> Expect {
    let forwarded = |slots| forwarded_to(slot, slots);
    match context.path.form {
        PathForm::Forwarded | PathForm::First if forwarded(context.cell.method_slots()) => {
            Expect::Twin
        }
        PathForm::Default => Expect::Default,
        PathForm::Forwarded | PathForm::First | PathForm::Written if context.inherent() => {
            Expect::Constant(super::context::constant(
                &context.cell.declared_node(),
                "S::A(P0::new(424_242))",
            ))
        }
        PathForm::Forwarded | PathForm::First | PathForm::Written => {
            Expect::Constant(context.mismatched_output(&value_path(context)))
        }
    }
}

/// Returns what the law expects of the control method on a slot.
fn base_expectation(context: &Context, slot: &Slot) -> Expect {
    match forwarded_to(slot, context.cell.base_slots()) {
        true => Expect::Twin,
        false => Expect::Constant(String::from("0")),
    }
}

/// Returns what an inherent method's twin answers, observed: the twin's
/// own method, or the twin or its field read as the declared type asks,
/// wrapped in the lift.
fn inherent_expected(context: &Context, index: usize, payload: &str) -> String {
    let lift = context.cell.lift;
    match context.cell.callee {
        Callee::Method | Callee::Renamed => {
            let other = context.argument.kind.map(|_| "other_twin");
            let answer = call(context, payload, "twin", other, "x");
            match lift {
                Lift::Bare | Lift::Into => format!("observe_p{index}({answer})"),
                Lift::Some => format!("format!(\"Some({{}})\", observe_p{index}({answer}))"),
                Lift::Ok => format!("format!(\"Ok({{}})\", observe_p{index}({answer}))"),
                Lift::ByRef => format!("support::by_ref({answer}).to_string()"),
                Lift::ByValue => format!("support::by_value({answer}).to_string()"),
            }
        }
        Callee::Value | Callee::Field => {
            let place = match context.cell.callee {
                Callee::Value => "twin",
                Callee::Field | Callee::Method | Callee::Renamed => "twin.state",
            };
            let read = match context.cell.target_node() {
                Node::PayloadShared | Node::PayloadLent | Node::Shared | Node::Lent => {
                    format!("&{place}")
                }
                Node::PayloadExclusive | Node::Exclusive => format!("&mut {place}"),
                Node::Payload
                | Node::Tag
                | Node::Unit
                | Node::This
                | Node::Assoc
                | Node::Overridden(_)
                | Node::Fault
                | Node::Option(_)
                | Node::Result(_, _)
                | Node::Fallible(_)
                | Node::Tuple(_)
                | Node::Future(_, _)
                | Node::Iterator
                | Node::Vec(_) => String::from(place),
            };
            let lifted = match lift {
                Lift::Bare | Lift::Into => read,
                Lift::Some => format!("Some({read})"),
                Lift::Ok => format!("Ok({read})"),
                Lift::ByRef => format!("support::by_ref(&{place})"),
                Lift::ByValue => format!("support::by_value({place})"),
            };
            format!("observe({lifted})")
        }
    }
}

/// Writes the checks of one slot: the method under test's when `present`,
/// then the control method's.
fn write_slot(writer: &mut Writer, context: &Context, slot: &Slot, present: bool) {
    let index = slot.index;
    let name = slot.name();
    let payload = slot.payload();
    let seed = seed(index);
    let base = base_expectation(context, slot);
    if present {
        let method = method_expectation(context, slot);
        let base_twin = base == Expect::Twin;
        write_method_checks(writer, context, slot, slot, &method, base_twin);
        // An inherent method hands its `Self` argument over as the enum
        // (inherent-methods.md), which the payload's answer reads, so the
        // argument holds another variant too: an arm that handed over the
        // matched payload, or a pair of the same variant, shows there.
        let position = context
            .slots
            .iter()
            .position(|other| other.index == slot.index);
        let next = position.map(|position| &context.slots[(position + 1) % context.slots.len()]);
        let another = next.filter(|next| next.index != slot.index);
        if let (true, Expect::Twin, Some(another)) = (context.hands_the_enum(), &method, another) {
            write_method_checks(writer, context, slot, another, &method, base_twin);
        }
    }
    writer.open("{");
    write_subject(writer, &value_path(context), slot, "subject", seed, false);
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
    // The second trait's method answers what a twin's does, on every
    // variant that holds a payload.
    if context.stack.present && slot.target().is_some() {
        write_subject(writer, &value_path(context), slot, "stacked", seed, false);
        writer.line(format!("let stacked_twin = {payload}::new({seed});"));
        writer.line(format!(
            "assert_eq!({}, {}, \"other on {name}\");",
            context.other_call("S", "stacked"),
            context.other_call(&payload, "stacked_twin")
        ));
    }
    writer.close("}");
}

/// Writes the calls of the method under test on one slot, for every
/// sample, with what each is expected to answer and leave behind; a `Self`
/// argument holds the variant of `argument`, which is the slot itself
/// unless the enum is handed over as it is. `base_twin` says whether the
/// control method answers what a twin does.
fn write_method_checks(
    writer: &mut Writer,
    context: &Context,
    slot: &Slot,
    argument: &Slot,
    method: &Expect,
    base_twin: bool,
) {
    let index = slot.index;
    let name = slot.name();
    let payload = slot.payload();
    let seed = seed(index);
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
    if argument.index != slot.index {
        writer.line(format!(
            "// The `Self` argument is the enum holding `{}`.",
            argument.name()
        ));
    }
    let samples: Vec<String> = SAMPLES.iter().map(u64::to_string).collect();
    writer.open(format!("for x in [{}u64, {}] {{", samples[0], samples[1]));
    write_subject(
        writer,
        &value_path(context),
        slot,
        "subject",
        seed,
        exclusive,
    );
    let twin_needed = *method == Expect::Twin;
    // A twin is borrowed mutably by its own method under a `&mut` receiver,
    // or, for an inherent place, where the declared type borrows it so.
    let twin_exclusive = match context.inherent() && !context.takes_x() {
        true => context.node.borrows_mutably(),
        false => exclusive,
    };
    if twin_needed {
        let binding = if twin_exclusive { "let mut" } else { "let" };
        writer.line(format!("{binding} twin = {payload}::new({seed});"));
    }
    // The twin of the `Self` argument is the same variant's payload, or,
    // where an inherent method hands the enum over as it is, the enum.
    let enum_argument = context.hands_the_enum();
    if context.argument.kind.is_some() {
        write_subject(
            writer,
            &value_path(context),
            argument,
            "other",
            OTHER_SEED,
            other_exclusive,
        );
        if twin_needed && enum_argument {
            write_subject(
                writer,
                &value_path(context),
                argument,
                "other_twin",
                OTHER_SEED,
                other_exclusive,
            );
        }
        if twin_needed && !enum_argument {
            let binding = if other_exclusive { "let mut" } else { "let" };
            writer.line(format!(
                "{binding} other_twin = {payload}::new({OTHER_SEED});"
            ));
        }
    }
    let other = context.argument.kind.map(|_| "other");
    // A return for the subject's lifetime is the very reference the
    // variant holds, or the place behind it, not a copy of its value.
    if matches!(context.node, Node::Lent) && twin_needed {
        write_subject(writer, &value_path(context), slot, "lent", seed, exclusive);
        let place = match context.cell.callee {
            Callee::Field => "&lent_held.state",
            Callee::Value | Callee::Method | Callee::Renamed => "&lent_held",
        };
        writer.line(format!(
            "assert!(core::ptr::eq({}, {place}), \"m on {name} hands over the place itself\");",
            call(context, "S", "lent", None, "x")
        ));
    }
    writer.line(format!(
        "let answer = observe({});",
        call(context, "S", "subject", other, "x")
    ));
    match method {
        Expect::Twin if context.inherent() => {
            writer.line(format!(
                "let expected = {};",
                inherent_expected(context, index, &payload)
            ));
        }
        Expect::Twin => {
            let other_twin = context.argument.kind.map(|_| "other_twin");
            writer.line(format!(
                "let expected = observe_p{index}({});",
                call(context, &payload, "twin", other_twin, "x")
            ));
        }
        Expect::Constant(constant) => {
            let written = match context.inherent() {
                true => context.cell.declared_node(),
                false => context.output(),
            };
            writer.line(format!(
                "let written: {} = {constant};",
                spell(&written, "S", Associates::Qualified).replace("&'a ", "&")
            ));
            writer.line("let expected = observe(written);");
        }
        Expect::Default => {
            // The default body reads the control method, which answers what
            // a payload of the slot answers, or the block's body.
            let base_of = |seed: u64| match base_twin {
                true => context.base_call(&payload, &format!("{payload}::new({seed})")),
                false => String::from("0"),
            };
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
        let state = match (method, slot.variant) {
            (Expect::Twin, _) => String::from("twin.state()"),
            (Expect::Constant(_) | Expect::Default, SlotVariant::Unit) => String::from("\"unit\""),
            (Expect::Constant(_) | Expect::Default, SlotVariant::Live(_) | SlotVariant::Pair) => {
                seed.to_string()
            }
        };
        writer.line(format!(
            "assert_eq!(seen(&subject), format!(\"{index}:{{}}\", {state}), \"{name} after m with x = {{x}}\");"
        ));
        if twin_needed && base_twin && !context.base_receiver().descriptor().consumes {
            writer.line(format!(
                "assert_eq!({}, {}, \"base on {name} after m\");",
                context.base_call("S", "subject"),
                context.base_call(&payload, "twin")
            ));
        }
    }
    if other_exclusive && twin_needed && !enum_argument {
        writer.line(format!(
            "assert_eq!(seen(&other), format!(\"{index}:{{}}\", other_twin.state()), \"the argument after m with x = {{x}}\");"
        ));
    }
    writer.close("}");
}

fn write_mismatch(writer: &mut Writer, context: &Context) {
    // An inherent method hands its `Self` argument over and has no
    // mismatch.
    if context.argument.kind.is_none() || context.inherent() {
        return;
    }
    let checked = match context.path.form {
        PathForm::Forwarded => context.argument.mismatch != MismatchSite::Missing,
        PathForm::First => true,
        PathForm::Default | PathForm::Written => false,
    };
    if !checked || context.slots.len() < 2 {
        return;
    }
    // Each variant against the next, and over three variants the last
    // against the first, so that every variant meets another on either
    // side of the `match`.
    let slots = &context.slots;
    let mut pairs: Vec<(&Slot, &Slot)> = slots.iter().zip(slots.iter().skip(1)).collect();
    if slots.len() > 2 {
        pairs.push((&slots[slots.len() - 1], &slots[0]));
    }
    let exclusive = context.receiver.call.contains("&mut");
    let other_exclusive = context.argument.kind == Some(ArgumentKind::Exclusive);
    for (first, second) in pairs {
        writer.line(format!(
            "// `{}` against `{}`: two different variants answer the mismatch.",
            first.name(),
            second.name()
        ));
        writer.open("{");
        write_subject(
            writer,
            &value_path(context),
            first,
            "subject",
            seed(first.index),
            exclusive,
        );
        write_subject(
            writer,
            &value_path(context),
            second,
            "other",
            OTHER_SEED,
            other_exclusive,
        );
        let output = context.output();
        writer.line(format!(
            "let mismatch: {} = {};",
            spell(&output, "S", Associates::Qualified),
            context.mismatched_output(&value_path(context))
        ));
        writer.line(format!(
            "let answer = observe({});",
            call(context, "S", "subject", Some("other"), "2u64")
        ));
        writer.line(format!(
            "assert_eq!(answer, observe(mismatch), \"m on `{}` against `{}`\");",
            first.name(),
            second.name()
        ));
        writer.close("}");
    }
}
