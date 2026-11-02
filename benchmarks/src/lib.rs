//! What the three benchmark binaries share: the collection size, the seed,
//! the criterion configuration, the loops that are timed and the macro that
//! names every subject.
//!
//! The loops are generic over [`Dispatch`], so each subject gets its own
//! monomorphised copy with nothing between the loop and the strategy's method
//! but an `#[inline(always)]` adapter. They live here, one crate away from
//! the enums, on purpose; see the `dispatch-subjects` crate docs.

use std::time::Duration;

use criterion::Criterion;
use dispatch_subjects::{Dispatch, Resize};

/// Elements per collection in the dispatch and method benchmarks: 24 KiB of
/// enums, comfortably inside L1, so the loop measures dispatch and not memory.
pub const LEN: usize = 1024;

/// The seed every collection is built from.
pub const SEED: u64 = 0x005E_ED0F_D159_A7C4;

/// Shorter than criterion's defaults, because the full matrix has a few
/// hundred cells; `cargo bench -- --warm-up-time 3 --measurement-time 5`
/// restores them, and `--quick` goes faster still. Ten thousand bootstrap
/// resamples instead of a hundred thousand keep the analysis of each cell
/// well under the time spent measuring it; the confidence intervals in the
/// report are not visibly affected.
pub fn criterion() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .nresamples(10_000)
}

/// Calls a `&self` method on every element and folds the results, so no
/// call can be dropped and the work per element stays a wrapping add.
#[inline]
pub fn sum_area<T: Dispatch>(items: &[T]) -> u64 {
    items
        .iter()
        .fold(0u64, |acc, item| acc.wrapping_add(item.area()))
}

/// Calls a `&mut self` method on every element.
#[inline]
pub fn scale_all<T: Dispatch>(items: &mut [T], factor: u64) {
    for item in items {
        item.scale(factor);
    }
}

/// Calls the method with a default body on every element.
#[inline]
pub fn total_name_len<T: Dispatch>(items: &[T]) -> usize {
    items.iter().map(|item| item.name().len()).sum()
}

/// Calls the `-> Self` method on every element, writing into a reused buffer
/// so no allocation is timed.
#[inline]
pub fn scaled_into<T: Resize>(items: &[T], factor: u64, out: &mut Vec<T>) {
    out.clear();
    out.extend(items.iter().map(|item| item.scaled(factor)));
}

/// Runs `$body` once per subject with `$T` bound to that subject's type at
/// the given family and variant count: `for_each_subject!(opaque::Any8, |T|
/// run::<T>(..))`. The order here is the order of the rows in the reports.
#[macro_export]
macro_rules! for_each_subject {
    ($family:ident :: $Any:ident, |$T:ident| $body:expr) => {{
        {
            type $T = ::dispatch_subjects::hand::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::whichever::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::enum_dispatch::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::ambassador::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::enum_delegate::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::delegation::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::delegate::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::dynamic::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::mono::$family::$Any;
            $body;
        }
    }};
}

/// Like [`for_each_subject!`], over the subjects whose strategy can forward a
/// method returning `Self`.
#[macro_export]
macro_rules! for_each_resize_subject {
    ($family:ident :: $Any:ident, |$T:ident| $body:expr) => {{
        {
            type $T = ::dispatch_subjects::hand::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::whichever::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::enum_dispatch::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::enum_delegate::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::delegate::$family::$Any;
            $body;
        }
        {
            type $T = ::dispatch_subjects::mono::$family::$Any;
            $body;
        }
    }};
}
