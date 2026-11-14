//! A writer of indented lines that records where each part of a cell and
//! each anchor lands.

use std::collections::BTreeMap;

/// A part of a rendered cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Part {
    /// The header comment and the imports.
    Header,
    /// The trait.
    Trait,
    /// The payloads, their wrappers and their impls.
    Payloads,
    /// The subject spelled with the attributes.
    Subject,
    /// The subject's inherent block, on an inherent path.
    Inherent,
    /// The call through the trait that shows a withheld impl.
    Probe,
    /// The hand's subject and its forwarding.
    Hand,
    /// The law.
    Law,
}

impl Part {
    /// Returns the part's name.
    pub fn name(self) -> &'static str {
        match self {
            Part::Header => "header",
            Part::Trait => "trait",
            Part::Payloads => "payloads",
            Part::Subject => "subject",
            Part::Inherent => "inherent",
            Part::Probe => "probe",
            Part::Hand => "hand",
            Part::Law => "law",
        }
    }
}

/// A range of lines, both ends included and counted from 1, that one part
/// holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Section {
    /// The part.
    pub part: Part,
    /// The first line.
    pub first: usize,
    /// The last line.
    pub last: usize,
}

/// A writer of indented lines.
#[derive(Debug, Default)]
pub struct Writer {
    lines: Vec<String>,
    depth: usize,
    sections: Vec<Section>,
    open: Option<(Part, usize)>,
    anchors: BTreeMap<String, usize>,
}

const INDENT: &str = "    ";

impl Writer {
    /// Returns an empty writer.
    pub fn new() -> Writer {
        Writer::default()
    }

    /// Writes one line at the current depth; a line holding a newline is
    /// written as several.
    pub fn line(&mut self, text: impl AsRef<str>) {
        for line in text.as_ref().split('\n') {
            if line.is_empty() {
                self.lines.push(String::new());
                continue;
            }
            self.lines
                .push(format!("{}{line}", INDENT.repeat(self.depth)));
        }
    }

    /// Writes an empty line, unless no line is written yet, the last line
    /// is empty already, or it opens a block.
    pub fn blank(&mut self) {
        let Some(last) = self.lines.last() else {
            return;
        };
        if last.is_empty() || last.ends_with('{') {
            return;
        }
        self.lines.push(String::new());
    }

    /// Writes a line and indents what follows: `fn m() {`.
    pub fn open(&mut self, text: impl AsRef<str>) {
        self.line(text);
        self.depth += 1;
    }

    /// Dedents and writes a line: `}`.
    pub fn close(&mut self, text: impl AsRef<str>) {
        self.depth = self.depth.saturating_sub(1);
        self.line(text);
    }

    /// Records that the next line written is the anchor `name`.
    pub fn anchor(&mut self, name: impl Into<String>) {
        self.anchors.insert(name.into(), self.lines.len() + 1);
    }

    /// Starts a part; the part open before it ends.
    pub fn begin(&mut self, part: Part) {
        self.end();
        self.open = Some((part, self.lines.len() + 1));
    }

    /// Ends the open part.
    pub fn end(&mut self) {
        let Some((part, first)) = self.open.take() else {
            return;
        };
        let last = self.lines.len();
        if last >= first {
            self.sections.push(Section { part, first, last });
        }
    }

    /// Returns the text, the sections and the anchors.
    pub fn finish(mut self) -> (String, Vec<Section>, BTreeMap<String, usize>) {
        self.end();
        while self.lines.last().is_some_and(|line| line.is_empty()) {
            self.lines.pop();
        }
        let mut source = self.lines.join("\n");
        source.push('\n');
        (source, self.sections, self.anchors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_sections_and_anchors_are_counted_from_one() {
        let mut writer = Writer::new();
        writer.begin(Part::Header);
        writer.line("// a header");
        writer.blank();
        writer.begin(Part::Trait);
        writer.open("trait T {");
        writer.anchor("method");
        writer.line("fn m(&self);");
        writer.close("}");
        let (source, sections, anchors) = writer.finish();
        assert_eq!(source, "// a header\n\ntrait T {\n    fn m(&self);\n}\n");
        assert_eq!(
            sections,
            vec![
                Section {
                    part: Part::Header,
                    first: 1,
                    last: 2
                },
                Section {
                    part: Part::Trait,
                    first: 3,
                    last: 5
                },
            ]
        );
        assert_eq!(anchors.get("method"), Some(&4));
    }
}
