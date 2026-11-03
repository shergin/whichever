//! A partial block that declares one generic associated type and not the
//! other is told which one is missing, and where it goes.
#[whichever::dispatchable]
trait Make {
    type Out<T>
    where
        T: Send;
    type View<'s>
    where
        Self: 's;
    fn make<T: Send>(&self) -> Self::Out<T>;
    fn view(&self) -> Self::View<'_>;
}

struct A;

impl Make for A {
    type Out<T> = Vec<T> where T: Send;
    type View<'s> = &'s A where Self: 's;
    fn make<T: Send>(&self) -> Vec<T> {
        vec![]
    }
    fn view(&self) -> &A {
        self
    }
}

#[whichever::dispatch]
enum Kind {
    A(A),
}

#[whichever::dispatch]
impl Make for Kind {
    type Out<T> = Vec<T> where T: Send;
}

fn main() {}
