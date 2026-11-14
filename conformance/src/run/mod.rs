//! A run: the cells written out as a workspace, compiled and tested by
//! cargo, and each cell's outcome compared with its verdict.
//!
//! The steps are: select the suites' cells; render them into shard crates
//! by verdict and hash; `cargo check` every shard, setting aside the cells
//! of pass shards that have errors and checking again until the pass shards
//! are clean, since an early error can hide a later one; `cargo test` the
//! pass shards and read each cell's two tests; judge every outcome against
//! its verdict and the ledger.

pub mod cargo;
pub mod outcome;
mod report;
pub mod workspace;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use crate::cell::Cell;
use crate::ledger::{self, Files, Known};
use crate::model::Verdict;
use crate::render::{self, Rendered};
use crate::select::{self, Selection};
use cargo::{Diagnostic, Level};
use outcome::{Judgement, Outcome};
use workspace::{Shard, ShardKind, Workspace};

/// What the command line asks of a run.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// The suites to run; empty runs every suite.
    pub suites: Vec<String>,
    /// Only cells whose id contains this text.
    pub filter: Option<String>,
    /// Rewrite `census.txt` instead of comparing it.
    pub bless: bool,
    /// The `whichever` to judge, instead of this checkout's.
    pub whichever: Option<PathBuf>,
    /// The jobs cargo runs at once.
    pub jobs: Option<usize>,
}

/// Why a run could not judge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    /// The command line asks for something that does not exist.
    Usage(String),
    /// The harness failed: cargo, a file, or a diagnostic no cell owns.
    Harness(String),
}

impl core::fmt::Display for RunError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RunError::Usage(message) => write!(formatter, "usage: {message}"),
            RunError::Harness(message) => write!(formatter, "harness: {message}"),
        }
    }
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Nothing diverged and the ledger agrees.
    Clean,
    /// A cell diverged, or the ledger disagrees.
    Diverged,
    /// A hand does not compile or fails its law: the model is wrong.
    ModelBroken,
}

/// A cell of the run.
#[derive(Clone, Debug)]
pub struct Entry {
    /// The cell.
    pub cell: Cell,
    /// Its id.
    pub id: String,
    /// Its module.
    pub module: String,
    /// Its verdict.
    pub verdict: Verdict,
    /// Its source.
    pub rendered: Rendered,
    /// The suites that select it.
    pub suites: Vec<String>,
    /// Its shard.
    pub shard: String,
    /// What the run observed.
    pub outcome: Outcome,
}

/// Returns the repository the crate is built in.
pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate is a member of the repository's workspace")
        .to_path_buf()
}

fn progress(message: &str) {
    eprintln!("conformance: {message}");
}

fn harness(error: impl core::fmt::Display) -> RunError {
    RunError::Harness(error.to_string())
}

/// Returns every suite: the zones, the pinned cells and the known cells.
pub fn suites(files: &Files) -> Result<(Vec<Selection>, Vec<Known>), RunError> {
    let known_text = std::fs::read_to_string(&files.known).unwrap_or_default();
    let known = ledger::parse_known(&known_text).map_err(harness)?;
    let pinned_text = std::fs::read_to_string(&files.pinned).unwrap_or_default();
    let pinned = ledger::parse_pinned(&pinned_text).map_err(harness)?;
    let mut selections = select::zones().map_err(harness)?;
    let parse = |id: &str| Cell::parse(id).expect("the ledger's ids parse");
    selections.push(select::select(
        "pinned",
        pinned.iter().map(|entry| parse(&entry.id)).collect(),
    ));
    selections.push(select::select(
        "known",
        known.iter().map(|entry| parse(&entry.id)).collect(),
    ));
    Ok((selections, known))
}

