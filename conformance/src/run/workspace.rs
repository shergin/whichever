//! The workspace a run writes: one shard crate per kind and hash bucket,
//! one file per cell, each file written only when its content changed so
//! that cargo's incremental build stays warm.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::Verdict;
use crate::render::support;

/// What a shard holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShardKind {
    /// Passing cells, checked and then tested.
    Pass,
    /// Limit cells, checked.
    Limit,
    /// Malformed cells, checked.
    Malformed,
    /// Rejected cells, checked.
    Rejected,
    /// The traits and payloads of the cells whose trait lives in another
    /// crate, which every other shard depends on.
    Traits,
}

impl ShardKind {
    /// Returns the kind of shard a verdict's cells go in.
    pub fn of(verdict: &Verdict) -> ShardKind {
        match verdict {
            Verdict::Pass => ShardKind::Pass,
            Verdict::Limit(_) => ShardKind::Limit,
            Verdict::Rejected { .. } => ShardKind::Rejected,
            Verdict::Malformed(_) | Verdict::Unmodelled(_) | Verdict::NonCanonical => {
                ShardKind::Malformed
            }
        }
    }

    /// Returns the kind's name, the prefix of its shards' names.
    pub fn name(self) -> &'static str {
        match self {
            ShardKind::Pass => "pass",
            ShardKind::Limit => "limit",
            ShardKind::Malformed => "malformed",
            ShardKind::Rejected => "rejected",
            ShardKind::Traits => "traits",
        }
    }

    /// Returns the fewest shards the kind is spread over, so that a run of
    /// a few thousand cells still builds on several cores at once.
    fn spread(self) -> u64 {
        match self {
            ShardKind::Pass => 16,
            ShardKind::Limit => 24,
            ShardKind::Malformed => 1,
            ShardKind::Rejected => 8,
            ShardKind::Traits => TRAIT_SHARDS,
        }
    }

    /// Returns the most cells a shard of the kind holds on average. One
    /// compiler builds a shard, and its memory grows with the shard's
    /// cells, about linearly: on 2026-11-17, on macOS arm64 with stable and
    /// `CARGO_INCREMENTAL=0`, the test build of a passing shard of the
    /// `array` suite peaked at 1.35 GB resident over 500 cells, 2.45 GB
    /// over 1,000 and 5.2 GB over 2,150, and a check of a limit shard of
    /// 1,179 cells at 0.6 GB. CI's runner has 16 GB and builds four shards
    /// at once, which the passing shards of 2,150 cells starved. Limits,
    /// rejected and malformed cells are only checked, and are held to about
    /// the size measured for limits.
    fn capacity(self) -> usize {
        match self {
            ShardKind::Pass => PASS_CAPACITY,
            ShardKind::Limit | ShardKind::Malformed | ShardKind::Rejected => 1_500,
            // A traits shard holds a trait and its payloads, which are small
            // beside the cells that read them.
            ShardKind::Traits => usize::MAX,
        }
    }

    /// Returns how many shards a run's `cells` cells of the kind are spread
    /// over: as many as keep each to the kind's capacity, and no fewer than
    /// its spread. A cell's shard is its hash modulo this count, so a cell
    /// moves only when the count does, and a run of one cell builds one
    /// shard. The traits shards are as many as their spread whatever the
    /// run, since a cell's source names its traits crate before the run's
    /// cells are counted.
    pub fn count(self, cells: usize) -> u64 {
        let needed = u64::try_from(cells.div_ceil(self.capacity())).unwrap_or(u64::MAX);
        needed.max(self.spread())
    }

    /// Returns the name of the shard, among `count`, a cell's hash falls
    /// in.
    pub fn shard(self, hash: u64, count: u64) -> String {
        format!("{}-{:03}", self.name(), hash % count)
    }
}

/// The traits shards of every run; see [`ShardKind::count`].
const TRAIT_SHARDS: u64 = 8;

/// The most cells a passing shard holds on average. Over the matrix of
/// 2026-11-17 the largest held 547 cells and its test build peaked at
/// 1.24 GB, so four at once take about a third of CI's runner, with room
/// for the matrix's cells growing costlier. See [`ShardKind::capacity`].
const PASS_CAPACITY: usize = 500;

/// Returns the name of the traits shard of a cell whose trait lives in a
/// crate of its own.
pub fn traits_shard(hash: u64) -> String {
    ShardKind::Traits.shard(hash, TRAIT_SHARDS)
}

/// A shard crate and the modules it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shard {
    /// The crate's name, `pass-003`.
    pub name: String,
    /// What it holds.
    pub kind: ShardKind,
    /// The modules of its cells, sorted.
    pub modules: Vec<String>,
    /// The shards it depends on: every traits shard, for any other.
    pub dependencies: Vec<String>,
}

