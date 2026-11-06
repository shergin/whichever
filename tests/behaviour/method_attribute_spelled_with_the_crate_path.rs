#[whichever::dispatchable]
trait Eq2 {
    #[whichever::dispatch(mismatch = false)]
    fn eq2(&self, other: &Self) -> bool;
}

struct A;
struct B;

impl Eq2 for A {
    fn eq2(&self, _: &Self) -> bool {
        true
    }
}

impl Eq2 for B {
    fn eq2(&self, _: &Self) -> bool {
        true
    }
}

#[whichever::dispatch(Eq2)]
enum Either {
    A(A),
    B(B),
}

#[test]
fn both_spellings_are_accepted() {
    assert!(Either::from(A).eq2(&A.into()));
    assert!(!Either::from(A).eq2(&B.into()));
}
