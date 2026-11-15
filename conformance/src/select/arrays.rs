//! The covering-array suites: within each class of verdict, every
//! combination of values of any few axes meets in some cell.
//!
//! A class is a predicate on the verdict, and only a cell of the class may
//! be held, so a combination no cell of the class holds is not required.
//! The zones and the pinned cells are held already; the array adds what
//! they miss.

use std::cell::RefCell;
use std::collections::HashMap;

use super::covering::{self, Demand, Effort, Space};
use super::{select, Selection};
use crate::cell::Cell;
use crate::model::Verdict;

/// The seed every array is built with, so that the same model gives the
/// same cells.
const SEED: u64 = 0x5eed_c0de;

/// The cells an array suite holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Cells that pass.
    Pass,
    /// Limits answered by exactly one sentence.
    SingleLimit,
    /// Rejected cells.
    Rejected,
}

impl Class {
    /// Returns whether a verdict is of the class.
    pub fn holds(self, verdict: &Verdict) -> bool {
        match (self, verdict) {
            (Class::Pass, Verdict::Pass) => true,
            (Class::SingleLimit, Verdict::Limit(expected)) => expected.len() == 1,
            (Class::Rejected, Verdict::Rejected { .. }) => true,
            (
                Class::Pass | Class::SingleLimit | Class::Rejected,
                Verdict::Pass
                | Verdict::Limit(_)
                | Verdict::Rejected { .. }
                | Verdict::Malformed(_)
                | Verdict::Unmodelled(_)
                | Verdict::NonCanonical,
            ) => false,
        }
    }
}

/// An array suite.
#[derive(Clone, Copy, Debug)]
pub struct Array {
    /// The suite's name.
    pub name: &'static str,
    /// The class of its cells.
    pub class: Class,
    /// What it covers: the axes, by name, and how many of them meet in
    /// every combination; no axis named is every axis.
    pub demands: &'static [(&'static [&'static str], usize)],
}

/// The axes whose triples the passing cells cover: how a variant reaches
/// its target, how the method is unwrapped, answered and reached, and the
/// variants' `#[cfg]`. Every axis meets every other in pairs; strength 3
/// over every axis grows with the product of the axes added, and the
/// meetings of three the zones do not hold are these axes'.
const STRONG: &[&str] = &[
    "path", "callee", "lift", "s0", "s1", "recv", "ret", "arg", "cfg",
];

/// Every array suite.
pub const ARRAYS: &[Array] = &[
    Array {
        name: "array",
        class: Class::Pass,
        demands: &[(STRONG, 3), (&[], 2)],
    },
    Array {
        name: "array-limits",
        class: Class::SingleLimit,
        demands: &[(&[], 2)],
    },
    Array {
        name: "array-rejected",
        class: Class::Rejected,
        demands: &[(&[], 2)],
    },
];

/// Returns an array's demands, the axes named by index.
fn demands(array: &Array) -> Vec<Demand> {
    array
        .demands
        .iter()
        .map(|(names, strength)| {
            let axes = match names.is_empty() {
                true => (0..Cell::AXES.len()).collect(),
                false => names
                    .iter()
                    .map(|name| {
                        Cell::AXES
                            .iter()
                            .position(|axis| axis.name == *name)
                            .expect("an array names the cells' axes")
                    })
                    .collect(),
            };
            Demand {
                axes,
                strength: *strength,
            }
        })
        .collect()
}

/// What an array's covering found, for the census.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    /// The rows the array adds to the held ones.
    pub rows: usize,
    /// The combinations some cell of the class, held or added, holds.
    pub covered: usize,
    /// The combinations the search found no cell of the class for.
    pub unreached: usize,
}

/// Returns a cell of the class to start every search from: the first, in
/// order, of the neutral cell, its departures on one axis, and the held
/// cells.
fn base(class: Class, held: &[Cell]) -> Option<Cell> {
    let neutral = Cell::neutral();
    let mut candidates = vec![neutral];
    let row = neutral.row();
    for (axis, info) in Cell::AXES.iter().enumerate() {
        for index in 1..(info.words)().len() {
            let mut departed = row.clone();
            departed[axis] = index;
            candidates.extend(Cell::from_row(&departed));
        }
    }
    candidates.extend_from_slice(held);
    candidates
        .into_iter()
        .find(|cell| class.holds(&cell.verdict()))
}

/// Returns the selection of an array, holding `held`, the cells the other
/// suites select, and its coverage.
pub fn build(array: &Array, held: &[Cell]) -> (Selection, Coverage) {
    let held: Vec<Cell> = held
        .iter()
        .copied()
        .filter(|cell| array.class.holds(&cell.verdict()))
        .collect();
    let Some(base) = base(array.class, &held) else {
        return (select(array.name, Vec::new()), Coverage::default());
    };
    let domains: Vec<usize> = Cell::AXES.iter().map(|axis| (axis.words)().len()).collect();
    let memo: RefCell<HashMap<Vec<usize>, bool>> = RefCell::new(HashMap::new());
    let valid = |row: &[usize]| {
        if let Some(known) = memo.borrow().get(row) {
            return *known;
        }
        let holds = Cell::from_row(row).is_some_and(|cell| array.class.holds(&cell.verdict()));
        memo.borrow_mut().insert(row.to_vec(), holds);
        holds
    };
    let base_row = base.row();
    let space = Space {
        domains: &domains,
        base: &base_row,
        valid: &valid,
    };
    let demands = demands(array);
    let held_rows: Vec<Vec<usize>> = held.iter().map(Cell::row).collect();
    let covering = covering::cover(&space, &demands, &held_rows, SEED, Effort::default());
    let cells: Vec<Cell> = covering
        .rows
        .iter()
        .map(|row| Cell::from_row(row).expect("rows of the cells' axes"))
        .collect();
    let coverage = Coverage {
        rows: covering.rows.len(),
        covered: covering.covered,
        unreached: covering.unreached,
    };
    (select(array.name, cells), coverage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_array_names_axes_that_exist() {
        for array in ARRAYS {
            for demand in demands(array) {
                assert!(!demand.axes.is_empty() && demand.strength <= demand.axes.len());
            }
        }
    }

    #[test]
    fn every_class_has_a_base_cell() {
        for class in [Class::Pass, Class::SingleLimit, Class::Rejected] {
            let base = base(class, &[]).expect("a cell of the class");
            assert!(class.holds(&base.verdict()));
        }
    }
}
