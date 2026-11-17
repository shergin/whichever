//! An inherent method without a receiver has no value whose field `field`
//! or `value` could hand over.
#[whichever::dispatch]
struct Number(u32);

#[whichever::dispatch]
impl Number {
    #[dispatch(*, value)]
    fn zero() -> u32;
}

fn main() {}
