#[whichever::dispatchable]
trait Store {
    type Item;
    fn first(&self) -> Self::Item;
    fn all(&self) -> Vec<Self::Item>;
}

struct Shelf;
struct Crate;

impl Store for Shelf {
    type Item = u32;
    fn first(&self) -> u32 {
        1
    }
    fn all(&self) -> Vec<u32> {
        vec![1, 2]
    }
}

impl Store for Crate {
    type Item = u32;
    fn first(&self) -> u32 {
        10
    }
    fn all(&self) -> Vec<u32> {
        vec![10]
    }
}

#[whichever::dispatch(Store)]
enum Storage {
    Shelf(Shelf),
    Crate(Crate),
}

#[whichever::dispatchable]
trait Fallible {
    type Error;
    fn run(&self) -> Result<u32, Self::Error>;
    fn fail(&self) -> Self::Error;
}

struct Narrow;
struct Wide;

impl Fallible for Narrow {
    type Error = u8;
    fn run(&self) -> Result<u32, u8> {
        Err(8)
    }
    fn fail(&self) -> u8 {
        1
    }
}

impl Fallible for Wide {
    type Error = u16;
    fn run(&self) -> Result<u32, u16> {
        Ok(16)
    }
    fn fail(&self) -> u16 {
        2
    }
}

#[whichever::dispatch(Fallible, Error = u16)]
enum Width {
    Narrow(Narrow),
    Wide(Wide),
}

#[test]
fn the_first_variant_fixes_the_type_and_an_override_converts() {
    let item: <Storage as Store>::Item = Storage::from(Crate).first();
    assert_eq!(item, 10);
    assert_eq!(Storage::from(Shelf).all(), vec![1, 2]);

    let narrow: Width = Narrow.into();
    let wide: Width = Wide.into();
    let error: Result<u32, u16> = narrow.run();
    assert_eq!(error, Err(8));
    assert_eq!(wide.run(), Ok(16));
    let failure: u16 = narrow.fail();
    assert_eq!((failure, wide.fail()), (1, 2));
}
