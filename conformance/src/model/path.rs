//! What a path is: where the trait's impl for the subject is written, and
//! how the method under test reaches the variants.

use super::axes::Path;

/// Where the forwarding is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Site {
    /// The trait named on the subject's `#[whichever::dispatch(..)]`.
    Attribute,
    /// A `#[whichever::dispatch] impl Probe for E` block, the subject
    /// carrying a bare `#[whichever::dispatch]`.
    Block,
    /// An inherent `#[whichever::dispatch] impl E` block, beside the trait
    /// named on the subject for the control method.
    Inherent,
}

/// What the method under test is on the subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// Forwarded to every variant.
    Forwarded,
    /// Forwarded to the first variant, with a body for the others.
    First,
    /// Left to the trait's default with `#[dispatch(default)]`.
    Default,
    /// Written by the block itself.
    Written,
    /// Forwarded to every variant but those the body's own arms answer,
    /// `#[dispatch(*)]` with a body that is one `match self`.
    Arms,
}

/// A path's description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathDescriptor {
    /// Where the forwarding is asked for.
    pub site: Site,
    /// What the method under test is.
    pub form: Form,
    /// The `#[dispatch(..)]` on the method in the block: `*`, `A` or
    /// `default`; `None` when the method carries none or is not in it.
    pub mark: Option<&'static str>,
    /// Whether the block holds the method under test.
    pub in_block: bool,
    /// Whether the block holds a macro invocation, which the block's
    /// reader cannot see into and reports (limits.md).
    pub invocation: bool,
    /// The predicate of the `#[cfg]` the invocation is under, if any. "A
    /// macro invocation under `#[cfg]` is reported only where the `cfg`
    /// holds; where it does not, the invocation is not in the block, the
    /// block is read without it" (limits.md).
    pub invocation_cfg: Option<&'static str>,
}

/// The mark of a method that names two fields of a struct, the held field
/// first and the label second.
pub const TWO_FIELDS: &str = "held, label";

impl PathDescriptor {
    /// Returns whether the block holds a macro invocation in this build,
    /// which it is then not read for.
    pub fn reads_invocation(&self) -> bool {
        self.invocation && self.invocation_cfg != Some("any()")
    }
}

impl Path {
    /// Returns the path's description.
    pub fn descriptor(self) -> PathDescriptor {
        let with = |site, form, mark, in_block| PathDescriptor {
            site,
            form,
            mark,
            in_block,
            invocation: false,
            invocation_cfg: None,
        };
        match self {
            Path::Attribute => with(Site::Attribute, Form::Forwarded, None, false),
            Path::Block => with(Site::Block, Form::Forwarded, None, false),
            Path::BlockStar => with(Site::Block, Form::Forwarded, Some("*"), true),
            Path::BlockSome => with(Site::Block, Form::First, Some("A"), true),
            Path::BlockDefault => with(Site::Block, Form::Default, Some("default"), true),
            Path::BlockWritten => with(Site::Block, Form::Written, None, true),
            Path::Inherent => with(Site::Inherent, Form::Forwarded, Some("*"), true),
            Path::InherentSome => with(Site::Inherent, Form::First, Some("A"), true),
            Path::BlockArms => with(Site::Block, Form::Arms, Some("*"), true),
            Path::InherentArms => with(Site::Inherent, Form::Arms, Some("*"), true),
            Path::InherentFields => with(Site::Inherent, Form::Forwarded, Some(TWO_FIELDS), true),
            Path::BlockFields => with(Site::Block, Form::Forwarded, Some(TWO_FIELDS), true),
            Path::BlockMacro => PathDescriptor {
                invocation: true,
                ..with(Site::Block, Form::Forwarded, None, false)
            },
            Path::BlockMacroOn => PathDescriptor {
                invocation: true,
                invocation_cfg: Some("all()"),
                ..with(Site::Block, Form::Forwarded, None, false)
            },
            Path::BlockMacroOff => PathDescriptor {
                invocation: true,
                invocation_cfg: Some("any()"),
                ..with(Site::Block, Form::Forwarded, None, false)
            },
        }
    }
}
