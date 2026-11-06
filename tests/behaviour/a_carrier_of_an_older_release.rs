//! The carrier that the `#[dispatchable]` of 0.4.0 wrote, next to a trait
//! written by hand: `implement!` keeps reading the message of that release.

pub trait Size {
    fn size(&self) -> usize;
}

#[allow(unused_macros)]
macro_rules! __whichever_Size_of_0_4_0 {
    (@v1 $facade:ident $($rest:tt)*) => {
        ::$facade::__private::implement! {
            @v1 trait { pub trait Size { fn size(&self) -> usize; } } $($rest)*
        }
    };
}

#[allow(unused_imports)]
pub(crate) use __whichever_Size_of_0_4_0 as Size;

pub struct Fixed(pub usize);

impl Size for Fixed {
    fn size(&self) -> usize {
        self.0
    }
}

#[whichever::dispatch(Size)]
pub enum Any {
    Fixed(Fixed),
}

#[test]
fn it_is_still_read() {
    assert_eq!(Any::Fixed(Fixed(3)).size(), 3);
}
