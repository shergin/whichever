//! A partial block's header may name the enum's parameters as it likes; the
//! arms follow the header, not the declaration.

#[whichever::dispatchable]
pub trait Get<V> {
    fn get(&self) -> V;
}

pub struct Node(u32);

impl Get<u32> for Node {
    fn get(&self) -> u32 {
        self.0
    }
}

pub struct Imm(u32);

impl Get<u32> for Imm {
    fn get(&self) -> u32 {
        self.0 * 10
    }
}

#[whichever::dispatch(from = false)]
pub enum ImmOrNode<T> {
    Imm(T),
    Node(Node),
}

// The block calls the enum's `T` `Ty` and uses `T` for the trait's
// parameter, as cameleon's header does.
#[whichever::dispatch]
impl<T, Ty> Get<T> for ImmOrNode<Ty>
where
    Ty: Get<T>,
    Node: Get<T>,
{
}

#[whichever::dispatchable]
pub trait Count {
    fn count(&self) -> usize;
}

pub struct Many<'a>(&'a [u8]);

impl Count for Many<'_> {
    fn count(&self) -> usize {
        self.0.len()
    }
}

#[whichever::dispatch(from = false)]
pub enum Holder<'a, T> {
    Many(Many<'a>),
    One(T),
}

// A lifetime renamed, and a type parameter fixed to a concrete type.
#[whichever::dispatch]
impl<'b> Count for Holder<'b, Many<'b>> {}

#[test]
fn the_arms_follow_the_blocks_names() {
    let imm: ImmOrNode<Imm> = ImmOrNode::Imm(Imm(4));
    let node: ImmOrNode<Imm> = ImmOrNode::Node(Node(3));
    assert_eq!((imm.get(), node.get()), (40, 3));
    let bytes = [1u8, 2, 3];
    let holder: Holder<'_, Many<'_>> = Holder::One(Many(&bytes[..2]));
    assert_eq!(holder.count(), 2);
    assert_eq!(Holder::<Many<'_>>::Many(Many(&bytes)).count(), 3);
}
