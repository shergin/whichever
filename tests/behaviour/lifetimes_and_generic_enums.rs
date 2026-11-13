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
trait Held<'a> {
    fn held(&'a self) -> &'a str;
    fn held_mut(&'a mut self) -> &'a mut String;
}

struct Front<'a>(&'a str, String);
struct Back<'a>(&'a str, String);

impl<'a> Held<'a> for Front<'a> {
    fn held(&'a self) -> &'a str {
        self.0
    }
    fn held_mut(&'a mut self) -> &'a mut String {
        &mut self.1
    }
}

impl<'a> Held<'a> for Back<'a> {
    fn held(&'a self) -> &'a str {
        &self.1
    }
    fn held_mut(&'a mut self) -> &'a mut String {
        &mut self.1
    }
}

#[whichever::dispatch(Held<'b>)]
enum Either<'b> {
    Front(Front<'b>),
    Back(Back<'b>),
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

#[test]
fn a_trait_lifetime_on_the_receiver_takes_the_enums() {
    let text = String::from("front");
    let front: Either<'_> = Front(&text, String::new()).into();
    assert_eq!(front.held(), "front");
    let back: Either<'_> = Back(&text, String::from("back")).into();
    assert_eq!(back.held(), "back");
    // `&'a mut self` holds the enum for all of `'a`, so the result is
    // checked through the reference it returns.
    let mut back = back;
    let tail = back.held_mut();
    tail.push('!');
    assert_eq!(tail, "back!");
    let mut front: Either<'_> = Front(&text, String::from("x")).into();
    assert_eq!(front.held_mut(), "x");
}
