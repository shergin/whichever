//! The mutants: small bugs injected into the macros one at a time, each
//! judged by whether the matrix notices it.
//!
//! A mutant is a file of `conformance/mutants/`, `<name>.mutant`, holding a
//! source file of the crate under test, a text that occurs in it exactly
//! once and what replaces it, the promise the change breaks, and whether
//! the matrix is expected to notice. A text that must occur exactly once is
//! the point of the format: when the macros move on, the mutant no longer
//! applies and says so, instead of silently mutating something else.
//!
//! A mutant expected to be killed names its witnesses, cells that agree on
//! the unmodified crate and diverge under the change. By default the
//! command runs the witnesses of the mutants asked for once on the
//! unmodified crate, then each mutant's own on a copy of the crate under
//! test with the change applied, as `run --cell ID --whichever PATH` does:
//! a mutant is killed when a witness that agreed now diverges. With
//! `--full` it runs the matrix, or the suites asked for, once on the
//! unmodified crate and once per mutant, and a mutant is killed when a cell
//! that agreed in the baseline diverges, or a line of `known.tsv` goes
//! stale: how witnesses are found, and how widely a mutant is noticed.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use crate::cell::Cell;
use crate::ledger::{self, Files, Known};
use crate::run::{self, cargo, Options, RunError, Status};

/// What a mutant's author expects of the matrix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expect {
    /// The matrix notices the change.
    Killed,
    /// The matrix does not notice it, for the reason given: what the suite
    /// lacks, in one sentence.
    Survives(String),
}

/// One small change to a source file of the crate under test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mutant {
    /// The file's name without `.mutant`.
    pub name: String,
    /// The source file it changes, relative to the repository's root.
    pub file: String,
    /// The promise it breaks and the document that makes it.
    pub breaks: String,
    /// Whether the matrix is expected to notice.
    pub expect: Expect,
    /// The text to find, which occurs exactly once in the file.
    pub find: String,
    /// What replaces it.
    pub replace: String,
    /// The cells that agree on the unmodified crate and diverge under the
    /// change, by id; none for a mutant expected to survive.
    pub witnesses: Vec<String>,
}

/// Returns the lines from `lines` up to the next line that is exactly
/// `end`, or to the last line, joined with their line breaks but for the
/// last.
fn block<'a>(lines: &mut impl Iterator<Item = &'a str>, end: Option<&str>) -> (String, bool) {
    let mut taken: Vec<&str> = Vec::new();
    for line in lines.by_ref() {
        if Some(line) == end {
            return (taken.join("\n"), true);
        }
        taken.push(line);
    }
    (taken.join("\n"), false)
}

impl Mutant {
    /// Parses a mutant file. The header is `key: value` lines, `file`,
    /// `breaks`, `expect` and, after `expect: survives`, `reason`, with `#`
    /// comments and blank lines, and after `expect: killed` any number of
    /// `witness:` lines, one cell id each, spelled as the tool spells it;
    /// then a line `find:` opens the text to
    /// find, a line `replace:` ends it and opens its replacement, which runs
    /// to the end of the file. Both texts keep their lines as written,
    /// without the line break of their last line.
    pub fn parse(name: &str, text: &str) -> Result<Mutant, String> {
        let mut lines = text.lines();
        let mut header: BTreeMap<&str, String> = BTreeMap::new();
        let mut witnesses: Vec<String> = Vec::new();
        let mut opened = false;
        for line in lines.by_ref() {
            if line == "find:" {
                opened = true;
                break;
            }
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once(':') else {
                return Err(format!("`{line}` is no `key: value` line"));
            };
            let key = key.trim();
            if key == "witness" {
                let id = value.trim();
                let cell = Cell::parse(id).map_err(|error| format!("witness `{id}`: {error}"))?;
                if cell.id() != id {
                    return Err(format!("witness `{id}` is spelled `{}`", cell.id()));
                }
                if witnesses.iter().any(|witness| witness == id) {
                    return Err(format!("witness `{id}` is given twice"));
                }
                witnesses.push(String::from(id));
                continue;
            }
            if !["file", "breaks", "expect", "reason"].contains(&key) {
                return Err(format!(
                    "no key `{key}`; the keys are `file`, `breaks`, `expect`, `reason` and `witness`"
                ));
            }
            if header.insert(key, String::from(value.trim())).is_some() {
                return Err(format!("`{key}` is given twice"));
            }
        }
        if !opened {
            return Err(String::from("no `find:` line opens the text to find"));
        }
        let (find, closed) = block(&mut lines, Some("replace:"));
        if !closed {
            return Err(String::from(
                "no `replace:` line ends the text to find and opens its replacement",
            ));
        }
        let (replace, _) = block(&mut lines, None);
        if find.trim().is_empty() {
            return Err(String::from("the text to find is empty"));
        }
        if find == replace {
            return Err(String::from("the replacement is the text it replaces"));
        }
        let mut take = |key: &str| {
            header
                .remove(key)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("`{key}:` is missing or empty"))
        };
        let file = take("file")?;
        let breaks = take("breaks")?;
        let expect = match take("expect")?.as_str() {
            "killed" => {
                if header.contains_key("reason") {
                    return Err(String::from("`reason:` goes with `expect: survives` alone"));
                }
                Expect::Killed
            }
            "survives" => {
                if !witnesses.is_empty() {
                    return Err(String::from(
                        "a mutant expected to survive has no witness, which would kill it",
                    ));
                }
                let reason = take("reason")?;
                if !reason.ends_with('.') {
                    return Err(String::from(
                        "the reason is one sentence ending in a period",
                    ));
                }
                Expect::Survives(reason)
            }
            other => {
                return Err(format!(
                    "`expect: {other}` is neither `killed` nor `survives`"
                ))
            }
        };
        if !breaks.ends_with('.') {
            return Err(String::from("`breaks:` is one sentence ending in a period"));
        }
        let under_test = file.starts_with("src/") || file.starts_with("macros/src/");
        if !under_test || file.contains("..") {
            return Err(format!(
                "`{file}` is no source file of the crate under test, under `src/` or `macros/src/`"
            ));
        }
        Ok(Mutant {
            name: String::from(name),
            file,
            breaks,
            expect,
            find,
            replace,
            witnesses,
        })
    }

    /// Returns `source` with the change applied, or how many times the text
    /// to find occurs when that is not exactly once.
    pub fn apply(&self, source: &str) -> Result<String, usize> {
        let occurrences = source.matches(self.find.as_str()).count();
        if occurrences != 1 {
            return Err(occurrences);
        }
        Ok(source.replacen(self.find.as_str(), &self.replace, 1))
    }

    /// Returns the sentence saying that the mutant no longer applies.
    pub fn stale(&self, occurrences: usize) -> String {
        format!(
            "the text to find of mutant `{}` occurs {occurrences} times in `{}`, not once; update the mutant or delete it",
            self.name, self.file
        )
    }
}

