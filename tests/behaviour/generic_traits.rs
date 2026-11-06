#[whichever::dispatchable]
trait Convert<X> {
    fn convert(&self, x: X) -> X;
}

struct Inc;
struct Dec;

impl Convert<u32> for Inc {
    fn convert(&self, x: u32) -> u32 {
        x + 1
    }
}

impl Convert<u32> for Dec {
    fn convert(&self, x: u32) -> u32 {
        x - 1
    }
}

#[whichever::dispatch(Convert<u32>)]
enum Step {
    Inc(Inc),
    Dec(Dec),
}

trait Format {
    type Value;
}

struct Document<F: Format>(F::Value);

#[whichever::dispatchable]
trait Encode<F: Format> {
    fn encode(&self, value: &F::Value) -> String;
    fn encode_document(&self, document: &Document<F>) -> String {
        self.encode(&document.0)
    }
}

struct Json;

impl Format for Json {
    type Value = u32;
}

struct Compact;
struct Pretty;

impl Encode<Json> for Compact {
    fn encode(&self, value: &u32) -> String {
        format!("{value}")
    }
}

impl Encode<Json> for Pretty {
    fn encode(&self, value: &u32) -> String {
        format!("{value} ")
    }
}

#[whichever::dispatch(Encode<Json>)]
enum AnyEncoder {
    Compact(Compact),
    Pretty(Pretty),
}

#[whichever::dispatchable]
trait Buffer<const N: usize> {
    fn fill(&self) -> [u8; N];
}

struct Ones;
struct Twos;

impl Buffer<4> for Ones {
    fn fill(&self) -> [u8; 4] {
        [1; 4]
    }
}

impl Buffer<4> for Twos {
    fn fill(&self) -> [u8; 4] {
        [2; 4]
    }
}

#[whichever::dispatch(Buffer<4>)]
enum Filler {
    Ones(Ones),
    Twos(Twos),
}

#[test]
fn parameters_are_substituted_with_the_arguments_the_enum_names() {
    assert_eq!(Step::from(Inc).convert(1), 2);
    assert_eq!(Step::from(Dec).convert(1), 0);
    assert_eq!(AnyEncoder::from(Pretty).encode(&7), "7 ");
    assert_eq!(AnyEncoder::from(Compact).encode_document(&Document(3)), "3");
    assert_eq!(Filler::from(Twos).fill(), [2, 2, 2, 2]);
}
