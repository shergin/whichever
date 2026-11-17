//! The words a sentence speaks of a subject.
//!
//! An enum forwards to its variants and a struct to a field, and a sentence
//! about either says so in the words of the item the user wrote: a struct
//! has no variant to name, and nothing it does is the enum's. A sentence is
//! written where the subject is known, at the subject's own attribute and
//! in the two implementation macros, and takes its words from the subject.
//! The attribute of an impl block is read before the subject's carrier
//! brings the subject, so a sentence written there names neither.

/// Whose words a sentence speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Words {
    /// An enum's, which forwards to its variants.
    Enum,
    /// A struct's, which forwards to a field.
    Struct,
}

impl Words {
    /// Returns what the subject is called.
    pub(crate) fn subject(self) -> &'static str {
        match self {
            Words::Enum => "enum",
            Words::Struct => "struct",
        }
    }

    /// Returns what the subject's targets are called.
    pub(crate) fn target(self) -> &'static str {
        match self {
            Words::Enum => "variant",
            Words::Struct => "field",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_enum_forwards_to_variants_and_a_struct_to_a_field() {
        assert_eq!(
            (Words::Enum.subject(), Words::Enum.target()),
            ("enum", "variant")
        );
        assert_eq!(
            (Words::Struct.subject(), Words::Struct.target()),
            ("struct", "field")
        );
    }
}
