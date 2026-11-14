//! A cell rendered as one source file: the trait, the payloads, the
//! subject spelled with the attributes, the hand's subject, and the law.
//!
//! The text is what a person reads when a cell fails, so it is written
//! line by line as tidy Rust, with a header naming the cell and its
//! verdict, rather than with `quote!`.

pub mod context;
mod declarations;
mod hand;
mod law;
mod payload;
pub mod support;
pub mod writer;

use std::collections::BTreeMap;

use crate::cell::Cell;
use crate::model::{Anchor, Verdict};
use context::Context;
use writer::{Part, Section, Writer};

/// A rendered cell.
#[derive(Clone, Debug)]
pub struct Rendered {
    /// The source of the cell's module.
    pub source: String,
    /// The lines each part holds.
    pub sections: Vec<Section>,
    /// The line of each anchor, by name.
    pub anchors: BTreeMap<String, usize>,
}

impl Rendered {
    /// Returns the part that holds a line, counted from 1.
    pub fn part_of(&self, line: usize) -> Option<Part> {
        self.sections
            .iter()
            .find(|section| section.first <= line && line <= section.last)
            .map(|section| section.part)
    }

    /// Returns the line of an anchor; `None` for `Anywhere` and for an
    /// anchor the cell does not hold.
    pub fn line_of(&self, anchor: Anchor) -> Option<usize> {
        self.anchors.get(&anchor.name()).copied()
    }
}

/// The command that shows a cell.
pub fn show_command(id: &str) -> String {
    format!("cargo run -p whichever-conformance --release -- show '{id}'")
}

/// Renders a cell as the source of its module.
pub fn render(cell: &Cell) -> Rendered {
    let context = Context::new(cell);
    let draft = write(&context, &IMPORTS.map(|(line, _)| line));
    let body: Vec<&str> = draft
        .source
        .lines()
        .filter(|line| !line.starts_with("use "))
        .collect();
    let used: Vec<&str> = IMPORTS
        .iter()
        .filter(|(_, names)| {
            names
                .iter()
                .any(|name| body.iter().any(|line| line.contains(name)))
        })
        .map(|(line, _)| *line)
        .collect();
    write(&context, &used)
}

/// The imports a cell may need, each with the spellings that use it. An
/// unused import is a warning, and every warning is part of the outcome, so
/// a cell imports what its text uses and nothing else.
const IMPORTS: [(&str, &[&str]); 5] = [
    ("use core::future::Future;", &["Future<"]),
    ("use core::pin::Pin;", &["Pin<", "Pin::"]),
    ("use std::rc::Rc;", &["Rc<", "Rc::"]),
    ("use std::sync::Arc;", &["Arc<", "Arc::"]),
    ("use crate::support;", &["support::"]),
];

fn write(context: &Context, imports: &[&str]) -> Rendered {
    let cell = &context.cell;
    let mut writer = Writer::new();
    writer.begin(Part::Header);
    let id = cell.id();
    writer.line(format!("//! Cell `{id}`."));
    writer.line("//!");
    writer.line(format!("//! Verdict: {}.", context.verdict.describe()));
    writer.line(format!("//! Shown by `{}`.", show_command(&id)));
    let (subject, hand) = match &context.verdict {
        Verdict::Pass => (true, true),
        Verdict::Limit(_) => (true, false),
        Verdict::Malformed(_) => (false, false),
        Verdict::Unmodelled(_) | Verdict::NonCanonical => {
            let (source, sections, anchors) = writer.finish();
            return Rendered {
                source,
                sections,
                anchors,
            };
        }
    };
    writer.blank();
    for import in imports {
        writer.line(*import);
    }
    writer.blank();
    writer.begin(Part::Trait);
    declarations::write_trait(&mut writer, context);
    writer.blank();
    writer.begin(Part::Payloads);
    payload::write(&mut writer, context);
    if subject {
        writer.begin(Part::Subject);
        declarations::write_subject(&mut writer, context);
        writer.blank();
    }
    if let Verdict::Limit(expected) = &context.verdict {
        if expected
            .iter()
            .all(|expected| expected.limit.entry().keeps_impl)
        {
            writer.begin(Part::Probe);
            declarations::write_probe(&mut writer);
            writer.blank();
        }
    }
    if hand {
        writer.begin(Part::Hand);
        hand::write(&mut writer, context);
        writer.blank();
        writer.begin(Part::Law);
        law::write(&mut writer, context);
    }
    let (source, sections, anchors) = writer.finish();
    Rendered {
        source,
        sections,
        anchors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rendered_neutral_cell_holds_its_sections_and_anchors() {
        let rendered = render(&Cell::neutral());
        for part in [
            Part::Header,
            Part::Trait,
            Part::Payloads,
            Part::Subject,
            Part::Hand,
            Part::Law,
        ] {
            assert!(
                rendered.sections.iter().any(|section| section.part == part),
                "no {} section",
                part.name()
            );
        }
        let lines: Vec<&str> = rendered.source.lines().collect();
        let method = rendered.line_of(Anchor::Method).expect("a method anchor");
        assert!(lines[method - 1].contains("fn m(&self, x: u64) -> u64;"));
        let attribute = rendered
            .line_of(Anchor::Attribute)
            .expect("an attribute anchor");
        assert!(lines[attribute - 1].contains("#[whichever::dispatch(Probe)]"));
        let first = rendered
            .line_of(Anchor::Variant(0))
            .expect("a variant anchor");
        assert!(lines[first - 1].contains("A(P0),"));
        assert_eq!(rendered.part_of(first), Some(Part::Subject));
        assert!(rendered.source.starts_with("//! Cell `base`."));
        assert!(rendered.source.contains("mod hand {"));
        assert!(rendered.source.contains("mod law {"));
    }

    #[test]
    fn a_limit_cell_holds_no_hand_and_a_malformed_one_no_subject() {
        let limit = render(&Cell::parse("recv=rc").expect("an id"));
        assert!(!limit.source.contains("mod hand"));
        assert!(limit.source.contains("pub fn probe"));
        let malformed = render(&Cell::parse("ret=mut").expect("an id"));
        assert!(!malformed.source.contains("whichever::dispatch("));
        assert!(malformed.source.contains("#[whichever::dispatchable]"));
    }
}
