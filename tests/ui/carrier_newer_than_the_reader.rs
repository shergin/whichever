//! A carrier macro written by a newer `whichever` than the one dispatching
//! it puts a prefix in front that this reader does not know.
pub trait Size {
    fn size(&self) -> usize;
}

#[allow(unused_macros)]
macro_rules! __whichever_Size_of_a_later_release {
    (@v1 $facade:ident $($rest:tt)*) => {
        ::$facade::__private::implement! {
            @v9 trait { pub trait Size { fn size(&self) -> usize; } } $($rest)*
        }
    };
}

#[allow(unused_imports)]
pub(crate) use __whichever_Size_of_a_later_release as Size;

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

fn main() {}
