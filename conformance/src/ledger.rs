//! The checked-in record: `known.tsv`, the cells that diverge from the
//! model with the reason for each, `pinned.txt`, the cells always
//! selected, and `census.txt`, how many cells each suite holds by verdict.

use std::path::{Path, PathBuf};

use crate::cell::{Cell, Pattern};
use crate::select::Selection;

/// A line of `known.tsv`: the cells that diverge, how, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Known {
    /// The cells, by a pattern in id syntax.
    pub pattern: Pattern,
    /// The outcome it diverges with, as a run spells it.
    pub outcome: String,
    /// What is wrong, in one sentence.
    pub reason: String,
    /// The line of the file it was read from.
    pub line: usize,
}

/// A line of `pinned.txt`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pinned {
    /// The cell, by its id.
    pub id: String,
    /// Where the shape came from.
    pub origin: String,
}

/// Why a ledger file does not parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerError(pub String);

impl core::fmt::Display for LedgerError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The ledger's files, under the crate's directory.
#[derive(Clone, Debug)]
pub struct Files {
    /// `known.tsv`.
    pub known: PathBuf,
    /// `pinned.txt`.
    pub pinned: PathBuf,
    /// `census.txt`.
    pub census: PathBuf,
}

impl Files {
    /// Returns the files of the crate at `directory`.
    pub fn at(directory: &Path) -> Files {
        Files {
            known: directory.join("known.tsv"),
            pinned: directory.join("pinned.txt"),
            census: directory.join("census.txt"),
        }
    }
}

/// Checks that an id parses and is spelled as the cell spells it.
fn canonical(id: &str, file: &str, line: usize) -> Result<Cell, LedgerError> {
    let cell = Cell::parse(id).map_err(|error| LedgerError(format!("{file}:{line}: {error}")))?;
    if cell.id() != id {
        return Err(LedgerError(format!(
            "{file}:{line}: `{id}` is spelled `{}`",
            cell.id()
        )));
    }
    Ok(cell)
}

/// Parses `known.tsv`: tab-separated `id`, `outcome` and `reason`, with
/// `#` comments and blank lines.
pub fn parse_known(text: &str) -> Result<Vec<Known>, LedgerError> {
    let mut known: Vec<Known> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        if raw.trim().is_empty() || raw.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = raw.split('\t').collect();
        let [id, outcome, reason] = fields.as_slice() else {
            return Err(LedgerError(format!(
                "known.tsv:{line}: expected three tab-separated fields, `id`, `outcome` and `reason`; found {}",
                fields.len()
            )));
        };
        let pattern = Pattern::parse(id)
            .map_err(|error| LedgerError(format!("known.tsv:{line}: {error}")))?;
        if outcome.trim().is_empty() || reason.trim().is_empty() {
            return Err(LedgerError(format!(
                "known.tsv:{line}: the outcome and the reason are not empty"
            )));
        }
        if !reason.trim_end().ends_with('.') {
            return Err(LedgerError(format!(
                "known.tsv:{line}: the reason is one sentence ending in a period"
            )));
        }
        let outcome = outcome.trim();
        if let Some(earlier) = known
            .iter()
            .find(|entry| entry.pattern.text == pattern.text && entry.outcome == outcome)
        {
            return Err(LedgerError(format!(
                "known.tsv:{line}: `{id}` with this outcome is listed on line {} already",
                earlier.line
            )));
        }
        known.push(Known {
            pattern,
            outcome: String::from(outcome),
            reason: String::from(reason.trim()),
            line,
        });
    }
    Ok(known)
}

/// Parses `pinned.txt`: one id per line, with an optional `#` comment
/// naming where the shape came from.
pub fn parse_pinned(text: &str) -> Result<Vec<Pinned>, LedgerError> {
    let mut pinned: Vec<Pinned> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let (id, origin) = match raw.split_once('#') {
            Some((id, origin)) => (id.trim(), origin.trim()),
            None => (raw.trim(), ""),
        };
        if id.is_empty() {
            continue;
        }
        canonical(id, "pinned.txt", line)?;
        if pinned.iter().any(|entry| entry.id == id) {
            return Err(LedgerError(format!(
                "pinned.txt:{line}: `{id}` is listed twice"
            )));
        }
        pinned.push(Pinned {
            id: String::from(id),
            origin: String::from(origin),
        });
    }
    Ok(pinned)
}

