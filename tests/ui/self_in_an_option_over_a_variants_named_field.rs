//! A variant whose one field is named is rebuilt by its braces, and a
//! `Self` under an `Option` is mapped with a path, which no braces are: the
//! sentence names the variant and the tuple form that a path builds.
#[whichever::dispatchable]
trait Grow {
    fn grown(&self) -> Option<Self>
    where
        Self: Sized;
}

struct Circle(u32);

impl Grow for Circle {
    fn grown(&self) -> Option<Self> {
        Some(Circle(self.0 + 1))
    }
}

#[whichever::dispatch(Grow)]
enum Any {
    Named {
        #[dispatch]
        inner: Circle,
    },
    Plain(Circle),
}

fn main() {}
