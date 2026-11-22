//! The impl of a struct that marks no field holds its sentence in every
//! item a field would have supplied: an associated type, an associated
//! const and a method returning `impl Trait`, and their uses compile.
#[whichever::dispatchable]
pub trait Source {
    type Item;
    const WIDTH: usize;
    fn values(&self) -> impl Iterator<Item = u32>;
}

pub struct Numbers(Vec<u32>);

impl Source for Numbers {
    type Item = u32;
    const WIDTH: usize = 4;
    fn values(&self) -> impl Iterator<Item = u32> {
        self.0.iter().copied()
    }
}

#[whichever::dispatch(Source)]
pub struct Labelled {
    numbers: Numbers,
    label: u8,
}

pub fn total(source: &Labelled) -> u32 {
    source.values().sum()
}

pub fn width() -> usize {
    <Labelled as Source>::WIDTH
}

pub fn first(item: <Labelled as Source>::Item) -> <Labelled as Source>::Item {
    item
}

fn main() {}
