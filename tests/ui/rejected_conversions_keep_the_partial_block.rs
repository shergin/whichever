//! A conversion that cannot be written is reported on its payload, and the
//! enum, its carrier and its partial impl block go on as if it had been
//! written: the block is forwarded in full, and the calls resolve.
#[whichever::dispatchable]
trait Get<V> {
    fn get(&self) -> V;
}

struct Imm(u32);
struct Node(u32);

impl Get<u32> for Imm {
    fn get(&self) -> u32 {
        self.0
    }
}

impl Get<u32> for Node {
    fn get(&self) -> u32 {
        self.0
    }
}

#[whichever::dispatch]
enum ImmOrNode<T> {
    Imm(T),
    Node(Node),
}

#[whichever::dispatch]
impl<T: Get<u32>> Get<u32> for ImmOrNode<T> {}

fn main() {
    let value: ImmOrNode<Imm> = ImmOrNode::Node(Node(1));
    let _: u32 = value.get();
}
