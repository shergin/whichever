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
        }
    }

    /// Returns how many shards the kind is spread over. A cell's shard is
    /// its hash modulo this count, so adding a cell moves no other.
    pub fn buckets(self) -> u64 {
        match self {
            ShardKind::Pass => 16,
            ShardKind::Limit => 24,
            ShardKind::Malformed => 1,
            ShardKind::Rejected => 8,
        }
    }

    /// Returns the name of the shard a cell's hash falls in.
    pub fn shard(self, hash: u64) -> String {
        format!("{}-{:03}", self.name(), hash % self.buckets())
    }
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
    /// `source` returns the source of a module.
    pub fn write(
        &self,
        whichever: &Path,
        lock: Option<&Path>,
        shards: &[Shard],
        source: &dyn Fn(&str) -> String,
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
             # The attributes run once per cell; an optimised macro crate\n\
             # expands them faster than it takes to build.\n\
             [profile.dev.build-override]\n\
             opt-level = 3\n",
            members.join(", ")
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
            write_if_changed(&directory.join("Cargo.toml"), &manifest)?;
            write_if_changed(&directory.join("src").join("support.rs"), support::SOURCE)?;
            self.write_lib(shard, &shard.modules)?;
            for module in &shard.modules {
                if write_if_changed(&self.cell_path(&shard.name, module), &source(module))? {
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
        for module in modules {
            text.push_str(&format!("mod {module};\n"));
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
    fn a_shard_is_chosen_by_the_hash_alone() {
        assert_eq!(ShardKind::Pass.shard(19), "pass-003");
        assert_eq!(ShardKind::Limit.shard(25), "limit-001");
        assert_eq!(ShardKind::Malformed.shard(13), "malformed-000");
        assert_eq!(toml_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }
}
