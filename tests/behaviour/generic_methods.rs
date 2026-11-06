#[whichever::dispatchable]
trait Apply {
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32;
    fn feed(&self, items: impl Iterator<Item = u32>) -> u32;
}

struct One;
struct Two;

impl Apply for One {
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32 {
        f(1)
    }
    fn feed(&self, items: impl Iterator<Item = u32>) -> u32 {
        items.sum()
    }
}

impl Apply for Two {
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32 {
        f(2)
    }
    fn feed(&self, items: impl Iterator<Item = u32>) -> u32 {
        items.count() as u32
    }
}

#[whichever::dispatch(Apply)]
enum Which {
    One(One),
    Two(Two),
}

#[test]
fn closures_and_argument_position_impl_trait_pass_through() {
    assert_eq!(Which::from(Two).apply(|x| x * 10), 20);
    assert_eq!(Which::from(One).feed([1, 2, 3].into_iter()), 6);
    assert_eq!(Which::from(Two).feed([1, 2, 3].into_iter()), 3);
}
