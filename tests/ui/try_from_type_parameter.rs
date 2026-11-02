//! `TryFrom<Enum<T>> for T` would implement a foreign trait for a type
//! parameter, which the orphan rules forbid; the message names the switch.
#[whichever::dispatchable]
trait Tag {
    fn tag(&self) -> u8;
}

struct Fixed;

impl Tag for Fixed {
    fn tag(&self) -> u8 {
        1
    }
}

#[whichever::dispatch(Tag, try_from = true)]
enum Kind<T: Tag> {
    Any(T),
}

fn main() {}
