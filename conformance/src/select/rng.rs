//! A small deterministic generator, so that selection is a function of
//! the model and a seed and the crate needs no dependency for it.

/// SplitMix64.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// A generator whose sequence is a function of `seed`.
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    /// Returns the next 64 bits.
    pub fn bits(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^ (mixed >> 31)
    }

    /// Returns an index below `bound`, which must not be zero.
    pub fn below(&mut self, bound: usize) -> usize {
        (self.bits() % bound as u64) as usize
    }

    /// Shuffles `items` in place.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for last in (1..items.len()).rev() {
            items.swap(last, self.below(last + 1));
        }
    }
}
