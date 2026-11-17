//! The parts of a cell whose trait is a restatement `whichever::foreign`
//! ships (restatements.md, golden `foreign`, behaviour test
//! `foreign_traits`).
//!
//! Under `whichever::foreign::Display` the payloads implement
//! `core::fmt::Display`, and the method under test is its `fmt`, which
//! takes `&self` and an argument. Under `whichever::foreign::Iterator` the
//! payloads implement `core::iter::Iterator` and override the provided
//! `any`, which a restatement that left it out would answer with the
//! default; the law calls `any` and then `next`. The hand and the law are
//! written beside the trait's in their own modules.

use super::context::Context;
use super::writer::Writer;
use crate::model::declaration::Foreign;
use crate::model::path::Form as PathForm;

/// `fmt` as an impl of `core::fmt::Display` spells it.
pub const SIGNATURE: &str = "fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result";

/// `next` as an impl of `core::iter::Iterator` over `u64` spells it.
pub const NEXT: &str = "fn next(&mut self) -> Option<u64>";

/// `any` as an impl of `core::iter::Iterator` over `u64` spells it.
pub const ANY: &str = "fn any<F>(&mut self, f: F) -> bool where F: FnMut(u64) -> bool";

/// What a body the user writes for `fmt` prints.
pub const WRITTEN: &str = "written";

/// Returns the body the user writes for `fmt`.
pub fn body() -> String {
    format!("f.write_str(\"{WRITTEN}\")")
}

/// Writes the note the trait's part holds: the trait is the crate's.
pub fn write_trait(writer: &mut Writer, foreign: Foreign) {
    writer.line(format!(
        "// The trait is `{}`, which the crate ships.",
        foreign.path()
    ));
}

/// Writes the payload of slot `index`: its struct, its inherent methods,
/// an inherent method of the name the law calls that answers a poison, and
/// its impl of the real trait.
pub fn write_payload(writer: &mut Writer, context: &Context, index: usize, foreign: Foreign) {
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
    writer.line("/// Answers what no forwarding may: an arm that calls it instead of the");
    writer.line("/// trait's method shows in the law.");
    match foreign {
        Foreign::Display => {
            writer.open(format!("pub {SIGNATURE} {{"));
            writer.line("f.write_str(\"poison\")");
        }
        Foreign::Iterator => {
            writer.line("#[allow(unused_variables)]");
            writer.open(format!("pub {ANY} {{"));
            writer.line("false");
        }
    }
    writer.close("}");
    writer.close("}");
    writer.blank();
    match foreign {
        Foreign::Display => {
            writer.open(format!("impl core::fmt::Display for {payload} {{"));
            writer.open(format!("{SIGNATURE} {{"));
            writer.line(format!("write!(f, \"{payload}:{{}}\", self.state)"));
            writer.close("}");
            writer.close("}");
        }
        Foreign::Iterator => {
            // `next` yields the state once. The override of `any` asks the
            // predicate about a number `next` never yields and takes
            // nothing, so it answers `true` where the default, which runs
            // `next`, answers `false` and leaves nothing to yield.
            writer.open(format!("impl core::iter::Iterator for {payload} {{"));
            writer.line("type Item = u64;");
            writer.blank();
            writer.open(format!("{NEXT} {{"));
            writer.line("self.spare += 1;");
            writer.line("(self.spare == 1).then_some(self.state)");
            writer.close("}");
            writer.blank();
            writer.open("fn any<F>(&mut self, mut f: F) -> bool where F: FnMut(u64) -> bool {");
            writer.line("f(self.state + 1)");
            writer.close("}");
            writer.close("}");
        }
    }
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
        PathForm::Arms => unreachable!("the verdict gives `fmt` no arms of its own"),
    }
}

/// Writes a function that calls the subject through the trait, so that a
/// withheld impl shows as an error of its own.
pub fn write_probe(writer: &mut Writer, foreign: Foreign) {
    match foreign {
        Foreign::Display => {
            writer.line("/// Prints the subject through the trait, so that a withheld impl shows");
            writer.line("/// as an error of its own.");
            writer.open("pub fn probe(subject: &E) -> String {");
            writer.line("format!(\"{subject}\")");
        }
        Foreign::Iterator => {
            writer.line("/// Advances the subject through the trait, so that a withheld impl");
            writer.line("/// shows as an error of its own.");
            writer.open("pub fn probe(subject: &mut E) -> Option<u64> {");
            writer.line("core::iter::Iterator::next(subject)");
        }
    }
    writer.close("}");
}
