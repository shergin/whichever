//! An override or a mismatch naming nothing in the trait is a typo, and is
//! reported with what was probably meant: a near miss of an associated
//! item or a method, a near miss of an enum option, a field-choosing option
//! of another crate, or the list of what there is to name.
#[whichever::dispatchable]
trait Storage {
    type Error;
    type Unit;
    fn load(&self) -> Result<u32, Self::Error>;
    #[dispatch(mismatch = false)]
    fn same(&self, other: &Self) -> bool;
    #[dispatch(mismatch = false)]
    fn eq(&self, other: &Self) -> bool;
}

#[whichever::dispatchable]
trait Shape {
    fn area(&self) -> u32;
}

struct Memory;

impl Storage for Memory {
    type Error = ();
    type Unit = u8;
    fn load(&self) -> Result<u32, ()> {
        Ok(1)
    }
    fn same(&self, _: &Self) -> bool {
        true
    }
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Shape for Memory {
    fn area(&self) -> u32 {
        1
    }
}

#[whichever::dispatch(Storage, Eror = (), Unti = u8, form = false, mismatch(sane = false, eqq = false))]
#[whichever::dispatch(Shape, target = inner, mismatch(area = 0))]
enum Kind {
    Memory(Memory),
}

#[whichever::dispatch(Storage, nothing = u8, target = inner)]
struct Wrapper {
    #[dispatch]
    inner: Memory,
    tag: u8,
}

fn main() {}
