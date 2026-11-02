#![allow(async_fn_in_trait)]

pub trait Format {
    type Value;
}

pub struct Json;

impl Format for Json {
    type Value = u32;
}

#[whichever::dispatchable]
pub trait Encoder<F: Format> {
    type Error;
    async fn encode(&self, value: &F::Value) -> Result<String, Self::Error>;
}

pub struct Compact;
pub struct Pretty;

impl Encoder<Json> for Compact {
    type Error = u8;
    async fn encode(&self, value: &u32) -> Result<String, u8> {
        Ok(value.to_string())
    }
}

impl Encoder<Json> for Pretty {
    type Error = u16;
    async fn encode(&self, value: &u32) -> Result<String, u16> {
        Ok(format!("{value} "))
    }
}

#[whichever::dispatch(Encoder<Json>, Error = u16)]
pub enum AnyEncoder {
    Compact(Compact),
    Pretty(Pretty),
}

fn main() {}