/// Runs the suites and reports.
pub fn run(options: &Options) -> Result<Status, RunError> {
    let started = Instant::now();
    let repo = repository();
    let files = Files::at(&repo.join("conformance"));
    let (selections, known) = suites(&files)?;
    let names: Vec<&str> = selections
        .iter()
        .map(|selection| selection.suite.as_str())
        .collect();
    for suite in &options.suites {
        if !names.contains(&suite.as_str()) {
            return Err(RunError::Usage(format!(
                "no suite `{suite}`; the suites are {}",
                names.join(", ")
            )));
        }
    }

    let census = ledger::census(&selections);
    let census_differences = match options.bless {
        true => {
            std::fs::write(&files.census, &census).map_err(harness)?;
            progress("census.txt rewritten");
            Vec::new()
        }
        false => {
            let pinned = std::fs::read_to_string(&files.census).unwrap_or_default();
            ledger::differences(&pinned, &census)
        }
    };

    let entries = gather(&selections, options)?;
    if entries.is_empty() {
        return Err(RunError::Usage(String::from("the selection holds no cell")));
    }
    for entry in &entries {
        if !entry.verdict.is_selectable() {
            return Err(RunError::Harness(format!(
                "`{}` is listed but its verdict is {}",
                entry.id,
                entry.verdict.describe()
            )));
        }
    }
    let mut entries = entries;
    let mut timings: Vec<(String, Duration)> =
        vec![(String::from("select and render"), started.elapsed())];

    let whichever = options.whichever.clone().unwrap_or_else(|| repo.clone());
    let whichever = whichever
        .canonicalize()
        .map_err(|error| RunError::Usage(format!("`{}`: {error}", whichever.display())))?;
    let workspace = Workspace::under(&repo);
    let shards = shards(&entries);
    let lock = repo.join("Cargo.lock");
    let step = Instant::now();
    let sources: BTreeMap<String, String> = entries
        .iter()
        .map(|entry| (entry.module.clone(), entry.rendered.source.clone()))
        .collect();
    let written = workspace
        .write(
            &whichever,
            lock.exists().then_some(lock.as_path()),
            &shards,
            &|module| sources[module].clone(),
        )
        .map_err(harness)?;
    progress(&format!(
        "{} cells in {} shards, {written} files written, judging `{}`",
        entries.len(),
        shards.len(),
        whichever.display()
    ));
    timings.push((String::from("write"), step.elapsed()));

    let by_module: BTreeMap<String, usize> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.module.clone(), index))
        .collect();
    let mut set_aside: BTreeSet<String> = BTreeSet::new();
    let mut round = 0;
    let step = Instant::now();
    loop {
        round += 1;
        progress(&format!("cargo check, round {round}"));
        for entry in entries.iter_mut() {
            if !set_aside.contains(&entry.module) {
                entry.outcome = Outcome::default();
            }
        }
        let diagnostics = check(&workspace, options.jobs)?;
        for diagnostic in &diagnostics {
            let Some(site) = &diagnostic.site else {
                return Err(RunError::Harness(format!(
                    "a diagnostic no cell owns: {}",
                    diagnostic.rendered
                )));
            };
            let Some(index) = by_module.get(&site.module) else {
                return Err(RunError::Harness(format!(
                    "a diagnostic in `{}`, which is no cell of this run: {}",
                    site.module, diagnostic.rendered
                )));
            };
            let entry = &mut entries[*index];
            if set_aside.contains(&entry.module) {
                continue;
            }
            outcome::observe_diagnostic(&mut entry.outcome, &entry.rendered, diagnostic);
        }
        let failing: Vec<String> = entries
            .iter()
            .filter(|entry| entry.verdict == Verdict::Pass && entry.outcome.has_error())
            .filter(|entry| !set_aside.contains(&entry.module))
            .map(|entry| entry.module.clone())
            .collect();
        if failing.is_empty() {
            break;
        }
        progress(&format!(
            "{} pass cells have errors and are set aside",
            failing.len()
        ));
        set_aside.extend(failing);
        for shard in shards.iter().filter(|shard| shard.kind == ShardKind::Pass) {
            let kept: Vec<String> = shard
                .modules
                .iter()
                .filter(|module| !set_aside.contains(*module))
                .cloned()
                .collect();
            workspace.write_lib(shard, &kept).map_err(harness)?;
        }
    }
    timings.push((format!("check, {round} rounds"), step.elapsed()));

    let step = Instant::now();
    let pass_shards: Vec<&Shard> = shards
        .iter()
        .filter(|shard| shard.kind == ShardKind::Pass)
        .collect();
    if !pass_shards.is_empty() {
        progress("cargo test");
        let results = test(&workspace, &pass_shards, options.jobs)?;
        for entry in entries.iter_mut() {
            if entry.verdict != Verdict::Pass || set_aside.contains(&entry.module) {
                continue;
            }
            for test in ["subject", "hand"] {
                let name = format!("{}::law::{test}", entry.module);
                outcome::observe_test(&mut entry.outcome, test, results.get(&name));
            }
        }
    }
    timings.push((String::from("test"), step.elapsed()));
    timings.push((String::from("total"), started.elapsed()));

    let judgements: Vec<Judgement> = entries
        .iter()
        .map(|entry| outcome::judge(&entry.verdict, &entry.outcome, &entry.rendered))
        .collect();
    let report = report::Report {
        entries: &entries,
        judgements: &judgements,
        known: &known,
        workspace: &workspace,
        census_differences: &census_differences,
        timings: &timings,
    };
    let status = report.print();
    report
        .write_table(&repo.join("target").join("conformance").join("report.tsv"))
        .map_err(harness)?;
    Ok(status)
}

