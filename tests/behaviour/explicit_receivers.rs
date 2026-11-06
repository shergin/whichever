//! The explicit forms are the point of this module.
#![allow(clippy::needless_arbitrary_self_type, clippy::needless_lifetimes)]

#[whichever::dispatchable]
trait Counter {
    fn value<'a>(self: &'a Self) -> &'a u32;
    fn bump(self: &mut Self);
    fn take(self: Self) -> u32;
}

struct Ones(u32);
struct Tens(u32);

impl Counter for Ones {
    fn value<'a>(self: &'a Self) -> &'a u32 {
        &self.0
    }
    fn bump(self: &mut Self) {
        self.0 += 1;
    }
    fn take(self: Self) -> u32 {
        self.0
    }
}

impl Counter for Tens {
    fn value<'a>(self: &'a Self) -> &'a u32 {
        &self.0
    }
    fn bump(self: &mut Self) {
        self.0 += 10;
    }
    fn take(self: Self) -> u32 {
        self.0
    }
}

impl Ones {
    fn doubled(&self) -> u32 {
        self.0 * 2
    }
}

impl Tens {
    fn doubled(&self) -> u32 {
        self.0 * 2
    }
}

#[whichever::dispatch(Counter)]
enum AnyCounter {
    Ones(Ones),
    Tens(Tens),
}

#[whichever::dispatch]
impl AnyCounter {
    #[dispatch(*)]
    fn doubled(self: &Self) -> u32;
}

#[test]
fn explicit_forms_dispatch_like_the_shorthand() {
    let mut counter: AnyCounter = Tens(5).into();
    counter.bump();
    assert_eq!(*counter.value(), 15);
    assert_eq!(counter.doubled(), 30);
    assert_eq!(counter.take(), 15);
}
