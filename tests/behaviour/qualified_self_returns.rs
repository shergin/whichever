use std::io;

#[whichever::dispatchable]
trait Rebuild {
    fn reload(&self) -> io::Result<Self>
    where
        Self: Sized;
    fn twin(&self) -> std::result::Result<Self, u8>
    where
        Self: Sized;
    fn maybe(&self) -> core::option::Option<Self>
    where
        Self: Sized;
}

#[derive(Debug, PartialEq)]
struct Small(u8);
#[derive(Debug, PartialEq)]
struct Big(u32);

impl Rebuild for Small {
    fn reload(&self) -> io::Result<Self> {
        Ok(Small(self.0))
    }
    fn twin(&self) -> Result<Self, u8> {
        Ok(Small(self.0))
    }
    fn maybe(&self) -> Option<Self> {
        None
    }
}

impl Rebuild for Big {
    fn reload(&self) -> io::Result<Self> {
        Err(io::Error::other("offline"))
    }
    fn twin(&self) -> Result<Self, u8> {
        Err(self.0 as u8)
    }
    fn maybe(&self) -> Option<Self> {
        Some(Big(self.0))
    }
}

impl Small {
    fn copy(&self) -> Self {
        Small(self.0)
    }
    fn halved(&self) -> Option<Self> {
        self.0.is_multiple_of(2).then_some(Small(self.0 / 2))
    }
    fn grown(&self) -> Self {
        Small(self.0 + 1)
    }
}

impl Big {
    fn copy(&self) -> Self {
        Big(self.0)
    }
    fn halved(&self) -> Option<Self> {
        Some(Big(self.0 / 2))
    }
}

#[derive(Debug, PartialEq)]
#[whichever::dispatch(Rebuild)]
enum Number {
    Small(Small),
    Big(Big),
}

#[whichever::dispatch]
impl Number {
    #[dispatch(*)]
    fn copy(&self) -> Self;

    #[dispatch(*)]
    fn halved(&self) -> Option<Self>;

    /// The payload returns the plain value; `with` lifts the variant.
    #[dispatch(Small, with = Some)]
    fn grown(&self) -> Option<Self> {
        None
    }
}

#[test]
fn qualified_and_aliased_paths_are_mapped_like_the_bare_ones() {
    let small: Number = Small(4).into();
    let big: Number = Big(7).into();
    assert_eq!(small.reload().unwrap(), Number::Small(Small(4)));
    assert_eq!(big.reload().unwrap_err().to_string(), "offline");
    assert_eq!(small.twin(), Ok(Number::Small(Small(4))));
    assert_eq!(big.twin(), Err(7));
    assert_eq!(
        (small.maybe(), big.maybe()),
        (None, Some(Number::Big(Big(7))))
    );
}

#[test]
fn inherent_methods_wrap_self_back_into_the_variant() {
    let small: Number = Small(4).into();
    let big: Number = Big(7).into();
    assert_eq!(small.copy(), Number::Small(Small(4)));
    assert_eq!(big.halved(), Some(Number::Big(Big(3))));
    assert_eq!(Number::from(Small(3)).halved(), None);
    assert_eq!(small.grown(), Some(Number::Small(Small(5))));
    assert_eq!(big.grown(), None);
}