/// Returns the directory of the checked-in mutants.
pub fn directory(repo: &Path) -> PathBuf {
    repo.join("conformance").join("mutants")
}

/// Reads every mutant of `directory`, sorted by name.
pub fn load(directory: &Path) -> Result<Vec<Mutant>, String> {
    let entries =
        fs::read_dir(directory).map_err(|error| format!("`{}`: {error}", directory.display()))?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "mutant")
        {
            paths.push(path);
        }
    }
    paths.sort();
    let mut mutants = Vec::new();
    for path in paths {
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let kebab = !name.is_empty()
            && name.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
            });
        if !kebab {
            return Err(format!("`{}` is not named in kebab case", path.display()));
        }
        let text =
            fs::read_to_string(&path).map_err(|error| format!("`{}`: {error}", path.display()))?;
        let mutant =
            Mutant::parse(name, &text).map_err(|error| format!("`{}`: {error}", path.display()))?;
        mutants.push(mutant);
    }
    Ok(mutants)
}

/// Returns a manifest of the crate under test that stands alone: without
/// the sections a member of the repository's workspace has and a path
/// dependency needs not, the workspace's table, the profiles and the
/// development dependencies, and with an empty workspace of its own, so
/// that cargo looks no further up for one.
pub fn standalone_manifest(manifest: &str) -> String {
    let mut output = String::new();
    let mut kept = true;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            let section = trimmed.trim_start_matches('[').trim_end_matches(']');
            kept = !(section == "workspace"
                || section.starts_with("workspace.")
                || section.starts_with("profile.")
                || section == "dev-dependencies"
                || section.starts_with("dev-dependencies."));
        }
        if kept {
            output.push_str(line);
            output.push('\n');
        }
    }
    output.push_str(
        "\n# Written by the conformance matrix's mutants: a crate of its own.\n[workspace]\n",
    );
    output
}

fn copy_directory(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let destination = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&path, &destination)?;
        } else {
            fs::copy(&path, &destination)?;
        }
    }
    Ok(())
}

/// Copies the crate under test, the facade's manifest and sources and the
/// macro crate's, from `repo` to `destination`, which is emptied first.
pub fn copy_crate(repo: &Path, destination: &Path) -> std::io::Result<()> {
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    fs::create_dir_all(destination.join("macros"))?;
    let manifest = fs::read_to_string(repo.join("Cargo.toml"))?;
    fs::write(
        destination.join("Cargo.toml"),
        standalone_manifest(&manifest),
    )?;
    let lock = repo.join("Cargo.lock");
    if lock.exists() {
        fs::copy(&lock, destination.join("Cargo.lock"))?;
    }
    copy_directory(&repo.join("src"), &destination.join("src"))?;
    // The macro crate's manifest has no section of the workspace's and is
    // copied as it is; the facade's workspace holds it as a path
    // dependency.
    fs::copy(
        repo.join("macros").join("Cargo.toml"),
        destination.join("macros").join("Cargo.toml"),
    )?;
    copy_directory(
        &repo.join("macros").join("src"),
        &destination.join("macros").join("src"),
    )
}

/// A row of the table a run writes, as far as the mutants read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The cell's id.
    pub id: String,
    /// How the cell stands once the ledger is read: `agrees`, `known`,
    /// `diverges` or `model-broken`.
    pub standing: String,
    /// The findings that make the cell diverge; for a broken model, every
    /// finding of the outcome.
    pub findings: Vec<String>,
    /// The first message of the outcome.
    pub message: String,
}

/// Parses the table a run writes to `target/conformance/report.tsv`.
pub fn parse_table(text: &str) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate().skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        let [id, _, _, _, outcome, standing, message, _, divergent] = fields.as_slice() else {
            return Err(format!(
                "report.tsv:{}: expected nine tab-separated fields; found {}",
                index + 1,
                fields.len()
            ));
        };
        let listed = match *standing {
            "model-broken" => outcome,
            _ => divergent,
        };
        let findings = listed
            .split(", ")
            .filter(|finding| !finding.is_empty() && *finding != "pass")
            .map(String::from)
            .collect();
        rows.push(Row {
            id: String::from(*id),
            standing: String::from(*standing),
            findings,
            message: String::from(*message),
        });
    }
    Ok(rows)
}

/// Returns the lines of `known` that excuse no divergence of `rows`, as a
/// run judges them: only in a complete run, and a line about a shape only
/// when the shape was compared. A line is judged whether or not the cell
/// its pattern spells ran, since a pattern may spell a cell the model never
/// selects.
pub fn stale_lines<'a>(
    known: &'a [Known],
    rows: &[Row],
    complete: bool,
    shape: bool,
) -> Result<Vec<&'a Known>, String> {
    if !complete {
        return Ok(Vec::new());
    }
    let mut used = BTreeSet::new();
    for row in rows {
        let cell = Cell::parse(&row.id).map_err(|error| error.to_string())?;
        if row.standing != "known" && row.standing != "diverges" {
            continue;
        }
        for finding in &row.findings {
            let line = known
                .iter()
                .position(|line| line.finding == *finding && line.pattern.matches(&cell));
            if let Some(index) = line {
                used.insert(index);
            }
        }
    }
    Ok(known
        .iter()
        .enumerate()
        .filter(|(_, line)| shape || !line.finding.starts_with("shape:"))
        .filter(|(index, _)| !used.contains(index))
        .map(|(_, line)| line)
        .collect())
}

