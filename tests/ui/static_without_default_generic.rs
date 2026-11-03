//! The block a receiverless method goes in is spelled with the trait's
//! arguments and the enum's generics, so that it compiles as suggested.
#[whichever::dispatchable]
trait GridTrait<T> {
    fn presets() -> Vec<T>;
    fn cell(&self) -> T;
}

struct Linear<T>(T);
struct Log<T>(T);

impl<T: Copy> GridTrait<T> for Linear<T> {
    fn presets() -> Vec<T> {
        vec![]
    }
    fn cell(&self) -> T {
        self.0
    }
}

impl<T: Copy> GridTrait<T> for Log<T> {
    fn presets() -> Vec<T> {
        vec![]
    }
    fn cell(&self) -> T {
        self.0
    }
}

#[whichever::dispatch(GridTrait<T>)]
enum Grid<T: Copy> {
    Linear(Linear<T>),
    Log(Log<T>),
}

fn main() {}
