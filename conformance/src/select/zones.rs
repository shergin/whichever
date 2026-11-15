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
    "fallible-self",
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

/// The receivers that forward; the three a sentence rejects meet the
/// other axes in `shapes`, `pairs`, `boxed` and the arrays.
const FORWARDED: &[&str] = &["ref", "mut", "value", "box", "pin-ref", "pin-mut"];

/// The routes of several steps: a dereference then a field, behind a
/// `Box` and an `Rc`, two fields, and a field of what a method lends.
const ROUTES: &[&str] = &[
    "deref-box-field",
    "deref-rc-field",
    "via-index-field",
    "via-method-field",
];

/// The target kinds and the unit variant a third variant and the second
/// beside it take.
const LATER: &[&str] = &["plain", "deref-box", "via-field", "unit"];

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
            ("s1", &["plain", "none"]),
            ("recv", FORWARDED),
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
            ("arg", &["none", "ref-trait", "mut"]),
        ],
        fixed: &[("s0", "deref-box"), ("s1", "deref-box")],
    },
    Zone {
        name: "boxed-futures",
        summary: "a boxed future spelled as `#[async_trait]` writes it, forwarded as it is or converted, under every receiver that forwards, with the `Self` arguments and type parameters its lifetimes bound and a variant over an `Rc` its `+ Send` meets",
        vary: &[
            ("ret", &["boxed-future", "boxed-future-out"]),
            ("s0", &["plain", "deref-rc", "deref-ref", "struct"]),
            ("s1", &["plain", "none"]),
            ("recv", FORWARDED),
            ("arg", &["none", "ref-trait", "value", "mut"]),
            ("qual", &["plain", "generic", "turbofish"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "qualifiers",
        summary: "every qualifier under every receiver and return type, over a few target kinds",
        vary: &[("qual", &[]), ("s0", SOME_TARGETS), ("recv", FORWARDED), ("ret", &[])],
        fixed: &[],
    },
    Zone {
        name: "value-method-route",
        summary: "a route ending in a method that yields its target by value, which serves `self` by value",
        vary: &[
            ("recv", &["ref", "value", "pin-ref", "box"]),
            ("ret", &[]),
            ("s1", &["none", "plain"]),
            ("arg", &["none", "value", "ref-enum"]),
        ],
        fixed: &[("s0", "via-method-value")],
    },
    Zone {
        name: "paths",
        summary: "every path under a few receivers, every return type and a few second variants, required or provided",
        vary: &[
            ("path", &[]),
            ("recv", &["ref", "mut", "value", "box", "pin-mut"]),
            ("ret", &[]),
            ("s1", &["plain", "unit", "deref-box"]),
            ("qual", &["plain", "provided"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "path-arguments",
        summary: "every path with every `Self` argument",
        vary: &[
            ("path", &[]),
            ("arg", &[]),
            ("recv", &["ref", "mut", "value"]),
            ("ret", &["u64", "unit", "self", "option-self"]),
            ("s1", &["plain", "none", "unit"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "inherent",
        summary: "the inherent paths over every target kind, under the receivers, returns and qualifiers the inherent docs describe",
        vary: &[
            ("path", &["inherent", "inherent-some"]),
            ("s0", &[]),
            ("s1", &["plain", "none", "unit"]),
            ("recv", &["ref", "mut", "value"]),
            ("ret", &["u64", "unit", "ref", "mut", "self", "option-self", "result-self", "future"]),
            ("qual", &["plain", "async"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "inherent-places",
        summary: "the 408-cell probe of 2026-11-13: `value` and `field` over the target kinds, receivers, returns and lifts",
        vary: &[
            ("s0", &["plain", "deref-box", "deref-rc", "via-field", "marked-tuple", "marked-struct", "marked-via"]),
            ("recv", &["value", "ref", "mut", "box", "pin-mut"]),
            ("callee", &["value", "field"]),
            ("ret", &["u64", "ref", "mut"]),
            ("lift", &["none", "some", "ok", "by-ref", "by-value"]),
        ],
        fixed: &[("path", "inherent"), ("s1", "none")],
    },
    Zone {
        name: "inherent-calls",
        summary: "an inherent method's call, renamed or not, under every lift",
        vary: &[
            ("path", &["inherent", "inherent-some"]),
            ("callee", &["method", "renamed"]),
            ("lift", &[]),
            ("s0", &["plain", "deref-box", "via-field", "marked-tuple"]),
            ("recv", &["ref", "mut", "value"]),
            ("ret", &["u64", "ref", "self", "option-self", "future"]),
            ("s1", &["plain", "none"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "inherent-scope",
        summary: "an inherent method's `Self` argument, its boxed and pinned receivers and its qualifiers, over enums and structs",
        vary: &[
            ("path", &["inherent", "inherent-some"]),
            ("s0", &["plain", "deref-box", "via-field", "marked-tuple", "struct"]),
            ("s1", &["plain", "none"]),
            ("recv", &["ref", "value", "box", "pin-ref", "pin-mut"]),
            ("arg", &["none", "ref-bare", "value", "mut"]),
            ("qual", &["plain", "unsafe", "generic", "turbofish"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "inherent-struct-returns",
        summary: "an inherent method of a struct subject over every return type, a return-position `impl Trait` forwarded as it is, under the lifts that wrap it",
        vary: &[
            ("s0", &["struct", "struct-marked", "struct-named"]),
            ("ret", &[]),
            ("recv", &["ref", "mut", "value"]),
            ("lift", &["none", "some"]),
        ],
        fixed: &[("path", "inherent"), ("s1", "none")],
    },
    Zone {
        name: "inherent-into",
        summary: "`with = Into::into` over the answers it converts, through the enum's `From` impls or into themselves",
        vary: &[
            ("path", &["inherent", "inherent-some"]),
            ("callee", &["method", "renamed"]),
            ("ret", &["u64", "unit", "ref", "mut", "self", "option-self"]),
            ("s0", &["plain", "deref-box", "via-field", "marked-tuple", "struct"]),
            ("s1", &["plain", "none", "deref-ref", "unit"]),
            ("recv", &["ref", "mut", "value"]),
        ],
        fixed: &[("lift", "into")],
    },
    Zone {
        name: "inherent-shared",
        summary: "`value` over two variants that hold one payload type, reached by different target kinds",
        vary: &[
            ("s0", TARGETS),
            ("s1", &["plain", "deref-box", "deref-rc", "via-field", "via-index", "marked-tuple", "marked-struct", "marked-via"]),
            ("recv", &["ref", "mut", "value"]),
            ("ret", &["u64", "ref"]),
        ],
        fixed: &[("path", "inherent"), ("callee", "value")],
    },
    Zone {
        name: "structs",
        summary: "the struct subjects under every receiver, return type and qualifier",
        vary: &[
            ("s0", &["struct", "struct-marked", "struct-named"]),
            ("recv", &[]),
            ("ret", &[]),
            ("qual", &[]),
        ],
        fixed: &[("s1", "none")],
    },
    Zone {
        name: "struct-arguments",
        summary: "the struct subjects with every `Self` argument",
        vary: &[
            ("s0", &["struct", "struct-marked", "struct-named"]),
            ("arg", &[]),
            ("recv", &["ref", "mut", "value"]),
            ("ret", &["u64", "unit", "self", "option-self", "future"]),
        ],
        fixed: &[("s1", "none")],
    },
    Zone {
        name: "struct-paths",
        summary: "the struct subjects through the block paths",
        vary: &[
            ("s0", &["struct", "struct-marked", "struct-named"]),
            ("path", &["attribute", "block", "block-star", "block-default", "block-written"]),
            ("ret", &[]),
            ("qual", &["plain", "provided"]),
        ],
        fixed: &[("s1", "none")],
    },
    Zone {
        name: "cfg",
        summary: "variants under `#[cfg]`, present and compiled out, over the second variants, receivers and returns they meet",
        vary: &[
            ("cfg", &[]),
            ("s1", &["plain", "none", "unit", "pair", "deref-box", "via-field"]),
            ("recv", &["ref", "mut", "value", "box", "pin-mut"]),
            ("ret", &["u64", "self", "assoc", "out", "ref", "future"]),
            ("path", &["attribute", "block", "block-star"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "inherent-cfg",
        summary: "an inherent `#[dispatch(*)]` method over variants under `#[cfg]`, an enum that may be empty included, whose method allows its arguments to go unused",
        vary: &[
            ("cfg", &[]),
            ("s1", &["plain", "none", "deref-box"]),
            ("recv", &["ref", "mut", "value"]),
            ("ret", &["u64", "ref", "self", "future"]),
        ],
        fixed: &[("path", "inherent")],
    },
    Zone {
        name: "cfg-arguments",
        summary: "a `Self` argument beside variants under `#[cfg]`, whose pair keeps its fallback arm in every build, through the enum's attribute, the blocks and the inherent paths",
        vary: &[
            ("cfg", &[]),
            ("arg", &[]),
            ("s1", &["plain", "none"]),
            ("recv", &["ref", "value"]),
            ("ret", &["u64", "self"]),
            ("path", &["attribute", "block-star", "block-some", "inherent", "inherent-some"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "cfg-marks",
        summary: "a mark naming a variant under `#[cfg]`, which takes the method's body where it is compiled out, in a partial block and an inherent block, beside the variants and returns a body meets",
        vary: &[
            ("cfg", &[]),
            ("path", &["block-some", "inherent-some"]),
            ("s1", &["plain", "none", "unit", "pair", "deref-box"]),
            ("recv", &["ref", "mut", "value"]),
            ("ret", &["u64", "unit", "ref", "self", "option-self"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "homes",
        summary: "the trait in a sibling module or another crate, through every path, over the target kinds, returns and qualifiers whose signatures it writes into the subject's module",
        vary: &[
            ("home", &["sibling", "crate"]),
            ("path", &[]),
            ("s0", &["plain", "deref-box", "via-field", "marked-tuple", "struct"]),
            ("ret", &["u64", "self", "assoc", "out", "result-error", "future"]),
            ("qual", &["plain", "provided", "generic"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "generics",
        summary: "a type-parameter payload and a generic trait at a concrete argument, at the enum's own parameter and with a default left out, over the paths, target kinds, receivers and returns that name them",
        vary: &[
            ("generic", &["payload", "concrete", "own", "defaulted", "paired"]),
            ("path", &["attribute", "block", "block-star", "block-some", "block-default"]),
            ("s0", &["plain", "deref-box", "struct"]),
            ("s1", &["plain", "none"]),
            ("recv", &["ref", "value"]),
            ("ret", &["u64", "self", "option-self", "assoc", "future"]),
            ("qual", &["plain", "provided"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "conversions",
        summary: "the `From`, `TryFrom` and `into` impls over the variants that take part and those that do not, with the trait at home and in another crate",
        vary: &[
            ("conv", &[]),
            ("s0", &["plain", "deref-box", "deref-rc", "via-field", "marked-tuple", "struct"]),
            ("s1", &["plain", "none", "unit", "pair", "deref-box", "marked-struct"]),
            ("home", &["local", "crate"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "parameter-conversions",
        summary: "the conversions of an enum whose first payload is its type parameter, which overlap the other variants' `From` and fall foul of the orphan rules for `TryFrom`, and their ways out",
        vary: &[
            ("conv", &["from", "from-off", "try", "variant-from-off"]),
            ("s1", &["plain", "none", "unit", "deref-box", "marked-struct"]),
            ("recv", &["ref", "value", "box"]),
            ("ret", &["u64", "self"]),
        ],
        fixed: &[("generic", "payload")],
    },
    Zone {
        name: "items",
        summary: "the attributes on the trait's method through every path, over the qualifiers, second variants and receivers a block's mark or a carried attribute meets",
        vary: &[
            ("item", &[]),
            ("path", &[]),
            ("qual", &["plain", "provided", "generic"]),
            ("s1", &["plain", "unit", "pair"]),
            ("recv", &["ref", "value", "rc"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "agreements",
        summary: "the trait's own associated items, agreed, disagreed, unchecked and fixed on the enum, over the variants that take part in the checks and those that do not, under `#[cfg]` or not",
        vary: &[
            ("agree", &[]),
            ("s0", &["plain", "deref-box", "deref-ref", "via-field", "marked-tuple", "struct"]),
            ("s1", &["plain", "none", "unit", "deref-rc", "via-field", "marked-struct"]),
            ("cfg", &[]),
        ],
        fixed: &[],
    },
    Zone {
        name: "returned-types",
        summary: "an associated type the payloads give differently, returned by a method every subject forwards, over the variants that take part in the check and those that do not, through the enum's attribute and the blocks, under `#[cfg]` or not",
        vary: &[
            ("agree", &["type-differs", "type-returned"]),
            ("s0", &["plain", "deref-box", "via-field", "marked-tuple", "struct"]),
            ("s1", &["plain", "none", "unit", "via-field", "deref-box"]),
            ("s2", &["none", "plain"]),
            ("path", &["attribute", "block", "block-star"]),
            ("cfg", &["none", "second-off"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "agreement-paths",
        summary: "the trait's own associated items through every path, on generic subjects and traits, with the trait at home, in a sibling module and in another crate",
        vary: &[
            ("agree", &[]),
            ("path", &[]),
            ("generic", &[]),
            ("home", &[]),
        ],
        fixed: &[],
    },
    Zone {
        name: "stacks",
        summary: "a second trait and a supertrait through their own attribute beside every path of the trait under test, over the target kinds that change its receiver, the variants it reaches and the associated items both traits take",
        vary: &[
            ("stack", &["second", "supertrait"]),
            ("path", &[]),
            ("s0", &["plain", "deref-box", "via-method-value", "struct"]),
            ("s1", &["plain", "none", "unit"]),
            ("agree", &["none", "type-differs", "const-override"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "stack-homes",
        summary: "a second trait under the variants' `#[cfg]`, the receivers, the trait's homes and its generics",
        vary: &[
            ("stack", &["second", "supertrait"]),
            ("cfg", &[]),
            ("recv", &["ref", "mut", "value", "box", "pin-ref"]),
            ("home", &[]),
            ("generic", &["none", "concrete", "defaulted"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "deprecations",
        summary: "a deprecated variant through every path, beside the variants, arguments and conversions that name it",
        vary: &[
            ("deprecated", &["first", "second"]),
            ("path", &[]),
            ("s1", &["plain", "unit", "deref-box", "marked-struct"]),
            ("arg", &["none", "ref-trait", "ref-enum"]),
            ("conv", &["none", "from", "try-ref", "into"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "deprecation-cfg",
        summary: "a deprecated variant under the variants' `#[cfg]`, beside a second trait, over the returns that rewrap it",
        vary: &[
            ("deprecated", &["first", "second"]),
            ("stack", &[]),
            ("cfg", &[]),
            ("ret", &["u64", "self", "option-self", "future-self", "assoc"]),
            ("recv", &["ref", "value"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "lent-places",
        summary: "a place read for the subject's own lifetime through variants over `&'a P0`, under every receiver that forwards",
        vary: &[
            ("recv", FORWARDED),
            ("callee", &["value", "field"]),
            ("s1", &["none", "deref-ref"]),
            ("path", &["inherent", "inherent-some"]),
        ],
        fixed: &[("s0", "deref-ref"), ("ret", "lent")],
    },
    Zone {
        name: "routes",
        summary: "the routes of several steps under every receiver and return type",
        vary: &[("s0", ROUTES), ("recv", &[]), ("ret", &[])],
        fixed: &[],
    },
    Zone {
        name: "route-pairs",
        summary: "a route of several steps on the second variant beside a first of one step or several, under the receivers that forward",
        vary: &[
            ("s0", &["plain", "deref-rc", "via-method", "deref-box-field", "via-method-field"]),
            ("s1", ROUTES),
            ("recv", FORWARDED),
        ],
        fixed: &[],
    },
    Zone {
        name: "route-paths",
        summary: "the routes of several steps through every path, with the `Self` arguments that borrow or move through them",
        vary: &[
            ("s0", ROUTES),
            ("path", &[]),
            ("arg", &["none", "ref-trait", "value", "mut"]),
            ("recv", &["ref", "mut", "value"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "trait-shapes",
        summary: "an `unsafe trait`, a trait with a lifetime parameter the enum names and a trait that requires a supertrait dispatched by one more attribute, through every path, over the target kinds, receivers and returns whose impls they head",
        vary: &[
            ("decl", &["dispatchable", "unsafe"]),
            ("generic", &["none", "lifetime"]),
            ("stack", &["none", "second", "required"]),
            ("path", &["attribute", "block", "block-star", "block-some", "block-default", "inherent"]),
            ("s0", &["plain", "deref-box", "struct"]),
            ("ret", &["u64", "self"]),
            ("qual", &["plain", "provided"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "restatements",
        summary: "a trait restated beside the subject, every method listed or the control method alone, through the paths, with its home, its provided method, its `Self` argument and the returns that name `Self` or an associated type",
        vary: &[
            ("decl", &["remote", "remote-base"]),
            ("path", &["attribute", "block", "block-star", "block-some", "block-default", "block-written"]),
            ("home", &[]),
            ("qual", &["plain", "provided"]),
            ("ret", &["u64", "self", "assoc"]),
            ("arg", &["none", "ref-trait"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "restatement-shapes",
        summary: "a restated trait over the target kinds, receivers, generics and second traits whose signatures the restatement spells",
        vary: &[
            ("s0", &["plain", "deref-box", "via-field", "struct"]),
            ("recv", &["ref", "mut", "value", "box", "pin-mut"]),
            ("generic", &["none", "concrete", "own", "defaulted"]),
            ("stack", &[]),
        ],
        fixed: &[("decl", "remote")],
    },
    Zone {
        name: "foreign",
        summary: "`whichever::foreign::Display` over the target kinds and the second variants, through the enum's attribute and the blocks, beside variants under `#[cfg]`",
        vary: &[
            ("s0", &["plain", "deref-box", "deref-rc", "deref-ref", "via-field", "via-index", "via-method", "via-method-value", "marked-tuple", "marked-struct", "struct", "struct-named"]),
            ("s1", &["plain", "none", "unit", "pair"]),
            ("path", &["attribute", "block", "block-star", "block-some", "block-written"]),
            ("cfg", &["none", "second-off", "every-on"]),
        ],
        fixed: &[("decl", "foreign")],
    },
    Zone {
        name: "foreign-conversions",
        summary: "`whichever::foreign::Display` with the conversions, a third variant and a deprecated one",
        vary: &[
            ("conv", &[]),
            ("s1", &["plain", "none", "unit"]),
            ("s2", &["none", "plain"]),
            ("deprecated", &["none", "first"]),
        ],
        fixed: &[("decl", "foreign")],
    },
    Zone {
        name: "restated-enums",
        summary: "an enum of another crate restated beside the subject, over its payloads, boxed payloads and unit variants, through the enum's attribute and the blocks, with the returns that rewrap it",
        vary: &[
            ("s0", &["plain", "deref-box"]),
            ("s1", &["plain", "none", "deref-box", "unit"]),
            ("s2", &["none", "plain", "unit"]),
            ("path", &["attribute", "block", "block-star", "block-some", "block-written"]),
            ("ret", &["u64", "self", "assoc"]),
        ],
        fixed: &[("owner", "remote")],
    },
    Zone {
        name: "restated-enum-items",
        summary: "a restated enum with the receivers, the `Self` arguments, the conversion options it refuses, the associated items and a second trait",
        vary: &[
            ("recv", &["ref", "mut", "value", "box", "pin-mut"]),
            ("arg", &["none", "ref-trait", "value"]),
            ("conv", &["none", "from-off", "try", "into"]),
            ("agree", &["none", "type-differs", "const-differs"]),
            ("stack", &["none", "second"]),
        ],
        fixed: &[("owner", "remote"), ("s1", "deref-box")],
    },
    Zone {
        name: "thirds",
        summary: "a third variant beside a second, over the target kinds of the first, the receivers that hand each arm over and the returns that rewrap it",
        vary: &[
            ("s2", LATER),
            ("s1", LATER),
            ("s0", &["plain", "deref-box", "via-field", "marked-tuple"]),
            ("recv", &["ref", "mut", "value", "box"]),
            ("ret", &["u64", "self", "option-self", "ref"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "third-paths",
        summary: "a third variant through every path, with every `Self` argument, so that a mismatch meets three variants and a body meets two forwarded ones",
        vary: &[
            ("s2", LATER),
            ("path", &[]),
            ("arg", &["none", "ref-trait", "ref-enum", "value", "mut"]),
            ("s1", &["plain", "unit"]),
            ("recv", &["ref", "value"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "third-items",
        summary: "a third variant under the associated items checked across its payload, the variants' `#[cfg]`, the conversions and a second trait",
        vary: &[
            ("s2", LATER),
            ("s1", &["plain", "unit", "deref-box"]),
            ("agree", &[]),
            ("cfg", &[]),
        ],
        fixed: &[],
    },
    Zone {
        name: "third-conversions",
        summary: "the conversions over three variants, with the trait at home and in another crate",
        vary: &[
            ("s2", LATER),
            ("s1", &["plain", "unit", "deref-box"]),
            ("conv", &[]),
            ("home", &["local", "crate"]),
        ],
        fixed: &[],
    },
    Zone {
        name: "trait-objects",
        summary: "a trait object in a `deref` variant beside a sized payload, under a method that requires `Self: Sized` or not, through the enum's attribute and the blocks, a block's `#[dispatch(default)]` included, under the receivers, qualifiers and returns a dyn-compatible trait takes",
        vary: &[
            ("item", &["none", "sized"]),
            ("recv", &["ref", "mut", "value", "box", "pin-ref", "rc"]),
            ("path", &["attribute", "block", "block-star", "block-some", "block-default", "block-written"]),
            ("qual", &["plain", "unsafe", "provided"]),
            ("ret", &["u64", "ref", "boxed-future"]),
        ],
        fixed: &[("s1", "deref-dyn")],
    },
    Zone {
        name: "trait-object-builds",
        summary: "a trait object beside variants under `#[cfg]` and a third variant, under which the sentence of a method that requires `Self: Sized` is the trait object's arm",
        vary: &[
            ("item", &["none", "sized"]),
            ("cfg", &[]),
            ("path", &["attribute", "block-star", "block-some", "block-default"]),
            ("qual", &["plain", "provided"]),
            ("s2", &["none", "plain"]),
        ],
        fixed: &[("s1", "deref-dyn")],
    },
    Zone {
        name: "third-stacks",
        summary: "a second trait and a supertrait over three variants, beside the block paths of the trait under test and the associated items both take",
        vary: &[
            ("s2", LATER),
            ("s1", &["plain", "unit"]),
            ("stack", &["second", "supertrait"]),
            ("path", &["attribute", "block", "block-star", "block-some"]),
            ("agree", &["none", "type-differs", "const-differs"]),
        ],
        fixed: &[],
    },
];
