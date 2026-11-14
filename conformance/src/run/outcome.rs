//! What a run observed for a cell, and whether it is what the verdict
//! allows.

use std::collections::BTreeMap;

use super::cargo::{Diagnostic, Level, TestResult};
use crate::model::{Anchor, Limit, Verdict};
use crate::render::writer::Part;
use crate::render::Rendered;

/// One thing a run observed for a cell.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Finding {
    /// A sentence of the macro: its catalogue key, or its text in quotes
    /// when the catalogue does not hold it; and where it was reported, an
    /// anchor's name or `line-N`.
    Sentence {
        /// The catalogue key or the quoted text.
        key: String,
        /// Where.
        place: String,
    },
    /// A rustc error, by code, or its message in quotes when it has none.
    Error {
        /// The code.
        code: String,
        /// The part of the cell it was reported in.
        part: String,
    },
    /// A warning, by lint name.
    Warning {
        /// The lint's name.
        lint: String,
        /// The part of the cell it was reported in.
        part: String,
    },
    /// A test of the law failed.
    Law {
        /// `subject` or `hand`.
        test: String,
    },
    /// A test of the law reported no result.
    Untested {
        /// `subject` or `hand`.
        test: String,
    },
}

impl Finding {
    /// Returns the finding as the outcome spells it.
    pub fn spell(&self) -> String {
        match self {
            Finding::Sentence { key, place } => format!("sentence:{key}@{place}"),
            Finding::Error { code, part } => format!("error:{code}@{part}"),
            Finding::Warning { lint, part } => format!("warning:{lint}@{part}"),
            Finding::Law { test } => format!("law:{test}"),
            Finding::Untested { test } => format!("untested:{test}"),
        }
    }

    /// Returns whether the finding says the hand is wrong: the model's
    /// fault, never the macro's.
    pub fn blames_the_hand(&self) -> bool {
        match self {
            Finding::Error { part, .. } | Finding::Warning { part, .. } => {
                matches!(
                    part.as_str(),
                    "hand" | "law-hand" | "header" | "payloads" | "probe"
                )
            }
            Finding::Law { test } | Finding::Untested { test } => test == "hand",
            Finding::Sentence { .. } => false,
        }
    }
}

/// What a run observed for a cell.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The findings, sorted and without repetition.
    pub findings: Vec<Finding>,
    /// The first message observed: a diagnostic's or a failed test's.
    pub first_message: Option<String>,
}

impl Outcome {
    /// Returns the outcome as `known.tsv` spells it: `pass`, or the
    /// findings joined by `, `.
    pub fn spell(&self) -> String {
        if self.findings.is_empty() {
            return String::from("pass");
        }
        let findings: Vec<String> = self.findings.iter().map(Finding::spell).collect();
        findings.join(", ")
    }

    /// Returns whether the cell has an error, so a pass shard sets it
    /// aside before it is tested.
    pub fn has_error(&self) -> bool {
        self.findings.iter().any(|finding| match finding {
            Finding::Sentence { .. } | Finding::Error { .. } => true,
            Finding::Warning { .. } | Finding::Law { .. } | Finding::Untested { .. } => false,
        })
    }
}

/// Returns the part a line of a cell is in, with the law split by the test
/// whose expansion of it the backtrace passed through.
fn part_of(rendered: &Rendered, line: usize, chain: &[usize]) -> String {
    let Some(part) = rendered.part_of(line) else {
        return String::from("outside");
    };
    if part != Part::Law {
        return String::from(part.name());
    }
    for test in ["subject", "hand"] {
        let anchor = rendered.anchors.get(&format!("law-{test}"));
        if anchor.is_some_and(|anchor| chain.contains(anchor)) {
            return format!("law-{test}");
        }
    }
    String::from("law")
}

/// Returns the name of the anchor at a line, or `line-N`.
fn place_of(rendered: &Rendered, line: usize) -> String {
    rendered
        .anchors
        .iter()
        .find(|(_, anchor)| **anchor == line)
        .map(|(name, _)| name.clone())
        .unwrap_or_else(|| format!("line-{line}"))
}

/// Adds a diagnostic to the outcome of the cell it is attributed to.
pub fn observe_diagnostic(outcome: &mut Outcome, rendered: &Rendered, diagnostic: &Diagnostic) {
    let Some(site) = &diagnostic.site else {
        return;
    };
    let chain: Vec<usize> = diagnostic
        .chain
        .iter()
        .filter(|other| other.module == site.module)
        .map(|other| other.line)
        .collect();
    let part = part_of(rendered, site.line, &chain);
    let finding = match diagnostic.level {
        Level::Warning => Finding::Warning {
            lint: diagnostic
                .code
                .clone()
                .unwrap_or_else(|| quoted(&diagnostic.message)),
            part,
        },
        Level::Error => match Limit::of_message(&diagnostic.message) {
            Some(limit) => Finding::Sentence {
                key: String::from(limit.entry().key),
                place: place_of(rendered, site.line),
            },
            None if diagnostic.from_compile_error || is_sentence(diagnostic) => Finding::Sentence {
                key: quoted(&diagnostic.message),
                place: place_of(rendered, site.line),
            },
            None => Finding::Error {
                code: diagnostic
                    .code
                    .clone()
                    .unwrap_or_else(|| quoted(&diagnostic.message)),
                part,
            },
        },
    };
    if outcome.first_message.is_none() {
        outcome.first_message = Some(diagnostic.message.clone());
    }
    if !outcome.findings.contains(&finding) {
        outcome.findings.push(finding);
        outcome.findings.sort();
    }
}

