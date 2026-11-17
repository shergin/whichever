//! Under `#[dispatch(*)]` a method's body holds the arms the enum answers
//! itself: the `match` is the forwarding arms in the enum's order, then
//! those arms as written, with no wildcard. `Opus` holds a payload with the
//! impl and is answered by the enum all the same. The arm of `size` under
//! `#[cfg]` answers where the feature is on, and `Hev1` is forwarded where
//! it is off; the forwarding arm is written under the negated `cfg`, so
//! this golden shows the build where the feature is off.
#[whichever::dispatchable]
pub trait Encode {
    fn encode(&self, buf: &mut Vec<u8>) -> Result<(), EncodeError>;
    fn size(&self) -> usize;
    async fn flush(&mut self) -> Result<(), EncodeError>;
}

pub struct EncodeError;

pub struct Avc1(pub u8);
pub struct Hev1(pub u8);
pub struct Opus(pub u8);

macro_rules! encoded {
    ($($codec:ident),*) => {$(
        impl Encode for $codec {
            fn encode(&self, buf: &mut Vec<u8>) -> Result<(), EncodeError> {
                buf.push(self.0);
                Ok(())
            }
            fn size(&self) -> usize {
                1
            }
            async fn flush(&mut self) -> Result<(), EncodeError> {
                Ok(())
            }
        }

        impl $codec {
            pub fn tag(&self) -> u8 {
                self.0
            }
        }
    )*};
}

encoded!(Avc1, Hev1, Opus);

#[whichever::dispatch]
pub enum Codec {
    Avc1(Avc1),
    Unknown(u32, Vec<u8>),
    Hev1(Hev1),
    Raw { kind: u32 },
    Opus(Opus),
    Empty,
}

#[whichever::dispatch]
impl Encode for Codec {
    #[dispatch(*)]
    fn encode(&self, buf: &mut Vec<u8>) -> Result<(), EncodeError> {
        match self {
            Self::Unknown(kind, data) => {
                buf.extend_from_slice(&kind.to_be_bytes());
                buf.extend_from_slice(data);
                Ok(())
            }
            Self::Raw { kind } => {
                buf.extend_from_slice(&kind.to_be_bytes());
                Ok(())
            }
            Self::Opus(_) | Self::Empty => Err(EncodeError),
        }
    }

    #[dispatch(*)]
    fn size(&self) -> usize {
        match self {
            #[cfg(feature = "sized")]
            Self::Hev1(..) => 0,
            Self::Unknown(_, data) => 4 + data.len(),
            Self::Raw { .. } => 4,
            Self::Opus(..) | Self::Empty => 0,
        }
    }

    #[dispatch(*)]
    async fn flush(&mut self) -> Result<(), EncodeError> {
        match self {
            Self::Unknown(_, data) => {
                data.clear();
                Ok(())
            }
            Self::Raw { .. } | Self::Opus(_) | Self::Empty => Err(EncodeError),
        }
    }
}

#[whichever::dispatch]
impl Codec {
    #[dispatch(*)]
    pub fn tag(&self) -> u8 {
        match self {
            Self::Unknown(kind, _) => *kind as u8,
            Self::Raw { .. } | Self::Empty => 0,
        }
    }
}

fn main() {}
