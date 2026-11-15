//! What a return type is: a small tree whose leaves are the values a
//! payload answers with and whose inner nodes are the wrappers around
//! them. The type's spelling, the payload's body, the hand's conversion and
//! the law's observation all recurse over it.

use super::axes::Return;

/// An associated type of the trait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Associated {
    /// `type Out;`.
    Out,
    /// `type Error;`.
    Error,
    /// `type Item<'v> where Self: 'v;`, a generic associated type.
    Item,
}

impl Associated {
    /// Returns the associated type's name.
    pub fn name(self) -> &'static str {
        match self {
            Associated::Out => "Out",
            Associated::Error => "Error",
            Associated::Item => "Item",
        }
    }

    /// Returns the associated type as the trait declares it.
    pub fn declaration(self) -> &'static str {
        match self {
            Associated::Out => "type Out;",
            Associated::Error => "type Error;",
            Associated::Item => "type Item<'v> where Self: 'v;",
        }
    }

    /// Returns the associated type as an impl defines it, `value` being
    /// its type there, which a generic one spells with its parameter `'a`.
    pub fn definition(self, value: &str) -> String {
        match self {
            Associated::Out | Associated::Error => format!("type {} = {value};", self.name()),
            Associated::Item => format!("type Item<'v> = {value} where Self: 'v;"),
        }
    }
}

/// A future in return position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FutureKind {
    /// `impl Future<Output = T>`.
    Opaque,
    /// `Pin<Box<dyn Future<Output = T> + Send + 'async_trait>>`, as
    /// `#[async_trait]` writes it.
    Boxed,
}

