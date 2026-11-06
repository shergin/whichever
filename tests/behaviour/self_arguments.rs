#[whichever::dispatchable]
trait Same {
    #[dispatch(mismatch = false)]
    fn same_as(&self, other: &Self) -> bool;
    fn merge(&mut self, other: &mut Self);
    fn absorb(self, other: Self) -> u32
    where
        Self: Sized;
}

struct Small(u8);
struct Big(u32);

impl Same for Small {
    fn same_as(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn merge(&mut self, other: &mut Self) {
        self.0 += other.0;
        other.0 = 0;
    }
    fn absorb(self, other: Self) -> u32 {
        u32::from(self.0 + other.0)
    }
}

impl Same for Big {
    fn same_as(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    fn merge(&mut self, other: &mut Self) {
        self.0 += other.0;
        other.0 = 0;
    }
    fn absorb(self, other: Self) -> u32 {
        self.0 + other.0
    }
}

#[whichever::dispatch(Same, mismatch(merge = (), absorb = 0))]
enum Number {
    Small(Small),
    Big(Big),
}

#[test]
fn same_variants_meet_and_different_ones_take_the_fallback() {
    let a: Number = Small(2).into();
    let b: Number = Small(2).into();
    let c: Number = Big(2).into();
    assert!(a.same_as(&b));
    assert!(!a.same_as(&c));

    let mut x: Number = Small(2).into();
    let mut y: Number = Small(3).into();
    x.merge(&mut y);
    assert!(matches!(
        (x, y),
        (Number::Small(Small(5)), Number::Small(Small(0)))
    ));

    let mut big: Number = Big(10).into();
    let mut small: Number = Small(1).into();
    big.merge(&mut small);
    assert!(matches!(
        (big, small),
        (Number::Big(Big(10)), Number::Small(Small(1)))
    ));

    assert_eq!(Number::from(Big(1)).absorb(Big(2).into()), 3);
    assert_eq!(Number::from(Big(1)).absorb(Small(2).into()), 0);
}
