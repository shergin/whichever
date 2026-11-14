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
//!
//! # The algorithm
//!
//! Every set of axes a demand names is a group, and every combination of
//! every group has a dense index, so the bookkeeping is one status byte
//! per combination. Rows are built one at a time. The next row starts
//! from an uncovered combination of the group with the most uncovered
//! combinations; a valid row holding it is searched for, starting from
//! the base row and varying the other axes; the row is then improved
//! greedily, axis by axis in a seeded order, each axis taking the value
//! that adds the most uncovered combinations among the axes already
//! settled while the row stays valid. A combination the search cannot
//! place is deferred to the end, when every chosen row is one more place
//! to start from, and only then counted as unreached.

use super::rng::Rng;
use std::collections::{BTreeMap, BTreeSet};

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
    /// The candidate rows built for one uncovered combination, of which
    /// the one adding the most combinations is kept.
    pub candidates: usize,
}

impl Default for Effort {
    fn default() -> Self {
        Effort {
            completions: 2000,
            candidates: 3,
        }
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
    let mut ledger = Ledger::new(space.domains, &groups(space.domains.len(), demands));
    for row in held {
        ledger.cover(row);
    }
    let mut search = Search {
        space,
        effort,
        rng: Rng::new(seed),
        known: held.to_vec(),
        pairs: BTreeMap::new(),
    };
    let mut rows = Vec::new();

    // The first pass: every combination either seeds a row or is deferred.
    while let Some(index) = ledger.next_uncovered() {
        let combination = ledger.decode(index);
        if search.has_failed_pair(&combination) {
            ledger.set(index, DEFERRED);
            continue;
        }
        let Some(start) = search.complete(&combination) else {
            ledger.set(index, DEFERRED);
            search.probe_pairs(&combination);
            continue;
        };
        let row = search.improve(&ledger, start, &combination);
        ledger.cover(&row);
        search.known.push(row.clone());
        rows.push(row);
    }

    // The second pass: every deferred combination is searched again, with
    // every chosen row as a start, lower strengths first so that a pair
    // found unreached settles the triples holding it.
    let mut deferred = ledger.with_status(DEFERRED);
    deferred.sort_by_key(|&index| (ledger.strength_of(index), index));
    search.pairs.retain(|_, reached| *reached);
    for index in deferred {
        if ledger.status[index] != DEFERRED {
            continue;
        }
        let combination = ledger.decode(index);
        if search.has_failed_pair(&combination) {
            ledger.set(index, UNREACHED);
            continue;
        }
        let Some(start) = search.complete(&combination) else {
            ledger.set(index, UNREACHED);
            if combination.len() == 2 {
                search.pairs.insert(pair_key(&combination), false);
            } else {
                search.probe_pairs(&combination);
            }
            continue;
        };
        let row = search.improve(&ledger, start, &combination);
        ledger.cover(&row);
        search.known.push(row.clone());
        rows.push(row);
    }

    Covering {
        rows,
        covered: ledger.with_status(COVERED).len(),
        unreached: ledger.with_status(UNREACHED).len(),
    }
}

/// Returns how many of the combinations the demands name are held by no
/// row of `rows`, counting only combinations some row of `reachable`
/// holds. A test passes every valid row of a small space as `reachable`
/// and expects zero.
pub fn missing(demands: &[Demand], rows: &[Vec<usize>], reachable: &[Vec<usize>]) -> usize {
    let mut wanted = BTreeSet::new();
    let mut held = BTreeSet::new();
    for demand in demands {
        for axes in subsets(&demand.axes, demand.strength) {
            let mut axes = axes;
            axes.sort_unstable();
            for row in reachable {
                let values: Vec<usize> = axes.iter().map(|&axis| row[axis]).collect();
                wanted.insert((axes.clone(), values));
            }
            for row in rows {
                let values: Vec<usize> = axes.iter().map(|&axis| row[axis]).collect();
                held.insert((axes.clone(), values));
            }
        }
    }
    wanted.difference(&held).count()
}

/// The status of a combination nothing holds yet.
const UNCOVERED: u8 = 0;
/// The status of a combination some row holds.
const COVERED: u8 = 1;
/// The status of a combination the first pass could not place.
const DEFERRED: u8 = 2;
/// The status of a combination no valid row was found for.
const UNREACHED: u8 = 3;

/// The passes that refine a row once every axis is settled.
const REFINEMENTS: usize = 2;

/// Returns every distinct set of axes a demand names, each sorted, in a
/// fixed order.
fn groups(axis_count: usize, demands: &[Demand]) -> Vec<Vec<usize>> {
    let mut groups = BTreeSet::new();
    for demand in demands {
        let distinct: BTreeSet<usize> = demand.axes.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            demand.axes.len(),
            "a demand names an axis twice"
        );
        assert!(
            demand.axes.iter().all(|&axis| axis < axis_count),
            "a demand names an axis the space does not have"
        );
        if demand.strength == 0 {
            continue;
        }
        for mut axes in subsets(&demand.axes, demand.strength) {
            axes.sort_unstable();
            groups.insert(axes);
        }
    }
    groups.into_iter().collect()
}

