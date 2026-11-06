//! A generic method whose parameter no argument fixes is called with the
//! parameters spelled out.

use std::str::FromStr;

#[whichever::dispatchable]
pub trait Source {
    fn parse<T: FromStr>(&self) -> Option<T>;
    fn zeros<const N: usize>(&self) -> [u8; N];
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32;
}

pub struct Text(pub &'static str);

impl Source for Text {
    fn parse<T: FromStr>(&self) -> Option<T> {
        self.0.parse().ok()
    }
    fn zeros<const N: usize>(&self) -> [u8; N] {
        [0; N]
    }
    fn apply<F: Fn(u32) -> u32>(&self, f: F) -> u32 {
        f(self.0.len() as u32)
    }
}

#[whichever::dispatch(Source)]
pub enum Any {
    Text(Text),
}

#[test]
fn the_parameters_are_spelled_out_when_needed() {
    let any = Any::Text(Text("42"));
    assert_eq!(any.parse::<u32>(), Some(42));
    assert_eq!(any.zeros::<3>(), [0, 0, 0]);
    assert_eq!(any.apply(|n| n + 1), 3);
}
