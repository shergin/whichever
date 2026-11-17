//! `remote` restates the enum or the struct, so it goes on its own
//! attribute; a block's attribute takes the trait's options only.
#[whichever::dispatchable]
trait Value {
    fn value(&self) -> u32;
}

struct A;

impl Value for A {
    fn value(&self) -> u32 {
        1
    }
}

#[whichever::dispatch]
struct Solo(A);

#[whichever::dispatch(remote = Solo)]
impl Value for Solo {}

fn main() {}
