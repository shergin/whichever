//! A deprecated method or variant is forwarded under `#[allow(deprecated)]`.
pub trait Versioned {
    #[deprecated(note = "use `name`")]
    fn kind(&self) -> &'static str;
    fn name(&self) -> &'static str;
}
#[doc(hidden)]
#[allow(unused_imports)]
pub use __whichever_Versioned_a116b997baf99684_2_1 as Versioned;
pub struct Old;
pub struct New;
impl Versioned for Old {
    fn kind(&self) -> &'static str {
        "old"
    }
    fn name(&self) -> &'static str {
        "old"
    }
}
impl Versioned for New {
    fn kind(&self) -> &'static str {
        "new"
    }
    fn name(&self) -> &'static str {
        "new"
    }
}
pub enum Any {
    #[deprecated(note = "use `New`")]
    Old(Old),
    New(New),
}
#[allow(deprecated)]
#[automatically_derived]
impl ::core::convert::From<Old> for Any {
    #[inline]
    fn from(__value: Old) -> Self {
        Self::Old(__value)
    }
}
#[automatically_derived]
impl ::core::convert::From<New> for Any {
    #[inline]
    fn from(__value: New) -> Self {
        Self::New(__value)
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Any_cf97171b26303cdb_30_1 as Any;
#[automatically_derived]
impl Versioned for Any {
    #[allow(deprecated)]
    #[inline]
    fn kind(&self) -> &'static str {
        match self {
            #[allow(deprecated)]
            Any::Old(__inner) => <Old as Versioned>::kind(__inner),
            Any::New(__inner) => <New as Versioned>::kind(__inner),
        }
    }
    #[inline]
    fn name(&self) -> &'static str {
        match self {
            #[allow(deprecated)]
            Any::Old(__inner) => <Old as Versioned>::name(__inner),
            Any::New(__inner) => <New as Versioned>::name(__inner),
        }
    }
}
fn main() {}