/// Returns the name of a shard as a crate's identifier: `traits_003`.
pub fn crate_name(shard: &str) -> String {
    shard.replace('-', "_")
}

/// The workspace's directories.
#[derive(Clone, Debug)]
pub struct Workspace {
    /// Where the workspace's manifest is.
    pub root: PathBuf,
    /// Where cargo builds it.
    pub target: PathBuf,
}

/// Writes `content` to `path` unless it holds it already; returns whether
/// it wrote.
/// The fewest cells a run builds the macro crate optimised for: below it,
/// building `syn` and the macro crate with `opt-level = 3` costs more than
/// the faster expansion saves. Measured on 2026-11-16 against a fresh copy
/// of the crate under test, `cargo check` took 2.3s unoptimised and 8.6s
/// optimised over ten cells, 2.8s and 13.6s over the 149 pinned cells, and
/// 5.1s and 12.7s over the 2,721 of `shapes`; the full matrix, some sixty
/// thousand cells, is where the optimised build was measured to pay.
pub const OPTIMISED_FROM: usize = 10_000;

/// Returns whether a run of `cells` cells builds the macro crate optimised.
pub fn optimised(cells: usize) -> bool {
    cells >= OPTIMISED_FROM
}

/// Returns the `opt-level` the build scripts and procedural macros take.
fn build_opt_level(optimised: bool) -> u8 {
    match optimised {
        true => 3,
        false => 0,
    }
}

fn write_if_changed(path: &Path, content: &str) -> io::Result<bool> {
    if fs::read_to_string(path).is_ok_and(|existing| existing == content) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(true)
}

