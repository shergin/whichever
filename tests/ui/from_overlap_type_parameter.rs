#[whichever::dispatchable]
trait Len {
    fn len(&self) -> usize;
}

impl<I: Iterator> Len for I {
    fn len(&self) -> usize {
        self.size_hint().0
    }
}

#[whichever::dispatch(Len)]
enum Adapter<I: Iterator> {
    Iter(I),
    Collected(std::vec::IntoIter<I::Item>),
}

fn main() {}
