#[whichever::dispatchable]
trait Proof {
    const KIND: &'static str;
    const ARITY: usize;
    const RATIO: f64;
    const UNCHECKED: Option<u8>;
    fn check(&self) -> bool;
}

struct Encoding;
struct Ordering;

impl Proof for Encoding {
    const KIND: &'static str = "encoding";
    const ARITY: usize = 1;
    const RATIO: f64 = 0.5;
    const UNCHECKED: Option<u8> = Some(1);
    fn check(&self) -> bool {
        true
    }
}

impl Proof for Ordering {
    const KIND: &'static str = "ordering";
    const ARITY: usize = 1;
    const RATIO: f64 = 0.5;
    const UNCHECKED: Option<u8> = Some(2);
    fn check(&self) -> bool {
        false
    }
}

const ANY: &str = "any";

// `KIND` differs per payload, so the enum sets its own; `ARITY` and
// `RATIO` agree and are checked; `UNCHECKED` has a type the check cannot compare and
// comes from the first variant.
#[whichever::dispatch(Proof, KIND = ANY)]
enum AnyProof {
    Encoding(Encoding),
    Ordering(Ordering),
}

#[whichever::dispatch(Proof, KIND = "literal", ARITY = 1 + 1)]
enum OtherProof {
    Encoding(Encoding),
    Ordering(Ordering),
}

#[test]
fn consts_are_overridden_or_checked_to_agree() {
    assert_eq!(<AnyProof as Proof>::KIND, "any");
    assert_eq!(<AnyProof as Proof>::ARITY, 1);
    assert_eq!(<AnyProof as Proof>::UNCHECKED, Some(1));
    assert_eq!(<OtherProof as Proof>::KIND, "literal");
    assert_eq!(<OtherProof as Proof>::ARITY, 2);
    assert!(AnyProof::Encoding(Encoding).check());
}

/// A payload that borrows, so that the enum holding it has a lifetime.
struct Viewed<'a>(&'a Ordering);

impl Proof for Viewed<'_> {
    const KIND: &'static str = "ordering";
    const ARITY: usize = 1;
    const RATIO: f64 = 0.5;
    const UNCHECKED: Option<u8> = Some(3);
    fn check(&self) -> bool {
        self.0.check()
    }
}

// An enum with lifetime parameters alone is checked as one without: no
// lifetime changes a const's value. `ARITY` and `RATIO` agree across a
// borrowed payload and one that is reached through `deref`.
#[whichever::dispatch(Proof, KIND = ANY)]
enum BorrowedProof<'a, 'b> {
    Encoding(Encoding),
    #[dispatch(deref)]
    Ordering(&'a Ordering),
    Viewed(Viewed<'b>),
}

#[test]
fn consts_of_an_enum_with_lifetimes_alone_are_checked_and_read() {
    let ordering = Ordering;
    assert_eq!(<BorrowedProof<'_, '_> as Proof>::ARITY, 1);
    assert_eq!(<BorrowedProof<'_, '_> as Proof>::RATIO, 0.5);
    assert_eq!(<BorrowedProof<'_, '_> as Proof>::UNCHECKED, Some(1));
    assert!(!BorrowedProof::Ordering(&ordering).check());
    assert!(!BorrowedProof::Viewed(Viewed(&ordering)).check());
}
