//! The zones of stage 1, each the full product of a few axes.
//!
//! Each zone crosses the axes whose meeting a forwarding has to get right
//! at once: how the enum is unwrapped and the target reached (`recv`,
//! `s0`), how the answer is converted (`ret`), what else the arm matches
//! (`arg`, `s1`) and how the call is made (`qual`). The axes a zone holds
//! neutral are the ones it does not test.

use super::Zone;

/// The returns `arguments` crosses: one of each conversion.
const SOME_RETURNS: &[&str] = &[
    "u64",
    "unit",
    "ref",
    "mut",
    "self",
    "option-self",
    "tuple-self",
    "out",
    "result-error",
    "result-self-error",
    "future",
    "boxed-future",
];

/// Every target kind but `via-method-value`, whose every passing cell
/// diverges for one reason, so that a zone of its own holds it.
const TARGETS: &[&str] = &[
    "plain",
    "deref-box",
    "deref-rc",
    "deref-arc",
    "deref-ref",
    "deref-mut",
    "via-field",
    "via-index",
    "via-method",
    "marked-tuple",
    "marked-struct",
    "marked-via",
];

/// The target kinds `arguments` and `qualifiers` cross: the payload, a
/// pointer that owns, one that lends, a field and a marked field.
const SOME_TARGETS: &[&str] = &[
    "plain",
    "deref-box",
    "deref-ref",
    "via-field",
    "marked-tuple",
];

/// Every zone, in the order a run reports them.
pub const ZONES: &[Zone] = &[
    Zone {
        name: "shapes",
        summary: "every receiver, return type and target kind of the first variant",
        vary: &[("s0", TARGETS), ("recv", &[]), ("ret", &[])],
        fixed: &[],
    },
    Zone {
        name: "pairs",
        summary: "every pair of variants under every receiver",
        vary: &[("s0", TARGETS), ("s1", &[]), ("recv", &[])],
        fixed: &[],
    },
    Zone {
        name: "arguments",
        summary: "every `Self` argument under every receiver, over one or two variants of a few target kinds",
        vary: &[
            ("arg", &[]),
            ("s0", SOME_TARGETS),
            ("s1", &["plain", "none", "deref-box"]),
            ("recv", &[]),
            ("ret", SOME_RETURNS),
        ],
        fixed: &[],
    },
    Zone {
        name: "boxed",
        summary: "every receiver, return type and qualifier over two `deref` variants holding a `Box`, which `self: Box<Self>` needs",
        vary: &[
            ("recv", &[]),
            ("ret", &[]),
            ("qual", &[]),
            ("arg", &["none", "ref-trait", "value", "mut"]),
        ],
        fixed: &[("s0", "deref-box"), ("s1", "deref-box")],
    },
    Zone {
        name: "qualifiers",
        summary: "every qualifier under every receiver and return type, over a few target kinds",
        vary: &[("qual", &[]), ("s0", SOME_TARGETS), ("recv", &[]), ("ret", &[])],
        fixed: &[],
    },
    Zone {
        name: "value-method-route",
        summary: "a route ending in a method that returns its target by value, which `docs/routes.md` says serves `&self` and `self` methods",
        vary: &[("recv", &["ref", "value", "pin-ref"]), ("ret", &["u64", "self"])],
        fixed: &[("s0", "via-method-value")],
    },
];
