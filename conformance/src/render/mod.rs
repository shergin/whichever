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
    /// The lines that start with an attribute of the macro's:
    /// `#[whichever::dispatchable]`, `#[whichever::dispatch(..)]` or
    /// `#[dispatch(..)]`. A rustc error whose span covers one sits on an
    /// attribute, which `macros/ARCHITECTURE.md` 3.8 calls a defect.
    pub attributes: Vec<usize>,
}

impl Rendered {
    /// Returns the part that holds a line, counted from 1.
    pub fn part_of(&self, line: usize) -> Option<Part> {
        self.sections
            .iter()
            .find(|section| section.first <= line && line <= section.last)
            .map(|section| section.part)
    }

    /// Returns whether a span from `first` to `last` covers an attribute
    /// line.
    pub fn covers_attribute(&self, first: usize, last: usize) -> bool {
        self.attributes
            .iter()
            .any(|line| first <= *line && *line <= last)
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

/// What a rendering holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// What the verdict asks for.
    Full,
    /// The trait, the payloads, the hand and the hand's law only: a passing
    /// cell whose subject fails is checked this way, so that its hand is
    /// known to be right.
    HandOnly,
}

/// Renders a cell as the source of its module.
pub fn render(cell: &Cell) -> Rendered {
    render_as(cell, Mode::Full)
}

/// Renders a cell as `mode` says.
pub fn render_as(cell: &Cell, mode: Mode) -> Rendered {
    let context = Context::new(cell);
    let draft = write(&context, mode, &IMPORTS.map(|(line, _)| line));
    // A limit's inherent method is not emitted, since the inherent path
    // keeps only the methods it could forward (macros/ARCHITECTURE.md
    // 3.8), so what its signature alone names is not imported.
    let dropped = matches!(context.verdict, Verdict::Limit(_));
    let body: Vec<&str> = draft
        .source
        .lines()
        .enumerate()
        .filter(|(index, _)| !(dropped && draft.part_of(index + 1) == Some(Part::Inherent)))
        .map(|(_, line)| line)
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
    write(&context, mode, &used)
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

/// The parts a rendering holds.
struct Parts {
    subject: bool,
    probe: bool,
    hand: bool,
    law: &'static [&'static str],
}

fn write(context: &Context, mode: Mode, imports: &[&str]) -> Rendered {
    let cell = &context.cell;
    let mut writer = Writer::new();
    writer.begin(Part::Header);
    let id = cell.id();
    writer.line(format!("//! Cell `{id}`."));
    writer.line("//!");
    writer.line(format!("//! Verdict: {}.", context.verdict.describe()));
    writer.line(format!("//! Shown by `{}`.", show_command(&id)));
    if mode == Mode::HandOnly {
        writer.line("//! Rendered with the hand alone, since the subject fails.");
    }
    let nothing = Parts {
        subject: false,
        probe: false,
        hand: false,
        law: &[],
    };
    let parts = match (&context.verdict, mode) {
        (Verdict::Pass, Mode::Full) => Parts {
            subject: true,
            hand: true,
            law: &["subject", "hand"],
            ..nothing
        },
        (Verdict::Pass, Mode::HandOnly) => Parts {
            hand: true,
            law: &["hand"],
            ..nothing
        },
        (Verdict::Limit(expected), _) => Parts {
            subject: true,
            probe: expected
                .iter()
                .all(|expected| expected.limit.entry().keeps_impl),
            ..nothing
        },
        (Verdict::Rejected { .. }, _) => Parts {
            subject: true,
            hand: true,
            ..nothing
        },
        (Verdict::Malformed(_), _) => nothing,
        (Verdict::Unmodelled(_) | Verdict::NonCanonical, _) => {
            return finish(writer);
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
    if parts.subject {
        writer.begin(Part::Subject);
        declarations::write_subject(&mut writer, context);
        writer.blank();
    }
    if parts.probe {
        writer.begin(Part::Probe);
        declarations::write_probe(&mut writer, context);
        writer.blank();
    }
    if parts.hand {
        writer.begin(Part::Hand);
        hand::write(&mut writer, context);
        writer.blank();
    }
    if !parts.law.is_empty() {
        writer.begin(Part::Law);
        law::write(&mut writer, context, parts.law);
    }
    finish(writer)
}

fn finish(writer: Writer) -> Rendered {
    let (source, sections, anchors) = writer.finish();
    let attributes = source
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let line = line.trim_start();
            line.starts_with("#[whichever::") || line.starts_with("#[dispatch")
        })
        .map(|(index, _)| index + 1)
        .collect();
    Rendered {
        source,
        sections,
        anchors,
        attributes,
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

    #[test]
    fn a_cell_rendered_with_its_hand_alone_tests_the_hand_only() {
        let rendered = render_as(&Cell::neutral(), Mode::HandOnly);
        assert!(!rendered.source.contains("#[whichever::dispatch("));
        assert!(rendered.source.contains("mod hand {"));
        assert!(rendered.source.contains("fn hand()"));
        assert!(!rendered.source.contains("fn subject()"));
    }

    #[test]
    fn attribute_lines_are_recorded() {
        let rendered = render(&Cell::neutral());
        let attribute = rendered
            .line_of(Anchor::Attribute)
            .expect("an attribute anchor");
        assert!(rendered.attributes.contains(&attribute));
        assert!(rendered.covers_attribute(attribute - 1, attribute + 1));
        let variant = rendered.line_of(Anchor::Variant(0)).expect("a variant");
        assert!(!rendered.covers_attribute(variant, variant));
    }
}
