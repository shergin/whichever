//! A trait method that cannot be forwarded is still written, with its
//! sentence as its body, so the impl is complete and the calls around it
//! resolve: the one error stands alone.
#[whichever::dispatchable]
trait Items {
    fn items(&self) -> impl Iterator<Item = u32> + '_;
    fn len(&self) -> usize;
}

struct A(Vec<u32>);

impl Items for A {
    fn items(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().copied()
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

#[whichever::dispatch(Items)]
enum Kind {
    A(A),
}

fn total(kind: &Kind) -> usize {
    kind.len()
}

fn main() {
    let kind: Kind = A(vec![1]).into();
    assert_eq!(total(&kind), 1);
}
