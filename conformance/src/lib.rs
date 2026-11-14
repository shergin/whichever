//! The conformance matrix of `whichever`.
//!
//! The hand-picked tests of the facade pin points: one behaviour module per
//! capability, one golden per expansion worth reading, one compile-fail
//! case per sentence. The defects the surveys found sat between those
//! points, where two or three features meet. This crate tests the space
//! between them: it holds a model of the supported features, generates a
//! small program per combination, and judges every program by checks that
//! need no expectation written by hand.
//!
//! # The words
//!
//! - An **axis** is one feature with its values: the receiver, the return
//!   shape, how a variant reaches its target. Each is an enum in [`model`].
//! - A **cell** is one value per axis ([`cell::Cell`]). Its id spells the
//!   values that differ from the neutral cell, so an id written today stays
//!   valid when an axis is added.
//! - The **verdict** is what the model says a cell is: it passes, it is a
//!   limit answered by named sentences, or it is not Rust at all
//!   ([`model::Verdict`]). The limits table is `docs/limits.md` made
//!   executable, and every entry names its `tests/ui` case.
//! - The **outcome** is what a run observed for a cell ([`run::Outcome`]).
//!   A cell whose outcome is not its verdict has **diverged**.
//! - A **suite** is a selection of cells ([`select`]): a zone, which is the
//!   full product of a few axes; a covering array, in which every
//!   combination of values of any few axes meets in some cell; the pinned
//!   cells; or seeded random cells.
//!
//! # The checks
//!
//! A cell is rendered ([`render`]) as one source file holding the trait,
//! the payloads, the subject spelled with the attributes, the same subject
//! with the forwarding written by hand, and a test. Four checks follow.
//!
//! 1. **The verdict.** A passing cell compiles without a diagnostic. A
//!    limit is answered by its sentences on the named items and by nothing
//!    else, so the item is still emitted and no error cascades. A rustc
//!    error on generated code, a sentence the model does not predict, or a
//!    pass where a limit is predicted, is a divergence.
//! 2. **The law.** Forwarding commutes with the variant's constructor:
//!    calling the method on the subject holding a payload answers what
//!    calling it on that payload answers, converted as the return type
//!    asks, and leaves the payload in the same state. The payloads are
//!    generated to answer with who they are and what they were given. The
//!    law is checked on the attribute's subject and on the hand's, so a
//!    failure says which of the two is wrong.
//! 3. **The shape.** The expansion performs the operations the hand
//!    performs, arm by arm and in the same order, and nothing else: no
//!    closure, no call the hand does not make ([`shape`]).
//! 4. **The census.** The number of cells each suite holds, by verdict, is
//!    pinned in `census.txt`, so a constraint that silently empties a suite
//!    shows as a diff ([`ledger`]).
//!
//! A divergence that is a known defect is a line of `known.tsv` with its
//! reason. Such a line fails once the cell stops diverging, so a fix cannot
//! land without removing it.
//!
//! # The rules of this crate
//!
//! - The hand is written from the public documents, `docs/` and the
//!   goldens of `tests/expand`, never from the macro crate's sources. A
//!   model that mirrors the implementation agrees with its defects.
//! - A divergence is settled in one of three ways: the model was wrong and
//!   is corrected; the macro is wrong and the cell goes to `known.tsv`
//!   with a note; or a limit was missing from the table and is added with
//!   its `tests/ui` case. The hand is never bent to the macro to make a
//!   cell pass.
//! - Adding a value to an axis is adding a variant to its enum. Every
//!   `match` over an axis is exhaustive, with no wildcard arm, so the
//!   compiler asks for the spelling, the hand's spelling and the verdict of
//!   the new value.
//! - Selection is deterministic: the same model gives the same cells.

pub mod axis;
pub mod cell;
pub mod ledger;
pub mod model;
pub mod render;
pub mod run;
pub mod select;
pub mod shape;
