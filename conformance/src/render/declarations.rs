//! The trait, the subject spelled with the attributes, and the probe that
//! calls through the trait in a limit cell.

use super::context::{is_struct, variant, write_struct, Context, Site, TRAIT};
use super::payload;
use super::writer::{Part, Writer};
use crate::model::argument::MismatchSite;
use crate::model::path::{Form as PathForm, Site as PathSite};
use crate::model::Anchor;

/// Writes the trait with `#[whichever::dispatchable]`: the control method
/// `base` and the method under test `m`.
pub fn write_trait(writer: &mut Writer, context: &Context) {
    // A trait in a sibling module is imported where the subject is, the
    // carrier macro with it, as a user imports a trait of another module.
    let Some(module) = context.home.module else {
        write_trait_items(writer, context);
        return;
    };
    writer.open(format!("pub mod {module} {{"));
    // What the trait's signature names it reaches through its parent.
    writer.line("#[allow(unused_imports)]");
    writer.line("use super::*;");
    writer.blank();
    write_trait_items(writer, context);
    writer.close("}");
    writer.blank();
    writer.line(format!("use {module}::{TRAIT};"));
}

fn write_trait_items(writer: &mut Writer, context: &Context) {
    writer.line("#[whichever::dispatchable]");
    writer.open(format!(
        "pub trait {TRAIT}{} {{",
        context.trait_parameters()
    ));
    for (associated, _) in context.associated() {
        writer.line(format!("type {};", associated.name()));
    }
    for item in context.agreement.items {
        writer.line(item.declaration());
    }
    writer.line("/// The control method, which every subject forwards.");
    writer.line(format!("{};", context.base_signature()));
    let mismatched = context.argument.kind.is_some() && !context.inherent();
    if mismatched && context.argument.mismatch == MismatchSite::Trait {
        writer.line(format!(
            "#[dispatch(mismatch = {})]",
            context.mismatch("Self")
        ));
    }
    if context.inherent() {
        writer.close("}");
        return;
    }
    if let Some(attribute) = context.item.attribute {
        writer.line(format!("#[{attribute}]"));
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
    let mut arguments = vec![context.trait_path()];
    arguments.extend(context.options());
    writer.anchor(Anchor::Attribute.name());
    match block {
        true => writer.line("#[whichever::dispatch]"),
        false => writer.line(format!("#[whichever::dispatch({})]", arguments.join(", "))),
    }
    if is_struct(&context.slots) {
        write_struct(writer, &context.slots[0], &context.generics(), true, true);
    } else {
        writer.anchor(Anchor::Enum.name());
        writer.open(format!("pub enum E{} {{", context.generics()));
        for slot in &context.declared {
            let (attribute, declaration) = variant(slot, true);
            if let Some(predicate) = slot.cfg {
                // A sentence on the whole variant spans its attributes.
                writer.anchor(format!("{}-cfg", Anchor::Variant(slot.index).name()));
                writer.line(format!("#[cfg({predicate})]"));
            }
            if let Some(attribute) = attribute {
                writer.line(attribute);
            }
            let conversion = context.cell.conv.descriptor().variant_option;
            if let (0, Some(option)) = (slot.index, conversion) {
                writer.line(format!("#[dispatch({option})]"));
            }
            writer.anchor(Anchor::Variant(slot.index).name());
            writer.line(declaration);
        }
        writer.close("}");
    }
    writer.blank();
    super::conversions::write_proofs(writer, context);
    if block {
        writer.blank();
        write_block(writer, context, &arguments[1..]);
    }
    if context.inherent() {
        writer.blank();
        writer.begin(Part::Inherent);
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
        writer.line(format!("#[dispatch({mark}{})]", context.inherent_options()));
    }
    let signature = context.declared_signature();
    writer.anchor(Anchor::BlockSignature.name());
    match context.path.form {
        PathForm::First => {
            writer.open(format!("{signature} {{"));
            writer.line(context.unused(Site::Inherent, true));
            writer.line(context.declared_constant());
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
    writer.anchor(Anchor::BlockAttribute.name());
    match options.is_empty() {
        true => writer.line("#[whichever::dispatch]"),
        false => writer.line(format!("#[whichever::dispatch({})]", options.join(", "))),
    }
    writer.open(format!(
        "impl{} {} for E{} {{",
        context.impl_generics(),
        context.trait_path(),
        context.type_generics()
    ));
    if !context.cell.base_reaches_all() {
        writer.line("#[dispatch(A)]");
        writer.open(format!("{} {{", context.base_signature()));
        writer.line("0");
        writer.close("}");
    }
    if context.path.in_block {
        // A mark, or a method the block writes, is under the trait method's
        // own `#[cfg]`, as a hand writes the item (partial-impl-blocks.md).
        if let Some(predicate) = context.item.cfg {
            writer.line(format!("#[cfg({predicate})]"));
        }
        writer.anchor(Anchor::BlockMethod.name());
        if let Some(mark) = context.path.mark {
            writer.line(format!("#[dispatch({mark})]"));
        }
        // The block's method is the user's item, which keeps the attributes
        // written on it: it repeats the signature the trait allows a lint
        // on, so it allows the lint too, and the trait method's own
        // `#[inline(..)]`, which it would otherwise not carry.
        //
        // Open question: what-is-generated.md gives the forwarding "the
        // trait method's own `#[inline(..)]`", which is read as the plain
        // forwarding's; a mark's method takes what the user writes on it.
        for attribute in context.block_attributes() {
            writer.line(attribute);
        }
        let signature = context.signature(Site::Impl);
        writer.anchor(Anchor::BlockSignature.name());
        match context.path.form {
            PathForm::Forwarded | PathForm::Default => writer.line(format!("{signature};")),
            PathForm::First | PathForm::Written => {
                writer.open(format!("{signature} {{"));
                writer.line(context.unused(Site::Impl, context.path.form == PathForm::Written));
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
    let subject = context.subject_instance("E");
    let taken = match context.base_receiver().descriptor().consumes {
        true => subject.clone(),
        false => format!("&{subject}"),
    };
    writer.open(format!("pub fn probe(subject: {taken}) -> u64 {{"));
    writer.line(format!(
        "<{subject} as {}>::base(subject)",
        context.trait_instance()
    ));
    writer.close("}");
}