/// Returns the check a finding comes from: the verdict, the law or the
/// shape, or the hand where the model broke.
fn check_of(row: &Row, finding: &str) -> &'static str {
    if row.standing == "model-broken" {
        return "hand";
    }
    if finding.starts_with("law:") {
        return "law";
    }
    if finding.starts_with("shape:") {
        return "shape";
    }
    "verdict"
}

/// What a mutant did to the matrix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fate {
    /// The run diverged because of it.
    Killed {
        /// The checks that caught it, sorted.
        checks: Vec<String>,
        /// How many cells newly diverge.
        cells: usize,
        /// The first witness that diverges, in a run of witnesses.
        witness: Option<String>,
        /// One of the cells, with its finding, or the stale line.
        example: String,
    },
    /// No cell that agreed in the baseline diverges, and no known line
    /// goes stale; some cells may newly diverge where `known.tsv` excuses
    /// them, which a line wider than its defect does.
    Survived {
        /// How many cells that agreed in the baseline are excused now.
        excused: usize,
        /// One of them, with its finding.
        example: String,
    },
    /// A mutant expected to survive, which a run of witnesses does not
    /// judge: it has none, and only `--full` can show that it survives.
    Unjudged,
    /// Its text to find does not occur exactly once.
    Stale(String),
    /// The changed crate does not compile.
    Uncompiled(String),
    /// The run could not judge.
    Harness(String),
}

impl Fate {
    fn word(&self) -> &'static str {
        match self {
            Fate::Killed { .. } => "killed",
            Fate::Survived { .. } => "survived",
            Fate::Unjudged => "not judged",
            Fate::Stale(_) => "stale",
            Fate::Uncompiled(_) => "uncompiled",
            Fate::Harness(_) => "harness",
        }
    }

    /// Returns whether the result is what `expect` says, or says nothing
    /// against it.
    fn meets(&self, expect: &Expect) -> bool {
        matches!(
            (self, expect),
            (Fate::Killed { .. }, Expect::Killed)
                | (Fate::Survived { .. } | Fate::Unjudged, Expect::Survives(_))
        )
    }
}

/// Compares a mutant's run with the baseline's.
pub fn compare(baseline: &[Row], mutated: &[Row], stale: &[&Known]) -> Fate {
    let before: BTreeMap<&str, &Row> = baseline.iter().map(|row| (row.id.as_str(), row)).collect();
    let mut checks = BTreeSet::new();
    let mut cells = 0;
    let mut example = None;
    let mut excused = 0;
    let mut excused_example = String::new();
    for row in mutated {
        let earlier = before.get(row.id.as_str());
        let new: Vec<&String> = row
            .findings
            .iter()
            .filter(|finding| earlier.is_none_or(|earlier| !earlier.findings.contains(finding)))
            .collect();
        if row.standing == "known" && !new.is_empty() {
            excused += 1;
            if excused_example.is_empty() {
                excused_example = format!("`{}` {}: {}", row.id, new[0], row.message);
            }
        }
        if row.standing != "diverges" && row.standing != "model-broken" {
            continue;
        }
        cells += 1;
        for finding in &new {
            checks.insert(check_of(row, finding));
        }
        if new.is_empty() {
            checks.insert(check_of(row, ""));
        }
        if example.is_none() {
            let finding = new.first().map_or("", |finding| finding.as_str());
            example = Some(format!("`{}` {finding}: {}", row.id, row.message));
        }
    }
    for line in stale {
        checks.insert("stale");
        if example.is_none() {
            example = Some(format!(
                "known.tsv:{} `{}` {} is stale",
                line.line, line.pattern.text, line.finding
            ));
        }
    }
    match example {
        None => Fate::Survived {
            excused,
            example: excused_example,
        },
        Some(example) => Fate::Killed {
            checks: checks.into_iter().map(String::from).collect(),
            cells,
            witness: None,
            example,
        },
    }
}

/// Returns why the baseline of a run of witnesses measures nothing for
/// `mutant`: a witness missing from it, or one that does not agree on the
/// unmodified crate, which a witness has to.
pub fn baseline_problem(mutant: &Mutant, baseline: &[Row]) -> Option<String> {
    for witness in &mutant.witnesses {
        let Some(row) = baseline.iter().find(|row| row.id == *witness) else {
            return Some(format!(
                "mutant `{}`: witness `{witness}` did not run in the baseline",
                mutant.name
            ));
        };
        if row.standing != "agrees" {
            return Some(format!(
                "mutant `{}`: witness `{witness}` {} on the unmodified crate, so it measures nothing; find another with `--full`",
                mutant.name, row.standing
            ));
        }
    }
    None
}

/// Judges a mutant by its witnesses' run on the changed crate: killed when
/// one of them, all of which agreed in the baseline, diverges now.
pub fn judge_witnesses(mutant: &Mutant, baseline: &[Row], mutated: &[Row]) -> Fate {
    let witnessed: Vec<Row> = mutated
        .iter()
        .filter(|row| mutant.witnesses.contains(&row.id))
        .cloned()
        .collect();
    match compare(baseline, &witnessed, &[]) {
        Fate::Killed {
            checks,
            cells,
            example,
            ..
        } => {
            let witness = mutant
                .witnesses
                .iter()
                .find(|witness| {
                    witnessed.iter().any(|row| {
                        row.id == **witness
                            && (row.standing == "diverges" || row.standing == "model-broken")
                    })
                })
                .cloned();
            Fate::Killed {
                checks,
                cells,
                witness,
                example,
            }
        }
        fate => fate,
    }
}

