//! The by-reference `TryFrom` forms, and `into = Target` through every
//! payload's own `Into`.
pub struct Text(pub String);
pub struct Number(pub u32);
pub struct Printed(pub String);
impl From<Text> for Printed {
    fn from(text: Text) -> Self {
        Printed(text.0)
    }
}
impl From<Number> for Printed {
    fn from(number: Number) -> Self {
        Printed(number.0.to_string())
    }
}
pub enum Value {
    Text(Text),
    Number(Number),
}
#[automatically_derived]
impl ::core::convert::From<Text> for Value {
    #[inline]
    fn from(__value: Text) -> Self {
        Self::Text(__value)
    }
}
#[automatically_derived]
impl ::core::convert::TryFrom<Value> for Text {
    type Error = Value;
    #[inline]
    #[allow(unreachable_patterns)]
    fn try_from(__value: Value) -> ::core::result::Result<Self, Self::Error> {
        match __value {
            Value::Text(__value) => ::core::result::Result::Ok(__value),
            __other => ::core::result::Result::Err(__other),
        }
    }
}
#[automatically_derived]
impl<'__enum> ::core::convert::TryFrom<&'__enum Value> for &'__enum Text {
    type Error = &'__enum Value;
    #[inline]
    #[allow(unreachable_patterns)]
    fn try_from(__value: &'__enum Value) -> ::core::result::Result<Self, Self::Error> {
        match __value {
            Value::Text(__value) => ::core::result::Result::Ok(__value),
            __other => ::core::result::Result::Err(__other),
        }
    }
}
#[automatically_derived]
impl<'__enum> ::core::convert::TryFrom<&'__enum mut Value> for &'__enum mut Text {
    type Error = &'__enum mut Value;
    #[inline]
    #[allow(unreachable_patterns)]
    fn try_from(
        __value: &'__enum mut Value,
    ) -> ::core::result::Result<Self, Self::Error> {
        match __value {
            Value::Text(__value) => ::core::result::Result::Ok(__value),
            __other => ::core::result::Result::Err(__other),
        }
    }
}
#[automatically_derived]
impl ::core::convert::From<Number> for Value {
    #[inline]
    fn from(__value: Number) -> Self {
        Self::Number(__value)
    }
}
#[automatically_derived]
impl ::core::convert::TryFrom<Value> for Number {
    type Error = Value;
    #[inline]
    #[allow(unreachable_patterns)]
    fn try_from(__value: Value) -> ::core::result::Result<Self, Self::Error> {
        match __value {
            Value::Number(__value) => ::core::result::Result::Ok(__value),
            __other => ::core::result::Result::Err(__other),
        }
    }
}
#[automatically_derived]
impl<'__enum> ::core::convert::TryFrom<&'__enum Value> for &'__enum Number {
    type Error = &'__enum Value;
    #[inline]
    #[allow(unreachable_patterns)]
    fn try_from(__value: &'__enum Value) -> ::core::result::Result<Self, Self::Error> {
        match __value {
            Value::Number(__value) => ::core::result::Result::Ok(__value),
            __other => ::core::result::Result::Err(__other),
        }
    }
}
#[automatically_derived]
impl<'__enum> ::core::convert::TryFrom<&'__enum mut Value> for &'__enum mut Number {
    type Error = &'__enum mut Value;
    #[inline]
    #[allow(unreachable_patterns)]
    fn try_from(
        __value: &'__enum mut Value,
    ) -> ::core::result::Result<Self, Self::Error> {
        match __value {
            Value::Number(__value) => ::core::result::Result::Ok(__value),
            __other => ::core::result::Result::Err(__other),
        }
    }
}
#[automatically_derived]
impl ::core::convert::From<Value> for Printed {
    #[inline]
    fn from(__value: Value) -> Self {
        match __value {
            Value::Text(__value) => ::core::convert::Into::into(__value),
            Value::Number(__value) => ::core::convert::Into::into(__value),
        }
    }
}
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) use __whichever_dispatch_Value_137c5dfeb17a4583_19_1 as Value;
fn main() {}
