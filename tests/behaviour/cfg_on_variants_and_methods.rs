#[whichever::dispatchable]
trait Gated {
    fn always(&self) -> u8;
    #[cfg(any())]
    fn never(&self) -> Missing;
}

struct Present;

impl Gated for Present {
    fn always(&self) -> u8 {
        1
    }
}

#[whichever::dispatch(Gated)]
enum Gate {
    Present(Present),
    #[cfg(any())]
    Absent(Missing),
}

#[test]
fn disabled_items_are_disabled_everywhere() {
    assert_eq!(Gate::from(Present).always(), 1);
}

/// A variant the method cannot be forwarded to, a `deref` one beside a
/// method returning `Self`, under a `cfg` that does not hold: its arm, and
/// the sentence the arm would carry, go with it, as a hand's arm does.
mod refused_variant_compiled_out {
    #[whichever::dispatchable]
    pub trait Grow {
        fn grown(&self) -> Self;
        fn side(&self) -> u64;
    }

    #[derive(Debug, PartialEq)]
    pub struct Square(pub u64);

    impl Grow for Square {
        fn grown(&self) -> Self {
            Square(self.0 + 1)
        }
        fn side(&self) -> u64 {
            self.0
        }
    }

    #[derive(Debug, PartialEq)]
    #[whichever::dispatch(Grow)]
    pub enum Shape {
        Plain(Square),
        #[cfg(any())]
        #[dispatch(deref)]
        Boxed(Box<Square>),
    }

    #[test]
    fn the_variant_present_is_forwarded() {
        let grown = Shape::Plain(Square(2)).grown();
        assert_eq!(grown, Shape::Plain(Square(3)));
        assert_eq!(grown.side(), 3);
    }
}
