//! The trait, the subject spelled with the attributes, and the probe that
//! calls through the trait in a limit cell.

use super::context::{variant, Context, Site, TRAIT};
use super::payload;
use super::writer::Writer;
use crate::model::argument::MismatchSite;
use crate::model::path::{Form as PathForm, Site as PathSite};
use crate::model::Anchor;

/// Writes the trait with `#[whichever::dispatchable]`: the control method
/// `base` and the method under test `m`.
pub fn write_trait(writer: &mut Writer, context: &Context) {
    writer.line("#[whichever::dispatchable]");
    writer.open(format!("pub trait {TRAIT} {{"));
    for (associated, _) in context.associated() {
        writer.line(format!("type {};", associated.name()));
    }
    writer.line("/// The control method, which every subject forwards.");
    writer.line(format!("{};", context.base_signature()));
    if context.argument.kind.is_some() && context.argument.mismatch == MismatchSite::Trait {
        writer.line(format!(
            "#[dispatch(mismatch = {})]",
            context.mismatch("Self")
        ));
    }
    if context.inherent() {
        writer.close("}");
        return;
    }
    writer.anchor(Anchor::Method.name());
    let signature = context.signature(Site::Trait);
    if !context.qualifier.provided {
        writer.line(format!("{signature};"));
        writer.close("}");
        return;
    }
    writer.open(format!("{signature} {{"));
    payload::default_body(writer, context);
    writer.close("}");
    writer.close("}");
}

/// Writes the subject with `#[whichever::dispatch(..)]`.
pub fn write_subject(writer: &mut Writer, context: &Context) {
    let block = context.path.site == PathSite::Block;
    let mut arguments = vec![String::from(TRAIT)];
    arguments.extend(context.options());
    writer.anchor(Anchor::Attribute.name());
    match block {
        true => writer.line("#[whichever::dispatch]"),
        false => writer.line(format!("#[whichever::dispatch({})]", arguments.join(", "))),
    }
    writer.anchor(Anchor::Enum.name());
    writer.open(format!("pub enum E{} {{", context.generics()));
    for slot in &context.slots {
        let (attribute, declaration) = variant(slot, true);
        if let Some(attribute) = attribute {
            writer.line(attribute);
        }
        writer.anchor(Anchor::Variant(slot.index).name());
        writer.line(declaration);
    }
    writer.close("}");
    if block {
        writer.blank();
        write_block(writer, context, &arguments[1..]);
    }
    if context.inherent() {
        writer.blank();
        write_inherent(writer, context);
    }
}

/// Writes the inherent block an inherent path holds.
fn write_inherent(writer: &mut Writer, context: &Context) {
    writer.line("#[whichever::dispatch]");
    let generics = context.generics();
    writer.open(format!("impl{generics} E{generics} {{"));
    writer.anchor(Anchor::BlockMethod.name());
    if let Some(mark) = context.path.mark {
        writer.line(format!("#[dispatch({mark})]"));
    }
    let signature = format!("pub {}", context.signature(Site::Inherent));
    writer.anchor(Anchor::BlockSignature.name());
    match context.path.form {
        PathForm::First => {
            writer.open(format!("{signature} {{"));
            writer.line(context.unused(false));
            writer.line(context.mismatch("E"));
            writer.close("}");
        }
        PathForm::Forwarded | PathForm::Default | PathForm::Written => {
            writer.line(format!("{signature};"))
        }
    }
    writer.close("}");
}

/// Writes the partial impl block a block path holds.
fn write_block(writer: &mut Writer, context: &Context, options: &[String]) {
    match options.is_empty() {
        true => writer.line("#[whichever::dispatch]"),
        false => writer.line(format!("#[whichever::dispatch({})]", options.join(", "))),
    }
    let generics = context.generics();
    writer.open(format!("impl{generics} {TRAIT} for E{generics} {{"));
    if !context.cell.base_reaches_all() {
        writer.line("#[dispatch(A)]");
        writer.open(format!("{} {{", context.base_signature()));
        writer.line("0");
        writer.close("}");
    }
    if context.path.in_block {
        writer.anchor(Anchor::BlockMethod.name());
        if let Some(mark) = context.path.mark {
            writer.line(format!("#[dispatch({mark})]"));
        }
        let signature = context.signature(Site::Impl);
        writer.anchor(Anchor::BlockSignature.name());
        match context.path.form {
            PathForm::Forwarded | PathForm::Default => writer.line(format!("{signature};")),
            PathForm::First | PathForm::Written => {
                writer.open(format!("{signature} {{"));
                writer.line(context.unused(context.path.form == PathForm::Written));
                writer.line(context.mismatch("E"));
                writer.close("}");
            }
        }
    }
    writer.close("}");
}

/// Writes a function that calls `base` through the trait on the subject,
/// so that a cascade or a withheld impl shows as a second error.
pub fn write_probe(writer: &mut Writer, context: &Context) {
    writer.line("/// Calls the control method through the trait, so that a withheld impl");
    writer.line("/// shows as an error of its own.");
    let taken = match context.base_receiver().descriptor().consumes {
        true => "E",
        false => "&E",
    };
    writer.open(format!("pub fn probe(subject: {taken}) -> u64 {{"));
    writer.line(format!("<E as {TRAIT}>::base(subject)"));
    writer.close("}");
}
