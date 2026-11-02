pub trait Describe {
    fn name(&self) -> &'static str;
    fn kind(&self) -> &'static str {
        "default"
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Describe_ee8911ec9f8c3da5_1_1 as Describe;
pub struct Cat;
pub struct Dog;
impl Describe for Cat {
    fn name(&self) -> &'static str {
        "cat"
    }
}
impl Describe for Dog {
    fn name(&self) -> &'static str {
        "dog"
    }
}
pub enum Animal {
    Cat(Cat),
    Dog(Dog),
}
#[automatically_derived]
impl ::core::convert::From<Cat> for Animal {
    #[inline]
    fn from(__value: Cat) -> Self {
        Self::Cat(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Dog> for Animal {
    #[inline]
    fn from(__value: Dog) -> Self {
        Self::Dog(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_enum_Animal_52774b0d1fd69278_24_1 as Animal;
#[automatically_derived]
impl Describe for Animal {
    #[inline]
    fn name(&self) -> &'static str {
        match self {
            Animal::Cat(__inner) => <Cat as Describe>::name(__inner),
            Animal::Dog(__inner) => <Dog as Describe>::name(__inner),
        }
    }
    #[inline]
    fn kind(&self) -> &'static str {
        match self {
            Animal::Cat(__inner) => <Cat as Describe>::kind(__inner),
            Animal::Dog(__inner) => <Dog as Describe>::kind(__inner),
        }
    }
}
fn main() {}