/// A node of a return type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Node {
    /// The first variant's payload by value, `P0`: an inherent method's
    /// target itself.
    Payload,
    /// `&P0`.
    PayloadShared,
    /// `&mut P0`.
    PayloadExclusive,
    /// `&'a P0`, the payload a variant over `&'a P0` holds, for the
    /// subject's own lifetime.
    PayloadLent,
    /// `()`.
    Unit,
    /// `u64`.
    Tag,
    /// `&u64`, borrowed from the payload's state.
    Shared,
    /// `&mut u64`, borrowed mutably from the payload's state.
    Exclusive,
    /// `&'a u64`, borrowed from the payload's state for the subject's own
    /// lifetime: what a place reached through a variant over `&'a P0` is.
    Lent,
    /// `Self`.
    This,
    /// `Self::Out`, taken from the first variant: `u64` in every payload.
    Assoc,
    /// `Self::Out` or `Self::Error`, overridden on the enum with `Wide`,
    /// which every payload's own type converts into.
    Overridden(Associated),
    /// `Self::Item<'_>`, a generic associated type: a loan of the payload's
    /// state in every payload, `support::Loan0<'a>`, which converts into
    /// the `support::View<'v>` the enum declares.
    Gat,
    /// `Fault`, an error type of the support module.
    Fault,
    /// `Option<T>`.
    Option(&'static Node),
    /// `Result<T, E>`.
    Result(&'static Node, &'static Node),
    /// `support::fallible::Result<T>`, a one-argument alias whose error is
    /// `support::Fault`: what-is-generated.md maps `Result<Self, E>`
    /// "under any path, `io::Result<Self>` included".
    Fallible(&'static Node),
    /// A tuple.
    Tuple(&'static [Node]),
    /// A future of the output.
    Future(FutureKind, &'static Node),
    /// `impl Iterator<Item = u64>`.
    Iterator,
    /// `Vec<T>`.
    Vec(&'static Node),
}

/// What the hand does to a payload's answer to give the enum's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conversion {
    /// The answer as it is.
    Keep,
    /// The variant's constructor around it.
    Rewrap,
    /// `Into::into`.
    Into,
    /// `.map(path)` over an `Option`, the path being the inner conversion.
    Map(Box<Conversion>),
    /// `.map(path)` and `.map_err(path)` over a `Result`, each kept where
    /// its side is.
    Result(Box<Conversion>, Box<Conversion>),
    /// Taken apart, each element converted, and built again.
    Tuple(Vec<Conversion>),
}

impl Conversion {
    /// Returns whether the answer is kept as it is.
    pub fn is_keep(&self) -> bool {
        matches!(self, Conversion::Keep)
    }

    /// Returns whether the conversion is a path a `.map(..)` can take,
    /// rather than an expression that would need a closure.
    pub fn is_path(&self) -> bool {
        match self {
            Conversion::Rewrap | Conversion::Into => true,
            Conversion::Keep
            | Conversion::Map(_)
            | Conversion::Result(_, _)
            | Conversion::Tuple(_) => false,
        }
    }
}

impl Node {
    /// Returns whether `Self` occurs in the type by value, so that the
    /// answer has to be rewrapped.
    pub fn holds_this(&self) -> bool {
        self.any(&|node| matches!(node, Node::This))
    }

    /// Returns whether `Self` occurs as a type argument, under an `Option`,
    /// a `Result`, a tuple, a future or a `Vec`, where it has to be
    /// `Sized`.
    pub fn holds_this_below(&self) -> bool {
        match self {
            Node::Payload
            | Node::PayloadShared
            | Node::PayloadLent
            | Node::PayloadExclusive
            | Node::Unit
            | Node::Tag
            | Node::Shared
            | Node::Lent
            | Node::Exclusive
            | Node::This
            | Node::Assoc
            | Node::Overridden(_)
            | Node::Gat
            | Node::Fault
            | Node::Iterator => false,
            Node::Option(inner) | Node::Future(_, inner) | Node::Vec(inner) => inner.holds_this(),
            Node::Result(ok, error) => ok.holds_this() || error.holds_this(),
            Node::Fallible(ok) => ok.holds_this(),
            Node::Tuple(elements) => elements.iter().any(Node::holds_this),
        }
    }

    /// Returns whether the type borrows mutably.
    pub fn borrows_mutably(&self) -> bool {
        self.any(&|node| matches!(node, Node::Exclusive))
    }

    /// Returns whether the type borrows from the receiver or an argument,
    /// rather than for the subject's own lifetime: a reference, or the
    /// generic associated type at the call's lifetime, `Self::Item<'_>`.
    pub fn borrows_from_the_call(&self) -> bool {
        self.any(&|node| matches!(node, Node::Shared | Node::Exclusive | Node::Gat))
    }

    /// Returns whether the type borrows at all.
    pub fn borrows(&self) -> bool {
        self.any(&|node| {
            matches!(
                node,
                Node::Shared | Node::Lent | Node::Exclusive | Node::Gat
            )
        })
    }

    /// Returns the associated types the trait declares for this type, with
    /// whether the enum overrides each.
    pub fn associated(&self) -> Vec<(Associated, bool)> {
        let mut found = Vec::new();
        self.visit(&mut |node| {
            let entry = match node {
                Node::Assoc => (Associated::Out, false),
                Node::Overridden(associated) => (*associated, true),
                // The enum declares it, in a partial block, or nothing can.
                Node::Gat => (Associated::Item, true),
                Node::Payload
                | Node::PayloadShared
                | Node::PayloadLent
                | Node::PayloadExclusive
                | Node::Unit
                | Node::Tag
                | Node::Shared
                | Node::Lent
                | Node::Exclusive
                | Node::This
                | Node::Fault
                | Node::Option(_)
                | Node::Result(_, _)
                | Node::Fallible(_)
                | Node::Tuple(_)
                | Node::Future(_, _)
                | Node::Iterator
                | Node::Vec(_) => return,
            };
            if !found.contains(&entry) {
                found.push(entry);
            }
        });
        found
    }

    /// Returns the future the type is, with its output.
    pub fn future(&self) -> Option<(FutureKind, &'static Node)> {
        if let Node::Future(kind, output) = self {
            return Some((*kind, output));
        }
        None
    }

    /// Returns whether the type holds a future or an iterator anywhere, an
    /// opaque or boxed type the law cannot compare directly.
    pub fn holds_future(&self) -> bool {
        self.any(&|node| matches!(node, Node::Future(_, _)))
    }

    /// Returns the conversion a subject of one target writes: a struct
    /// hands an opaque iterator over as it is.
    pub fn conversion_of_one(&self) -> Option<Conversion> {
        match self {
            Node::Iterator => Some(Conversion::Keep),
            other => other.conversion(),
        }
    }

    /// Returns the conversion the hand writes, or `None` where it would
    /// need a closure or an allocation: `Self` in a `Vec`, a tuple under an
    /// `Option` or a `Result`, an iterator, which every payload answers
    /// with an opaque type of its own.
    pub fn conversion(&self) -> Option<Conversion> {
        match self {
            Node::Payload
            | Node::PayloadShared
            | Node::PayloadLent
            | Node::PayloadExclusive
            | Node::Unit
            | Node::Tag
            | Node::Shared
            | Node::Lent
            | Node::Exclusive
            | Node::Assoc
            | Node::Fault => Some(Conversion::Keep),
            Node::This => Some(Conversion::Rewrap),
            Node::Overridden(_) | Node::Gat => Some(Conversion::Into),
            Node::Option(inner) => {
                let inner = inner.conversion()?;
                if inner.is_keep() {
                    return Some(Conversion::Keep);
                }
                if !inner.is_path() {
                    return None;
                }
                Some(Conversion::Map(Box::new(inner)))
            }
            Node::Fallible(ok) => Node::Result(ok, &Node::Fault).conversion(),
            Node::Result(ok, error) => {
                let ok = ok.conversion()?;
                let error = error.conversion()?;
                if ok.is_keep() && error.is_keep() {
                    return Some(Conversion::Keep);
                }
                let mapped = |side: &Conversion| side.is_keep() || side.is_path();
                if !mapped(&ok) || !mapped(&error) {
                    return None;
                }
                Some(Conversion::Result(Box::new(ok), Box::new(error)))
            }
            Node::Tuple(elements) => {
                let elements = elements
                    .iter()
                    .map(Node::conversion)
                    .collect::<Option<Vec<_>>>()?;
                if elements.iter().all(Conversion::is_keep) {
                    return Some(Conversion::Keep);
                }
                Some(Conversion::Tuple(elements))
            }
            Node::Future(_, output) => output.conversion(),
            Node::Iterator => None,
            Node::Vec(inner) => {
                let inner = inner.conversion()?;
                if inner.is_keep() {
                    return Some(Conversion::Keep);
                }
                None
            }
        }
    }

    /// Returns whether a value of the type can be written without a
    /// payload, as a mismatch expression is: `None` for an `Option`, `Err`
    /// for a `Result`, a constant for a value or a shared reference, and a
    /// variant built around a fresh payload for `Self` where `this` says
    /// the expression can name the enum's variant.
    pub fn has_constant(&self, this: bool) -> bool {
        match self {
            Node::Unit
            | Node::Tag
            | Node::Shared
            | Node::Lent
            | Node::Assoc
            | Node::Overridden(_)
            | Node::Gat
            | Node::Fault
            | Node::Option(_) => true,
            Node::This => this,
            Node::Result(_, error) => error.has_constant(this),
            Node::Fallible(_) => true,
            Node::Tuple(elements) => elements.iter().all(|element| element.has_constant(this)),
            Node::Exclusive
            | Node::Payload
            | Node::PayloadShared
            | Node::PayloadLent
            | Node::PayloadExclusive
            | Node::Future(_, _)
            | Node::Iterator
            | Node::Vec(_) => false,
        }
    }

    /// Returns whether a trait's default body can make a value of the type
    /// knowing no payload: from `self.base()`, a number it leaks a place
    /// for where the type borrows, `self` itself where the method `owns`
    /// it, `None`, an `Err` or an empty `Vec` where the inner type has no
    /// default; an associated type, which the trait would have to bound to
    /// make, and the payload's own lifetime are no default's.
    pub fn has_default(&self, owns: bool) -> bool {
        match self {
            Node::Unit
            | Node::Tag
            | Node::Fault
            | Node::Iterator
            | Node::Shared
            | Node::Exclusive
            | Node::Option(_)
            | Node::Fallible(_)
            | Node::Vec(_) => true,
            Node::This => owns,
            Node::Future(_, inner) => inner.has_default(owns),
            Node::Result(ok, error) => ok.has_default(owns) || error.has_default(owns),
            Node::Tuple(elements) => elements.iter().all(|element| element.has_default(owns)),
            Node::Lent
            | Node::Payload
            | Node::PayloadShared
            | Node::PayloadLent
            | Node::PayloadExclusive
            | Node::Assoc
            | Node::Overridden(_)
            | Node::Gat => false,
        }
    }

    fn any(&self, predicate: &dyn Fn(&Node) -> bool) -> bool {
        let mut found = false;
        self.visit(&mut |node| found |= predicate(node));
        found
    }

    fn visit(&self, visitor: &mut dyn FnMut(&Node)) {
        visitor(self);
        match self {
            Node::Payload
            | Node::PayloadShared
            | Node::PayloadLent
            | Node::PayloadExclusive
            | Node::Unit
            | Node::Tag
            | Node::Shared
            | Node::Lent
            | Node::Exclusive
            | Node::This
            | Node::Assoc
            | Node::Overridden(_)
            | Node::Gat
            | Node::Fault
            | Node::Iterator => {}
            Node::Option(inner) | Node::Future(_, inner) | Node::Vec(inner) => inner.visit(visitor),
            Node::Result(ok, error) => {
                ok.visit(visitor);
                error.visit(visitor);
            }
            Node::Fallible(ok) => {
                ok.visit(visitor);
                Node::Fault.visit(visitor);
            }
            Node::Tuple(elements) => {
                for element in elements.iter() {
                    element.visit(visitor);
                }
            }
        }
    }
}

const OUT: Node = Node::Overridden(Associated::Out);
const ERROR: Node = Node::Overridden(Associated::Error);

impl Return {
    /// Returns the return type's tree.
    pub fn node(self) -> Node {
        match self {
            Return::Tag => Node::Tag,
            Return::Unit => Node::Unit,
            Return::Shared => Node::Shared,
            Return::Lent => Node::Lent,
            Return::Exclusive => Node::Exclusive,
            Return::This => Node::This,
            Return::OptionThis => Node::Option(&Node::This),
            Return::ResultThis => Node::Result(&Node::This, &Node::Fault),
            Return::FallibleThis => Node::Fallible(&Node::This),
            Return::TupleThis => Node::Tuple(&[Node::This, Node::Tag]),
            Return::Assoc => Node::Assoc,
            Return::Out => OUT,
            Return::OptionOut => Node::Option(&OUT),
            Return::ResultError => Node::Result(&Node::Tag, &ERROR),
            Return::ResultThisError => Node::Result(&Node::This, &ERROR),
            Return::Future => Node::Future(FutureKind::Opaque, &Node::Tag),
            Return::FutureThis => Node::Future(FutureKind::Opaque, &Node::This),
            Return::BoxedFuture => Node::Future(FutureKind::Boxed, &Node::Tag),
            Return::BoxedFutureOut => Node::Future(FutureKind::Boxed, &OUT),
            Return::Iterator => Node::Iterator,
            Return::VecThis => Node::Vec(&Node::This),
            Return::OptionTupleThis => Node::Option(&Node::Tuple(&[Node::This, Node::Tag])),
            Return::Gat | Return::GatUndeclared => Node::Gat,
        }
    }
}
