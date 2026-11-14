//! A cell: one value per axis, with the id that spells it.
//!
//! The id spells the axes whose value differs from the neutral cell's, as
//! space-separated `name=word` pairs in axis order; the neutral cell is
//! `base`. An axis added later takes its neutral value in every id written
//! before it, so an id stays valid. A stable hash of the id names the
//! cell's module and file.

use crate::axis::Axis;
use crate::model::{Argument, Qualifier, Receiver, Return, Second, Target};

/// The id of the neutral cell.
pub const BASE: &str = "base";

/// An axis as the cell sees it: its name in ids and the words of its
/// values.
#[derive(Clone, Copy, Debug)]
pub struct AxisInfo {
    /// The axis's name in ids.
    pub name: &'static str,
    /// What the axis is.
    pub summary: &'static str,
    /// Returns the words of its values, the neutral first.
    pub words: fn() -> Vec<&'static str>,
}

/// Why an id does not parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError(pub String);

impl core::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

macro_rules! cell {
    (
        $(
            #[doc = $summary:literal]
            $field:ident: $axis:ty,
        )+
    ) => {
        /// One value per axis.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct Cell {
            $(
                #[doc = $summary]
                pub $field: $axis,
            )+
        }

        impl Cell {
            /// The axes, in id order.
            pub const AXES: &'static [AxisInfo] = &[
                $(AxisInfo {
                    name: stringify!($field),
                    summary: $summary,
                    words: <$axis as Axis>::words,
                },)+
            ];

            /// Returns the neutral cell, which every id is spelled against.
            pub fn neutral() -> Cell {
                Cell {
                    $($field: <$axis as Axis>::neutral(),)+
                }
            }

            /// Returns the cell's row: the index of its value on each axis.
            pub fn row(&self) -> Vec<usize> {
                vec![$(self.$field.index(),)+]
            }

            /// Returns the cell a row describes, or `None` when the row has
            /// the wrong length or an index past its axis.
            pub fn from_row(row: &[usize]) -> Option<Cell> {
                if row.len() != Cell::AXES.len() {
                    return None;
                }
                let mut indices = row.iter().copied();
                Some(Cell {
                    $($field: *<$axis as Axis>::VALUES.get(indices.next()?)?,)+
                })
            }

            /// Returns the cell's id.
            pub fn id(&self) -> String {
                let neutral = Cell::neutral();
                let mut parts: Vec<String> = Vec::new();
                $(
                    if self.$field != neutral.$field {
                        parts.push(format!("{}={}", stringify!($field), self.$field.word()));
                    }
                )+
                if parts.is_empty() {
                    return String::from(BASE);
                }
                parts.join(" ")
            }

            /// Returns the cell an id spells. The pairs may come in any
            /// order and may name a neutral value; an unknown name or word,
            /// or a name given twice, is an error.
            pub fn parse(id: &str) -> Result<Cell, ParseError> {
                let mut cell = Cell::neutral();
                let id = id.trim();
                if id == BASE {
                    return Ok(cell);
                }
                if id.is_empty() {
                    return Err(ParseError(String::from("an empty id; the neutral cell is `base`")));
                }
                let mut seen: Vec<&str> = Vec::new();
                for part in id.split_whitespace() {
                    let Some((name, word)) = part.split_once('=') else {
                        return Err(ParseError(format!(
                            "`{part}` is not a `name=word` pair"
                        )));
                    };
                    if seen.contains(&name) {
                        return Err(ParseError(format!("`{name}` is given twice")));
                    }
                    seen.push(name);
                    match name {
                        $(
                            stringify!($field) => {
                                cell.$field = <$axis as Axis>::from_word(word).ok_or_else(|| {
                                    ParseError(format!(
                                        "`{word}` is no value of `{name}`; its values are {}",
                                        <$axis as Axis>::words().join(", ")
                                    ))
                                })?;
                            }
                        )+
                        _ => {
                            let names: Vec<&str> = Cell::AXES.iter().map(|axis| axis.name).collect();
                            return Err(ParseError(format!(
                                "`{name}` is no axis; the axes are {}",
                                names.join(", ")
                            )));
                        }
                    }
                }
                Ok(cell)
            }
        }
    };
}

