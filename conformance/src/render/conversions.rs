//! The conversions the subject's attribute writes: the proofs of absence
//! beside the subject, the impls a hand writes for its own enum, and the
//! law's checks of each (options.md).
//!
//! An impl the docs say is not written is proven absent by writing it by
//! hand beside the subject, which compiles only if the attribute wrote
//! none.

use super::context::{storage, Context, LABEL};
use super::writer::Writer;
use crate::model::conversion::TryForm;
use crate::model::target::{Form, Pointer, Storage, TargetDescriptor};
use crate::model::{Slot, SlotVariant, Verdict};

/// Returns `value` built into the slot's variant of the subject at `path`.
fn construct(path: &str, slot: &Slot, target: &TargetDescriptor, value: &str) -> String {
    let name = slot.name();
    match target.form {
        Form::Single => format!("{path}::{name}({value})"),
        Form::MarkedTuple => format!("{path}::{name}({value}, {LABEL})"),
        Form::MarkedStruct => format!("{path}::{name} {{ label: {LABEL}, held: {value} }}"),
        Form::Struct => format!("{path}({value})"),
        Form::StructMarked => format!("{path} {{ label: {LABEL}, held: {value} }}"),
        Form::StructNamed => format!("{path} {{ held: {value} }}"),
    }
}

/// Returns the value a slot's variant stores around a fresh payload made
/// from `seed`; the verdict leaves no borrowing variant to convert.
pub fn stored_value(slot: &Slot, target: &TargetDescriptor, seed: u64) -> String {
    let payload = format!("{}::new({seed})", slot.payload());
    match target.storage {
        Storage::Payload => payload,
        Storage::Pointer(Pointer::Box) => format!("Box::new({payload})"),
        Storage::Pointer(Pointer::Rc) => format!("Rc::new({payload})"),
        Storage::Pointer(Pointer::Arc) => format!("Arc::new({payload})"),
        Storage::Pointer(Pointer::Shared | Pointer::Exclusive) => {
            unreachable!("the verdict converts no borrowing variant")
        }
        Storage::Wrapper => format!(
            "{} {{ label: {LABEL}, payload: {payload} }}",
            slot.wrapper()
        ),
        Storage::Pair => format!("({LABEL}, {payload})"),
    }
}

/// Writes `#[allow(deprecated)]` before an impl that names a deprecated
/// variant, as a hand writes it.
fn allow_deprecated(writer: &mut Writer, deprecated: bool) {
    if deprecated {
        writer.line("#[allow(deprecated)]");
    }
}

/// Writes the impls the docs say the attribute does not write, by hand
/// beside a passing subject.
pub fn write_proofs(writer: &mut Writer, context: &Context) {
    let conversion = context.cell.conv.descriptor();
    if !conversion.checked || context.verdict != Verdict::Pass {
        return;
    }
    let from = context.cell.from_slots();
    for (slot, _, target) in context.live() {
        if from.iter().any(|other| other.index == slot.index) {
            continue;
        }
        let stored = storage(&slot, &target);
        writer.line(format!(
            "/// No `From<{stored}>` is written for `{}`, or this impl would conflict.",
            slot.name()
        ));
        allow_deprecated(writer, slot.deprecated);
        writer.open(format!("impl From<{stored}> for E {{"));
        writer.open(format!("fn from(value: {stored}) -> E {{"));
        writer.line(construct("E", &slot, &target, "value"));
        writer.close("}");
        writer.close("}");
        writer.blank();
    }
    if !conversion.first_try {
        return;
    }
    let tried = context.cell.try_slots();
    for (slot, _, _) in context.live() {
        if tried.iter().any(|other| other.index == slot.index) {
            continue;
        }
        let payload = slot.payload();
        writer.line(format!(
            "/// No `TryFrom<E>` is written for `{payload}`, or this impl would conflict."
        ));
        writer.open(format!("impl TryFrom<E> for {payload} {{"));
        writer.line("type Error = E;");
        writer.open(format!("fn try_from(value: E) -> Result<{payload}, E> {{"));
        writer.line("Err(value)");
        writer.close("}");
        writer.close("}");
        writer.blank();
    }
}

/// Writes the `From` impl a payload has into `support::Sink`, which
/// `into = support::Sink` converts through.
pub fn write_payload_into(writer: &mut Writer, context: &Context, index: usize) {
    if !context.cell.conv.descriptor().into {
        return;
    }
    writer.open(format!("impl From<P{index}> for support::Sink {{"));
    writer.open(format!("fn from(payload: P{index}) -> support::Sink {{"));
    writer.line(format!("support::Sink({index}, payload.state)"));
    writer.close("}");
    writer.close("}");
    writer.blank();
}

