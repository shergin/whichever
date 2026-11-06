#[whichever::dispatchable]
trait Borrowed<'a> {
    fn get(&self) -> &'a str;
}

struct Head<'a>(&'a str);
struct Tail<'a>(&'a str);

impl<'a> Borrowed<'a> for Head<'a> {
    fn get(&self) -> &'a str {
        &self.0[..1]
    }
}

impl<'a> Borrowed<'a> for Tail<'a> {
    fn get(&self) -> &'a str {
        &self.0[1..]
    }
}

#[whichever::dispatch(Borrowed<'a>)]
enum Part<'a> {
    Head(Head<'a>),
    Tail(Tail<'a>),
}

#[whichever::dispatchable]
trait Len {
    fn len(&self) -> usize;
}

impl<T> Len for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
}

impl<T> Len for Option<T> {
    fn len(&self) -> usize {
        usize::from(self.is_some())
    }
}

#[whichever::dispatch(Len)]
enum Container<T>
where
    T: Clone,
{
    Many(Vec<T>),
    One(Option<T>),
}

#[test]
fn generics_of_the_enum_carry_into_the_impl() {
    let text = String::from("abc");
    let head: Part<'_> = Head(&text).into();
    let tail: Part<'_> = Tail(&text).into();
    assert_eq!((head.get(), tail.get()), ("a", "bc"));
    let many: Container<u8> = vec![1, 2, 3].into();
    let one: Container<u8> = Some(1).into();
    assert_eq!((many.len(), one.len()), (3, 1));
}
