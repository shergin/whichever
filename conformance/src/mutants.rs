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
//! The command runs the matrix once on the unmodified crate, then once per
//! mutant on a copy of the crate under test with the change applied, as
//! `run --whichever PATH` does, and compares the two tables the runs write.
//! A mutant is killed when a cell that agreed in the baseline diverges, or
//! a line of `known.tsv` goes stale.

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
    /// comments and blank lines; then a line `find:` opens the text to
    /// find, a line `replace:` ends it and opens its replacement, which runs
    /// to the end of the file. Both texts keep their lines as written,
    /// without the line break of their last line.
    pub fn parse(name: &str, text: &str) -> Result<Mutant, String> {
        let mut lines = text.lines();
        let mut header: BTreeMap<&str, String> = BTreeMap::new();
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
            if !["file", "breaks", "expect", "reason"].contains(&key) {
                return Err(format!(
                    "no key `{key}`; the keys are `file`, `breaks`, `expect` and `reason`"
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
        /// One of them, with its finding, or the stale line.
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
            Fate::Stale(_) => "stale",
            Fate::Uncompiled(_) => "uncompiled",
            Fate::Harness(_) => "harness",
        }
    }

    /// Returns whether the result is what `expect` says.
    fn meets(&self, expect: &Expect) -> bool {
        matches!(
            (self, expect),
            (Fate::Killed { .. }, Expect::Killed) | (Fate::Survived { .. }, Expect::Survives(_))
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
            example,
        },
    }
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
    suites: Vec<String>,
    shape: bool,
    shard: Option<Shard>,
}

/// One of `count` even parts of the mutants, taken by their place in name
/// order, so that jobs running in parallel share the mutants between them
/// and each runs its own baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shard {
    /// The part, counted from 1.
    number: usize,
    /// How many parts there are.
    count: usize,
}

impl Shard {
    /// Parses `K/N`, the `K`th of `N` parts.
    fn parse(text: &str) -> Result<Shard, String> {
        let refused = || format!("`--shard {text}` is not `K/N` with `1 <= K <= N`");
        let (number, count) = text.split_once('/').ok_or_else(refused)?;
        let number: usize = number.parse().map_err(|_| refused())?;
        let count: usize = count.parse().map_err(|_| refused())?;
        if number == 0 || number > count {
            return Err(refused());
        }
        Ok(Shard { number, count })
    }

    /// Returns whether the mutant at `index` in name order is this part's.
    fn holds(self, index: usize) -> bool {
        index % self.count == self.number - 1
    }
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
            "--suite" => asked.suites.push(value("--suite")?),
            "--no-shape" => asked.shape = false,
            "--shard" => {
                let shard = Shard::parse(&value("--shard")?).map_err(RunError::Usage)?;
                asked.shard = Some(shard);
            }
            other => return Err(RunError::Usage(format!("unknown option `{other}`"))),
        }
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

fn table(lines: &[Line]) -> String {
    let mut text =
        String::from("mutant\tfile\tresult\texpected\tchecks\tcells\tseconds\texample\n");
    for line in lines {
        let expected = match &line.mutant.expect {
            Expect::Killed => "killed",
            Expect::Survives(_) => "survives",
        };
        let (checks, cells, example) = match &line.result {
            Fate::Killed {
                checks,
                cells,
                example,
            } => (checks.join(","), cells.to_string(), example.clone()),
            Fate::Survived { excused, example } => {
                let example = match excused {
                    0 => String::new(),
                    _ => format!("{excused} cells newly excused by known.tsv, such as {example}"),
                };
                (String::new(), String::from("0"), example)
            }
            Fate::Stale(message) | Fate::Uncompiled(message) | Fate::Harness(message) => {
                (String::new(), String::new(), message.clone())
            }
        };
        text.push_str(&format!(
            "{}\t{}\t{}\t{expected}\t{checks}\t{cells}\t{:.0}\t{}\n",
            line.mutant.name,
            line.mutant.file,
            line.result.word(),
            line.seconds,
            example.replace(['\t', '\n'], " ")
        ));
    }
    text
}

/// Runs `mutants [--only NAME]... [--suite NAME]... [--no-shape]`: exits 0
/// when every mutant's result is what it expects, 1 when one expected to be
/// killed survived or one expected to survive was killed, and 2 on a usage
/// or harness error, a stale or uncompiled mutant included.
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
    if let Some(shard) = asked.shard {
        let mut index = 0;
        mutants.retain(|_| {
            index += 1;
            shard.holds(index - 1)
        });
        if mutants.is_empty() {
            progress("no mutant falls in this shard");
            return Ok(ExitCode::SUCCESS);
        }
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
    let options = Options {
        suites: asked.suites.clone(),
        shape: asked.shape,
        quiet: true,
        ..Options::default()
    };
    let complete = asked.suites.is_empty();
    let files = Files::at(&repo.join("conformance"));
    let known_text = fs::read_to_string(&files.known).unwrap_or_default();
    let known = ledger::parse_known(&known_text).map_err(harness)?;

    progress("the baseline, on the unmodified crate");
    let (status, baseline) = judged(&options, &report)?;
    if status != Status::Clean {
        return Err(RunError::Harness(String::from(
            "the baseline run does not agree, so it measures nothing; its divergences are in target/conformance/report.tsv",
        )));
    }
    let mut lines: Vec<Line> = Vec::new();
    for (index, (mutant, source)) in mutants.iter().zip(changed).enumerate() {
        let step = Instant::now();
        progress(&format!(
            "mutant {}/{}, `{}`",
            index + 1,
            mutants.len(),
            mutant.name
        ));
        let copy = base.join("mutants").join(&mutant.name);
        copy_crate(&repo, &copy).map_err(harness)?;
        let source = source.expect("every mutant applies");
        fs::write(copy.join(&mutant.file), source).map_err(harness)?;
        let result = match compiles(&copy, &check_target) {
            Err(error) => Fate::Uncompiled(error),
            Ok(()) => {
                let target = base.join("target");
                let before = artifacts(&target);
                let options = Options {
                    whichever: Some(copy.clone()),
                    ..options.clone()
                };
                let judged = judged(&options, &report);
                prune(&target, &before);
                match judged {
                    Err(RunError::Usage(message)) => return Err(RunError::Usage(message)),
                    Err(RunError::Harness(message)) => Fate::Harness(message),
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

    let text = table(&lines);
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
        let note = match (&line.result, &line.mutant.expect) {
            (Fate::Killed { .. }, Expect::Survives(_)) => {
                "was killed although it expects to survive; update its `expect:` line"
            }
            (Fate::Survived { .. }, Expect::Killed) => {
                "survived although it expects to be killed: the matrix no longer notices it"
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
        if let Expect::Survives(reason) = &line.mutant.expect {
            println!("  reason    {reason}");
        }
    }
    if broken {
        code = ExitCode::from(2);
    }
    Ok(code)
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
    fn shards_share_the_mutants_between_them() {
        let shards: Vec<Shard> = (1..=3)
            .map(|number| Shard::parse(&format!("{number}/3")).unwrap())
            .collect();
        for index in 0..10 {
            let holders = shards.iter().filter(|shard| shard.holds(index)).count();
            assert_eq!(holders, 1, "mutant {index} is in {holders} shards");
        }
        assert!(shards[0].holds(0) && shards[0].holds(3) && shards[2].holds(2));
        for refused in ["0/3", "4/3", "3", "a/3", "1/0", "1/"] {
            assert!(Shard::parse(refused).is_err(), "`{refused}` was accepted");
        }
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
