//! The subjects of the `whichever` benchmarks.
//!
//! One trait shape, 32 payload types and enums of 2, 4, 8, 16 and 32 variants,
//! dispatched by every strategy under comparison. Each strategy is a module
//! with two *families*:
//!
//! - `trivial`: payload methods are a couple of instructions and `#[inline]`,
//!   so a strategy that lets the optimiser see through the dispatch can fold
//!   the method into the loop. This is where enum dispatch shines.
//! - `opaque`: payload methods are `#[inline(never)]`, so every call is a real
//!   call and what remains is the cost of choosing which function to call.
//!
//! Every family exports `Any2`, `Any4`, `Any8`, `Any16` and `Any32`, each
//! implementing [`Dispatch`], so a benchmark names a subject by path alone:
//! `whichever::opaque::Any8`.
//!
//! The crate that owns the enums is deliberately not the crate that runs the
//! loops: the benchmarks live in `whichever-benchmarks`, one crate boundary
//! away, as an application using a dispatch enum would. Without link-time
//! optimisation, rustc only inlines a function from another crate when it is
//! generic, `#[inline]`, or small *and free of calls*, so a generated method
//! that omits `#[inline]` stays an out-of-line call. That difference between
//! strategies is real and is meant to show.

#[macro_use]
mod macros;

pub mod payloads;

pub mod ambassador;
pub mod delegate;
pub mod delegation;
pub mod dynamic;
pub mod enum_delegate;
pub mod enum_dispatch;
pub mod hand;
pub mod mono;
pub mod whichever;

/// A collection element the benchmarks can build and call, implemented by
/// every family's enums, by [`dynamic::trivial::Boxed`] and by
/// [`mono::trivial::Mono`].
///
/// The adapter methods are `#[inline(always)]` wrappers around the subject's
/// own trait method, so they add nothing of their own.
pub trait Dispatch: Sized {
    /// The strategy's name in the reports.
    const SUBJECT: &'static str;

    /// How many variants the type has; `make` takes its variant modulo this.
    const VARIANTS: usize;

    /// Builds variant `variant % VARIANTS` holding the payload `(a, b)`.
    fn make(variant: usize, a: u64, b: u64) -> Self;

    /// A `&self` method returning a value.
    fn area(&self) -> u64;

    /// A `&mut self` method.
    fn scale(&mut self, factor: u64);

    /// A method the trait gives a default body, which half the payloads
    /// override.
    fn name(&self) -> &'static str;
}

/// A `-> Self` method, for the subjects whose strategy can forward one.
pub trait Resize: Dispatch {
    fn scaled(&self, factor: u64) -> Self;
}

/// Which variant each element of a collection holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    /// Every element is the first variant: the branch is always taken the
    /// same way, the best case for any dispatch.
    Uniform,
    /// Variants cycle in order, `0, 1, .., V-1, 0, ..`: a fixed period a
    /// branch predictor may or may not learn.
    RoundRobin,
    /// Variants drawn uniformly at random: the realistic heterogeneous
    /// collection, where each element's branch is unpredictable once the
    /// collection is too long for the predictor to memorise.
    Shuffled,
}

impl Pattern {
    pub const ALL: [Pattern; 3] = [Pattern::Uniform, Pattern::RoundRobin, Pattern::Shuffled];

    pub fn name(self) -> &'static str {
        match self {
            Pattern::Uniform => "uniform",
            Pattern::RoundRobin => "round_robin",
            Pattern::Shuffled => "shuffled",
        }
    }

    fn variant(self, index: usize, variants: usize, rng: &mut SplitMix64) -> usize {
        match self {
            Pattern::Uniform => 0,
            Pattern::RoundRobin => index % variants,
            Pattern::Shuffled => rng.below(variants),
        }
    }
}

/// A small seeded generator, so every subject sees the same sequence of
/// variants and payload values and no run depends on an external crate.
#[derive(Clone, Debug)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        SplitMix64(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

/// Builds `len` elements whose variants follow `pattern`, with payload values
/// the optimiser cannot know. For a `Box<dyn>` subject the boxes are allocated
/// in element order, which is the friendliest heap layout it can get; see
/// [`scatter`] for the other case.
pub fn build<T: Dispatch>(len: usize, pattern: Pattern, seed: u64) -> Vec<T> {
    let mut rng = SplitMix64::new(seed);
    (0..len)
        .map(|index| {
            let variant = pattern.variant(index, T::VARIANTS, &mut rng);
            let a = rng.next_u64() >> 54;
            let b = rng.next_u64() >> 54;
            T::make(variant, a, b)
        })
        .collect()
}

