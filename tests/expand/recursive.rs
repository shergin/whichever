#[whichever::dispatchable]
pub trait Eval {
    type Out;
    fn eval(&self) -> Self::Out;
}

pub struct Leaf(pub u32);
pub struct Wrap<T>(pub Box<T>);

impl Eval for Leaf {
    type Out = u32;
    fn eval(&self) -> u32 {
        self.0
    }
}

impl<T: Eval<Out = u32>> Eval for Wrap<T> {
    type Out = u32;
    fn eval(&self) -> u32 {
        self.0.eval() + 1
    }
}

#[whichever::dispatch(Eval, try_from = true)]
pub enum Node {
    Leaf(Leaf),
    Wrap(Wrap<Self>),
}

fn main() {}