/// Returns a TOML basic string.
fn toml_string(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

impl Workspace {
    /// Returns the workspace under `<repo>/target/conformance`.
    pub fn under(repo: &Path) -> Workspace {
        let base = repo.join("target").join("conformance");
        Workspace {
            root: base.join("ws"),
            target: base.join("target"),
        }
    }

    /// Returns the path of a cell's file.
    pub fn cell_path(&self, shard: &str, module: &str) -> PathBuf {
        self.root
            .join(shard)
            .join("src")
            .join(format!("{module}.rs"))
    }

    /// Writes the manifest, the lock file and every shard with its cells'
    /// files, and removes the shards and the files no longer selected.
    /// `source` returns the source of a module; `optimised` says whether
    /// the macro crate and its dependencies are built optimised.
    pub fn write(
        &self,
        whichever: &Path,
        lock: Option<&Path>,
        shards: &[Shard],
        source: &dyn Fn(&Shard, &str) -> String,
        optimised: bool,
    ) -> io::Result<usize> {
        fs::create_dir_all(&self.root)?;
        let members: Vec<String> = shards
            .iter()
            .map(|shard| toml_string(&shard.name))
            .collect();
        let manifest = format!(
            "# Written by the conformance matrix; every run rewrites it.\n\
             [workspace]\n\
             resolver = \"2\"\n\
             members = [{}]\n\
             \n\
             [profile.dev]\n\
             debug = 0\n\
             \n\
             # The attributes run once per cell; over many cells an optimised\n\
             # macro crate expands them faster than it takes to build, and\n\
             # over a few it is built for longer than it saves.\n\
             [profile.dev.build-override]\n\
             opt-level = {}\n",
            members.join(", "),
            build_opt_level(optimised)
        );
        write_if_changed(&self.root.join("Cargo.toml"), &manifest)?;
        if let Some(lock) = lock {
            let content = fs::read_to_string(lock)?;
            write_if_changed(&self.root.join("Cargo.lock"), &content)?;
        }
        let mut written = 0;
        for shard in shards {
            let directory = self.root.join(&shard.name);
            let manifest = format!(
                "[package]\n\
                 name = {}\n\
                 version = \"0.0.0\"\n\
                 edition = \"2021\"\n\
                 publish = false\n\
                 \n\
                 [lib]\n\
                 doctest = false\n\
                 \n\
                 [dependencies]\n\
                 whichever = {{ path = {} }}\n",
                toml_string(&shard.name),
                toml_string(&whichever.display().to_string())
            );
            let mut manifest = manifest;
            for dependency in &shard.dependencies {
                manifest.push_str(&format!(
                    "{dependency} = {{ path = {} }}\n",
                    toml_string(&format!("../{dependency}"))
                ));
            }
            write_if_changed(&directory.join("Cargo.toml"), &manifest)?;
            write_if_changed(&directory.join("src").join("support.rs"), support::SOURCE)?;
            self.write_lib(shard, &shard.modules)?;
            for module in &shard.modules {
                if write_if_changed(&self.cell_path(&shard.name, module), &source(shard, module))? {
                    written += 1;
                }
            }
            for entry in fs::read_dir(directory.join("src"))? {
                let path = entry?.path();
                let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                    continue;
                };
                if stem.starts_with("c_") && !shard.modules.iter().any(|module| module == stem) {
                    fs::remove_file(&path)?;
                }
            }
        }
        for entry in fs::read_dir(&self.root)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !shards.iter().any(|shard| shard.name == name) {
                fs::remove_dir_all(&path)?;
            }
        }
        Ok(written)
    }

    /// Writes a shard's `lib.rs` holding `modules`; returns whether it
    /// wrote.
    pub fn write_lib(&self, shard: &Shard, modules: &[String]) -> io::Result<bool> {
        let mut text = format!(
            "//! Shard `{}` of the conformance matrix: one module per cell.\n\
             //!\n\
             //! A cell's items are read by its law only, and its traits use\n\
             //! `async fn`; every other warning is part of a cell's outcome.\n\
             #![allow(dead_code, async_fn_in_trait)]\n\
             \n\
             pub mod support;\n\
             \n",
            shard.name
        );
        // A traits shard's modules are reached from the subject's crate, and
        // its root declares the type its traits name by a `crate::` path,
        // which no other shard declares.
        let visibility = match shard.kind {
            ShardKind::Traits => {
                text.push_str(
                    "/// The argument a trait of this crate names by a `crate::` path.\n\
                     pub type Tally = u64;\n\n",
                );
                "pub "
            }
            ShardKind::Pass | ShardKind::Limit | ShardKind::Malformed | ShardKind::Rejected => "",
        };
        for module in modules {
            text.push_str(&format!("{visibility}mod {module};\n"));
        }
        write_if_changed(
            &self.root.join(&shard.name).join("src").join("lib.rs"),
            &text,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shard_is_chosen_by_the_hash_among_the_count() {
        assert_eq!(ShardKind::Pass.shard(19, 16), "pass-003");
        assert_eq!(ShardKind::Pass.shard(19, 40), "pass-019");
        assert_eq!(ShardKind::Limit.shard(25, 24), "limit-001");
        assert_eq!(ShardKind::Malformed.shard(13, 1), "malformed-000");
        assert_eq!(traits_shard(13), "traits-005");
        assert_eq!(toml_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }

    #[test]
    fn a_kind_has_its_spread_until_its_cells_outgrow_it() {
        // A small run keeps the spread, and its cells fill a shard each at
        // most: one cell is one shard.
        assert_eq!(ShardKind::Pass.count(0), 16);
        assert_eq!(ShardKind::Pass.count(1), 16);
        assert_eq!(ShardKind::Pass.count(16 * PASS_CAPACITY), 16);
        // Past it, the count follows the cells.
        assert_eq!(ShardKind::Pass.count(16 * PASS_CAPACITY + 1), 17);
        assert_eq!(ShardKind::Pass.count(34_505), 70);
        assert_eq!(ShardKind::Limit.count(28_115), 24);
        assert_eq!(ShardKind::Limit.count(60_000), 40);
        assert_eq!(ShardKind::Malformed.count(27), 1);
        assert_eq!(ShardKind::Rejected.count(2_296), 8);
        // The traits shards are named before the run's cells are counted.
        assert_eq!(ShardKind::Traits.count(1_000_000), TRAIT_SHARDS);
    }

    #[test]
    fn hashing_holds_a_shard_near_its_capacity() {
        // The cells' hashes spread them evenly: over a matrix of twice
        // today's passing cells, hashed as a cell's id is, no shard holds
        // more than a tenth over the capacity.
        let cells = 70_000;
        let count = ShardKind::Pass.count(cells);
        let mut sizes = vec![0_usize; usize::try_from(count).unwrap()];
        for index in 0..cells {
            let hash = crate::cell::fnv1a(format!("cell {index}").as_bytes());
            sizes[usize::try_from(hash % count).unwrap()] += 1;
        }
        let largest = sizes.iter().copied().max().unwrap();
        assert!(largest <= PASS_CAPACITY * 11 / 10, "{largest}");
    }

    #[test]
    fn a_small_selection_builds_the_macro_crate_unoptimised() {
        assert!(!optimised(1));
        assert!(!optimised(OPTIMISED_FROM - 1));
        assert!(optimised(OPTIMISED_FROM));
        assert_eq!(build_opt_level(false), 0);
        assert_eq!(build_opt_level(true), 3);
    }
}