/// Returns the mutants of shard `index` of `count`, counted from one: every
/// `count`-th of the sorted list, starting at the `index`-th.
pub fn shard(mutants: Vec<Mutant>, index: usize, count: usize) -> Vec<Mutant> {
    mutants
        .into_iter()
        .enumerate()
        .filter(|(position, _)| position % count == index - 1)
        .map(|(_, mutant)| mutant)
        .collect()
}

/// Returns the mutants expected to be killed that name no witness, which
/// a run of witnesses cannot judge.
pub fn without_witnesses(mutants: &[Mutant]) -> Vec<&Mutant> {
    mutants
        .iter()
        .filter(|mutant| mutant.expect == Expect::Killed && mutant.witnesses.is_empty())
        .collect()
}

/// The artifacts under a target directory, four levels deep:
/// `debug/build/<package>/<unit>`, `debug/incremental/<unit>` and the like.
fn artifacts(target: &Path) -> BTreeSet<PathBuf> {
    let mut found = BTreeSet::new();
    let mut level = vec![target.to_path_buf()];
    for _ in 0..4 {
        let mut next = Vec::new();
        for directory in &level {
            let Ok(entries) = fs::read_dir(directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                found.insert(path.clone());
                if path.is_dir() {
                    next.push(path);
                }
            }
        }
        level = next;
    }
    found
}

/// Removes the artifacts a mutant's run added beside `before`: they are
/// built against the copy, whose path is in every hash, and are never used
/// again, while a full set of them is gigabytes.
fn prune(target: &Path, before: &BTreeSet<PathBuf>) {
    for path in artifacts(target).difference(before) {
        if path
            .parent()
            .is_some_and(|parent| !before.contains(parent) && parent != target)
        {
            continue;
        }
        let _ = match path.is_dir() {
            true => fs::remove_dir_all(path),
            false => fs::remove_file(path),
        };
    }
}

/// Checks that the copy at `copy` compiles without a warning, building it
/// in `target`, and returns the compiler's first error or warning when it
/// does not. A warning counts: the run reads every diagnostic, and one in
/// the macro crate belongs to no cell.
fn compiles(copy: &Path, target: &Path) -> Result<(), String> {
    let output = cargo::command(copy, target)
        .args(["check", "--quiet", "--message-format=short"])
        .output()
        .map_err(|error| error.to_string())?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let first = stderr
        .lines()
        .find(|line| line.contains("error") || line.contains("warning"));
    match (output.status.success(), first) {
        (true, None) => Ok(()),
        (_, Some(first)) => Err(String::from(first)),
        (false, None) => Err(String::from("cargo check failed")),
    }
}

/// What the command line asks of the mutants.
#[derive(Clone, Debug, Default)]
struct Asked {
    only: Vec<String>,
    /// Run the matrix, or the suites asked for, per mutant, instead of its
    /// witnesses.
    full: bool,
    /// The shard of the sorted mutants to run, counted from one, and the
    /// number of shards.
    shard: Option<(usize, usize)>,
    suites: Vec<String>,
    shape: bool,
    /// The table of a run on the unmodified crate, with the same suites
    /// and shape, to compare against instead of running the baseline: so
    /// that jobs running one mutant each share one baseline.
    baseline: Option<PathBuf>,
}

/// Parses `I/N`, a shard counted from one of `N`.
fn parse_shard(text: &str) -> Result<(usize, usize), RunError> {
    let invalid = || {
        RunError::Usage(format!(
            "`--shard {text}` is no shard: write `I/N`, with `I` from 1 to `N`"
        ))
    };
    let (index, count) = text.split_once('/').ok_or_else(invalid)?;
    let index: usize = index.parse().map_err(|_| invalid())?;
    let count: usize = count.parse().map_err(|_| invalid())?;
    if count == 0 || index == 0 || index > count {
        return Err(invalid());
    }
    Ok((index, count))
}

fn asked(arguments: &[String]) -> Result<Asked, RunError> {
    let mut asked = Asked {
        shape: true,
        ..Asked::default()
    };
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        let mut value = |flag: &str| {
            rest.next()
                .cloned()
                .ok_or_else(|| RunError::Usage(format!("`{flag}` takes a value")))
        };
        match argument.as_str() {
            "--only" => asked.only.push(value("--only")?),
            "--full" => asked.full = true,
            "--shard" => asked.shard = Some(parse_shard(&value("--shard")?)?),
            "--suite" => asked.suites.push(value("--suite")?),
            "--no-shape" => asked.shape = false,
            "--baseline" => asked.baseline = Some(PathBuf::from(value("--baseline")?)),
            other => return Err(RunError::Usage(format!("unknown option `{other}`"))),
        }
    }
    let wide = !asked.suites.is_empty() || !asked.shape || asked.baseline.is_some();
    if wide && !asked.full {
        return Err(RunError::Usage(String::from(
            "`--suite`, `--no-shape` and `--baseline` go with `--full`; a run of witnesses runs each mutant's witnesses with the shape",
        )));
    }
    Ok(asked)
}

fn harness(error: impl core::fmt::Display) -> RunError {
    RunError::Harness(error.to_string())
}

fn progress(message: &str) {
    eprintln!("conformance: {message}");
}

/// Runs the matrix with `options` and returns its table.
fn judged(options: &Options, table: &Path) -> Result<(Status, Vec<Row>), RunError> {
    let status = run::run(options)?;
    let text = fs::read_to_string(table).map_err(harness)?;
    let rows = parse_table(&text).map_err(RunError::Harness)?;
    Ok((status, rows))
}

/// A line of the table the command prints and writes.
struct Line<'a> {
    mutant: &'a Mutant,
    result: Fate,
    seconds: f64,
}

