//! What the conversions are: the options the subject's attribute or its
//! first variant spells, and which impls each writes (options.md,
//! routes.md, structs.md).

use super::axes::Conversion;

/// How `TryFrom` is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TryForm {
    /// `TryFrom<Enum> for Payload`.
    Value,
    /// Also `TryFrom<&Enum> for &Payload`.
    Shared,
    /// Also `TryFrom<&mut Enum> for &mut Payload`.
    Exclusive,
}

/// The conversions' description, which the renderers interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConversionDescriptor {
    /// Whether the law checks the conversions at all.
    pub checked: bool,
    /// The option on the subject's attribute, if any.
    pub option: Option<&'static str>,
    /// The option on the first variant, if any.
    pub variant_option: Option<&'static str>,
    /// Whether `From<Payload>` is written for every variant that takes
    /// part, the first included.
    pub from: bool,
    /// Whether the first variant is left out of `From` and `TryFrom`.
    pub first_left_out: bool,
    /// The `TryFrom` impls the attribute writes for every payload.
    pub try_from: Option<TryForm>,
    /// Whether `TryFrom` is written for the first variant alone.
    pub first_try: bool,
    /// Whether `From<Enum> for support::Sink` is written.
    pub into: bool,
}

impl Conversion {
    /// Returns the conversions' description.
    pub fn descriptor(self) -> ConversionDescriptor {
        let checked = ConversionDescriptor {
            checked: true,
            option: None,
            variant_option: None,
            from: true,
            first_left_out: false,
            try_from: None,
            first_try: false,
            into: false,
        };
        let with_try = |option, form| ConversionDescriptor {
            option: Some(option),
            try_from: Some(form),
            ..checked
        };
        match self {
            Conversion::Unchecked => ConversionDescriptor {
                checked: false,
                ..checked
            },
            Conversion::From => checked,
            Conversion::FromOff => ConversionDescriptor {
                option: Some("from = false"),
                from: false,
                ..checked
            },
            Conversion::Try => with_try("try_from = true", TryForm::Value),
            Conversion::TryRef => with_try("try_from = ref", TryForm::Shared),
            Conversion::TryMut => with_try("try_from = ref_mut", TryForm::Exclusive),
            Conversion::Into => ConversionDescriptor {
                option: Some("into = support::Sink"),
                into: true,
                ..checked
            },
            Conversion::VariantFromOff => ConversionDescriptor {
                variant_option: Some("from = false"),
                first_left_out: true,
                ..checked
            },
            Conversion::VariantTry => ConversionDescriptor {
                variant_option: Some("try_from = true"),
                first_try: true,
                ..checked
            },
        }
    }
}
