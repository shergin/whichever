//! What a run observed for a cell, and whether it is what the verdict
//! allows.

use std::collections::BTreeMap;

use super::cargo::{Diagnostic, Level, TestResult};
use crate::cell::Cell;
use crate::model::{wording, Anchor, Limit, Verdict};
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
    /// The shape of a forwarded method differs from the hand's.
    Shape {
        /// The method.
        method: String,
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
            Finding::Shape { method } => format!("shape:{method}"),
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
            Finding::Sentence { .. } | Finding::Shape { .. } => false,
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
    /// Every sentence observed, by its catalogue key and its place, with
    /// its message, so that its words can be held.
    pub sentences: Vec<(String, String, String)>,
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
            Finding::Warning { .. }
            | Finding::Law { .. }
            | Finding::Shape { .. }
            | Finding::Untested { .. } => false,
        })
    }

    /// Adds a finding once, keeping the findings sorted.
    pub fn add(&mut self, finding: Finding, message: Option<String>) {
        if self.first_message.is_none() {
            self.first_message = message;
        }
        if !self.findings.contains(&finding) {
            self.findings.push(finding);
            self.findings.sort();
        }
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
    let part = match rendered.covers_attribute(site.line, site.last) {
        true => String::from("attribute"),
        false => part_of(rendered, site.line, &chain),
    };
    let finding = match diagnostic.level {
        Level::Warning => Finding::Warning {
            lint: diagnostic
                .code
                .clone()
                .unwrap_or_else(|| quoted(&diagnostic.message)),
            part,
        },
        Level::Error => match Limit::of_message(&diagnostic.message) {
            Some(limit) => {
                let key = String::from(limit.entry().key);
                let place = place_of(rendered, site.line);
                outcome
                    .sentences
                    .push((key.clone(), place.clone(), diagnostic.message.clone()));
                Finding::Sentence { key, place }
            }
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
    /// The outcome is not what the verdict allows, for these findings,
    /// spelled as the outcome spells them.
    Diverges(Vec<String>),
    /// The hand does not compile or fails its law: the model is wrong.
    Harness,
}

/// Returns how an outcome stands to a verdict. The findings that make a
/// cell diverge are decided here and nowhere else:
///
/// - for a pass, every finding;
/// - for a limit, every finding that is not a predicted sentence on its
///   anchor, `missing:sentence` when no predicted sentence appears, and
///   `missing:KEY@ANCHOR` for each predicted sentence about a variant that
///   does not, since every variant gets its own;
/// - for a rejected cell, `compiles:subject` when the subject compiles,
///   `missing:error:CODE@subject` when it fails without the code the model
///   names, and every finding of the subject that is not an error off the
///   attributes of a code the hand gets too, a sentence included, since a
///   rejected cell predicts none; the hand's expected failure is none;
/// - for a malformed cell, `compiles:trait` when nothing fails.
pub fn judge(cell: &Cell, verdict: &Verdict, outcome: &Outcome, rendered: &Rendered) -> Judgement {
    let divergent = |findings: Vec<String>| match findings.is_empty() {
        true => Judgement::Agrees,
        false => Judgement::Diverges(findings),
    };
    let is_error = |finding: &Finding| match finding {
        Finding::Sentence { .. } | Finding::Error { .. } => true,
        Finding::Warning { .. }
        | Finding::Law { .. }
        | Finding::Shape { .. }
        | Finding::Untested { .. } => false,
    };
    match verdict {
        Verdict::Pass => {
            if outcome.findings.iter().any(Finding::blames_the_hand) {
                return Judgement::Harness;
            }
            divergent(outcome.findings.iter().map(Finding::spell).collect())
        }
        Verdict::Limit(expected) => {
            let lines: BTreeMap<String, Anchor> = expected
                .iter()
                .filter_map(|expected| {
                    let line = rendered.line_of(expected.anchor)?;
                    Some((format!("line-{line}"), expected.anchor))
                })
                .collect();
            let predicted = |finding: &Finding| {
                let Finding::Sentence { key, place } = finding else {
                    return false;
                };
                expected.iter().any(|expected| {
                    expected.limit.entry().key == key
                        && match expected.anchor {
                            Anchor::Anywhere => true,
                            anchor => {
                                *place == anchor.name()
                                    || *place == format!("{}-cfg", anchor.name())
                                    || lines.get(place) == Some(&anchor)
                            }
                        }
                })
            };
            let mut findings: Vec<String> = outcome
                .findings
                .iter()
                .filter(|finding| !predicted(finding))
                .map(Finding::spell)
                .collect();
            if !outcome.findings.iter().any(predicted) {
                findings.push(String::from("missing:sentence"));
            }
            // Every variant gets a sentence about it, in its own arm or on
            // its own declaration, so each variant predicted one has to
            // show one. An arm carries one sentence, and which of several
            // the docs do not say, so a variant is held to a subset of its
            // own; so are the sentences on the method or the subject, where
            // one body stands for several a method could get.
            let appears = |wanted: &crate::model::Expected| {
                outcome.findings.iter().any(|finding| {
                    let Finding::Sentence { key, place } = finding else {
                        return false;
                    };
                    let anchor = wanted.anchor;
                    wanted.limit.entry().key == key
                        && (*place == anchor.name()
                            || *place == format!("{}-cfg", anchor.name())
                            || lines.get(place) == Some(&anchor))
                })
            };
            // A sentence with the right key says what the cell asks of it:
            // its way out, the method's own where it ends in one, and the
            // names it interpolates; one that does not is a finding of its
            // own.
            for (key, place, message) in &outcome.sentences {
                let wanted = expected.iter().find(|wanted| {
                    let anchor = wanted.anchor;
                    wanted.limit.entry().key == key
                        && (*place == anchor.name()
                            || *place == format!("{}-cfg", anchor.name())
                            || lines.get(place) == Some(&anchor)
                            || anchor == Anchor::Anywhere)
                });
                let Some(wanted) = wanted else {
                    continue;
                };
                let lacks = wording::fragments(cell, wanted)
                    .iter()
                    .any(|fragment| !fragment.is_in(message));
                let finding = format!("wording:{key}");
                if lacks && !findings.contains(&finding) {
                    findings.push(finding);
                }
            }
            let found_any = outcome.findings.iter().any(predicted);
            let variants: Vec<Anchor> = expected
                .iter()
                .map(|wanted| wanted.anchor)
                .filter(|anchor| matches!(anchor, Anchor::Variant(_)))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            for anchor in variants {
                let own: Vec<&crate::model::Expected> = expected
                    .iter()
                    .filter(|wanted| wanted.anchor == anchor)
                    .collect();
                if found_any && !own.iter().any(|wanted| appears(wanted)) {
                    findings.push(format!(
                        "missing:{}@{}",
                        own[0].limit.entry().key,
                        anchor.name()
                    ));
                }
            }
            divergent(findings)
        }
        Verdict::Rejected { code, .. } => {
            let quoted_code = quoted(code);
            let is_code = |found: &str| found == *code || found == quoted_code;
            let in_hand = |finding: &Finding| match finding {
                Finding::Error { part, .. } | Finding::Warning { part, .. } => part == "hand",
                Finding::Sentence { .. }
                | Finding::Law { .. }
                | Finding::Shape { .. }
                | Finding::Untested { .. } => false,
            };
            let expected = outcome.findings.iter().any(|finding| match finding {
                Finding::Error { code: found, part } => part == "hand" && is_code(found),
                Finding::Sentence { .. }
                | Finding::Warning { .. }
                | Finding::Law { .. }
                | Finding::Shape { .. }
                | Finding::Untested { .. } => false,
            });
            if !expected {
                return Judgement::Harness;
            }
            let subject: Vec<&Finding> = outcome
                .findings
                .iter()
                .filter(|finding| !in_hand(finding))
                .collect();
            // The subject fails as the hand does: with the code the model
            // names, and otherwise only with codes the hand gets too, off
            // the attributes. A rejected cell predicts no sentence, so a
            // sentence of the macro's is a wrong answer, and an error of a
            // code the hand does not get is a cascade.
            let hand_codes: Vec<&str> = outcome
                .findings
                .iter()
                .filter_map(|finding| match finding {
                    Finding::Error { code, part } if part == "hand" => Some(code.as_str()),
                    Finding::Sentence { .. }
                    | Finding::Error { .. }
                    | Finding::Warning { .. }
                    | Finding::Law { .. }
                    | Finding::Shape { .. }
                    | Finding::Untested { .. } => None,
                })
                .collect();
            let shared = |finding: &&Finding| match finding {
                Finding::Error { code: found, part } => {
                    hand_codes.contains(&found.as_str()) && part != "attribute"
                }
                Finding::Sentence { .. }
                | Finding::Warning { .. }
                | Finding::Law { .. }
                | Finding::Shape { .. }
                | Finding::Untested { .. } => false,
            };
            let own = |finding: &&Finding| match finding {
                Finding::Error { code: found, .. } => is_code(found) && shared(finding),
                Finding::Sentence { .. }
                | Finding::Warning { .. }
                | Finding::Law { .. }
                | Finding::Shape { .. }
                | Finding::Untested { .. } => false,
            };
            let mut findings: Vec<String> = subject
                .iter()
                .filter(|finding| !shared(finding))
                .map(|finding| finding.spell())
                .collect();
            if !subject.iter().any(|finding| is_error(finding)) {
                findings.push(String::from("compiles:subject"));
            } else if !subject.iter().any(own) {
                // The code as an error finding spells it: a code, or the
                // message of an error that has none, quoted.
                let numbered = code.strip_prefix('E').is_some_and(|digits| {
                    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
                });
                let spelled = match numbered {
                    true => String::from(*code),
                    false => quoted_code.clone(),
                };
                findings.push(format!("missing:error:{spelled}@subject"));
            }
            divergent(findings)
        }
        Verdict::Malformed(_) => match outcome.findings.iter().any(is_error) {
            true => Judgement::Agrees,
            false => Judgement::Diverges(vec![String::from("compiles:trait")]),
        },
        Verdict::Unmodelled(_) | Verdict::NonCanonical => Judgement::Harness,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render;

    #[test]
    fn a_clean_pass_agrees_and_a_finding_diverges() {
        let cell = Cell::neutral();
        let rendered = render(&cell);
        let mut outcome = Outcome::default();
        assert_eq!(
            judge(&cell, &Verdict::Pass, &outcome, &rendered),
            Judgement::Agrees
        );
        assert_eq!(outcome.spell(), "pass");
        observe_test(
            &mut outcome,
            "subject",
            Some(&TestResult::Failed(String::from("left"))),
        );
        assert!(matches!(
            judge(&cell, &Verdict::Pass, &outcome, &rendered),
            Judgement::Diverges(_)
        ));
        assert_eq!(outcome.spell(), "law:subject");
        observe_test(&mut outcome, "hand", None);
        assert_eq!(
            judge(&cell, &Verdict::Pass, &outcome, &rendered),
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
            sentences: Vec::new(),
        };
        assert_eq!(
            judge(&cell, &verdict, &on_method, &rendered),
            Judgement::Agrees
        );
        let elsewhere = Outcome {
            findings: vec![Finding::Sentence {
                key: String::from("receiver"),
                place: String::from("line-1"),
            }],
            first_message: None,
            sentences: Vec::new(),
        };
        assert!(matches!(
            judge(&cell, &verdict, &elsewhere, &rendered),
            Judgement::Diverges(_)
        ));
        assert!(matches!(
            judge(&cell, &verdict, &Outcome::default(), &rendered),
            Judgement::Diverges(_)
        ));
    }

    #[test]
    fn a_rejection_agrees_only_off_the_attributes() {
        let cell = Cell::parse("s0=via-method recv=value").expect("an id");
        let rendered = render(&cell);
        let verdict = cell.verdict();
        assert!(matches!(verdict, Verdict::Rejected { code: "E0277", .. }));
        let error = |code: &str, part: &str| Finding::Error {
            code: String::from(code),
            part: String::from(part),
        };
        let outcome = |findings: Vec<Finding>| Outcome {
            findings,
            first_message: None,
            sentences: Vec::new(),
        };
        let on_payload = outcome(vec![error("E0277", "hand"), error("E0277", "subject")]);
        assert_eq!(
            judge(&cell, &verdict, &on_payload, &rendered),
            Judgement::Agrees
        );
        let on_attribute = outcome(vec![error("E0277", "hand"), error("E0277", "attribute")]);
        assert!(matches!(
            judge(&cell, &verdict, &on_attribute, &rendered),
            Judgement::Diverges(_)
        ));
        let hand_only = outcome(vec![error("E0277", "hand")]);
        assert!(matches!(
            judge(&cell, &verdict, &hand_only, &rendered),
            Judgement::Diverges(_)
        ));
        // A second code the hand gets too is the hand's; a cascade, an
        // error of a code the hand does not get, and a sentence diverge,
        // and so does a subject that fails without the code the model names.
        let shared = outcome(vec![
            error("E0277", "hand"),
            error("E0308", "hand"),
            error("E0277", "subject"),
            error("E0308", "subject"),
        ]);
        assert_eq!(
            judge(&cell, &verdict, &shared, &rendered),
            Judgement::Agrees
        );
        let cascade = outcome(vec![
            error("E0277", "hand"),
            error("E0277", "subject"),
            error("E0282", "subject"),
        ]);
        assert_eq!(
            judge(&cell, &verdict, &cascade, &rendered),
            Judgement::Diverges(vec![String::from("error:E0282@subject")])
        );
        let sentence = Finding::Sentence {
            key: String::from("receiver"),
            place: String::from("method"),
        };
        let with_sentence = outcome(vec![
            error("E0277", "hand"),
            error("E0277", "subject"),
            sentence,
        ]);
        assert!(matches!(
            judge(&cell, &verdict, &with_sentence, &rendered),
            Judgement::Diverges(_)
        ));
        let other_code = outcome(vec![
            error("E0277", "hand"),
            error("E0308", "hand"),
            error("E0308", "subject"),
        ]);
        assert_eq!(
            judge(&cell, &verdict, &other_code, &rendered),
            Judgement::Diverges(vec![String::from("missing:error:E0277@subject")])
        );
        let hand_compiles = outcome(vec![error("E0277", "subject")]);
        assert_eq!(
            judge(&cell, &verdict, &hand_compiles, &rendered),
            Judgement::Harness
        );
    }
}