/// Returns the table of `lines`: its sixth column is the witness that
/// killed each mutant in a run of witnesses, and how many cells newly
/// diverge in a full run.
fn table(lines: &[Line], full: bool) -> String {
    let column = match full {
        true => "cells",
        false => "witness",
    };
    let mut text = format!("mutant\tfile\tresult\texpected\tchecks\t{column}\tseconds\texample\n");
    for line in lines {
        let expected = match &line.mutant.expect {
            Expect::Killed => "killed",
            Expect::Survives(_) => "survives",
        };
        let (checks, sixth, example) = match &line.result {
            Fate::Killed {
                checks,
                cells,
                witness,
                example,
            } => {
                let sixth = match (full, witness) {
                    (false, Some(witness)) => witness.clone(),
                    (true, _) | (false, None) => cells.to_string(),
                };
                (checks.join(","), sixth, example.clone())
            }
            Fate::Survived { excused, example } => {
                let example = match excused {
                    0 => String::new(),
                    _ => format!("{excused} cells newly excused by known.tsv, such as {example}"),
                };
                let sixth = match full {
                    true => String::from("0"),
                    false => String::new(),
                };
                (String::new(), sixth, example)
            }
            Fate::Unjudged => (
                String::new(),
                String::new(),
                String::from("a mutant expected to survive is judged by `--full` alone"),
            ),
            Fate::Stale(message) | Fate::Uncompiled(message) | Fate::Harness(message) => {
                (String::new(), String::new(), message.clone())
            }
        };
        text.push_str(&format!(
            "{}\t{}\t{}\t{expected}\t{checks}\t{sixth}\t{:.0}\t{}\n",
            line.mutant.name,
            line.mutant.file,
            line.result.word(),
            line.seconds,
            example.replace(['\t', '\n'], " ")
        ));
    }
    text
}

/// Runs `mutants [--only NAME]... [--shard I/N] [--full [--suite NAME]...
/// [--no-shape] [--baseline PATH]]`: exits 0 when every mutant's result is
/// what it expects, 1 when one expected to be killed survived or one
/// expected to survive was killed, and 2 on a usage or harness error, a
/// stale or uncompiled mutant, a mutant without a witness and a witness
/// that does not agree on the unmodified crate included.
pub fn command(arguments: &[String]) -> Result<ExitCode, RunError> {
    let started = Instant::now();
    let asked = asked(arguments)?;
    let repo = run::repository();
    let mut mutants = load(&directory(&repo)).map_err(RunError::Harness)?;
    for name in &asked.only {
        if !mutants.iter().any(|mutant| &mutant.name == name) {
            return Err(RunError::Usage(format!("no mutant `{name}`")));
        }
    }
    if !asked.only.is_empty() {
        mutants.retain(|mutant| asked.only.contains(&mutant.name));
    }
    if let Some((index, count)) = asked.shard {
        mutants = shard(mutants, index, count);
    }
    let mut changed = Vec::new();
    for mutant in &mutants {
        let source = fs::read_to_string(repo.join(&mutant.file)).map_err(|error| {
            harness(format!(
                "mutant `{}`: `{}`: {error}",
                mutant.name, mutant.file
            ))
        })?;
        match mutant.apply(&source) {
            Ok(source) => changed.push(Some(source)),
            Err(occurrences) => {
                println!("stale: {}", mutant.stale(occurrences));
                changed.push(None);
            }
        }
    }
    if changed.iter().any(Option::is_none) {
        return Err(RunError::Harness(String::from(
            "a mutant no longer applies; it is named above",
        )));
    }

    let base = repo.join("target").join("conformance");
    let report = base.join("report.tsv");
    let check_target = base.join("mutant-check");
    let files = Files::at(&repo.join("conformance"));
    let known_text = fs::read_to_string(&files.known).unwrap_or_default();
    let known = ledger::parse_known(&known_text).map_err(harness)?;
    let complete = asked.suites.is_empty();
    let options = Options {
        suites: asked.suites.clone(),
        shape: asked.shape,
        quiet: true,
        ..Options::default()
    };

    let baseline = match (asked.full, &asked.baseline) {
        (false, _) => witness_baseline(&mutants, &report)?,
        (true, Some(path)) => {
            progress(&format!("the baseline, read from `{}`", path.display()));
            let text = fs::read_to_string(path)
                .map_err(|error| harness(format!("`{}`: {error}", path.display())))?;
            let rows = parse_table(&text).map_err(RunError::Harness)?;
            // A cell a `known.tsv` line excuses agrees as far as a run is
            // concerned; a stale line or a moved census shows in the run's
            // exit, which the baseline's job checks.
            let agrees = |row: &Row| row.standing == "agrees" || row.standing == "known";
            if let Some(row) = rows.iter().find(|row| !agrees(row)) {
                return Err(RunError::Harness(format!(
                    "the baseline does not agree, so it measures nothing: `{}` {}",
                    row.id, row.standing
                )));
            }
            rows
        }
        (true, None) => {
            progress("the baseline, on the unmodified crate");
            let (status, rows) = judged(&options, &report)?;
            if status != Status::Clean {
                return Err(RunError::Harness(String::from(
                    "the baseline run does not agree, so it measures nothing; its divergences are in target/conformance/report.tsv",
                )));
            }
            rows
        }
    };
    let mut lines: Vec<Line> = Vec::new();
    for (index, (mutant, source)) in mutants.iter().zip(changed).enumerate() {
        let step = Instant::now();
        progress(&format!(
            "mutant {}/{}, `{}`",
            index + 1,
            mutants.len(),
            mutant.name
        ));
        if !asked.full && mutant.witnesses.is_empty() {
            lines.push(Line {
                mutant,
                result: Fate::Unjudged,
                seconds: 0.0,
            });
            continue;
        }
        let copy = base.join("mutants").join(&mutant.name);
        copy_crate(&repo, &copy).map_err(harness)?;
        let source = source.expect("every mutant applies");
        fs::write(copy.join(&mutant.file), source).map_err(harness)?;
        let result = match compiles(&copy, &check_target) {
            Err(error) => Fate::Uncompiled(error),
            Ok(()) => {
                let target = base.join("target");
                let before = artifacts(&target);
                let options = match asked.full {
                    true => Options {
                        whichever: Some(copy.clone()),
                        ..options.clone()
                    },
                    false => Options {
                        whichever: Some(copy.clone()),
                        cells: mutant.witnesses.clone(),
                        shape: true,
                        quiet: true,
                        ..Options::default()
                    },
                };
                let judged = judged(&options, &report);
                prune(&target, &before);
                match judged {
                    Err(RunError::Usage(message)) => return Err(RunError::Usage(message)),
                    Err(RunError::Harness(message)) => Fate::Harness(message),
                    Ok((_, rows)) if !asked.full => judge_witnesses(mutant, &baseline, &rows),
                    Ok((_, rows)) => {
                        let stale = stale_lines(&known, &rows, complete, asked.shape)
                            .map_err(RunError::Harness)?;
                        compare(&baseline, &rows, &stale)
                    }
                }
            }
        };
        progress(&format!("mutant `{}`: {}", mutant.name, result.word()));
        lines.push(Line {
            mutant,
            result,
            seconds: step.elapsed().as_secs_f64(),
        });
    }

    let text = table(&lines, asked.full);
    fs::write(base.join("mutants.tsv"), &text).map_err(harness)?;
    print_table(&text);
    let killed = lines
        .iter()
        .filter(|line| matches!(line.result, Fate::Killed { .. }))
        .count();
    let judged_count = lines
        .iter()
        .filter(|line| matches!(line.result, Fate::Killed { .. } | Fate::Survived { .. }))
        .count();
    println!();
    println!(
        "killed {killed} of {judged_count} mutants that compiled and ran ({:.0}%), in {:.0}s",
        if judged_count == 0 {
            0.0
        } else {
            100.0 * killed as f64 / judged_count as f64
        },
        started.elapsed().as_secs_f64()
    );
    let mut code = ExitCode::SUCCESS;
    let mut broken = false;
    for line in &lines {
        if line.result.meets(&line.mutant.expect) {
            continue;
        }
        let note = match (&line.result, &line.mutant.expect, asked.full) {
            (Fate::Killed { .. }, Expect::Survives(_), _) => {
                "was killed although it expects to survive; update its `expect:` line"
            }
            (Fate::Survived { .. }, Expect::Killed, true) => {
                "survived although it expects to be killed: the matrix no longer notices it"
            }
            (Fate::Survived { .. }, Expect::Killed, false) => {
                "survived its witnesses although it expects to be killed: run it with `--full` to find the cells that kill it, or the matrix lost the check"
            }
            _ => {
                broken = true;
                "could not be judged; fix the mutant or the harness"
            }
        };
        code = ExitCode::from(1);
        println!();
        println!("unexpected: `{}` {}", line.mutant.name, line.result.word());
        println!("  note      {note}");
        println!("  breaks    {}", line.mutant.breaks);
        if !line.mutant.witnesses.is_empty() {
            println!("  witness   {}", line.mutant.witnesses.join(", "));
        }
        if let Expect::Survives(reason) = &line.mutant.expect {
            println!("  reason    {reason}");
        }
    }
    if broken {
        code = ExitCode::from(2);
    }
    Ok(code)
}

