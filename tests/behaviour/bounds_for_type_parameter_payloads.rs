//! A payload that is a type parameter of the enum, plain or behind a
//! pointer, gets the bound a hand writes on the impl, `T: Trait`; a struct's
//! field the same.

#[whichever::dispatchable]
pub trait Weigh {
    fn weight(&self) -> u32;
}

pub struct Light;
pub struct Heavy;

impl Weigh for Light {
    fn weight(&self) -> u32 {
        1
    }
}

impl Weigh for Heavy {
    fn weight(&self) -> u32 {
        10
    }
}

#[whichever::dispatch(Weigh, from = false)]
pub enum Lazy<Raw, Parsed> {
    Raw(Raw),
    #[dispatch(deref)]
    Parsed(Box<Parsed>),
}

#[whichever::dispatch(Weigh)]
pub struct Wrapper<T>(T);

#[test]
fn the_parameters_are_bounded_by_the_trait() {
    let raw: Lazy<Light, Heavy> = Lazy::Raw(Light);
    let parsed: Lazy<Light, Heavy> = Lazy::Parsed(Box::new(Heavy));
    assert_eq!((raw.weight(), parsed.weight()), (1, 10));
    assert_eq!(Wrapper(Heavy).weight(), 10);
}

/// A payload that only mentions a parameter, `Boxed<T>`, gets no bound of
/// its own: what its impl asks of `T` is not written in the enum. The
/// attribute names no trait, and an empty block writes the hand's header,
/// with the bound on `T` or on the payload's type.
pub struct Boxed<T>(pub T);

impl<T: Weigh> Weigh for Boxed<T> {
    fn weight(&self) -> u32 {
        self.0.weight() + 1
    }
}

#[whichever::dispatch]
pub enum Mixed<T> {
    Boxed(Boxed<T>),
    Light(Light),
}

#[whichever::dispatch]
impl<T: Weigh> Weigh for Mixed<T> {}

#[whichever::dispatch]
pub struct Labelled<T>(Boxed<T>);

#[whichever::dispatch]
impl<T> Weigh for Labelled<T> where Boxed<T>: Weigh {}

/// An enum whose payload names the enum compiles with the automatic bound
/// alone, which a bound on every payload's type would make unsatisfiable.
#[whichever::dispatch(Weigh, from = false)]
pub enum Tree<T> {
    Leaf(T),
    #[dispatch(deref)]
    Node(Box<Tree<T>>),
}

#[test]
fn the_empty_block_writes_the_bound_a_payload_mentioning_a_parameter_needs() {
    let boxed: Mixed<Heavy> = Mixed::Boxed(Boxed(Heavy));
    assert_eq!(
        (boxed.weight(), Mixed::<Heavy>::Light(Light).weight()),
        (11, 1)
    );
    assert_eq!(Labelled(Boxed(Light)).weight(), 2);
}

#[test]
fn a_recursive_enum_keeps_the_automatic_bound() {
    let tree = Tree::Node(Box::new(Tree::Node(Box::new(Tree::Leaf(Heavy)))));
    assert_eq!(tree.weight(), 10);
}
