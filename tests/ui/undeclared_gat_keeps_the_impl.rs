//! A generic associated type the enum does not declare is one sentence, and
//! the impl is still written, holding it, so a call through the trait
//! resolves and the other methods' sentences are written as well: here a
//! `&mut self` method over a variant that holds an `Rc`.

use std::rc::Rc;

#[whichever::dispatchable]
pub trait Lend {
    type Item<'a>
    where
        Self: 'a;
    fn lend(&self) -> Self::Item<'_>;
    fn bump(&mut self) -> u32;
    fn len(&self) -> usize;
}

pub struct A(pub u32);

impl Lend for A {
    type Item<'a> = &'a u32;
    fn lend(&self) -> &u32 {
        &self.0
    }
    fn bump(&mut self) -> u32 {
        self.0 += 1;
        self.0
    }
    fn len(&self) -> usize {
        1
    }
}

#[whichever::dispatch(Lend)]
pub enum Kind {
    A(A),
    #[dispatch(deref)]
    Shared(Rc<A>),
}

/// A call through the trait, which a complete impl resolves.
pub fn len(kind: &Kind) -> usize {
    <Kind as Lend>::len(kind)
}

fn main() {}
