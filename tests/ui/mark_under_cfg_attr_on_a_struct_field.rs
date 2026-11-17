//! A mark under `cfg_attr` reaches the attribute before rustc decides it,
//! so it is read in no build; it is one sentence naming the way around, and
//! the struct is read as if the mark were plain, so nothing else follows,
//! whether the predicate holds or not.
#[whichever::dispatchable]
pub trait Shape {
    fn area(&self) -> u32;
}

pub struct Square(pub u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

#[whichever::dispatch(Shape)]
pub struct Named {
    #[cfg_attr(all(), dispatch)]
    pub inner: Square,
    pub name: String,
}

#[whichever::dispatch(Shape)]
pub struct Boxed {
    #[cfg_attr(any(), dispatch(deref), allow(unused))]
    pub inner: Box<Square>,
}

fn main() {
    let named = Named {
        inner: Square(2),
        name: String::new(),
    };
    let _ = named.area();
}
