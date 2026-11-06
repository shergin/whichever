#[whichever::dispatchable]
trait Named {
    fn name(&self) -> &'static str;
}

struct Printer;

impl Printer {
    fn name(&self) -> &'static str {
        "inherent"
    }
}

impl Named for Printer {
    fn name(&self) -> &'static str {
        "trait"
    }
}

struct Scanner;

impl Named for Scanner {
    fn name(&self) -> &'static str {
        "scanner"
    }
}

#[whichever::dispatch(Named)]
enum Device {
    Printer(Printer),
    Scanner(Scanner),
}

#[test]
fn calls_are_fully_qualified() {
    assert_eq!(Named::name(&Device::from(Printer)), "trait");
    assert_eq!(Device::from(Printer).name(), "trait");
    assert_eq!(Device::from(Scanner).name(), "scanner");
}