/// Runs the witnesses of `mutants` once on the unmodified crate, with the
/// shape, and returns the table: after checking that every mutant expected
/// to be killed names a witness, that every witness is a cell the model
/// selects, and that every one agrees.
fn witness_baseline(mutants: &[Mutant], report: &Path) -> Result<Vec<Row>, RunError> {
    if let Some(mutant) = without_witnesses(mutants).first() {
        return Err(RunError::Harness(format!(
            "mutant `{}` is expected to be killed and names no witness; find one with `mutants --full --only {}` and add a `witness:` line",
            mutant.name, mutant.name
        )));
    }
    let mut union: Vec<String> = Vec::new();
    for mutant in mutants {
        for witness in &mutant.witnesses {
            run::cells(std::slice::from_ref(witness)).map_err(|error| {
                let message = match error {
                    RunError::Usage(message) | RunError::Harness(message) => message,
                };
                RunError::Harness(format!("mutant `{}`: witness {message}", mutant.name))
            })?;
            if !union.contains(witness) {
                union.push(witness.clone());
            }
        }
    }
    if union.is_empty() {
        return Ok(Vec::new());
    }
    progress(&format!(
        "the baseline, {} witnesses on the unmodified crate",
        union.len()
    ));
    let options = Options {
        cells: union,
        shape: true,
        quiet: true,
        ..Options::default()
    };
    let (_, rows) = judged(&options, report)?;
    for mutant in mutants {
        if let Some(problem) = baseline_problem(mutant, &rows) {
            return Err(RunError::Harness(problem));
        }
    }
    Ok(rows)
}