cell! {
    /// The first variant's target kind.
    s0: Target,
    /// The second variant.
    s1: Second,
    /// The receiver of the method under test.
    recv: Receiver,
    /// The return type of the method under test.
    ret: Return,
    /// The method's `Self` argument and where its mismatch is spelled.
    arg: Argument,
    /// What qualifies the method.
    qual: Qualifier,
}

impl Cell {
    /// Returns the stable name of the cell's module and file:
    /// `c_` and the first twelve hex digits of the id's FNV-1a hash.
    pub fn module(&self) -> String {
        module_of(&self.id())
    }

    /// Returns the stable 64-bit hash of the cell's id.
    pub fn hash(&self) -> u64 {
        fnv1a(self.id().as_bytes())
    }
}

/// Returns the module name of an id.
pub fn module_of(id: &str) -> String {
    format!("c_{:012x}", fnv1a(id.as_bytes()) >> 16)
}

/// FNV-1a, 64 bits.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every single-axis departure from the neutral cell.
    fn departures() -> Vec<Cell> {
        let neutral = Cell::neutral().row();
        let mut cells = Vec::new();
        for (axis, info) in Cell::AXES.iter().enumerate() {
            for index in 0..(info.words)().len() {
                let mut row = neutral.clone();
                row[axis] = index;
                cells.push(Cell::from_row(&row).expect("an in-range row"));
            }
        }
        cells
    }

    #[test]
    fn the_neutral_cell_is_base() {
        assert_eq!(Cell::neutral().id(), BASE);
        assert_eq!(Cell::parse(BASE), Ok(Cell::neutral()));
    }

    #[test]
    fn every_value_round_trips_through_its_id() {
        for cell in departures() {
            let id = cell.id();
            assert_eq!(Cell::parse(&id), Ok(cell), "{id}");
            assert_eq!(Cell::from_row(&cell.row()), Some(cell), "{id}");
        }
    }

    #[test]
    fn a_cell_of_several_departures_round_trips() {
        let id = "s0=deref-box s1=none recv=pin-mut ret=option-self arg=ref-enum qual=async";
        let cell = Cell::parse(id).expect("a valid id");
        assert_eq!(cell.id(), id);
    }

    #[test]
    fn an_id_names_only_what_departs_in_axis_order() {
        let cell = Cell::parse("recv=mut s0=via-field s1=plain").expect("a valid id");
        assert_eq!(cell.id(), "s0=via-field recv=mut");
    }

    #[test]
    fn words_are_unique_kebab_case() {
        for info in Cell::AXES {
            let words = (info.words)();
            for (index, word) in words.iter().enumerate() {
                assert!(
                    !word.is_empty()
                        && word
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                    "`{word}` of `{}`",
                    info.name
                );
                assert!(
                    !words[..index].contains(word),
                    "`{word}` twice in `{}`",
                    info.name
                );
            }
        }
    }

    #[test]
    fn unknown_parts_are_errors() {
        for id in [
            "",
            "s0",
            "s0=box",
            "shape=plain",
            "recv=ref recv=mut",
            "base recv=mut",
        ] {
            assert!(Cell::parse(id).is_err(), "`{id}` parsed");
        }
        let error = Cell::parse("recv=boxed").unwrap_err();
        assert!(error.0.contains("its values are ref, mut"), "{error}");
        let error = Cell::parse("receiver=box").unwrap_err();
        assert!(error.0.contains("the axes are s0, s1"), "{error}");
    }

    #[test]
    fn rows_out_of_range_are_rejected() {
        let mut row = Cell::neutral().row();
        row[0] = 1000;
        assert_eq!(Cell::from_row(&row), None);
        assert_eq!(Cell::from_row(&[0]), None);
    }

    #[test]
    fn the_module_name_is_a_stable_hash_of_the_id() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        let module = Cell::neutral().module();
        assert_eq!(module.len(), 14);
        assert_eq!(module, module_of(BASE));
    }
}