/// Returns every subset of `items` of `size` elements, in lexicographic
/// order of positions. There is none when `size` exceeds the items.
fn subsets(items: &[usize], size: usize) -> Vec<Vec<usize>> {
    if size > items.len() {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut positions: Vec<usize> = (0..size).collect();
    loop {
        result.push(positions.iter().map(|&position| items[position]).collect());
        // Advance the rightmost position that can still move.
        let Some(movable) = (0..size)
            .rev()
            .find(|&slot| positions[slot] < items.len() - size + slot)
        else {
            return result;
        };
        positions[movable] += 1;
        for slot in movable + 1..size {
            positions[slot] = positions[slot - 1] + 1;
        }
    }
}

/// A combination: the value each of some axes takes, sorted by axis.
type Combination = Vec<(usize, usize)>;

/// Returns the key a pair of `combination` is remembered by.
fn pair_key(combination: &[(usize, usize)]) -> [(usize, usize); 2] {
    [combination[0], combination[1]]
}

/// A set of axes whose combinations are counted, with the dense index of
/// each combination.
struct Group {
    /// The axes, sorted.
    axes: Vec<usize>,
    /// The weight of each axis's value in the index, the last axis
    /// weighing one.
    strides: Vec<usize>,
    /// The index of the group's first combination.
    offset: usize,
    /// The number of combinations.
    size: usize,
}

/// The status of every combination the demands name.
struct Ledger {
    groups: Vec<Group>,
    /// The group each combination belongs to.
    owner: Vec<u32>,
    status: Vec<u8>,
    /// The uncovered combinations of each group.
    open: Vec<usize>,
    /// For each group, the position below which no combination is
    /// uncovered.
    cursor: Vec<usize>,
    /// The groups that hold each axis.
    by_axis: Vec<Vec<usize>>,
}

impl Ledger {
    fn new(domains: &[usize], sets: &[Vec<usize>]) -> Self {
        let mut groups = Vec::with_capacity(sets.len());
        let mut owner = Vec::new();
        let mut by_axis = vec![Vec::new(); domains.len()];
        let mut offset = 0;
        for (group_index, axes) in sets.iter().enumerate() {
            let mut strides = vec![0; axes.len()];
            let mut size = 1;
            for (slot, &axis) in axes.iter().enumerate().rev() {
                strides[slot] = size;
                size *= domains[axis];
            }
            for &axis in axes {
                by_axis[axis].push(group_index);
            }
            owner.extend(std::iter::repeat_n(group_index as u32, size));
            groups.push(Group {
                axes: axes.clone(),
                strides,
                offset,
                size,
            });
            offset += size;
        }
        Ledger {
            open: groups.iter().map(|group| group.size).collect(),
            cursor: vec![0; groups.len()],
            status: vec![UNCOVERED; offset],
            groups,
            owner,
            by_axis,
        }
    }

    /// Returns the index of the combination of `group` that `row` holds.
    fn index(&self, group: &Group, row: &[usize]) -> usize {
        group.offset
            + group
                .axes
                .iter()
                .zip(&group.strides)
                .map(|(&axis, &stride)| row[axis] * stride)
                .sum::<usize>()
    }

    fn decode(&self, index: usize) -> Combination {
        let group = &self.groups[self.owner[index] as usize];
        let mut rest = index - group.offset;
        let mut combination = Vec::with_capacity(group.axes.len());
        for (&axis, &stride) in group.axes.iter().zip(&group.strides) {
            combination.push((axis, rest / stride));
            rest %= stride;
        }
        combination
    }

    fn strength_of(&self, index: usize) -> usize {
        self.groups[self.owner[index] as usize].axes.len()
    }

    fn set(&mut self, index: usize, status: u8) {
        if self.status[index] == UNCOVERED {
            self.open[self.owner[index] as usize] -= 1;
        }
        self.status[index] = status;
    }

    /// Marks every combination `row` holds as covered.
    fn cover(&mut self, row: &[usize]) {
        for group_index in 0..self.groups.len() {
            let index = self.index(&self.groups[group_index], row);
            if self.status[index] != COVERED {
                self.set(index, COVERED);
            }
        }
    }

    /// Returns the number of combinations `row` holds that no row holds
    /// yet, deferred ones included.
    fn fresh(&self, row: &[usize]) -> usize {
        self.groups
            .iter()
            .filter(|group| self.is_wanted(self.index(group, row)))
            .count()
    }

    fn is_wanted(&self, index: usize) -> bool {
        matches!(self.status[index], UNCOVERED | DEFERRED)
    }

    /// Returns the uncovered combination to seed the next row: the first
    /// of the group with the most uncovered combinations.
    fn next_uncovered(&mut self) -> Option<usize> {
        let mut best: Option<usize> = None;
        for (group_index, &open) in self.open.iter().enumerate() {
            if open == 0 {
                continue;
            }
            if best.is_none_or(|best| open > self.open[best]) {
                best = Some(group_index);
            }
        }
        let group_index = best?;
        let group = &self.groups[group_index];
        let found = (self.cursor[group_index]..group.size)
            .find(|&position| self.status[group.offset + position] == UNCOVERED)
            .expect("a group with an open combination holds one past its cursor");
        self.cursor[group_index] = found;
        Some(group.offset + found)
    }

    fn with_status(&self, status: u8) -> Vec<usize> {
        (0..self.status.len())
            .filter(|&index| self.status[index] == status)
            .collect()
    }

    /// Returns how many wanted combinations `axis` taking `value` adds to
    /// `row`, counting only the groups whose other axes are settled.
    fn gain(&self, row: &mut [usize], settled: &[bool], axis: usize, value: usize) -> usize {
        let previous = row[axis];
        row[axis] = value;
        let gain = self.by_axis[axis]
            .iter()
            .map(|&group_index| &self.groups[group_index])
            .filter(|group| {
                group
                    .axes
                    .iter()
                    .all(|&other| other == axis || settled[other])
            })
            .filter(|group| self.is_wanted(self.index(group, row)))
            .count();
        row[axis] = previous;
        gain
    }
}

/// The state of the search for valid rows.
struct Search<'s, 'a> {
    space: &'s Space<'a>,
    effort: Effort,
    rng: Rng,
    /// The valid rows known: the held rows and the rows chosen so far.
    known: Vec<Vec<usize>>,
    /// The pairs searched for on their own, and whether a valid row holds
    /// each.
    pairs: BTreeMap<[(usize, usize); 2], bool>,
}

impl Search<'_, '_> {
    /// Returns whether a pair within `combination` is remembered as
    /// unreached, which makes the combination unreached as well.
    fn has_failed_pair(&self, combination: &[(usize, usize)]) -> bool {
        if combination.len() < 3 {
            return false;
        }
        subsets(&(0..combination.len()).collect::<Vec<_>>(), 2)
            .iter()
            .any(|slots| {
                let key = [combination[slots[0]], combination[slots[1]]];
                self.pairs.get(&key) == Some(&false)
            })
    }

    /// Searches for every pair within a combination that could not be
    /// placed, so that the other combinations holding an unreached pair
    /// are settled without a search.
    fn probe_pairs(&mut self, combination: &[(usize, usize)]) {
        if combination.len() < 3 {
            return;
        }
        for slots in subsets(&(0..combination.len()).collect::<Vec<_>>(), 2) {
            let key = [combination[slots[0]], combination[slots[1]]];
            if self.pairs.contains_key(&key) {
                continue;
            }
            let reached = self.complete(&key).is_some();
            self.pairs.insert(key, reached);
        }
    }

    /// Returns a valid row holding `combination`, or `None` when the tries
    /// the effort allows find none.
    ///
    /// The tries are, in order: the base row; the base row with one other
    /// axis changed, every value of every axis; each known row; and then
    /// the base row or a known row with more and more other axes drawn at
    /// random.
    fn complete(&mut self, combination: &[(usize, usize)]) -> Option<Vec<usize>> {
        let space = self.space;
        let budget = self.effort.completions.max(1);
        let mut tries = 0;
        let attempt = |row: &[usize], tries: &mut usize| -> bool {
            *tries += 1;
            (space.valid)(row)
        };

        let mut row = space.base.to_vec();
        apply(&mut row, combination);
        if attempt(&row, &mut tries) {
            return Some(row);
        }

        let others: Vec<usize> = (0..space.domains.len())
            .filter(|axis| combination.iter().all(|&(fixed, _)| fixed != *axis))
            .filter(|&axis| space.domains[axis] > 1)
            .collect();
        for &axis in &others {
            let original = row[axis];
            for value in 0..space.domains[axis] {
                if value == original {
                    continue;
                }
                if tries >= budget {
                    return None;
                }
                row[axis] = value;
                if attempt(&row, &mut tries) {
                    return Some(row);
                }
            }
            row[axis] = original;
        }

        for known in &self.known {
            if tries >= budget {
                return None;
            }
            let mut candidate = known.clone();
            apply(&mut candidate, combination);
            if attempt(&candidate, &mut tries) {
                return Some(candidate);
            }
        }

        if others.is_empty() {
            return None;
        }
        let mut shuffled = others.clone();
        let mut round = 0;
        while tries < budget {
            let start = if self.known.is_empty() || self.rng.below(2) == 0 {
                space.base
            } else {
                &self.known[self.rng.below(self.known.len())]
            };
            let mut candidate = start.to_vec();
            apply(&mut candidate, combination);
            let changed = (2 + round / 128).min(shuffled.len());
            for slot in 0..changed {
                let pick = slot + self.rng.below(shuffled.len() - slot);
                shuffled.swap(slot, pick);
                let axis = shuffled[slot];
                candidate[axis] = self.rng.below(space.domains[axis]);
            }
            round += 1;
            if attempt(&candidate, &mut tries) {
                return Some(candidate);
            }
        }
        None
    }

    /// Returns the best of the candidate rows built from `start` by
    /// [`Search::greedy`].
    fn improve(
        &mut self,
        ledger: &Ledger,
        start: Vec<usize>,
        combination: &[(usize, usize)],
    ) -> Vec<usize> {
        let mut best: Option<(usize, Vec<usize>)> = None;
        for _ in 0..self.effort.candidates.max(1) {
            let row = self.greedy(ledger, start.clone(), combination);
            let fresh = ledger.fresh(&row);
            if best.as_ref().is_none_or(|(most, _)| fresh > *most) {
                best = Some((fresh, row));
            }
        }
        best.expect("at least one candidate is built").1
    }

    /// Settles the axes of `row` outside `combination` one at a time, in a
    /// seeded order, each taking the value that adds the most wanted
    /// combinations among the settled axes while the row stays valid.
    fn greedy(
        &mut self,
        ledger: &Ledger,
        mut row: Vec<usize>,
        combination: &[(usize, usize)],
    ) -> Vec<usize> {
        let domains = self.space.domains;
        let mut settled = vec![false; domains.len()];
        for &(axis, _) in combination {
            settled[axis] = true;
        }
        let mut order: Vec<usize> = (0..domains.len()).filter(|&axis| !settled[axis]).collect();
        self.rng.shuffle(&mut order);
        for &axis in &order {
            // The current value is valid, so the scan ends there at worst;
            // a value of the same gain before it is taken, which spreads
            // the values the gains do not tell apart.
            for (_, value) in self.ranked(ledger, &mut row, &settled, axis) {
                if self.try_value(&mut row, axis, value) {
                    break;
                }
            }
            settled[axis] = true;
        }

        // With every axis settled, the gains are exact: an axis changes
        // only to a value that adds more than its current one, until a
        // pass changes nothing.
        for _ in 0..REFINEMENTS {
            let mut changed = false;
            for &axis in &order {
                let current = row[axis];
                let current_gain = ledger.gain(&mut row, &settled, axis, current);
                for (gain, value) in self.ranked(ledger, &mut row, &settled, axis) {
                    if gain <= current_gain {
                        break;
                    }
                    if self.try_value(&mut row, axis, value) {
                        changed = true;
                        break;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        row
    }

    /// Returns each value of `axis` with its gain, the highest gain first
    /// and ties in a seeded order.
    fn ranked(
        &mut self,
        ledger: &Ledger,
        row: &mut [usize],
        settled: &[bool],
        axis: usize,
    ) -> Vec<(usize, usize)> {
        let mut values: Vec<usize> = (0..self.space.domains[axis]).collect();
        self.rng.shuffle(&mut values);
        let mut ranked: Vec<(usize, usize)> = values
            .into_iter()
            .map(|value| (ledger.gain(row, settled, axis, value), value))
            .collect();
        ranked.sort_by_key(|&(gain, _)| std::cmp::Reverse(gain));
        ranked
    }

    /// Sets `axis` to `value` when the row stays valid, and returns whether
    /// it did; the current value is kept without asking the predicate.
    fn try_value(&self, row: &mut [usize], axis: usize, value: usize) -> bool {
        let current = row[axis];
        if value == current {
            return true;
        }
        row[axis] = value;
        if (self.space.valid)(row) {
            return true;
        }
        row[axis] = current;
        false
    }
}

/// Sets the values `combination` names in `row`.
fn apply(row: &mut [usize], combination: &[(usize, usize)]) {
    for &(axis, value) in combination {
        row[axis] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// Returns every row of the product of `domains`.
    fn product(domains: &[usize]) -> Vec<Vec<usize>> {
        let mut rows = vec![Vec::new()];
        for &domain in domains {
            rows = rows
                .into_iter()
                .flat_map(|row| {
                    (0..domain).map(move |value| {
                        let mut next = row.clone();
                        next.push(value);
                        next
                    })
                })
                .collect();
        }
        rows
    }

    fn all(axes: usize, strength: usize) -> Demand {
        Demand {
            axes: (0..axes).collect(),
            strength,
        }
    }

    /// Covers a small space and checks the result against brute force:
    /// every row valid, every combination some valid row holds covered,
    /// and the unreached count exactly the combinations no valid row
    /// holds.
    fn check(
        domains: &[usize],
        base: &[usize],
        valid: &dyn Fn(&[usize]) -> bool,
        demands: &[Demand],
        held: &[Vec<usize>],
        seed: u64,
    ) -> Covering {
        let space = Space {
            domains,
            base,
            valid,
        };
        let covering = cover(&space, demands, held, seed, Effort::default());
        assert!(covering.rows.iter().all(|row| valid(row)));
        assert!(covering.rows.iter().all(|row| !held.contains(row)));
        let product = product(domains);
        let reachable: Vec<Vec<usize>> = product.iter().filter(|row| valid(row)).cloned().collect();
        let mut rows = held.to_vec();
        rows.extend(covering.rows.iter().cloned());
        assert_eq!(missing(demands, &rows, &reachable), 0);
        let total = missing(demands, &[], &product);
        let unreachable = total - missing(demands, &[], &reachable);
        assert_eq!(covering.unreached, unreachable);
        assert_eq!(covering.covered, total - unreachable);
        covering
    }

    #[test]
    fn subsets_are_every_choice_once() {
        assert_eq!(
            subsets(&[4, 5, 6, 7], 2),
            vec![
                vec![4, 5],
                vec![4, 6],
                vec![4, 7],
                vec![5, 6],
                vec![5, 7],
                vec![6, 7]
            ]
        );
        assert_eq!(subsets(&[1, 2, 3, 4, 5], 3).len(), 10);
        assert!(subsets(&[1], 2).is_empty());
    }

    #[test]
    fn missing_counts_against_brute_force() {
        let domains = [2, 2, 2];
        let product = product(&domains);
        assert_eq!(missing(&[all(3, 2)], &[], &product), 12);
        assert_eq!(missing(&[all(3, 2)], &[vec![0, 0, 0]], &product), 9);
        assert_eq!(missing(&[all(3, 3)], &product, &product), 0);
    }

    #[test]
    fn covering_unconstrained_pairs_and_triples() {
        let valid = |_: &[usize]| true;
        for (domains, strength) in [
            (vec![2, 3, 4, 2, 3], 2),
            (vec![3; 6], 2),
            (vec![2, 3, 2, 4, 3], 3),
            (vec![3; 5], 3),
        ] {
            let base = vec![0; domains.len()];
            let covering = check(
                &domains,
                &base,
                &valid,
                &[all(domains.len(), strength)],
                &[],
                7,
            );
            assert_eq!(covering.unreached, 0);
        }
    }

    #[test]
    fn covering_ten_axes_of_four_values_is_small() {
        let domains = [4; 10];
        let base = [0; 10];
        let valid = |_: &[usize]| true;
        let space = Space {
            domains: &domains,
            base: &base,
            valid: &valid,
        };
        let covering = cover(&space, &[all(10, 2)], &[], 1, Effort::default());
        println!("ten axes of four values: {} rows", covering.rows.len());
        assert!(covering.rows.len() <= 30);
        // Every pair placed in an otherwise neutral row: every pair is
        // reachable, without the million rows of the product.
        let mut reachable = Vec::new();
        for first in 0..10 {
            for second in first + 1..10 {
                for (left, right) in product(&[4, 4]).iter().map(|pair| (pair[0], pair[1])) {
                    let mut row = vec![0; 10];
                    row[first] = left;
                    row[second] = right;
                    reachable.push(row);
                }
            }
        }
        assert_eq!(missing(&[all(10, 2)], &covering.rows, &reachable), 0);
        assert_eq!((covering.covered, covering.unreached), (720, 0));
    }

    /// The constraints of the small constrained space: an enabler, an
    /// exclusion and a three-axis constraint.
    fn constrained(row: &[usize]) -> bool {
        // Value 2 of axis 0 only with value 1 of axis 1.
        if row[0] == 2 && row[1] != 1 {
            return false;
        }
        // Value 0 of axis 2 never with value 3 of axis 3.
        if row[2] == 0 && row[3] == 3 {
            return false;
        }
        // Value 1 of axis 4 and value 1 of axis 5 exclude value 2 of axis 3.
        if row[4] == 1 && row[5] == 1 && row[3] == 2 {
            return false;
        }
        // Value 3 of axis 3 needs value 2 of axis 0, which needs value 1 of
        // axis 1: a chain two changes away from the base.
        if row[3] == 3 && row[0] != 2 {
            return false;
        }
        true
    }

    #[test]
    fn covering_constrained_matches_brute_force() {
        let domains = [3, 3, 2, 4, 2, 3];
        let base = [0; 6];
        assert!(constrained(&base));
        for strength in [2, 3] {
            for seed in [0, 1, 2] {
                check(
                    &domains,
                    &base,
                    &constrained,
                    &[all(6, strength)],
                    &[],
                    seed,
                );
            }
        }
    }

    #[test]
    fn covering_reports_an_unreachable_value() {
        let domains = [3, 3, 3, 3];
        let base = [0; 4];
        // Value 2 of axis 0 is never valid, so every pair and triple
        // holding it is unreached, and pruning settles the triples.
        let valid = |row: &[usize]| row[0] != 2 && !(row[1] == 1 && row[2] == 1);
        let covering = check(&domains, &base, &valid, &[all(4, 3)], &[], 3);
        assert!(covering.unreached > 0);
        check(&domains, &base, &valid, &[all(4, 2)], &[], 3);
    }

    #[test]
    fn covering_is_deterministic() {
        let domains = [3, 3, 2, 4, 2, 3];
        let base = [0; 6];
        let space = Space {
            domains: &domains,
            base: &base,
            valid: &constrained,
        };
        let demands = [
            all(6, 2),
            Demand {
                axes: vec![0, 1, 3, 5],
                strength: 3,
            },
        ];
        let first = cover(&space, &demands, &[], 11, Effort::default());
        let second = cover(&space, &demands, &[], 11, Effort::default());
        assert_eq!(first.rows, second.rows);
        assert_eq!(first.unreached, second.unreached);
        let other = check(&domains, &base, &constrained, &demands, &[], 12);
        assert_eq!(other.unreached, first.unreached);
    }

    #[test]
    fn covering_counts_held_rows() {
        let domains = [3, 3, 2, 4, 2, 3];
        let base = [0; 6];
        let demands = [all(6, 2)];
        let alone = check(&domains, &base, &constrained, &demands, &[], 5);
        let held: Vec<Vec<usize>> = alone.rows[..alone.rows.len() / 2].to_vec();
        let with_held = check(&domains, &base, &constrained, &demands, &held, 5);
        assert!(with_held.rows.len() < alone.rows.len());
        assert!(with_held.rows.iter().all(|row| !held.contains(row)));
    }

    #[test]
    fn covering_mixes_strengths() {
        let domains = [3, 2, 4, 3, 2, 3, 2];
        let base = [0; 7];
        let valid = |row: &[usize]| !(row[2] == 3 && row[4] == 1) && (row[0] != 1 || row[1] == 1);
        let demands = [
            all(7, 2),
            Demand {
                axes: vec![5, 0, 2, 3],
                strength: 3,
            },
        ];
        let covering = check(&domains, &base, &valid, &demands, &[], 9);
        let pairs_only = check(&domains, &base, &valid, &[all(7, 2)], &[], 9);
        assert!(covering.rows.len() > pairs_only.rows.len());
    }

    /// The performance case: eighteen axes, a dozen constraints of the
    /// kinds the model has, pairs over all and triples over eight.
    #[test]
    #[ignore = "a measurement; run in release"]
    fn covering_performance() {
        let domains = [3, 4, 5, 6, 25, 8, 3, 4, 12, 5, 3, 7, 4, 6, 10, 3, 5, 4];
        let base = [0; 18];
        let valid = |row: &[usize]| {
            let rules = [
                // Enablers: a value needs a value of another axis.
                row[0] != 2 || row[1] == 3,
                row[4] < 20 || row[5] == 7,
                row[8] < 10 || row[2] != 0,
                row[11] != 6 || row[10] == 2,
                row[14] != 9 || row[13] >= 3,
                // Exclusions.
                !(row[3] == 5 && row[6] == 1),
                !(row[7] == 2 && row[9] == 4),
                !(row[12] == 3 && row[15] == 2),
                !(row[16] == 4 && row[17] == 3),
                // Three-axis constraints.
                !(row[4] == 3 && row[8] == 4 && row[14] == 2),
                !(row[0] == 1 && row[2] == 1) || row[6] == 0,
                // With the first rule, a chain: value 2 of axis 0 needs
                // value 3 of axis 1 and value 0 of axis 17.
                row[0] != 2 || row[17] == 0,
            ];
            rules.iter().all(|&rule| rule)
        };
        assert!(valid(&base));
        let space = Space {
            domains: &domains,
            base: &base,
            valid: &valid,
        };
        let demands = [
            all(18, 2),
            Demand {
                axes: vec![0, 1, 2, 3, 6, 8, 11, 14],
                strength: 3,
            },
        ];
        let started = Instant::now();
        let covering = cover(&space, &demands, &[], 1, Effort::default());
        let elapsed = started.elapsed();
        println!(
            "eighteen axes: {} rows, {} covered, {} unreached, {:.2?}",
            covering.rows.len(),
            covering.covered,
            covering.unreached,
            elapsed
        );
        assert!(covering.rows.iter().all(|row| valid(row)));

        // A search fifty times harder reaches nothing more, so what was
        // counted unreached is unreached.
        let thorough = Effort {
            completions: 100_000,
            ..Effort::default()
        };
        let harder = cover(&space, &demands, &[], 2, thorough);
        println!(
            "with {} completions: {} rows, {} unreached",
            thorough.completions,
            harder.rows.len(),
            harder.unreached
        );
        assert_eq!(harder.unreached, covering.unreached);
    }
}
