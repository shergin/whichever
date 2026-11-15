//! The parts of a cell whose trait is `whichever::foreign::Display`, the
//! restatement the crate ships (restatements.md, golden `foreign`,
//! behaviour test `foreign_traits`): the payloads implement
//! `core::fmt::Display`, and the method under test is its `fmt`, which
//! takes `&self` and an argument. The hand and the law are written beside
//! the trait's in their own modules.

use super::context::Context;
use super::writer::Writer;
use crate::model::path::Form as PathForm;

/// `fmt` as an impl of `core::fmt::Display` spells it.
pub const SIGNATURE: &str = "fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result";

/// What a body the user writes for `fmt` prints.
pub const WRITTEN: &str = "written";

/// Returns the body the user writes for `fmt`.
pub fn body() -> String {
    format!("f.write_str(\"{WRITTEN}\")")
}

/// Writes the note the trait's part holds: the trait is the crate's.
pub fn write_trait(writer: &mut Writer) {
    writer.line("// The trait is `whichever::foreign::Display`, which the crate ships.");
}

/// Writes the payload of slot `index`: its struct, its inherent methods, an
/// inherent `fmt` that prints the poison, and its impl of
/// `core::fmt::Display`, which prints the payload and its state.
pub fn write_payload(writer: &mut Writer, context: &Context, index: usize) {
    let payload = format!("P{index}");
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
    writer.line("/// Prints what no forwarding may: an arm that calls it instead of the");
    writer.line("/// trait's method shows in the law.");
    writer.open(format!("pub {SIGNATURE} {{"));
    writer.line("f.write_str(\"poison\")");
    writer.close("}");
    writer.close("}");
    writer.blank();
    writer.open(format!("impl core::fmt::Display for {payload} {{"));
    writer.open(format!("{SIGNATURE} {{"));
    writer.line(format!("write!(f, \"{payload}:{{}}\", self.state)"));
    writer.close("}");
    writer.close("}");
    writer.blank();
    super::conversions::write_payload_into(writer, context, index);
}

/// Writes `fmt` in a partial block: bodiless under a mark, or with the
/// body that answers for the variants the mark leaves out, or for every
/// variant.
pub fn write_block_method(writer: &mut Writer, context: &Context) {
    match context.path.form {
        PathForm::Forwarded | PathForm::Default => writer.line(format!("{SIGNATURE};")),
        PathForm::First | PathForm::Written => {
            writer.open(format!("{SIGNATURE} {{"));
            writer.line(body());
            writer.close("}");
        }
    }
}

/// Writes a function that prints the subject through the trait, so that a
/// withheld impl shows as an error of its own.
pub fn write_probe(writer: &mut Writer) {
    writer.line("/// Prints the subject through the trait, so that a withheld impl shows");
    writer.line("/// as an error of its own.");
    writer.open("pub fn probe(subject: &E) -> String {");
    writer.line("format!(\"{subject}\")");
    writer.close("}");
}
