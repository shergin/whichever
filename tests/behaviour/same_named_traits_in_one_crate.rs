mod metric {
    #[whichever::dispatchable]
    pub trait Unit {
        fn label(&self) -> &'static str;
    }
    pub struct Meter;
    impl Unit for Meter {
        fn label(&self) -> &'static str {
            "m"
        }
    }
    #[whichever::dispatch(Unit)]
    pub enum Length {
        Meter(Meter),
    }
}

mod imperial {
    #[whichever::dispatchable]
    pub trait Unit {
        fn label(&self) -> &'static str;
    }
    pub struct Foot;
    impl Unit for Foot {
        fn label(&self) -> &'static str {
            "ft"
        }
    }
    #[whichever::dispatch(Unit)]
    pub enum Length {
        Foot(Foot),
    }
}

#[test]
fn exported_macros_do_not_collide() {
    use imperial::Unit as _;
    use metric::Unit as _;
    assert_eq!(metric::Length::from(metric::Meter).label(), "m");
    assert_eq!(imperial::Length::from(imperial::Foot).label(), "ft");
}
