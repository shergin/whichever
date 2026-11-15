//! What a cell's generics are: the trait's parameters and the argument it
//! is dispatched at, or the subject's own parameter (options.md, its last
//! paragraphs; goldens `generic_payloads` and
//! `associated_types_and_generics`).

use super::axes::Generics;

/// The generics' description, which the renderers interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenericsDescriptor {
    /// The trait's parameters as it declares them, `<F: support::Format>`.
    pub declared: &'static str,
    /// The trait's arguments where an impl names it, `<support::Json>`.
    pub argument: &'static str,
    /// The trait's arguments where the law names it on a value, `<3>`.
    pub instance: &'static str,
    /// The subject's parameter: as it declares it, as its type names it,
    /// as its impl declares it, and the argument the law takes it at.
    pub parameter: Option<Parameter>,
    /// Whether the method's argument is a projection through the trait's
    /// parameter, which is `u64` at `support::Json`.
    pub projected: bool,
    /// The projection the trait's signature spells the argument with:
    /// `F::Value`, or `F::Pair<F>`, whose later segment has generic
    /// arguments that are substituted too (options.md, the last paragraph).
    pub projection: &'static str,
    /// Whether the trait's bound fixes the projection to `u64` where a
    /// default body counts the argument.
    pub fixed: bool,
    /// Whether the first variant's payload is the subject's parameter.
    pub payload: bool,
    /// Whether the answers count the trait's const parameter.
    pub constant: bool,
}

/// A parameter of the subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parameter {
    /// As the subject declares it, `T` or `const N: u64`.
    pub declared: &'static str,
    /// As the subject's type names it, `T` or `N`.
    pub named: &'static str,
    /// As an impl declares it, with the bound a hand writes, `T: Probe`.
    pub bounded: &'static str,
    /// What the law instantiates it at, `P0` or `3`.
    pub instance: &'static str,
}

impl Generics {
    /// Returns the generics' description.
    pub fn descriptor(self) -> GenericsDescriptor {
        let bare = GenericsDescriptor {
            declared: "",
            argument: "",
            instance: "",
            parameter: None,
            projected: false,
            projection: "",
            fixed: false,
            payload: false,
            constant: false,
        };
        match self {
            Generics::Bare => bare,
            Generics::Paired => GenericsDescriptor {
                declared: "<F: support::Format>",
                argument: "<support::Json>",
                instance: "<support::Json>",
                projected: true,
                projection: "F::Pair<F>",
                ..bare
            },
            Generics::Payload => GenericsDescriptor {
                parameter: Some(Parameter {
                    declared: "T",
                    named: "T",
                    bounded: "T: Probe",
                    instance: "P0",
                }),
                payload: true,
                ..bare
            },
            Generics::Concrete => GenericsDescriptor {
                declared: "<F: support::Format>",
                argument: "<support::Json>",
                instance: "<support::Json>",
                projected: true,
                projection: "F::Value",
                fixed: true,
                ..bare
            },
            Generics::Own => GenericsDescriptor {
                declared: "<const N: u64>",
                argument: "<N>",
                instance: "<3>",
                parameter: Some(Parameter {
                    declared: "const N: u64",
                    named: "N",
                    bounded: "const N: u64",
                    instance: "3",
                }),
                constant: true,
                ..bare
            },
            Generics::Defaulted => GenericsDescriptor {
                declared: "<F: support::Format = support::Json>",
                projected: true,
                projection: "F::Value",
                ..bare
            },
        }
    }
}