/// Writes the conversions a hand writes for its own enum.
pub fn write_hand(writer: &mut Writer, context: &Context) {
    let conversion = context.cell.conv.descriptor();
    if !conversion.checked {
        return;
    }
    for slot in context.cell.from_slots() {
        let Some((_, target)) = slot.target() else {
            continue;
        };
        let stored = storage(&slot, &target);
        allow_deprecated(writer, slot.deprecated);
        writer.open(format!("impl From<{stored}> for E {{"));
        writer.open(format!("fn from(value: {stored}) -> E {{"));
        writer.line(construct("E", &slot, &target, "value"));
        writer.close("}");
        writer.close("}");
        writer.blank();
    }
    let form = match (conversion.try_from, conversion.first_try) {
        (Some(form), _) => Some(form),
        (None, true) => Some(TryForm::Value),
        (None, false) => None,
    };
    if let Some(form) = form {
        for slot in context.cell.try_slots() {
            let payload = slot.payload();
            let name = slot.name();
            let forms: &[(&str, &str)] = match form {
                TryForm::Value => &[("", "")],
                TryForm::Shared => &[("", ""), ("&'e ", "<'e>")],
                TryForm::Exclusive => &[("", ""), ("&'e ", "<'e>"), ("&'e mut ", "<'e>")],
            };
            for (reference, generics) in forms {
                allow_deprecated(writer, slot.deprecated);
                writer.open(format!(
                    "impl{generics} TryFrom<{reference}E> for {reference}{payload} {{"
                ));
                writer.line(format!("type Error = {reference}E;"));
                writer.line("#[allow(unreachable_patterns)]");
                writer.open(format!(
                    "fn try_from(value: {reference}E) -> Result<{reference}{payload}, {reference}E> {{"
                ));
                writer.open("match value {");
                writer.line(format!("E::{name}(payload) => Ok(payload),"));
                writer.line("other => Err(other),");
                writer.close("}");
                writer.close("}");
                writer.close("}");
                writer.blank();
            }
        }
    }
    if conversion.into {
        allow_deprecated(writer, context.slots.iter().any(|slot| slot.deprecated));
        writer.open("impl From<E> for support::Sink {");
        writer.open("fn from(value: E) -> support::Sink {");
        writer.open("match value {");
        for (slot, _, target) in context.live() {
            let pattern = super::context::pattern("E", &slot, &target, "payload");
            writer.line(format!("{pattern} => Into::into(payload),"));
        }
        writer.close("}");
        writer.close("}");
        writer.close("}");
        writer.blank();
    }
}

/// Writes the law's checks of the conversions, inside the law's body.
pub fn write_law(writer: &mut Writer, context: &Context) {
    let conversion = context.cell.conv.descriptor();
    if !conversion.checked {
        return;
    }
    let seed = |slot: &Slot| super::context::seed(slot.index);
    for slot in context.cell.from_slots() {
        let Some((_, target)) = slot.target() else {
            continue;
        };
        let stored = storage(&slot, &target);
        let value = stored_value(&slot, &target, seed(&slot));
        writer.line(format!(
            "// `From<{stored}>` builds `{}` around it.",
            slot.name()
        ));
        writer.open("{");
        writer.line(format!(
            "let built: S = <S as From<{stored}>>::from({value});"
        ));
        writer.line(format!(
            "assert_eq!(seen(&built), \"{}:{}\", \"From<{stored}>\");",
            slot.index,
            seed(&slot)
        ));
        writer.close("}");
    }
    let tried = context.cell.try_slots();
    let form = match (conversion.try_from, conversion.first_try) {
        (Some(form), _) => form,
        (None, _) => TryForm::Value,
    };
    let live: Vec<Slot> = context
        .live()
        .into_iter()
        .map(|(slot, _, _)| slot)
        .collect();
    for slot in &tried {
        let payload = slot.payload();
        for source in &live {
            let own = source.index == slot.index;
            writer.line(format!(
                "// `{payload}` from `{}` by `TryFrom`.",
                source.name()
            ));
            writer.open("{");
            let expected_back = format!("\"{}:{}\"", source.index, seed(source));
            let mut checks: Vec<(String, &str)> = vec![(
                format!("<{payload} as TryFrom<S>>::try_from(value)"),
                "seen(&back)",
            )];
            if form >= TryForm::Shared {
                checks.push((
                    format!("<&{payload} as TryFrom<&S>>::try_from(&value)"),
                    "seen(back)",
                ));
            }
            if form >= TryForm::Exclusive {
                checks.push((
                    format!("<&mut {payload} as TryFrom<&mut S>>::try_from(&mut value)"),
                    "seen(back)",
                ));
            }
            for (call, back) in checks {
                write_subject_value(writer, source, "value", seed(source), true);
                let answer = match own {
                    true => format!(
                        "match {call} {{ Ok(payload) => assert_eq!(payload.state(), {}), Err(_) => panic!(\"{payload} from its own variant\") }}",
                        seed(source)
                    ),
                    false => format!(
                        "match {call} {{ Ok(_) => panic!(\"{payload} from another variant\"), Err(back) => assert_eq!({back}, {expected_back}) }}"
                    ),
                };
                writer.line(format!("{answer};"));
            }
            writer.close("}");
        }
    }
    if conversion.into {
        for slot in &live {
            writer.open("{");
            write_subject_value(writer, slot, "value", seed(slot), false);
            writer.line(format!(
                "assert_eq!(support::Sink::from(value), support::Sink::from({}::new({})), \"into from {}\");",
                slot.payload(),
                seed(slot),
                slot.name()
            ));
            writer.close("}");
        }
    }
}

/// Writes `let name = S::..` around a fresh payload of the slot.
fn write_subject_value(writer: &mut Writer, slot: &Slot, name: &str, seed: u64, mutable: bool) {
    let SlotVariant::Live(target) = slot.variant else {
        unreachable!("a conversion is checked on a variant holding a payload");
    };
    let target = target.descriptor();
    let binding = if mutable { "let mut" } else { "let" };
    let value = construct("S", slot, &target, &stored_value(slot, &target, seed));
    writer.line("#[allow(unused_mut)]");
    writer.line(format!("{binding} {name} = {value};"));
}
