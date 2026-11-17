//! The trait, the subject spelled with the attributes, and the probe that
//! calls through the trait in a limit cell.

use super::context::{is_struct, variant, write_struct, Context, Site, TRAIT};
use super::payload;
use super::writer::{Part, Writer};
use crate::model::argument::MismatchSite;
use crate::model::path::{Form as PathForm, Site as PathSite};
use crate::model::returns::Associated;
use crate::model::stack::OTHER;
use crate::model::{Anchor, Arms};

/// Writes the trait with `#[whichever::dispatchable]`: the control method
/// `base` and the method under test `m`.
pub fn write_trait(writer: &mut Writer, context: &Context) {
    if let Some(foreign) = context.foreign_trait() {
        super::foreign::write_trait(writer, foreign);
        return;
    }
    // A trait in a sibling module is imported where the subject is, the
    // carrier macro with it, as a user imports a trait of another module;
    // a trait the subject restates is imported the same way, as the
    // behaviour test `pinned_writers_with_provided_methods` imports its
    // `io::AsyncWrite`.
    let Some(module) = context.trait_module() else {
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
    match context.stack.present {
        true => writer.line(format!("use {module}::{{{TRAIT}, {OTHER}}};")),
        false => writer.line(format!("use {module}::{TRAIT};")),
    }
}

fn write_trait_items(writer: &mut Writer, context: &Context) {
    write_probe_trait(writer, context);
    if !context.stack.present {
        return;
    }
    writer.blank();
    writer.line("#[whichever::dispatchable]");
    writer.open(format!("pub trait {OTHER}{} {{", context.supertrait()));
    for item in context.agreement.items {
        writer.line(item.declaration());
    }
    writer.line("/// The second trait's method, which every subject forwards.");
    writer.line(format!("{};", context.other_signature()));
    writer.close("}");
}

/// Writes the trait under test with `#[whichever::dispatchable]`, or
/// without it where the subject restates it.
fn write_probe_trait(writer: &mut Writer, context: &Context) {
    let restated = context.declaration.restated;
    if !restated {
        writer.line("#[whichever::dispatchable]");
    }
    let unsafety = match context.declaration.unsafety {
        true => "unsafe ",
        false => "",
    };
    writer.open(format!(
        "pub {unsafety}trait {TRAIT}{}{} {{",
        context.trait_parameters(),
        context.required()
    ));
    for (associated, _) in context.associated() {
        if associated == Associated::Item && !restated {
            writer.anchor(Anchor::Associated.name());
        }
        writer.line(associated.declaration());
    }
    for item in context.agreement.items {
        writer.line(item.declaration());
    }
    writer.line("/// The control method, which every subject forwards.");
    writer.line(format!("{};", context.base_signature()));
    if context.agreement.returned {
        writer.line("/// Returns the trait's own type, which every subject forwards.");
        writer.line(format!("{};", context.kind_signature("Self::Kind")));
    }
    let mismatched = context.argument.kind.is_some() && !context.inherent();
    if mismatched && context.argument.mismatch == MismatchSite::Trait && !restated {
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
    if !restated {
        writer.anchor(Anchor::Method.name());
    }
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

/// Writes the restatement of a trait declared without the attribute,
/// beside the subject: its items as the trait declares them, and the method
/// under test bodiless where the restatement lists it, with `remote` naming
/// the real trait (restatements.md).
pub fn write_restatement(writer: &mut Writer, context: &Context) {
    writer.line(format!(
        "#[whichever::dispatchable(remote = {})]",
        context.real_trait()
    ));
    writer.open(format!("trait {TRAIT}{} {{", context.trait_parameters()));
    for (associated, _) in context.associated() {
        if associated == Associated::Item {
            writer.anchor(Anchor::Associated.name());
        }
        writer.line(associated.declaration());
    }
    for item in context.agreement.items {
        writer.line(item.declaration());
    }
    writer.line(format!("{};", context.base_signature()));
    if context.declaration.lists_method {
        if context.argument.kind.is_some() && context.argument.mismatch == MismatchSite::Trait {
            writer.line(format!(
                "#[dispatch(mismatch = {})]",
                context.mismatch("Self")
            ));
        }
        writer.anchor(Anchor::Method.name());
        writer.line(format!("{};", context.restated_signature()));
    }
    writer.close("}");
}

/// Writes the enum another crate declares, which the subject restates:
/// generic over what its variants hold, as `either::Either` is.
pub fn write_real(writer: &mut Writer, context: &Context) {
    writer.line("/// The enum another crate declares, which the subject restates.");
    writer.open(format!("pub enum E{} {{", context.generics()));
    for slot in &context.declared {
        let (_, declaration) = variant(slot, false);
        writer.line(declaration);
    }
    writer.close("}");
}

/// Writes the subject with `#[whichever::dispatch(..)]`.
pub fn write_subject(writer: &mut Writer, context: &Context) {
    let block = context.path.site == PathSite::Block;
    let mut arguments = vec![context.trait_path()];
    arguments.extend(context.options());
    // A restated enum names the real one, and the attribute registers it
    // for a block as the bare one does (restatements.md).
    let restated = context.cell.owner == crate::model::Owner::Remote;
    if restated {
        writer.line(format!(
            "#[whichever::dispatch(remote = {}::{}::E)]",
            context.trait_crate(),
            context.cell.module()
        ));
    }
    writer.anchor(Anchor::Attribute.name());
    // In a block path the second trait is the subject's attribute, and the
    // block is the impl of the first.
    // The enum's own options stay on its attribute where a block is the
    // impl (partial-impl-blocks.md).
    let enum_options = context.enum_options();
    match (block, context.stack.present) {
        (true, true) => writer.line(context.other_attribute()),
        (true, false) if restated => {}
        (true, false) if !enum_options.is_empty() => writer.line(format!(
            "#[whichever::dispatch({})]",
            enum_options.join(", ")
        )),
        (true, false) => writer.line("#[whichever::dispatch]"),
        (false, stacked) => {
            writer.line(format!("#[whichever::dispatch({})]", arguments.join(", ")));
            if stacked {
                writer.line(context.other_attribute());
            }
        }
    }
    if is_struct(&context.slots) {
        write_struct(writer, &context.slots[0], &context.generics(), true, true);
    } else {
        writer.anchor(Anchor::Enum.name());
        writer.open(format!("pub enum E{} {{", context.generics()));
        for slot in &context.declared {
            let (attribute, declaration) = variant(slot, true);
            // A sentence on the whole variant spans its attributes, so the
            // line they start on, a `#[cfg]` or a `#[deprecated]`, is the
            // variant's too.
            let first_attribute = format!("{}-cfg", Anchor::Variant(slot.index).name());
            if let Some(predicate) = slot.cfg {
                writer.anchor(first_attribute.clone());
                writer.line(format!("#[cfg({predicate})]"));
            }
            if let Some(attribute) = attribute {
                writer.line(attribute);
            }
            let conversion = context.cell.conv.descriptor().variant_option;
            if let (0, Some(option)) = (slot.index, conversion) {
                writer.line(format!("#[dispatch({option})]"));
            }
            if slot.deprecated {
                if slot.cfg.is_none() {
                    writer.anchor(first_attribute);
                }
                writer.line("#[deprecated]");
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
        write_block(writer, context, &context.trait_options());
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
    // The method's own `#[cfg]`, under which it is forwarded or reported.
    if let Some(predicate) = context.item.cfg {
        writer.line(format!("#[cfg({predicate})]"));
    }
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
        PathForm::Arms => {
            writer.open(format!("{signature} {{"));
            write_own_arms(writer, context, Site::Inherent);
            writer.close("}");
        }
        PathForm::Forwarded | PathForm::Default | PathForm::Written => {
            writer.line(format!("{signature};"))
        }
    }
    writer.close("}");
}

/// Writes the body of a method under `*` that holds the enum's own arms:
/// one `match self` holding them, as written, and a wildcard arm where the
/// cell has one; or, for a body that is no `match self` and on a struct,
/// the answer alone.
fn write_own_arms(writer: &mut Writer, context: &Context, site: Site) {
    let (unused, constant) = match site {
        Site::Inherent => (
            context.unused(Site::Inherent, true),
            context.declared_constant(),
        ),
        Site::Trait | Site::Impl | Site::Payload => {
            (context.unused(Site::Impl, false), context.mismatch("E"))
        }
    };
    let unmatched = context.cell.arms == Arms::Unmatched || is_struct(&context.slots);
    if unmatched {
        writer.line(unused);
        writer.line(constant);
        return;
    }
    writer.open("match self {");
    for arm in context.cell.own_arms() {
        writer.line(context.own_arm(&arm, "Self"));
    }
    if context.cell.arms == Arms::Wildcard {
        // Where the arms name every variant in the build, the wildcard is
        // unreachable too, which the user's own code would be warned of.
        writer.anchor(Anchor::Arm.name());
        writer.line(format!(
            "#[allow(unreachable_patterns)] _ => {{ {unused} {constant} }}"
        ));
    }
    writer.close("}");
}

/// Writes the partial impl block a block path holds.
fn write_block(writer: &mut Writer, context: &Context, options: &[String]) {
    // The macro the block invokes writes nothing, so what the block says is
    // the same whether or not the invocation is read; the invocation alone
    // is the sentence (limits.md).
    let invocation = context.path.invocation;
    if invocation {
        // Unused where its invocation is compiled out.
        writer.line("#[allow(unused_macros)]");
        writer.line("macro_rules! written {");
        writer.line("    () => {};");
        writer.line("}");
        writer.blank();
    }
    writer.anchor(Anchor::BlockAttribute.name());
    match options.is_empty() {
        true => writer.line("#[whichever::dispatch]"),
        false => writer.line(format!("#[whichever::dispatch({})]", options.join(", "))),
    }
    writer.open(format!(
        "{}{} {} for E{} {{",
        context.impl_keyword(),
        context.impl_generics(),
        context.trait_path(),
        context.impl_type_generics()
    ));
    // The block declares the generic associated type, which no first
    // variant can supply (partial-impl-blocks.md, golden
    // `gat_in_a_partial_block`).
    if context.cell.declares_gat() {
        writer.line(Associated::Item.definition(super::context::VIEW));
    }
    if invocation {
        // The invocation is the item from its first attribute on, and its
        // sentence is spanned on all of it.
        writer.anchor(Anchor::BlockInvocation.name());
        if let Some(predicate) = context.path.invocation_cfg {
            writer.line(format!("#[cfg({predicate})]"));
        }
        writer.line("written!();");
    }
    if !context.cell.base_reaches_all() && !context.foreign() {
        let named: Vec<&str> = context
            .cell
            .base_named()
            .iter()
            .map(|slot| slot.name())
            .collect();
        writer.line(format!("#[dispatch({})]", named.join(", ")));
        writer.open(format!("{} {{", context.base_signature()));
        writer.line("0");
        writer.close("}");
        if context.agreement.returned {
            writer.line(format!("#[dispatch({})]", named.join(", ")));
            writer.open(format!("{} {{", context.kind_signature("Self::Kind")));
            writer.line("0");
            writer.close("}");
        }
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
        // `#[inline(..)]`, which it would otherwise not carry. "Only the
        // methods the block leaves to the plain forwarding, whose
        // signatures the attribute writes, carry the trait method's
        // `#[inline(..)]` and `#[allow]`" (partial-impl-blocks.md).
        for attribute in context.block_attributes() {
            writer.line(attribute);
        }
        let signature = context.signature(Site::Impl);
        writer.anchor(Anchor::BlockSignature.name());
        if context.foreign() {
            super::foreign::write_block_method(writer, context);
            writer.close("}");
            return;
        }
        match context.path.form {
            PathForm::Forwarded | PathForm::Default => writer.line(format!("{signature};")),
            PathForm::Arms => {
                writer.open(format!("{signature} {{"));
                write_own_arms(writer, context, Site::Impl);
                writer.close("}");
            }
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
    if let Some(foreign) = context.foreign_trait() {
        super::foreign::write_probe(writer, foreign);
        return;
    }
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
