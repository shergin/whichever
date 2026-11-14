//! What a run prints and the table it writes.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use super::outcome::Judgement;
use super::workspace::Workspace;
use super::{Entry, Status};
use crate::ledger::Known;
use crate::render::show_command;

/// What a run hands its report.
pub struct Report<'a> {
    /// The cells.
    pub entries: &'a [Entry],
    /// How each cell's outcome stands to its verdict, in the same order.
    pub judgements: &'a [Judgement],
    /// The known divergences.
    pub known: &'a [Known],
    /// The workspace, for the paths of the cells' files.
    pub workspace: &'a Workspace,
    /// How the census differs from `census.txt`.
    pub census_differences: &'a [String],
    /// The wall time of each step.
    pub timings: &'a [(String, Duration)],
    /// Whether the run held every suite unfiltered, so that a known line
    /// that excuses nothing is stale.
    pub complete: bool,
    /// Whether the shape step ran. A known line about a shape is judged
    /// stale only when it did, and any other line only when it did not,
    /// since the shape adds findings to the outcomes those lines name.
    pub shape: bool,
}

/// How a cell stands once the ledger is read.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Standing {
    Agrees,
    /// Every finding that makes it diverge is excused, by these lines.
    Known(Vec<usize>),
    /// These findings are excused by no line; the lines are those that
    /// excuse the others.
    Unexcused(Vec<String>, Vec<usize>),
    ModelBroken,
}

impl Standing {
    fn name(&self) -> &'static str {
        match self {
            Standing::Agrees => "agrees",
            Standing::Known(_) => "known",
            Standing::Unexcused(_, _) => "diverges",
            Standing::ModelBroken => "model-broken",
        }
    }
}

