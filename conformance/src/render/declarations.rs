//! The trait, the subject spelled with the attributes, and the probe that
//! calls through the trait in a limit cell.

use super::context::{variant, Context, Site, TRAIT};
use super::payload;
use super::writer::Writer;
use crate::model::argument::MismatchSite;
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
    writer.line("fn base(&self) -> u64;");
    if context.argument.kind.is_some() && context.argument.mismatch == MismatchSite::Trait {
        writer.line(format!(
            "#[dispatch(mismatch = {})]",
            context.mismatch("Self")
        ));
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
    let mut arguments = vec![String::from(TRAIT)];
    arguments.extend(context.options());
    writer.anchor(Anchor::Attribute.name());
    writer.line(format!("#[whichever::dispatch({})]", arguments.join(", ")));
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
}

/// Writes a function that calls `base` through the trait on the subject,
/// so that a cascade or a withheld impl shows as a second error.
pub fn write_probe(writer: &mut Writer) {
    writer.line("/// Calls the control method through the trait, so that a withheld impl");
    writer.line("/// shows as an error of its own.");
    writer.open("pub fn probe(subject: &E) -> u64 {");
    writer.line(format!("<E as {TRAIT}>::base(subject)"));
    writer.close("}");
}
