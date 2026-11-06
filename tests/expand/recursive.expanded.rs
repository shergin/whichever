pub trait Eval {
    type Out;
    fn eval(&self) -> Self::Out;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Eval_47abfbf61b073792_1_1 as Eval;
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
pub enum Node {
    Leaf(Leaf),
    Wrap(Wrap<Self>),
}
#[automatically_derived]
impl ::core::convert::From<Leaf> for Node {
    #[inline]
    fn from(__value: Leaf) -> Self {
        Self::Leaf(__value)
    }
}
#[automatically_derived]
impl ::core::convert::TryFrom<Node> for Leaf {
    type Error = Node;
    #[inline]
    #[allow(unreachable_patterns)]
    fn try_from(__value: Node) -> ::core::result::Result<Self, Self::Error> {
        match __value {
            Node::Leaf(__value) => ::core::result::Result::Ok(__value),
            __other => ::core::result::Result::Err(__other),
        }
    }
}
#[automatically_derived]
impl ::core::convert::From<Wrap<Self>> for Node {
    #[inline]
    fn from(__value: Wrap<Self>) -> Self {
        Self::Wrap(__value)
    }
}
#[automatically_derived]
impl ::core::convert::TryFrom<Node> for Wrap<Node> {
    type Error = Node;
    #[inline]
    #[allow(unreachable_patterns)]
    fn try_from(__value: Node) -> ::core::result::Result<Self, Self::Error> {
        match __value {
            Node::Wrap(__value) => ::core::result::Result::Ok(__value),
            __other => ::core::result::Result::Err(__other),
        }
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Node_6f1d5cd6fd29226c_24_1 as Node;
#[automatically_derived]
impl Eval for Node {
    type Out = <Leaf as Eval>::Out;
    #[inline]
    fn eval(&self) -> Self::Out {
        match self {
            Node::Leaf(__inner) => <Leaf as Eval>::eval(__inner),
            Node::Wrap(__inner) => <Wrap<Self> as Eval>::eval(__inner),
        }
    }
}
const _: () = {
    #[allow(dead_code)]
    struct __WhicheverAgreement(Node);
    #[allow(dead_code, non_snake_case)]
    impl __WhicheverAgreement {
        fn __first_implements<__First: ?Sized + Eval>() {}
        fn __same_as_first_variant<__Variant: ?Sized + Eval<Out = <Leaf as Eval>::Out>>() {}
        fn __whichever_associated_types_agree() {
            Self::__first_implements::<Leaf>();
            Self::__same_as_first_variant::<Wrap<Node>>();
        }
    }
};
fn main() {}
