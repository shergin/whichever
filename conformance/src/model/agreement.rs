//! What the trait's own associated items are: their kind, name and type,
//! what each payload gives them, and what the enum fixes them at
//! (what-is-generated.md, the first two rows of its table; options.md on
//! `Name = Type` and `Name = value`; limits.md on consts).

use super::axes::Agreement;

/// What an associated item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    /// `type Name;`.
    Type,
    /// `const NAME: Type;`.
    Const,
    /// `fn name() -> Type { default }`, a provided method without a
    /// receiver: "On an enum, left to the trait's default body [..]. On a
    /// struct, `<Target as Trait>::method(args..)`" (what-is-generated.md).
    Function,
}

/// An associated item of the trait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssociatedItem {
    /// Whether it is a type or a const.
    pub kind: ItemKind,
    /// Its name.
    pub name: &'static str,
    /// The type a const is declared with; empty for a type.
    pub ty: &'static str,
    /// What each payload gives it, by slot: a type, or a value.
    pub values: [&'static str; 3],
    /// What the enum's attribute fixes it at with `Name = ..`, if anything.
    pub overridden: Option<&'static str>,
    /// Whether the kind is one the macro checks the payloads agree on:
    /// every type, and a const of an integer, a float, `bool`, `char` or
    /// `&str`. A const is checked on an enum without a type or const parameter
    /// only.
    pub comparable: bool,
}

impl AssociatedItem {
    /// Returns whether two payloads, by slot, give the item different
    /// types or values.
    pub fn differs(&self, first: usize, second: usize) -> bool {
        self.values[first] != self.values[second]
    }

    /// Returns the item as the trait declares it.
    pub fn declaration(&self) -> String {
        match self.kind {
            ItemKind::Type => format!("type {};", self.name),
            ItemKind::Const => format!("const {}: {};", self.name, self.ty),
            ItemKind::Function => {
                format!(
                    "fn {}() -> {} {{ {} }}",
                    self.name, self.ty, FUNCTION_DEFAULT
                )
            }
        }
    }

    /// Returns the item as an impl defines it, `value` being its type or
    /// value there.
    pub fn definition(&self, value: &str) -> String {
        match self.kind {
            ItemKind::Type => format!("type {} = {value};", self.name),
            ItemKind::Const => format!("const {}: {} = {value};", self.name, self.ty),
            ItemKind::Function => format!("fn {}() -> {} {{ {value} }}", self.name, self.ty),
        }
    }
}

/// What the trait's default body of [`ItemKind::Function`] answers.
pub const FUNCTION_DEFAULT: &str = "7";

/// The items' description, which the renderers interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgreementDescriptor {
    /// The items the trait declares beside those its return types name.
    pub items: &'static [AssociatedItem],
    /// Whether the trait declares `fn kind(..) -> Self::Kind`, which every
    /// subject forwards as it forwards the control method, so that the
    /// type the payloads give meets in its arms.
    pub returned: bool,
}

/// The type the payloads give differently, `u32` in `P0` and `P2` and
/// `u64` in `P1`; the third agrees with the first, so a check across three
/// payloads that reports it would be wrong.
const KIND: AssociatedItem = AssociatedItem {
    kind: ItemKind::Type,
    name: "Kind",
    ty: "",
    values: ["u32", "u64", "u32"],
    overridden: None,
    comparable: true,
};

/// Returns a const of a checked type that every payload gives `value`.
const fn agreed(name: &'static str, ty: &'static str, value: &'static str) -> AssociatedItem {
    AssociatedItem {
        kind: ItemKind::Const,
        name,
        ty,
        values: [value, value, value],
        overridden: None,
        comparable: true,
    }
}

const LEVEL: AssociatedItem = agreed("LEVEL", "u32", "7");
const RATIO: AssociatedItem = agreed("RATIO", "f64", "0.5");
const ON: AssociatedItem = agreed("ON", "bool", "true");
const MARK: AssociatedItem = agreed("MARK", "char", "'m'");
const NAME: AssociatedItem = agreed("NAME", "&'static str", "\"probe\"");

impl Agreement {
    /// Returns the items' description.
    pub fn descriptor(self) -> AgreementDescriptor {
        let items: &'static [AssociatedItem] = match self {
            Agreement::Absent => &[],
            Agreement::TypeDiffers | Agreement::TypeReturned => &[KIND],
            Agreement::ConstAgrees => &[LEVEL, RATIO, ON, MARK, NAME],
            Agreement::ConstDiffers => &[
                // The third payload differs from both, so each variant that
                // disagrees is reported on its own.
                AssociatedItem {
                    values: ["7", "8", "9"],
                    ..LEVEL
                },
                RATIO,
                ON,
                MARK,
                NAME,
            ],
            Agreement::ConstUnchecked => &[AssociatedItem {
                kind: ItemKind::Const,
                name: "PAIR",
                ty: "Option<u8>",
                values: ["Some(1)", "Some(2)", "Some(3)"],
                overridden: None,
                comparable: false,
            }],
            // Every payload overrides it, so that the default the enum
            // keeps and the field's the struct takes tell apart.
            Agreement::Function => &[AssociatedItem {
                kind: ItemKind::Function,
                name: "label",
                ty: "u64",
                values: ["11", "12", "13"],
                overridden: None,
                comparable: false,
            }],
            Agreement::ConstOverride => &[
                AssociatedItem {
                    values: ["\"zero\"", "\"one\"", "\"two\""],
                    overridden: Some("\"any\""),
                    ..NAME
                },
                // A tuple of paths parses as a type first, and is read as a
                // value once the trait says the item is a const (behaviour
                // test `tuple_const_override`).
                AssociatedItem {
                    kind: ItemKind::Const,
                    name: "RANGE",
                    ty: "(u8, u8)",
                    values: ["(4, 5)", "(0, 255)", "(6, 7)"],
                    overridden: Some("(support::LOW, support::HIGH)"),
                    comparable: false,
                },
            ],
        };
        let returned = match self {
            Agreement::TypeReturned => true,
            Agreement::Absent
            | Agreement::TypeDiffers
            | Agreement::ConstAgrees
            | Agreement::ConstDiffers
            | Agreement::ConstUnchecked
            | Agreement::ConstOverride
            | Agreement::Function => false,
        };
        AgreementDescriptor { items, returned }
    }
}