/// Returns the run's cells: the suites asked for, the known cells always,
/// narrowed by the filter, each once and rendered.
fn gather(selections: &[Selection], options: &Options) -> Result<Vec<Entry>, RunError> {
    let mut order: Vec<Cell> = Vec::new();
    let mut suites: BTreeMap<Cell, Vec<String>> = BTreeMap::new();
    for selection in selections {
        let asked = options.suites.is_empty()
            || options.suites.contains(&selection.suite)
            || selection.suite == "known";
        if !asked {
            continue;
        }
        for cell in &selection.cells {
            let id = cell.id();
            if options
                .filter
                .as_ref()
                .is_some_and(|filter| !id.contains(filter.as_str()))
            {
                continue;
            }
            let listed = suites.entry(*cell).or_default();
            if listed.is_empty() {
                order.push(*cell);
            }
            listed.push(selection.suite.clone());
        }
    }
    let mut modules: BTreeMap<String, String> = BTreeMap::new();
    let mut entries = Vec::new();
    for cell in order {
        let id = cell.id();
        let module = cell.module();
        if let Some(other) = modules.insert(module.clone(), id.clone()) {
            return Err(RunError::Harness(format!(
                "`{id}` and `{other}` hash to the same module `{module}`"
            )));
        }
        let verdict = cell.verdict();
        let kind = match verdict {
            Verdict::Pass => ShardKind::Pass,
            Verdict::Limit(_) => ShardKind::Limit,
            Verdict::Malformed(_) | Verdict::Unmodelled(_) | Verdict::NonCanonical => {
                ShardKind::Malformed
            }
        };
        entries.push(Entry {
            rendered: render::render(&cell),
            shard: kind.shard(cell.hash()),
            suites: suites.remove(&cell).unwrap_or_default(),
            cell,
            id,
            module,
            verdict,
            outcome: Outcome::default(),
        });
    }
    Ok(entries)
}

/// Returns the shards the entries fall in, by name.
fn shards(entries: &[Entry]) -> Vec<Shard> {
    let mut shards: BTreeMap<String, Shard> = BTreeMap::new();
    for entry in entries {
        let kind = match entry.verdict {
            Verdict::Pass => ShardKind::Pass,
            Verdict::Limit(_) => ShardKind::Limit,
            Verdict::Malformed(_) | Verdict::Unmodelled(_) | Verdict::NonCanonical => {
                ShardKind::Malformed
            }
        };
        let shard = shards.entry(entry.shard.clone()).or_insert_with(|| Shard {
            name: entry.shard.clone(),
            kind,
            modules: Vec::new(),
        });
        shard.modules.push(entry.module.clone());
    }
    let mut shards: Vec<Shard> = shards.into_values().collect();
    for shard in &mut shards {
        shard.modules.sort();
    }
    shards
}

/// Runs `cargo check` on the workspace and returns its diagnostics.
fn check(workspace: &Workspace, jobs: Option<usize>) -> Result<Vec<Diagnostic>, RunError> {
    let mut command = cargo::command(&workspace.root, &workspace.target);
    command.args([
        "check",
        "--workspace",
        "--tests",
        "--keep-going",
        "--message-format=json",
    ]);
    if let Some(jobs) = jobs {
        command.arg(format!("--jobs={jobs}"));
    }
    command.stderr(Stdio::inherit());
    let output = command.output().map_err(harness)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let diagnostics = cargo::diagnostics(&stdout);
    let failed = !output.status.success();
    let erred = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.level == Level::Error);
    if failed && !erred {
        return Err(RunError::Harness(String::from(
            "cargo check failed without a compiler error; its messages are above",
        )));
    }
    Ok(diagnostics)
}

/// Runs `cargo test` on the pass shards and returns every test's result.
fn test(
    workspace: &Workspace,
    shards: &[&Shard],
    jobs: Option<usize>,
) -> Result<BTreeMap<String, cargo::TestResult>, RunError> {
    let mut command = cargo::command(&workspace.root, &workspace.target);
    command.args(["test", "--lib", "--no-fail-fast", "--message-format=json"]);
    for shard in shards {
        command.args(["-p", &shard.name]);
    }
    if let Some(jobs) = jobs {
        command.arg(format!("--jobs={jobs}"));
    }
    command.stderr(Stdio::inherit());
    let output = command.output().map_err(harness)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let erred = cargo::diagnostics(&stdout)
        .iter()
        .any(|diagnostic| diagnostic.level == Level::Error);
    if erred {
        return Err(RunError::Harness(String::from(
            "the pass shards failed to build for test after a clean check",
        )));
    }
    Ok(cargo::test_results(&stdout))
}