/// Prints the table with its columns aligned, but for the last.
fn print_table(text: &str) {
    let rows: Vec<Vec<&str>> = text
        .lines()
        .map(|line| line.split('\t').collect())
        .collect();
    let columns = rows.first().map_or(0, Vec::len);
    let widths: Vec<usize> = (0..columns)
        .map(|column| {
            rows.iter()
                .map(|row| row.get(column).map_or(0, |cell| cell.len()))
                .max()
                .unwrap_or(0)
        })
        .collect();
    for row in &rows {
        let mut line = String::new();
        for (column, cell) in row.iter().enumerate() {
            if column + 1 == row.len() {
                line.push_str(cell);
            } else {
                line.push_str(&format!("{cell:<width$}  ", width = widths[column]));
            }
        }
        println!("{}", line.trim_end());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MUTANT: &str = "\
# A comment.
file: macros/src/forwarding.rs
breaks: An arm borrows the target, as `docs/routes.md` says.
expect: survives
reason: No cell reaches it.

find:
        a
    b
replace:
    c
";

    #[test]
    fn a_mutant_is_read_with_its_texts_as_written() {
        let mutant = Mutant::parse("drop-borrow", MUTANT).unwrap();
        assert_eq!(mutant.file, "macros/src/forwarding.rs");
        assert_eq!(
            mutant.expect,
            Expect::Survives(String::from("No cell reaches it."))
        );
        assert_eq!(mutant.find, "        a\n    b");
        assert_eq!(mutant.replace, "    c");
    }

    #[test]
    fn a_mutant_without_its_parts_is_refused() {
        let refused = |text: &str| Mutant::parse("m", text).unwrap_err();
        assert!(
            refused(&MUTANT.replace("expect: survives", "expect: killed")).contains("`reason:`")
        );
        assert!(
            refused(&MUTANT.replace("reason: No cell reaches it.\n", "")).contains("`reason:`")
        );
        assert!(refused(&MUTANT.replace("replace:\n", "")).contains("`replace:`"));
        assert!(refused(&MUTANT.replace(
            "file: macros/src/forwarding.rs",
            "file: conformance/src/lib.rs"
        ))
        .contains("crate under test"));
        assert!(refused(&MUTANT.replace("expect: survives", "expect: maybe")).contains("neither"));
        assert!(refused(&MUTANT.replace("# A comment.", "colour: red")).contains("no key"));
    }

    #[test]
    fn a_change_applies_where_its_text_occurs_once() {
        let mutant = Mutant::parse("m", MUTANT).unwrap();
        assert_eq!(
            mutant.apply("x\n        a\n    b\ny").unwrap(),
            "x\n    c\ny"
        );
        assert_eq!(mutant.apply("nothing here"), Err(0));
        assert_eq!(mutant.apply("        a\n    b\n        a\n    b"), Err(2));
    }

    #[test]
    fn a_manifest_stands_alone_without_the_workspace() {
        let manifest = "[package]\nname = \"x\"\n\n[dependencies]\ny = \"1\"\n\n[dev-dependencies]\nz = \"1\"\n\n[workspace]\nmembers = [\n    \"a\",\n]\n\n[profile.bench-lto]\nlto = true\n";
        let standalone = standalone_manifest(manifest);
        assert!(standalone.starts_with("[package]\nname = \"x\"\n\n[dependencies]\ny = \"1\"\n\n"));
        assert!(!standalone.contains("z = "));
        assert!(!standalone.contains("members"));
        assert!(!standalone.contains("lto"));
        assert!(standalone.ends_with("[workspace]\n"));
    }

    #[test]
    fn a_mutant_is_killed_by_a_cell_that_newly_diverges() {
        let row = |id: &str, standing: &str, findings: &[&str]| Row {
            id: String::from(id),
            standing: String::from(standing),
            findings: findings
                .iter()
                .map(|finding| String::from(*finding))
                .collect(),
            message: String::from("m"),
        };
        let baseline = vec![
            row("base", "agrees", &[]),
            row("recv=mut", "known", &["error:E0596@attribute"]),
        ];
        assert_eq!(
            compare(&baseline, &baseline, &[]),
            Fate::Survived {
                excused: 0,
                example: String::new(),
            }
        );
        let excused = vec![
            row("base", "known", &["error:E0596@attribute"]),
            baseline[1].clone(),
        ];
        assert_eq!(
            compare(&baseline, &excused, &[]),
            Fate::Survived {
                excused: 1,
                example: String::from("`base` error:E0596@attribute: m"),
            }
        );
        let mutated = vec![
            row("base", "diverges", &["law:subject"]),
            row(
                "recv=mut",
                "diverges",
                &["error:E0596@attribute", "shape:m"],
            ),
        ];
        assert_eq!(
            compare(&baseline, &mutated, &[]),
            Fate::Killed {
                checks: vec![String::from("law"), String::from("shape")],
                cells: 2,
                witness: None,
                example: String::from("`base` law:subject: m"),
            }
        );
    }

    #[test]
    fn a_known_line_that_excuses_nothing_is_stale_whether_or_not_its_cell_ran() {
        let known = ledger::parse_known(
            "recv=mut\terror:E0596@subject\tA borrow is wrong.\nlift=by-ref\terror:E0308@subject\tA lift is wrong.\n",
        )
        .unwrap();
        let rows = vec![Row {
            id: String::from("recv=mut ret=ref"),
            standing: String::from("known"),
            findings: vec![String::from("error:E0596@subject")],
            message: String::from("m"),
        }];
        let stale = stale_lines(&known, &rows, true, true).unwrap();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].pattern.text, "lift=by-ref");
        assert!(stale_lines(&known, &rows, false, true).unwrap().is_empty());
    }

    const KILLED: &str = "\
file: macros/src/forwarding.rs
breaks: An arm borrows the target, as `docs/routes.md` says.
expect: killed
witness: base
witness: s0=deref-box recv=box

find:
    a
replace:
    b
