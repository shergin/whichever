//! A deprecated trait method, associated item or variant is forwarded under
//! `#[allow(deprecated)]`, as a hand writes it, so the enum compiles under
//! `deny(deprecated)`; the deprecation still reaches the callers through
//! the trait's item and the variant.
#![deny(deprecated)]

#[whichever::dispatchable]
pub trait Versioned {
    #[deprecated(note = "use `TAG`")]
    const KIND: u8;
    const TAG: u8;
    #[deprecated(note = "use `name`")]
    fn kind(&self) -> &'static str;
    fn name(&self) -> &'static str;
}

pub struct Old;
pub struct New;

impl Versioned for Old {
    const KIND: u8 = 1;
    const TAG: u8 = 1;
    fn kind(&self) -> &'static str {
        "old"
    }
    fn name(&self) -> &'static str {
        "old"
    }
}

impl Old {
    pub fn describe(&self) -> &'static str {
        "an old one"
    }
}

impl Versioned for New {
    const KIND: u8 = 1;
    const TAG: u8 = 1;
    fn kind(&self) -> &'static str {
        "new"
    }
    fn name(&self) -> &'static str {
        "new"
    }
}

impl New {
    pub fn describe(&self) -> &'static str {
        "a new one"
    }
}

#[whichever::dispatch(Versioned, try_from = true)]
pub enum Any {
    #[deprecated(note = "use `New`")]
    Old(Old),
    New(New),
}

#[whichever::dispatch]
impl Any {
    #[dispatch(*)]
    pub fn describe(&self) -> &'static str;
}

#[test]
fn deprecated_items_are_forwarded_under_allow() {
    let any: Any = New.into();
    assert_eq!(any.name(), "new");
    assert_eq!(any.describe(), "a new one");
    assert_eq!(<Any as Versioned>::TAG, 1);
    assert!(New::try_from(any).is_ok());
}