/// Returns the census of the suites: per suite, the cells by verdict and
/// the combinations never selected by reason.
pub fn census(selections: &[Selection]) -> String {
    let mut text = String::from(
        "# The cells each suite holds, by verdict, and the combinations it spans\n\
         # but never selects, by reason. A run compares this file exactly;\n\
         # `run --bless` rewrites it.\n",
    );
    for selection in selections {
        text.push('\n');
        text.push_str(&format!("suite {}\n", selection.suite));
        let counts = selection.by_verdict();
        for kind in ["pass", "limit", "rejected", "malformed"] {
            text.push_str(&format!(
                "  {kind} {}\n",
                counts.get(kind).copied().unwrap_or(0)
            ));
        }
        if let Some(coverage) = &selection.coverage {
            text.push_str(&format!(
                "  rows {}\n  covered {}\n  unreached {}\n",
                coverage.rows, coverage.covered, coverage.unreached
            ));
        }
        for (reason, count) in &selection.never {
            text.push_str(&format!("  never {count} {reason}\n"));
        }
    }
    text
}

/// Returns the lines of `found` that differ from `expected`, each marked
/// `-` for a line expected and `+` for a line found, in order.
pub fn differences(expected: &str, found: &str) -> Vec<String> {
    let expected: Vec<&str> = expected.lines().collect();
    let found: Vec<&str> = found.lines().collect();
    let mut lines = Vec::new();
    for line in &expected {
        if !found.contains(line) {
            lines.push(format!("- {line}"));
        }
    }
    for line in &found {
        if !expected.contains(line) {
            lines.push(format!("+ {line}"));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn known_lines_parse() {
        let text = "# A comment.\n\nrecv=mut\terror:E0308@subject\tThe arm borrows twice.\n";
        let known = parse_known(text).expect("a valid file");
        assert_eq!(known.len(), 1);
        assert_eq!(known[0].pattern.text, "recv=mut");
        assert_eq!(known[0].outcome, "error:E0308@subject");
        assert_eq!(known[0].reason, "The arm borrows twice.");
        assert_eq!(known[0].line, 3);
        let patterns = parse_known("s1=plain recv=mut\ta\tA.\nrecv=mut\tb\tB.\n")
            .expect("patterns name neutral values and repeat with other outcomes");
        assert_eq!(patterns.len(), 2);
    }

    #[test]
    fn bad_known_lines_are_errors() {
        for text in [
            "recv=mut\terror:E0308@subject\n",
            "recv=boxed\terror:E0308@subject\tA reason.\n",
            "recv=mut\t\tA reason.\n",
            "recv=mut\terror:E0308@subject\tno period\n",
            "recv=mut\ta\tA.\nrecv=mut\ta\tB.\n",
        ] {
            assert!(parse_known(text).is_err(), "{text:?} parsed");
        }
    }

    #[test]
    fn pinned_lines_parse_with_their_origin() {
        let text = "# The cells of bugs.\ns0=deref-rc recv=box  # a bug note\nbase\n";
        let pinned = parse_pinned(text).expect("a valid file");
        assert_eq!(pinned.len(), 2);
        assert_eq!(pinned[0].id, "s0=deref-rc recv=box");
        assert!(parse_pinned("recv=box s0=deref-rc\n").is_err());
        assert_eq!(pinned[0].origin, "a bug note");
        assert_eq!(pinned[1].origin, "");
        assert!(parse_pinned("recv=nothing\n").is_err());
        assert!(parse_pinned("base\nbase\n").is_err());
    }

    #[test]
    fn the_census_counts_by_verdict_and_reason() {
        let selection = Selection {
            suite: String::from("zone"),
            cells: vec![Cell::neutral()],
            never: BTreeMap::from([(String::from("non-canonical"), 2)]),
            coverage: None,
        };
        assert_eq!(
            census(&[selection]).lines().skip(4).collect::<Vec<_>>(),
            vec![
                "suite zone",
                "  pass 1",
                "  limit 0",
                "  rejected 0",
                "  malformed 0",
                "  never 2 non-canonical"
            ]
        );
        assert_eq!(differences("a\nb\n", "a\nc\n"), vec!["- b", "+ c"]);
    }
}
