//! Covering arrays under a validity predicate.
//!
//! A row holds one value index per axis. A demand of strength `t` over
//! some axes asks that every combination of values of every `t` of those
//! axes meets in at least one row. Rows are constrained: only a row the
//! predicate accepts may be held, so a combination no valid row holds is
//! not required. Whether one exists is decided by search, and a
//! combination the search could not place is counted as unreached rather
//! than hidden.
//!
//! This module knows nothing of cells: [`crate::select`] maps cells to
//! rows and back.

/// The rows a suite may hold.
pub struct Space<'a> {
    /// The number of values of each axis; a row has one index per axis,
    /// each below its axis's number.
    pub domains: &'a [usize],
    /// A valid row every search starts from.
    pub base: &'a [usize],
    /// Whether a complete row may be held.
    pub valid: &'a dyn Fn(&[usize]) -> bool,
}

/// Every combination of values of every `strength` of `axes` is to meet
/// in some row.
#[derive(Clone, Debug)]
pub struct Demand {
    /// Indices into [`Space::domains`], without repetition.
    pub axes: Vec<usize>,
    /// How many axes meet: 2 for pairs, 3 for triples.
    pub strength: usize,
}

/// How hard the search tries before it counts a combination as unreached.
#[derive(Clone, Copy, Debug)]
pub struct Effort {
    /// The completions tried for one combination.
    pub completions: usize,
}

impl Default for Effort {
    fn default() -> Self {
        Effort { completions: 2000 }
    }
}

/// What [`cover`] found.
#[derive(Clone, Debug, Default)]
pub struct Covering {
    /// The rows added, in the order they were chosen; `held` rows are not
    /// repeated.
    pub rows: Vec<Vec<usize>>,
    /// The combinations some row holds, `held` rows included.
    pub covered: usize,
    /// The combinations no valid row was found for.
    pub unreached: usize,
}

/// Returns rows that, together with `held`, meet every demand as far as
/// valid rows allow. The result is a function of the arguments alone: the
/// same space, demands, held rows, seed and effort give the same rows in
/// the same order.
pub fn cover(
    space: &Space<'_>,
    demands: &[Demand],
    held: &[Vec<usize>],
    seed: u64,
    effort: Effort,
) -> Covering {
    let _ = (space, demands, held, seed, effort);
    todo!("the covering-array generator")
}

/// Returns how many of the combinations the demands name are held by no
/// row of `rows`, counting only combinations some row of `reachable`
/// holds. A test passes every valid row of a small space as `reachable`
/// and expects zero.
pub fn missing(demands: &[Demand], rows: &[Vec<usize>], reachable: &[Vec<usize>]) -> usize {
    let _ = (demands, rows, reachable);
    todo!("the count of combinations left uncovered")
}
