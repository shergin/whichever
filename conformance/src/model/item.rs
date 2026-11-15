//! What the attributes on the trait's method under test are, and what each
//! asks of the impls and of the forwarding (what-is-generated.md, the last
//! rows of its table).

use super::axes::Item;

/// The attributes' description, which the renderers interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemDescriptor {
    /// The attribute the trait's method carries, without the `#[..]`.
    pub attribute: Option<&'static str>,
    /// The predicate of the `#[cfg]` the method is under. Every impl of the
    /// method, and a block's mark on it, is written under the same `cfg`,
    /// as a hand writes them.
    pub cfg: Option<&'static str>,
    /// Whether the method exists in this build.
    pub present: bool,
    /// The lint the method's signature trips and the trait allows, which
    /// every impl repeating the signature allows too.
    pub allowed: Option<&'static str>,
    /// Whether a call of the method is under `#[allow(deprecated)]`.
    pub deprecated: bool,
    /// The `#[inline]` the forwarding carries: the method's own, or the
    /// plain one.
    pub inline: &'static str,
    /// The name of the method's `u64` argument: one the lint the method
    /// allows would warn of, or the plain `x`.
    pub parameter: &'static str,
}

impl Item {
    /// Returns the attributes' description.
    pub fn descriptor(self) -> ItemDescriptor {
        let bare = ItemDescriptor {
            attribute: None,
            cfg: None,
            present: true,
            allowed: None,
            deprecated: false,
            inline: "inline",
            parameter: "x",
        };
        match self {
            Item::Bare => bare,
            Item::CfgOn => ItemDescriptor {
                attribute: Some("cfg(all())"),
                cfg: Some("all()"),
                ..bare
            },
            Item::CfgOff => ItemDescriptor {
                attribute: Some("cfg(any())"),
                cfg: Some("any()"),
                present: false,
                ..bare
            },
            Item::Deprecated => ItemDescriptor {
                attribute: Some("deprecated"),
                deprecated: true,
                ..bare
            },
            Item::Allow => ItemDescriptor {
                attribute: Some("allow(non_snake_case)"),
                allowed: Some("non_snake_case"),
                parameter: "Count",
                ..bare
            },
            Item::Inline => ItemDescriptor {
                attribute: Some("inline(always)"),
                inline: "inline(always)",
                ..bare
            },
        }
    }
}