/// Permutes the elements in place. An enum carries its payload with it, so
/// this changes nothing about its memory access; a `Box<dyn>` keeps pointing
/// at wherever its box was allocated, so after a scatter each element is a
/// jump to an unrelated address, as in a long-lived program.
pub fn scatter<T>(items: &mut [T], seed: u64) {
    let mut rng = SplitMix64::new(seed);
    for i in (1..items.len()).rev() {
        items.swap(i, rng.below(i + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u64 = 7;

    /// Every subject must answer every method exactly like the hand-written
    /// enum of the same variant count, before and after a mutation.
    fn agrees_with<Reference: Dispatch, Subject: Dispatch>(patterns: &[Pattern]) {
        for &pattern in patterns {
            let mut reference: Vec<Reference> = build(96, pattern, SEED);
            let mut subject: Vec<Subject> = build(96, pattern, SEED);
            for (r, s) in reference.iter().zip(&subject) {
                assert_eq!(r.area(), s.area(), "{} area, {pattern:?}", Subject::SUBJECT);
                assert_eq!(r.name(), s.name(), "{} name, {pattern:?}", Subject::SUBJECT);
            }
            for (r, s) in reference.iter_mut().zip(subject.iter_mut()) {
                r.scale(3);
                s.scale(3);
                assert_eq!(
                    r.area(),
                    s.area(),
                    "{} scale, {pattern:?}",
                    Subject::SUBJECT
                );
            }
        }
    }

    fn resizes_like<Reference: Resize, Subject: Resize>(pattern: Pattern) {
        let reference: Vec<Reference> = build(96, pattern, SEED);
        let subject: Vec<Subject> = build(96, pattern, SEED);
        for (r, s) in reference.iter().zip(&subject) {
            assert_eq!(
                r.scaled(5).area(),
                s.scaled(5).area(),
                "{} scaled",
                Subject::SUBJECT
            );
        }
    }

    macro_rules! agree {
        ($name:ident, $subject:ident, resize: $resize:tt) => {
            #[test]
            fn $name() {
                agrees_with::<hand::trivial::Any32, $subject::trivial::Any32>(&Pattern::ALL);
                agrees_with::<hand::opaque::Any32, $subject::opaque::Any32>(&Pattern::ALL);
                agrees_with::<hand::trivial::Any2, $subject::trivial::Any2>(&Pattern::ALL);
                agree!(@resize $resize, $subject);
            }
        };
        (@resize yes, $subject:ident) => {
            resizes_like::<hand::trivial::Any32, $subject::trivial::Any32>(Pattern::Shuffled);
            resizes_like::<hand::opaque::Any8, $subject::opaque::Any8>(Pattern::Shuffled);
        };
        (@resize no, $subject:ident) => {};
    }

    agree!(whichever_agrees_with_hand, whichever, resize: yes);
    agree!(enum_dispatch_agrees_with_hand, enum_dispatch, resize: yes);
    agree!(ambassador_agrees_with_hand, ambassador, resize: no);
    agree!(enum_delegate_agrees_with_hand, enum_delegate, resize: yes);
    agree!(delegation_agrees_with_hand, delegation, resize: no);
    agree!(delegate_agrees_with_hand, delegate, resize: yes);
    agree!(box_dyn_agrees_with_hand, dynamic, resize: no);

    #[test]
    fn monomorphic_agrees_with_a_uniform_hand_enum() {
        agrees_with::<hand::trivial::Any2, mono::trivial::Any2>(&[Pattern::Uniform]);
        agrees_with::<hand::opaque::Any2, mono::opaque::Any2>(&[Pattern::Uniform]);
        resizes_like::<hand::trivial::Any2, mono::trivial::Any2>(Pattern::Uniform);
    }

    #[test]
    fn hand_opaque_agrees_with_hand_trivial() {
        agrees_with::<hand::trivial::Any32, hand::opaque::Any32>(&Pattern::ALL);
    }

    #[test]
    fn patterns_hold_the_variants_they_promise() {
        let uniform: Vec<hand::trivial::Any32> = build(64, Pattern::Uniform, SEED);
        assert!(uniform
            .iter()
            .all(|e| matches!(e, hand::trivial::Any32::V0(_))));
        let cycle: Vec<hand::trivial::Any4> = build(8, Pattern::RoundRobin, SEED);
        assert!(matches!(cycle[0], hand::trivial::Any4::V0(_)));
        assert!(matches!(cycle[3], hand::trivial::Any4::V3(_)));
        assert!(matches!(cycle[4], hand::trivial::Any4::V0(_)));
        // Every variant of a 32-way enum appears in 4096 random draws, so
        // all 16 name overrides show up next to the default.
        let shuffled: Vec<hand::trivial::Any32> = build(4096, Pattern::Shuffled, SEED);
        let names: std::collections::HashSet<&str> = shuffled.iter().map(|e| e.name()).collect();
        assert_eq!(names.len(), 17);
    }

    #[test]
    fn scatter_is_a_permutation() {
        let mut items: Vec<u32> = (0..100).collect();
        scatter(&mut items, SEED);
        assert_ne!(items, (0..100).collect::<Vec<_>>());
        items.sort_unstable();
        assert_eq!(items, (0..100).collect::<Vec<_>>());
    }
}
