//! A field names a generic trait as a partial block's header spells it,
//! its generic arguments included; without them it is told on the block,
//! and the impl is written for the field that was meant, so the sentence
//! stands alone: no error on the bare field, which does not implement the
//! trait, and none at the call.
#[whichever::dispatchable]
trait Encoder<F> {
    fn encode(&self, format: F) -> u8;
}

struct Json;
struct JsonEncoder;
struct Label;

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

fn encode(env: &Env<JsonEncoder, Label>) -> u8 {
    env.encode(Json)
}

fn main() {}
