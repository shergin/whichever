//! Variants in `enum_dispatch`'s short form, which names the payload type
//! as the variant, hold nothing to forward to; each says so and what to
//! write instead, and nothing follows from the impl that is not written.
#[whichever::dispatchable]
trait Page {
    fn lines(&self) -> usize;
}

struct LoadedFile;
struct ControlledFile;

impl Page for LoadedFile {
    fn lines(&self) -> usize {
        1
    }
}

impl Page for ControlledFile {
    fn lines(&self) -> usize {
        2
    }
}

#[whichever::dispatch(Page)]
enum File {
    LoadedFile,
    ControlledFile,
}

fn main() {}