/// Adds the result of a test of the law, `test` being `subject` or `hand`.
pub fn observe_test(outcome: &mut Outcome, test: &str, result: Option<&TestResult>) {
    let finding = match result {
        Some(TestResult::Passed) => return,
        Some(TestResult::Failed(message)) => {
            if outcome.first_message.is_none() {
                outcome.first_message = Some(format!("law {test}: {message}"));
            }
            Finding::Law {
                test: String::from(test),
            }
        }
        None => Finding::Untested {
            test: String::from(test),
        },
    };
    if !outcome.findings.contains(&finding) {
        outcome.findings.push(finding);
        outcome.findings.sort();
    }
}

/// Returns whether an error the catalogue does not hold is a sentence of
/// the macro rather than rustc's. A sentence is raised by `compile_error!`
/// on a user's token, which leaves no backtrace in the JSON, so it is told
/// apart by having no code; the few rustc errors without one are listed.
fn is_sentence(diagnostic: &Diagnostic) -> bool {
    const RUSTC: &[&str] = &[
        "lifetime may not live long enough",
        "higher-ranked lifetime error",
        "implementation of",
        "cannot find",
        "expected",
        "unexpected",
        "mismatched closing delimiter",
        "this file contains an unclosed delimiter",
        "unknown start of token",
        "macro expansion ignored",
        "proc-macro derive panicked",
        "custom attribute panicked",
        "proc macro panicked",
        "recursion limit reached",
        "captured variable cannot escape",
        "temporary value dropped while borrowed",
        "unresolved import",
        "ambiguous",
    ];
    diagnostic.code.is_none()
        && !RUSTC
            .iter()
            .any(|prefix| diagnostic.message.starts_with(prefix))
}

/// Returns a message as a short quoted key, which holds no `,` and no tab.
fn quoted(message: &str) -> String {
    let cleaned: String = message
        .chars()
        .map(|c| match c {
            ',' | '\t' | '"' => ' ',
            other => other,
        })
        .take(60)
        .collect();
    format!("\"{}\"", cleaned.trim())
}

/// How a cell's outcome stands to its verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Judgement {
    /// The outcome is what the verdict allows.
    Agrees,
    /// The outcome is not what the verdict allows.
    Diverges,
    /// The hand does not compile or fails its law: the model is wrong.
    Harness,
}

/// Returns how an outcome stands to a verdict.
pub fn judge(verdict: &Verdict, outcome: &Outcome, rendered: &Rendered) -> Judgement {
    match verdict {
        Verdict::Pass => {
            if outcome.findings.iter().any(Finding::blames_the_hand) {
                return Judgement::Harness;
            }
            match outcome.findings.is_empty() {
                true => Judgement::Agrees,
                false => Judgement::Diverges,
            }
        }
        Verdict::Limit(expected) => {
            if outcome.findings.is_empty() {
                return Judgement::Diverges;
            }
            let lines: BTreeMap<String, Anchor> = expected
                .iter()
                .filter_map(|expected| {
                    let line = rendered.line_of(expected.anchor)?;
                    Some((format!("line-{line}"), expected.anchor))
                })
                .collect();
            let allowed = outcome.findings.iter().all(|finding| {
                let Finding::Sentence { key, place } = finding else {
                    return false;
                };
                expected.iter().any(|expected| {
                    expected.limit.entry().key == key
                        && match expected.anchor {
                            Anchor::Anywhere => true,
                            anchor => *place == anchor.name() || lines.get(place) == Some(&anchor),
                        }
                })
            });
            match allowed {
                true => Judgement::Agrees,
                false => Judgement::Diverges,
            }
        }
        Verdict::Malformed(_) => {
            let failed = outcome.findings.iter().any(|finding| match finding {
                Finding::Error { .. } | Finding::Sentence { .. } => true,
                Finding::Warning { .. } | Finding::Law { .. } | Finding::Untested { .. } => false,
            });
            match failed {
                true => Judgement::Agrees,
                false => Judgement::Diverges,
            }
        }
        Verdict::Unmodelled(_) | Verdict::NonCanonical => Judgement::Harness,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Cell;
    use crate::render::render;

    #[test]
    fn a_clean_pass_agrees_and_a_finding_diverges() {
        let cell = Cell::neutral();
        let rendered = render(&cell);
        let mut outcome = Outcome::default();
        assert_eq!(
            judge(&Verdict::Pass, &outcome, &rendered),
            Judgement::Agrees
        );
        assert_eq!(outcome.spell(), "pass");
        observe_test(
            &mut outcome,
            "subject",
            Some(&TestResult::Failed(String::from("left"))),
        );
        assert_eq!(
            judge(&Verdict::Pass, &outcome, &rendered),
            Judgement::Diverges
        );
        assert_eq!(outcome.spell(), "law:subject");
        observe_test(&mut outcome, "hand", None);
        assert_eq!(
            judge(&Verdict::Pass, &outcome, &rendered),
            Judgement::Harness
        );
    }

    #[test]
    fn a_limit_agrees_only_with_its_sentence_on_its_anchor() {
        let cell = Cell::parse("recv=rc").expect("an id");
        let rendered = render(&cell);
        let verdict = cell.verdict();
        let on_method = Outcome {
            findings: vec![Finding::Sentence {
                key: String::from("receiver"),
                place: String::from("method"),
            }],
            first_message: None,
        };
        assert_eq!(judge(&verdict, &on_method, &rendered), Judgement::Agrees);
        let elsewhere = Outcome {
            findings: vec![Finding::Sentence {
                key: String::from("receiver"),
                place: String::from("line-1"),
            }],
            first_message: None,
        };
        assert_eq!(judge(&verdict, &elsewhere, &rendered), Judgement::Diverges);
        assert_eq!(
            judge(&verdict, &Outcome::default(), &rendered),
            Judgement::Diverges
        );
    }
}
