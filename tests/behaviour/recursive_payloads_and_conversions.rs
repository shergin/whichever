#[whichever::dispatchable]
trait Eval {
    type Out;
    fn eval(&self) -> Self::Out;
}

#[derive(Debug, PartialEq)]
struct Leaf(u32);
#[derive(Debug, PartialEq)]
struct Wrap<T>(Box<T>);

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

/// Payloads naming the enum as `Self` convert in both directions.
#[derive(Debug, PartialEq)]
#[whichever::dispatch(Eval, try_from = true)]
enum Node {
    Leaf(Leaf),
    Wrap(Wrap<Self>),
}

/// `try_from` on one variant only.
#[derive(Debug, PartialEq)]
#[whichever::dispatch(Eval)]
enum Tree {
    Leaf(Leaf),
    #[dispatch(try_from = true)]
    Wrap(Wrap<Self>),
}

// Would conflict with a generated impl, so it proves the opt-in is per
// variant.
impl TryFrom<Tree> for Leaf {
    type Error = ();
    fn try_from(_tree: Tree) -> Result<Self, ()> {
        Err(())
    }
}

#[test]
fn self_in_a_payload_names_the_enum_everywhere() {
    let node: Node = Wrap(Box::new(Node::Leaf(Leaf(1)))).into();
    assert_eq!(node.eval(), 2);
    let back: Result<Wrap<Node>, Node> = node.try_into();
    assert_eq!(back, Ok(Wrap(Box::new(Node::Leaf(Leaf(1))))));
    let leaf: Result<Leaf, Node> = Node::Leaf(Leaf(3)).try_into();
    assert_eq!(leaf, Ok(Leaf(3)));
    let tree = Tree::Wrap(Wrap(Box::new(Tree::Leaf(Leaf(1)))));
    assert_eq!(tree.eval(), 2);
    let wrap: Result<Wrap<Tree>, Tree> = tree.try_into();
    assert!(wrap.is_ok());
    assert_eq!(Leaf::try_from(Tree::Leaf(Leaf(1))), Err(()));
}
