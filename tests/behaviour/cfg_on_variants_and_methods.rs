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