impl Report<'_> {
    fn standings(&self) -> Vec<Standing> {
        self.entries
            .iter()
            .zip(self.judgements)
            .map(|(entry, judgement)| match judgement {
                Judgement::Harness => Standing::ModelBroken,
                Judgement::Agrees => Standing::Agrees,
                Judgement::Diverges(findings) => {
                    let mut used = Vec::new();
                    let mut unexcused = Vec::new();
                    for finding in findings {
                        let line = self.known.iter().position(|line| {
                            line.finding == *finding && line.pattern.matches(&entry.cell)
                        });
                        match line {
                            Some(index) => used.push(index),
                            None => unexcused.push(finding.clone()),
                        }
                    }
                    match unexcused.is_empty() {
                        true => Standing::Known(used),
                        false => Standing::Unexcused(unexcused, used),
                    }
                }
            })
            .collect()
    }

    /// Returns the known lines that excuse nothing although their own cell
    /// ran.
    fn stale(&self, standings: &[Standing]) -> Vec<&Known> {
        self.known
            .iter()
            .enumerate()
            .filter(|_| self.complete)
            .filter(|(_, line)| self.shape || !line.finding.starts_with("shape:"))
            .filter(|(index, line)| {
                let ran = self
                    .entries
                    .iter()
                    .any(|entry| entry.cell == line.pattern.cell);
                let used = standings.iter().any(|standing| match standing {
                    Standing::Known(lines) | Standing::Unexcused(_, lines) => lines.contains(index),
                    Standing::Agrees | Standing::ModelBroken => false,
                });
                ran && !used
            })
            .map(|(_, line)| line)
            .collect()
    }

    /// Prints the summary and every divergence, and returns how the run
    /// ended.
    pub fn print(&self) -> Status {
        let standings = self.standings();
        let mut verdicts: BTreeMap<&str, usize> = BTreeMap::new();
        let mut outcomes: BTreeMap<String, usize> = BTreeMap::new();
        let mut suites: BTreeMap<&str, usize> = BTreeMap::new();
        let mut counted: BTreeMap<&str, usize> = BTreeMap::new();
        for (entry, standing) in self.entries.iter().zip(&standings) {
            *verdicts.entry(entry.verdict.kind()).or_insert(0) += 1;
            let kind = match entry.outcome.findings.first() {
                None => String::from("pass"),
                Some(_) => {
                    let kinds: Vec<String> = entry
                        .outcome
                        .findings
                        .iter()
                        .map(|finding| {
                            let spelled = finding.spell();
                            let kind = spelled.split(':').next().unwrap_or_default();
                            String::from(kind)
                        })
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .collect();
                    kinds.join("+")
                }
            };
            *outcomes.entry(kind).or_insert(0) += 1;
            for suite in &entry.suites {
                *suites.entry(suite.as_str()).or_insert(0) += 1;
            }
            *counted.entry(standing.name()).or_insert(0) += 1;
        }
        let join = |counts: Vec<String>| counts.join("  ");
        println!("conformance: {} cells", self.entries.len());
        println!(
            "  suites    {}",
            join(
                suites
                    .iter()
                    .map(|(name, count)| format!("{name} {count}"))
                    .collect()
            )
        );
        println!(
            "  verdicts  {}",
            join(
                verdicts
                    .iter()
                    .map(|(name, count)| format!("{name} {count}"))
                    .collect()
            )
        );
        println!(
            "  outcomes  {}",
            join(
                outcomes
                    .iter()
                    .map(|(name, count)| format!("{name} {count}"))
                    .collect()
            )
        );
        println!(
            "  standing  {}",
            join(
                counted
                    .iter()
                    .map(|(name, count)| format!("{name} {count}"))
                    .collect()
            )
        );
        println!(
            "  time      {}",
            join(
                self.timings
                    .iter()
                    .map(|(step, time)| format!("{step} {:.1}s", time.as_secs_f64()))
                    .collect()
            )
        );

        let mut status = Status::Clean;
        for (entry, standing) in self.entries.iter().zip(&standings) {
            let note = match standing {
                Standing::Agrees | Standing::Known(_) => continue,
                Standing::Unexcused(findings, _) => {
                    format!("diverged; no known line excuses {}", findings.join(", "))
                }
                Standing::ModelBroken => {
                    String::from("the hand does not compile or fails its law: the model is wrong")
                }
            };
            status = match (status, standing) {
                (_, Standing::ModelBroken) | (Status::ModelBroken, _) => Status::ModelBroken,
                (Status::Clean | Status::Diverged, _) => Status::Diverged,
            };
            println!();
            println!("{}: `{}`", standing.name(), entry.id);
            println!("  note      {note}");
            println!("  verdict   {}", entry.verdict.describe());
            println!("  outcome   {}", entry.outcome.spell());
            if let Some(message) = &entry.outcome.first_message {
                println!("  message   {message}");
            }
            println!(
                "  file      {}",
                self.workspace
                    .cell_path(&entry.shard, &entry.module)
                    .display()
            );
            println!("  show      {}", show_command(&entry.id));
        }
        for line in self.stale(&standings) {
            status = match status {
                Status::ModelBroken => Status::ModelBroken,
                Status::Clean | Status::Diverged => Status::Diverged,
            };
            println!();
            println!("stale: known.tsv:{} `{}`", line.line, line.pattern.text);
            println!("  note      excuses no divergence of this run; remove or update the line");
            println!("  finding   {}", line.finding);
        }
        if !self.census_differences.is_empty() {
            status = match status {
                Status::ModelBroken => Status::ModelBroken,
                Status::Clean | Status::Diverged => Status::Diverged,
            };
            println!();
            println!(
                "census.txt differs from the model; rerun with `--bless` if the change is meant:"
            );
            for line in self.census_differences {
                println!("  {line}");
            }
        }
        status
    }

    /// Writes the full table, one tab-separated line per cell.
    pub fn write_table(&self, path: &Path) -> std::io::Result<()> {
        let standings = self.standings();
        let mut text = String::from(
            "id\tmodule\tsuites\tverdict\toutcome\tstanding\tmessage\tfile\tdivergent\n",
        );
        for ((entry, standing), judgement) in
            self.entries.iter().zip(&standings).zip(self.judgements)
        {
            let divergent = match judgement {
                Judgement::Diverges(findings) => findings.join(", "),
                Judgement::Agrees | Judgement::Harness => String::new(),
            };
            let message = entry
                .outcome
                .first_message
                .as_deref()
                .unwrap_or_default()
                .replace('\t', " ");
            text.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                entry.id,
                entry.module,
                entry.suites.join(","),
                entry.verdict.describe(),
                entry.outcome.spell(),
                standing.name(),
                message,
                self.workspace
                    .cell_path(&entry.shard, &entry.module)
                    .display(),
                divergent
            ));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)
    }
}