";

    #[test]
    fn witnesses_are_read_in_order_as_the_tool_spells_them() {
        let mutant = Mutant::parse("m", KILLED).unwrap();
        assert_eq!(mutant.witnesses, vec!["base", "s0=deref-box recv=box"]);
        let refused = |text: &str| Mutant::parse("m", text).unwrap_err();
        assert!(
            refused(&KILLED.replace("s0=deref-box recv=box", "recv=box s0=deref-box"))
                .contains("is spelled `s0=deref-box recv=box`")
        );
        assert!(refused(&KILLED.replace("s0=deref-box recv=box", "base")).contains("twice"));
        assert!(
            refused(&KILLED.replace("witness: base", "witness: recv=boxed"))
                .contains("witness `recv=boxed`")
        );
        let survives = KILLED.replace(
            "expect: killed",
            "expect: survives\nreason: No cell reaches it.",
        );
        assert!(refused(&survives).contains("no witness"));
        assert!(Mutant::parse("m", MUTANT).unwrap().witnesses.is_empty());
    }

    fn row(id: &str, standing: &str, findings: &[&str]) -> Row {
        Row {
            id: String::from(id),
            standing: String::from(standing),
            findings: findings
                .iter()
                .map(|finding| String::from(*finding))
                .collect(),
            message: String::from("m"),
        }
    }

    #[test]
    fn a_mutant_is_killed_by_a_witness_that_newly_diverges() {
        let mutant = Mutant::parse("m", KILLED).unwrap();
        let baseline = vec![
            row("base", "agrees", &[]),
            row("s0=deref-box recv=box", "agrees", &[]),
            row("recv=mut", "agrees", &[]),
        ];
        assert_eq!(baseline_problem(&mutant, &baseline), None);
        assert_eq!(
            judge_witnesses(&mutant, &baseline, &baseline),
            Fate::Survived {
                excused: 0,
                example: String::new(),
            }
        );
        // A cell that is no witness of the mutant does not count.
        let elsewhere = vec![
            row("base", "agrees", &[]),
            row("recv=mut", "diverges", &["law:subject"]),
        ];
        assert!(matches!(
            judge_witnesses(&mutant, &baseline, &elsewhere),
            Fate::Survived { .. }
        ));
        let mutated = vec![
            row("base", "agrees", &[]),
            row("s0=deref-box recv=box", "diverges", &["shape:m"]),
        ];
        assert_eq!(
            judge_witnesses(&mutant, &baseline, &mutated),
            Fate::Killed {
                checks: vec![String::from("shape")],
                cells: 1,
                witness: Some(String::from("s0=deref-box recv=box")),
                example: String::from("`s0=deref-box recv=box` shape:m: m"),
            }
        );
    }

    #[test]
    fn a_witness_that_does_not_agree_in_the_baseline_measures_nothing() {
        let mutant = Mutant::parse("m", KILLED).unwrap();
        let diverging = vec![
            row("base", "agrees", &[]),
            row("s0=deref-box recv=box", "diverges", &["law:subject"]),
        ];
        let problem = baseline_problem(&mutant, &diverging).unwrap();
        assert!(
            problem.contains("`s0=deref-box recv=box` diverges"),
            "{problem}"
        );
        let excused = vec![
            row("base", "known", &["law:subject"]),
            row("s0=deref-box recv=box", "agrees", &[]),
        ];
        assert!(baseline_problem(&mutant, &excused)
            .unwrap()
            .contains("`base` known"));
        let missing = vec![row("base", "agrees", &[])];
        assert!(baseline_problem(&mutant, &missing)
            .unwrap()
            .contains("did not run"));
    }

    #[test]
    fn a_shard_takes_every_nth_mutant_from_its_own() {
        let mutants: Vec<Mutant> = (0..7)
            .map(|index| Mutant {
                name: format!("m{index}"),
                ..Mutant::parse("m", KILLED).unwrap()
            })
            .collect();
        let names = |index, count| -> Vec<String> {
            shard(mutants.clone(), index, count)
                .into_iter()
                .map(|mutant| mutant.name)
                .collect()
        };
        assert_eq!(names(1, 3), vec!["m0", "m3", "m6"]);
        assert_eq!(names(2, 3), vec!["m1", "m4"]);
        assert_eq!(names(3, 3), vec!["m2", "m5"]);
        assert_eq!(names(1, 1).len(), 7);
        assert_eq!(parse_shard("2/4").unwrap(), (2, 4));
        for wrong in ["0/4", "5/4", "1/0", "4", "a/b"] {
            assert!(parse_shard(wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn a_run_of_witnesses_takes_no_option_of_a_full_run() {
        let arguments = |list: &[&str]| -> Vec<String> {
            list.iter()
                .map(|argument| String::from(*argument))
                .collect()
        };
        assert!(asked(&arguments(&["--suite", "pinned"])).is_err());
        assert!(asked(&arguments(&["--no-shape"])).is_err());
        assert!(asked(&arguments(&["--full", "--suite", "pinned"])).is_ok());
        let shard = asked(&arguments(&["--shard", "3/4"])).unwrap();
        assert_eq!(shard.shard, Some((3, 4)));
    }

    /// Every checked-in mutant expected to be killed names a witness, which
    /// is a cell the model selects, spelled as the tool spells it.
    #[test]
    fn every_mutant_has_a_witness() {
        let repo = run::repository();
        let mutants = load(&directory(&repo)).unwrap();
        let missing: Vec<&str> = without_witnesses(&mutants)
            .iter()
            .map(|mutant| mutant.name.as_str())
            .collect();
        assert!(
            missing.is_empty(),
            "mutants without a witness: {}",
            missing.join(", ")
        );
        for mutant in &mutants {
            if let Err(error) = run::cells(&mutant.witnesses) {
                panic!("mutant `{}`: {error}", mutant.name);
            }
        }
    }

    /// Every checked-in mutant parses, names a source file of the crate
    /// under test, and still applies there.
    #[test]
    fn every_mutant_applies() {
        let repo = run::repository();
        let mutants = load(&directory(&repo)).unwrap();
        assert!(!mutants.is_empty(), "no mutant in conformance/mutants");
        for mutant in &mutants {
            let source = fs::read_to_string(repo.join(&mutant.file)).unwrap_or_else(|error| {
                panic!(
                    "mutant `{}` names `{}`, which cannot be read ({error}); update the mutant or delete it",
                    mutant.name, mutant.file
                )
            });
            if let Err(occurrences) = mutant.apply(&source) {
                panic!("{}", mutant.stale(occurrences));
            }
        }
    }
}
