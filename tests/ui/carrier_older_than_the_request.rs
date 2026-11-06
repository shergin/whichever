//! A request of a shape the carrier does not match, which a newer
//! `whichever` than the carrier's would send, hits the carrier's last rule.
#[whichever::dispatchable]
pub trait Size {
    fn size(&self) -> usize;
}

Size! { @v9 facade { ::whichever } enum { pub enum Any { Fixed(Fixed) } } spec { Size } }

pub struct Fixed(pub usize);

impl Size for Fixed {
    fn size(&self) -> usize {
        self.0
    }
}

fn main() {}
