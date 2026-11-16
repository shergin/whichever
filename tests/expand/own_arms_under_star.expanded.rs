//! Under `#[dispatch(*)]` a method's body holds the arms the enum answers
//! itself: the `match` is the forwarding arms in the enum's order, then
//! those arms as written, with no wildcard. `Opus` holds a payload with the
//! impl and is answered by the enum all the same. The arm of `size` under
//! `#[cfg]` answers where the feature is on, and `Hev1` is forwarded where
//! it is off; the forwarding arm is written under the negated `cfg`, so
//! this golden shows the build where the feature is off.
pub trait Encode {
    fn encode(&self, buf: &mut Vec<u8>) -> Result<(), EncodeError>;
    fn size(&self) -> usize;
    async fn flush(&mut self) -> Result<(), EncodeError>;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Encode_9ef7ae93561a3949_8_1 as Encode;
pub struct EncodeError;
pub struct Avc1(pub u8);
pub struct Hev1(pub u8);
pub struct Opus(pub u8);
impl Encode for Avc1 {
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
impl Avc1 {
    pub fn tag(&self) -> u8 {
        self.0
    }
}
impl Encode for Hev1 {
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
impl Hev1 {
    pub fn tag(&self) -> u8 {
        self.0
    }
}
impl Encode for Opus {
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
impl Opus {
    pub fn tag(&self) -> u8 {
        self.0
    }
}
pub enum Codec {
    Avc1(Avc1),
    Unknown(u32, Vec<u8>),
    Hev1(Hev1),
    Raw { kind: u32 },
    Opus(Opus),
    Empty,
}
#[automatically_derived]
impl ::core::convert::From<Avc1> for Codec {
    #[inline]
    fn from(__value: Avc1) -> Self {
        Self::Avc1(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Hev1> for Codec {
    #[inline]
    fn from(__value: Hev1) -> Self {
        Self::Hev1(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<Opus> for Codec {
    #[inline]
    fn from(__value: Opus) -> Self {
        Self::Opus(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Codec_bfaca1b45e299652_46_1 as Codec;
impl Encode for Codec {
    #[inline]
    fn encode(&self, buf: &mut Vec<u8>) -> Result<(), EncodeError> {
        match self {
            Codec::Avc1(__inner) => <Avc1 as Encode>::encode(__inner, buf),
            Codec::Hev1(__inner) => <Hev1 as Encode>::encode(__inner, buf),
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
    #[inline]
    fn size(&self) -> usize {
        match self {
            Codec::Avc1(__inner) => <Avc1 as Encode>::size(__inner),
            Codec::Hev1(__inner) => <Hev1 as Encode>::size(__inner),
            Self::Unknown(_, data) => 4 + data.len(),
            Self::Raw { .. } => 4,
            Self::Opus(..) | Self::Empty => 0,
        }
    }
    #[inline]
    async fn flush(&mut self) -> Result<(), EncodeError> {
        match self {
            Codec::Avc1(__inner) => <Avc1 as Encode>::flush(__inner).await,
            Codec::Hev1(__inner) => <Hev1 as Encode>::flush(__inner).await,
            Self::Unknown(_, data) => {
                data.clear();
                Ok(())
            }
            Self::Raw { .. } | Self::Opus(_) | Self::Empty => Err(EncodeError),
        }
    }
}
impl Codec {
    pub fn tag(&self) -> u8 {
        match self {
            Self::Avc1(__inner) => __inner.tag(),
            Self::Hev1(__inner) => __inner.tag(),
            Self::Opus(__inner) => __inner.tag(),
            Self::Unknown(kind, _) => *kind as u8,
            Self::Raw { .. } | Self::Empty => 0,
        }
    }
}
fn main() {}
