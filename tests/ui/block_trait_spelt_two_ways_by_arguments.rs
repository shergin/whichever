//! A field names a generic trait as a partial block's header spells it,
//! its generic arguments included; without them it is told on the block.
#[whichever::dispatchable]
trait Encoder<F> {
    fn encode(&self, format: F) -> u8;
}

struct Json;
struct JsonEncoder;

impl Encoder<Json> for JsonEncoder {
    fn encode(&self, _: Json) -> u8 {
        1
    }
}

#[whichever::dispatch]
struct Env<E, N> {
    #[dispatch(Encoder)]
    encoder: E,
    #[dispatch]
    named: N,
}

#[whichever::dispatch]
impl<F, E: Encoder<F>, N> Encoder<F> for Env<E, N> {}

fn main() {}
