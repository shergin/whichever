//! Suites: which cells a run holds.
//!
//! A zone is the full product of a few axes over a fixed base, declared as
//! data. The pinned cells and the known divergences are suites of their
//! own, always selected. Selection is a function of the model: the same
//! model gives the same cells in the same order.

pub mod covering;
pub mod rng;
mod zones;

use std::collections::BTreeMap;

use crate::cell::Cell;
use crate::model::Verdict;
pub use zones::ZONES;

/// A full product of a few axes over a fixed base.
#[derive(Clone, Copy, Debug)]
pub struct Zone {
    /// The suite's name.
    pub name: &'static str,
    /// What the zone is for.
    pub summary: &'static str,
    /// The axes that vary, each with the words it takes; an empty list
    /// takes every value.
    pub vary: &'static [(&'static str, &'static [&'static str])],
    /// The values the other axes hold where they are not neutral.
    pub fixed: &'static [(&'static str, &'static str)],
}

/// Why a zone does not describe cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZoneError(pub String);

impl core::fmt::Display for ZoneError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Zone {
    /// Returns every combination the zone spans, selected or not, in a
    /// fixed order: the last axis varies fastest.
    pub fn combinations(&self) -> Result<Vec<Cell>, ZoneError> {
        let mut base = Cell::neutral().row();
        for (name, word) in self.fixed {
            let (axis, index) = locate(self.name, name, word)?;
            base[axis] = index;
        }
        let mut rows = vec![base];
        for (name, words) in self.vary {
            let axis = axis_index(self.name, name)?;
            let indices: Vec<usize> = if words.is_empty() {
                (0..(Cell::AXES[axis].words)().len()).collect()
            } else {
                words
                    .iter()
                    .map(|word| locate(self.name, name, word).map(|(_, index)| index))
                    .collect::<Result<_, _>>()?
            };
            rows = rows
                .into_iter()
                .flat_map(|row| {
                    indices.iter().map(move |index| {
                        let mut row = row.clone();
                        row[axis] = *index;
                        row
                    })
                })
                .collect();
        }
        Ok(rows
            .iter()
            .map(|row| Cell::from_row(row).expect("rows built from the axes"))
            .collect())
    }
}

fn axis_index(zone: &str, name: &str) -> Result<usize, ZoneError> {
    Cell::AXES
        .iter()
        .position(|axis| axis.name == name)
        .ok_or_else(|| ZoneError(format!("zone `{zone}` names no axis `{name}`")))
}

fn locate(zone: &str, name: &str, word: &str) -> Result<(usize, usize), ZoneError> {
    let axis = axis_index(zone, name)?;
    let index = (Cell::AXES[axis].words)()
        .iter()
        .position(|candidate| candidate == &word)
        .ok_or_else(|| ZoneError(format!("zone `{zone}`: `{word}` is no value of `{name}`")))?;
    Ok((axis, index))
}

/// The cells a suite holds, and the combinations it spans but does not
/// select, by reason.
#[derive(Clone, Debug, Default)]
pub struct Selection {
    /// The suite's name.
    pub suite: String,
    /// The selected cells, in order.
    pub cells: Vec<Cell>,
    /// The combinations not selected, counted by reason.
    pub never: BTreeMap<String, usize>,
}

impl Selection {
    /// Returns the selected cells counted by verdict kind.
    pub fn by_verdict(&self) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        for cell in &self.cells {
            *counts.entry(cell.verdict().kind()).or_insert(0) += 1;
        }
        counts
    }
}

/// Returns the selection of a list of combinations: the selectable cells,
/// and the others counted by reason.
pub fn select(suite: &str, combinations: Vec<Cell>) -> Selection {
    let mut selection = Selection {
        suite: String::from(suite),
        ..Selection::default()
    };
    for cell in combinations {
        let verdict = cell.verdict();
        if verdict.is_selectable() {
            if !selection.cells.contains(&cell) {
                selection.cells.push(cell);
            }
            continue;
        }
        let reason = match verdict {
            Verdict::Unmodelled(reason) => format!("unmodelled: {reason}"),
            Verdict::NonCanonical => String::from("non-canonical"),
            Verdict::Pass | Verdict::Limit(_) | Verdict::Malformed(_) => {
                unreachable!("a selectable verdict is selected")
            }
        };
        *selection.never.entry(reason).or_insert(0) += 1;
    }
    selection
}

/// Returns the selection of every zone.
pub fn zones() -> Result<Vec<Selection>, ZoneError> {
    ZONES
        .iter()
        .map(|zone| Ok(select(zone.name, zone.combinations()?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_are_deterministic_and_not_empty() {
        let first = zones().expect("valid zones");
        let second = zones().expect("valid zones");
        assert_eq!(first.len(), ZONES.len());
        for (one, other) in first.iter().zip(&second) {
            assert!(
                !one.cells.is_empty(),
                "zone `{}` selects nothing",
                one.suite
            );
            assert_eq!(one.cells, other.cells);
            assert_eq!(one.never, other.never);
        }
    }

    #[test]
    fn zone_names_are_unique() {
        let mut names: Vec<&str> = ZONES.iter().map(|zone| zone.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), ZONES.len());
        assert!(!names.contains(&"pinned") && !names.contains(&"known"));
    }

    #[test]
    fn a_zone_naming_nothing_is_an_error() {
        let zone = Zone {
            name: "broken",
            summary: "",
            vary: &[("receiver", &[])],
            fixed: &[],
        };
        assert!(zone.combinations().is_err());
        let zone = Zone {
            name: "broken",
            summary: "",
            vary: &[("recv", &["boxed"])],
            fixed: &[],
        };
        assert!(zone.combinations().is_err());
    }

    #[test]
    fn a_zone_is_the_full_product() {
        let zone = Zone {
            name: "product",
            summary: "",
            vary: &[
                ("recv", &["ref", "mut"]),
                ("s0", &["plain", "deref-box", "via-field"]),
            ],
            fixed: &[("s1", "none")],
        };
        let cells = zone.combinations().expect("a valid zone");
        assert_eq!(cells.len(), 6);
        assert_eq!(cells[0].id(), "s1=none");
        assert_eq!(cells[5].id(), "s0=via-field s1=none recv=mut");
    }
}
