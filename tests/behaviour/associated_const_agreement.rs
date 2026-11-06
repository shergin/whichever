#[whichever::dispatchable]
trait Proof {
    const KIND: &'static str;
    const ARITY: usize;
    const UNCHECKED: Option<u8>;
    fn check(&self) -> bool;
}

struct Encoding;
struct Ordering;

impl Proof for Encoding {
    const KIND: &'static str = "encoding";
    const ARITY: usize = 1;
    const UNCHECKED: Option<u8> = Some(1);
    fn check(&self) -> bool {
        true
    }
}

impl Proof for Ordering {
    const KIND: &'static str = "ordering";
    const ARITY: usize = 1;
    const UNCHECKED: Option<u8> = Some(2);
    fn check(&self) -> bool {
        false
    }
}

const ANY: &str = "any";

// `KIND` differs per payload, so the enum sets its own; `ARITY` agrees
// and is checked; `UNCHECKED` has a type the check cannot compare and
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
