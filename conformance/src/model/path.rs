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
}

impl Path {
    /// Returns the path's description.
    pub fn descriptor(self) -> PathDescriptor {
        let with = |site, form, mark, in_block| PathDescriptor {
            site,
            form,
            mark,
            in_block,
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
        }
    }
}
