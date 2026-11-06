#[whichever::dispatchable]
trait Handler {
    type Value;
    fn from_handler(handler: AnyHandler) -> Option<Self>
    where
        Self: Sized;
    fn value(&self) -> Self::Value;
    fn is_attached(&self) -> bool;
    fn describe(&self) -> String {
        format!("attached: {}", self.is_attached())
    }
}

#[derive(Debug, PartialEq)]
struct Text(u32);
#[derive(Debug, PartialEq)]
struct Map(u8);

impl Handler for Text {
    type Value = u32;
    fn from_handler(handler: AnyHandler) -> Option<Self> {
        match handler {
            AnyHandler::Text(text) => Some(text),
            _ => None,
        }
    }
    fn value(&self) -> u32 {
        self.0
    }
    fn is_attached(&self) -> bool {
        true
    }
}

impl Handler for Map {
    type Value = u8;
    fn from_handler(handler: AnyHandler) -> Option<Self> {
        match handler {
            AnyHandler::Map(map) => Some(map),
            _ => None,
        }
    }
    fn value(&self) -> u8 {
        self.0
    }
    fn is_attached(&self) -> bool {
        false
    }
    fn describe(&self) -> String {
        String::from("a map")
    }
}

#[derive(Debug, PartialEq)]
#[whichever::dispatch]
enum AnyHandler {
    Text(Text),
    Map(Map),
}

#[whichever::dispatch]
impl Handler for AnyHandler {
    type Value = u64;

    fn from_handler(handler: AnyHandler) -> Option<Self> {
        Some(handler)
    }
}

#[test]
fn the_block_holds_the_enums_own_items_and_the_rest_is_forwarded() {
    let text = AnyHandler::Text(Text(7));
    let map = AnyHandler::Map(Map(3));
    assert_eq!(
        AnyHandler::from_handler(text),
        Some(AnyHandler::Text(Text(7)))
    );
    assert_eq!(Text::from_handler(map), None);
    assert_eq!(AnyHandler::Text(Text(7)).value(), 7u64);
    assert_eq!(AnyHandler::Map(Map(3)).value(), 3u64);
    assert_eq!(AnyHandler::Text(Text(7)).describe(), "attached: true");
    assert_eq!(AnyHandler::Map(Map(3)).describe(), "a map");
}

#[whichever::dispatchable]
trait Shell {
    fn name(&self) -> &'static str;
    fn prompt(&self) -> String {
        format!("{}$ ", self.name())
    }
}

struct Bash;
struct Fish;
struct Custom<T>(T);

impl Shell for Bash {
    fn name(&self) -> &'static str {
        "bash"
    }
}

impl Shell for Fish {
    fn name(&self) -> &'static str {
        "fish"
    }
    fn prompt(&self) -> String {
        String::from("> ")
    }
}

impl<T: Default + Copy> Shell for Custom<T> {
    fn name(&self) -> &'static str {
        "custom"
    }
}

#[whichever::dispatch]
enum AnyShell<T> {
    Bash(Bash),
    Fish(Fish),
    Custom(Custom<T>),
}

// The block's own header, bounds included, carries into the impl, and a
// method written here overrides every payload's, provided ones included.
#[whichever::dispatch]
impl<T: Default + Copy> Shell for AnyShell<T> {
    fn prompt(&self) -> String {
        format!("[{}] ", self.name())
    }
}

#[test]
fn a_method_written_in_the_block_overrides_the_payloads() {
    let fish: AnyShell<u8> = Fish.into();
    assert_eq!(fish.prompt(), "[fish] ");
    assert_eq!(AnyShell::<u8>::Bash(Bash).prompt(), "[bash] ");
    assert_eq!(AnyShell::Custom(Custom(1u8)).prompt(), "[custom] ");
}
