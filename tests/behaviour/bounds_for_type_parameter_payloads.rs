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
